//! A cache of parses, for measurement runs that parse the same sentences
//! again and again while only the checks after parsing change.
//!
//! Each parse is a file named by a hash of everything its result depends
//! on: the grammar's files, the configuration, the parser settings, the
//! source code of the parsing pipeline (see `build.rs`) and the sentence.
//! Editing the grammar or the parser therefore never returns a stale
//! parse. A parse cut short by the time limit is cached as it came out,
//! which also makes reruns repeatable. A missing or unreadable entry
//! simply means parsing.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use emdysi_hpsg::labels::Tree;
use emdysi_hpsg::mrs::{Ep, Mrs};
use emdysi_hpsg::parser::ParserConfig;

use crate::{InputToken, Node, Parse, Reading, Tag, Word};

/// Bump when the layout below changes.
const VERSION: u32 = 3;
const MAGIC: &[u8; 8] = b"EMDYSIPC";

/// Two-lane FNV-1a hash, as a hex string.
struct Hasher(u64, u64);

impl Hasher {
    fn new() -> Hasher {
        Hasher(0xcbf2_9ce4_8422_2325, 0x6c62_272e_07bb_0142)
    }
    fn feed(&mut self, bytes: &[u8]) {
        for &x in bytes {
            self.0 = (self.0 ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3);
            self.1 = (self.1 ^ x as u64)
                .wrapping_mul(0x0000_0100_0000_01b3)
                .rotate_left(7);
        }
        // A separator, so that ("ab", "c") and ("a", "bc") differ.
        self.0 = self.0.wrapping_mul(31).wrapping_add(bytes.len() as u64);
    }
    fn hex(&self) -> String {
        format!("{:016x}{:016x}", self.0, self.1)
    }
}

/// A hash of every file under the grammar directories, and of the
/// configuration file's name.
pub(crate) fn grammar_key(dirs: &[PathBuf], config: &str) -> String {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else {
                out.push(p);
            }
        }
    }
    let mut h = Hasher::new();
    h.feed(&VERSION.to_le_bytes());
    h.feed(config.as_bytes());
    for d in dirs {
        let mut files = Vec::new();
        walk(d, &mut files);
        files.sort();
        for f in files {
            h.feed(
                f.strip_prefix(d)
                    .unwrap_or(&f)
                    .as_os_str()
                    .as_encoded_bytes(),
            );
            h.feed(&std::fs::read(&f).unwrap_or_default());
        }
    }
    h.hex()
}

/// The key of one parse.
pub(crate) fn key(
    grammar: &str,
    config: &ParserConfig,
    first_beam: Option<usize>,
    trees_for: usize,
    text: &str,
) -> String {
    let mut h = Hasher::new();
    h.feed(env!("EMDYSI_PARSER_SOURCE").as_bytes());
    h.feed(grammar.as_bytes());
    let roots: Vec<&str> = config.roots.iter().map(|(n, _)| n.as_str()).collect();
    let settings = format!(
        "{:?} {} {} {} {:?} {} {:?} {:?} {} {} {} {} {:?} {}",
        roots,
        config.max_edges,
        config.max_nodes,
        config.timeout.as_millis(),
        config.packing_restrictor.as_ref().map(Vec::len),
        config.max_readings,
        config.packing_top_type,
        config.cell_beam,
        config.cell_beam_from,
        config.preferred_roots,
        config.unpack_beam,
        config.fragments,
        first_beam,
        trees_for,
    );
    h.feed(settings.as_bytes());
    h.feed(text.as_bytes());
    h.hex()
}

fn path(dir: &Path, key: &str) -> PathBuf {
    dir.join(&key[..2]).join(format!("{key}.bin"))
}

pub(crate) fn load(dir: &Path, key: &str) -> Option<Parse> {
    let bytes = std::fs::read(path(dir, key)).ok()?;
    let mut r = R {
        b: &bytes,
        i: 0,
        strings: Vec::new(),
    };
    if r.take(8)? != MAGIC || r.take(4)? != VERSION.to_le_bytes() {
        return None;
    }
    let n = r.len()?;
    for _ in 0..n {
        let len = r.u32()? as usize;
        let s = String::from_utf8(r.take(len)?.to_vec()).ok()?;
        r.strings.push(s);
    }
    let p = r.parse()?;
    (r.i == bytes.len()).then_some(p)
}

pub(crate) fn store(dir: &Path, key: &str, p: &Parse) {
    let mut w = W {
        body: Vec::new(),
        index: HashMap::new(),
        strings: Vec::new(),
    };
    w.parse(p);
    // The strings, each once (rule names, entries and ranking features
    // recur in every reading), then the parse, which refers to them by
    // number.
    let mut out = Vec::with_capacity(w.body.len());
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    put_var(&mut out, w.strings.len() as u64);
    for s in &w.strings {
        put_var(&mut out, s.len() as u64);
        out.extend_from_slice(s.as_bytes());
    }
    out.extend_from_slice(&w.body);
    let file = path(dir, key);
    if let Some(d) = file.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    // Write to a temporary name and rename, so that concurrent readers
    // never see a partial file.
    let tmp = file.with_extension(format!(
        "tmp{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    if std::fs::write(&tmp, &out).is_ok() {
        let _ = std::fs::rename(&tmp, &file);
    }
}

struct W {
    body: Vec<u8>,
    index: HashMap<String, u32>,
    strings: Vec<String>,
}

/// Append `x` as a variable-length integer (LEB128): most numbers in a
/// parse (spans, indices, lengths) take one byte.
fn put_var(out: &mut Vec<u8>, mut x: u64) {
    while x >= 0x80 {
        out.push((x as u8) | 0x80);
        x >>= 7;
    }
    out.push(x as u8);
}

impl W {
    fn u32(&mut self, x: u32) {
        put_var(&mut self.body, x.into());
    }
    fn u64(&mut self, x: u64) {
        put_var(&mut self.body, x);
    }
    fn usize(&mut self, x: usize) {
        self.u64(x as u64);
    }
    fn f64(&mut self, x: f64) {
        self.body.extend_from_slice(&x.to_bits().to_le_bytes());
    }
    fn bool(&mut self, x: bool) {
        self.body.push(u8::from(x));
    }
    fn str(&mut self, s: &str) {
        let i = match self.index.get(s) {
            Some(&i) => i,
            None => {
                let i = self.strings.len() as u32;
                self.index.insert(s.to_string(), i);
                self.strings.push(s.to_string());
                i
            }
        };
        self.u32(i);
    }
    fn strs(&mut self, v: &[String]) {
        self.u32(v.len() as u32);
        for s in v {
            self.str(s);
        }
    }
    fn opt_str(&mut self, s: Option<&str>) {
        self.bool(s.is_some());
        if let Some(s) = s {
            self.str(s);
        }
    }
    fn triples(&mut self, v: &[(String, String, String)]) {
        self.u32(v.len() as u32);
        for (a, b, c) in v {
            self.str(a);
            self.str(b);
            self.str(c);
        }
    }
    fn pairs(&mut self, v: &[(String, String)]) {
        self.u32(v.len() as u32);
        for (a, b) in v {
            self.str(a);
            self.str(b);
        }
    }
    fn tree(&mut self, t: &Tree) {
        match t {
            Tree::Node {
                label,
                start,
                end,
                kids,
            } => {
                self.bool(true);
                self.str(label);
                self.usize(*start);
                self.usize(*end);
                self.u32(kids.len() as u32);
                for k in kids {
                    self.tree(k);
                }
            }
            Tree::Leaf { form, start, end } => {
                self.bool(false);
                self.str(form);
                self.usize(*start);
                self.usize(*end);
            }
        }
    }
    fn mrs(&mut self, m: &Mrs) {
        self.str(&m.top);
        self.opt_str(m.index.as_deref());
        self.u32(m.eps.len() as u32);
        for ep in &m.eps {
            self.str(&ep.pred);
            self.bool(ep.lnk.is_some());
            if let Some((a, b)) = ep.lnk {
                self.usize(a);
                self.usize(b);
            }
            self.str(&ep.label);
            self.pairs(&ep.args);
            self.opt_str(ep.carg.as_deref());
        }
        self.triples(&m.hcons);
        self.triples(&m.icons);
        let mut props: Vec<_> = m.props.iter().collect();
        props.sort();
        self.u32(props.len() as u32);
        for (var, ps) in props {
            self.str(var);
            self.pairs(ps);
        }
    }
    fn reading(&mut self, r: &Reading) {
        self.str(&r.root);
        self.str(&r.derivation);
        self.bool(r.tree.is_some());
        if let Some(t) = &r.tree {
            self.tree(t);
        }
        self.strs(&r.features);
        self.f64(r.score);
        self.u32(r.nodes.len() as u32);
        for n in &r.nodes {
            self.str(&n.name);
            self.bool(n.leaf);
            self.usize(n.from);
            self.usize(n.to);
            self.bool(n.parent.is_some());
            self.usize(n.parent.unwrap_or(0));
            self.u32(n.children.len() as u32);
            for &c in &n.children {
                self.usize(c);
            }
        }
        self.u32(r.words.len() as u32);
        for w in &r.words {
            self.str(&w.surface);
            self.str(&w.entry);
            self.str(&w.lemma);
            self.str(&w.le_type);
            self.bool(w.generic);
            self.strs(&w.rules);
            self.usize(w.from);
            self.usize(w.to);
            self.usize(w.node);
        }
        self.bool(r.mrs.is_some());
        if let Some(m) = &r.mrs {
            self.mrs(m);
        }
    }
    fn parse(&mut self, p: &Parse) {
        self.u32(p.tokens.len() as u32);
        for t in &p.tokens {
            self.str(&t.form);
            self.usize(t.from);
            self.usize(t.to);
            self.u32(t.tags.len() as u32);
            for g in &t.tags {
                self.str(&g.tag);
                self.f64(g.prob);
            }
        }
        self.u32(p.readings.len() as u32);
        for r in &p.readings {
            self.reading(r);
        }
        self.usize(p.edges);
        self.usize(p.lexical_items);
        self.bool(p.exhausted);
        self.bool(p.complete);
        self.u64(p.elapsed.as_nanos() as u64);
        self.f64(p.temperature);
    }
}

struct R<'a> {
    b: &'a [u8],
    i: usize,
    strings: Vec<String>,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.b.get(self.i..self.i.checked_add(n)?)?;
        self.i += n;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        u32::try_from(self.u64()?).ok()
    }
    fn u64(&mut self) -> Option<u64> {
        let mut x: u64 = 0;
        for shift in (0..64).step_by(7) {
            let b = self.take(1)?[0];
            x |= u64::from(b & 0x7f) << shift;
            if b < 0x80 {
                return Some(x);
            }
        }
        None
    }
    fn usize(&mut self) -> Option<usize> {
        usize::try_from(self.u64()?).ok()
    }
    fn f64(&mut self) -> Option<f64> {
        Some(f64::from_bits(u64::from_le_bytes(
            self.take(8)?.try_into().ok()?,
        )))
    }
    fn bool(&mut self) -> Option<bool> {
        match self.take(1)?[0] {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        }
    }
    fn len(&mut self) -> Option<usize> {
        let n = self.u32()? as usize;
        // Every element takes at least a byte: a corrupt length cannot
        // make us allocate more than the file holds.
        (n <= self.b.len() - self.i).then_some(n)
    }
    fn str(&mut self) -> Option<String> {
        let i = self.u32()? as usize;
        self.strings.get(i).cloned()
    }
    fn strs(&mut self) -> Option<Vec<String>> {
        (0..self.len()?).map(|_| self.str()).collect()
    }
    fn opt_str(&mut self) -> Option<Option<String>> {
        Some(if self.bool()? {
            Some(self.str()?)
        } else {
            None
        })
    }
    fn triples(&mut self) -> Option<Vec<(String, String, String)>> {
        (0..self.len()?)
            .map(|_| Some((self.str()?, self.str()?, self.str()?)))
            .collect()
    }
    fn pairs(&mut self) -> Option<Vec<(String, String)>> {
        (0..self.len()?)
            .map(|_| Some((self.str()?, self.str()?)))
            .collect()
    }
    fn tree(&mut self) -> Option<Tree> {
        Some(if self.bool()? {
            let label = self.str()?;
            let start = self.usize()?;
            let end = self.usize()?;
            let kids = (0..self.len()?)
                .map(|_| self.tree())
                .collect::<Option<_>>()?;
            Tree::Node {
                label,
                start,
                end,
                kids,
            }
        } else {
            Tree::Leaf {
                form: self.str()?,
                start: self.usize()?,
                end: self.usize()?,
            }
        })
    }
    fn mrs(&mut self) -> Option<Mrs> {
        let top = self.str()?;
        let index = self.opt_str()?;
        let eps = (0..self.len()?)
            .map(|_| {
                let pred = self.str()?;
                let lnk = if self.bool()? {
                    Some((self.usize()?, self.usize()?))
                } else {
                    None
                };
                Some(Ep {
                    pred,
                    lnk,
                    label: self.str()?,
                    args: self.pairs()?,
                    carg: self.opt_str()?,
                })
            })
            .collect::<Option<_>>()?;
        let hcons = self.triples()?;
        let icons = self.triples()?;
        let props: HashMap<String, Vec<(String, String)>> = (0..self.len()?)
            .map(|_| Some((self.str()?, self.pairs()?)))
            .collect::<Option<_>>()?;
        Some(Mrs {
            top,
            index,
            eps,
            hcons,
            icons,
            props,
        })
    }
    fn reading(&mut self) -> Option<Reading> {
        let root = self.str()?;
        let derivation = self.str()?;
        let tree = if self.bool()? {
            Some(self.tree()?)
        } else {
            None
        };
        let features = self.strs()?;
        let score = self.f64()?;
        let nodes = (0..self.len()?)
            .map(|_| {
                let name = self.str()?;
                let leaf = self.bool()?;
                let from = self.usize()?;
                let to = self.usize()?;
                let has_parent = self.bool()?;
                let parent = self.usize()?;
                let children = (0..self.len()?)
                    .map(|_| self.usize())
                    .collect::<Option<_>>()?;
                Some(Node {
                    name,
                    leaf,
                    from,
                    to,
                    parent: has_parent.then_some(parent),
                    children,
                })
            })
            .collect::<Option<_>>()?;
        let words = (0..self.len()?)
            .map(|_| {
                Some(Word {
                    surface: self.str()?,
                    entry: self.str()?,
                    lemma: self.str()?,
                    le_type: self.str()?,
                    generic: self.bool()?,
                    rules: self.strs()?,
                    from: self.usize()?,
                    to: self.usize()?,
                    node: self.usize()?,
                })
            })
            .collect::<Option<_>>()?;
        let mrs = if self.bool()? {
            Some(self.mrs()?)
        } else {
            None
        };
        Some(Reading {
            root,
            derivation,
            tree,
            features,
            score,
            nodes,
            words,
            dag: None,
            mrs,
        })
    }
    fn parse(&mut self) -> Option<Parse> {
        let tokens = (0..self.len()?)
            .map(|_| {
                Some(InputToken {
                    form: self.str()?,
                    from: self.usize()?,
                    to: self.usize()?,
                    tags: (0..self.len()?)
                        .map(|_| {
                            Some(Tag {
                                tag: self.str()?,
                                prob: self.f64()?,
                            })
                        })
                        .collect::<Option<_>>()?,
                })
            })
            .collect::<Option<_>>()?;
        let readings = (0..self.len()?)
            .map(|_| self.reading())
            .collect::<Option<_>>()?;
        Some(Parse {
            tokens,
            readings,
            edges: self.usize()?,
            lexical_items: self.usize()?,
            exhausted: self.bool()?,
            complete: self.bool()?,
            elapsed: Duration::from_nanos(self.u64()?),
            temperature: self.f64()?,
        })
    }
}
