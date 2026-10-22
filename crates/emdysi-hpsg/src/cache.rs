//! A cache of grammar compilation: the closed type hierarchy, the expanded
//! type constraints, and the instances (lexical entries, rules) and letter
//! sets as read, so that a grammar unchanged since the last run is loaded
//! without reading its TDL at all (see [`Grammar::from_cache`]).
//!
//! The cache lives outside the source tree (by default in the user's cache
//! directory, see [`default_dir`]) and is keyed by a hash of every TDL file
//! the grammar was read from, so editing the grammar invalidates it. A
//! missing, stale or unreadable cache simply means compiling from scratch.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use crate::bitset::BitSet;
use crate::dag::{Dag, Node};
use crate::desc::ListConfig;
use crate::grammar::{Grammar, GrammarError, Instance};
use crate::types::Hierarchy;
use crate::typesys::{Features, LiteralKind, Literals, TypeSystem};
use emdysi_tdl::{Affix, Conj, FeatVal, LetterSet, PatElem, Term};

/// Bump when the serialized layout or the compilation algorithm changes.
const VERSION: u32 = 3;
const MAGIC: &[u8; 8] = b"EMDYSIGC";

/// `$EMDYSI_CACHE_DIR`, else `$XDG_CACHE_HOME/emdysi`, else
/// `~/.cache/emdysi`, else (on Windows) `%LOCALAPPDATA%\emdysi`.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("EMDYSI_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(d).join("emdysi"));
    }
    std::env::var_os("HOME")
        .map(|h| PathBuf::from(h).join(".cache/emdysi"))
        .or_else(|| std::env::var_os("LOCALAPPDATA").map(|d| PathBuf::from(d).join("emdysi")))
}

/// 128-bit FNV-1a style hash of the grammar sources (two independent 64-bit
/// lanes), as a hex string.
fn source_key(files: &[PathBuf]) -> Option<String> {
    let mut a: u64 = 0xcbf2_9ce4_8422_2325;
    let mut b: u64 = 0x6c62_272e_07bb_0142;
    let mut feed = |bytes: &[u8]| {
        for &x in bytes {
            a = (a ^ x as u64).wrapping_mul(0x0000_0100_0000_01b3);
            b = (b ^ x as u64)
                .wrapping_mul(0x0000_0100_0000_01b3)
                .rotate_left(7);
        }
    };
    feed(&VERSION.to_le_bytes());
    for f in files {
        feed(f.file_name()?.as_encoded_bytes());
        feed(&std::fs::read(f).ok()?);
    }
    Some(format!("{a:016x}{b:016x}"))
}

impl Grammar {
    /// Like [`Grammar::compile`], but reuse a cached compilation from `dir`
    /// when the sources are unchanged, and write one otherwise. Cache
    /// problems are never fatal.
    pub fn compile_cached(
        loaded: &emdysi_tdl::Loaded,
        dir: Option<&Path>,
    ) -> Result<Grammar, GrammarError> {
        let path = dir
            .zip(source_key(&loaded.files))
            .map(|(d, k)| d.join(format!("grammar-{k}.bin")));
        if let Some(p) = &path {
            if let Ok(bytes) = std::fs::read(p) {
                if let Some(g) = read(&bytes) {
                    touch(p);
                    return Ok(g);
                }
            }
        }
        let g = Grammar::compile(loaded)?;
        if let Some(p) = &path {
            let bytes = write(&g);
            if let Some(d) = p.parent() {
                let _ = std::fs::create_dir_all(d);
            }
            // Write to a temporary name and rename, so concurrent readers
            // never see a partial file.
            let tmp = p.with_extension(format!("tmp{}", std::process::id()));
            // A failed write (a full disk) leaves no partial file behind.
            if std::fs::write(&tmp, bytes).is_err() || std::fs::rename(&tmp, p).is_err() {
                let _ = std::fs::remove_file(&tmp);
            }
            if let Some(d) = dir {
                prune(d);
            }
        }
        Ok(g)
    }

    /// The grammar compiled before from the same sources, read back
    /// without reading its TDL: the files it was read from are listed
    /// under `manifest` (a name for one grammar and configuration, see
    /// [`Grammar::record_sources`]) in `dir`, and when every one hashes as
    /// it did, the cached compilation is complete. `None` when anything
    /// differs or is missing, and the sources must be read.
    pub fn from_cache(dir: &Path, manifest: &str) -> Option<Grammar> {
        let list = std::fs::read_to_string(dir.join(format!("sources-{manifest}.txt"))).ok()?;
        let files: Vec<PathBuf> = list.lines().map(PathBuf::from).collect();
        let key = source_key(&files)?;
        let path = dir.join(format!("grammar-{key}.bin"));
        let g = read(&std::fs::read(&path).ok()?)?;
        touch(&path);
        Some(g)
    }

    /// Record under `manifest` in `dir` the files a grammar was read from,
    /// for [`Grammar::from_cache`].
    pub fn record_sources(dir: &Path, manifest: &str, files: &[PathBuf]) {
        let list: String = files
            .iter()
            .filter_map(|f| Some(format!("{}\n", f.to_str()?)))
            .collect();
        let path = dir.join(format!("sources-{manifest}.txt"));
        if std::fs::read_to_string(&path).ok().as_deref() == Some(&list) {
            return;
        }
        let tmp = path.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, list).is_err() || std::fs::rename(&tmp, &path).is_err() {
            let _ = std::fs::remove_file(&tmp);
        }
    }
}

/// Compiled grammars kept: a grammar and its grammar-error variant, and
/// the two before them. Each takes 80 to 90 MB, and a new one is written
/// whenever the grammar or this format changes.
const KEEP: usize = 4;

/// Mark a compiled grammar as used, for [`prune`].
fn touch(path: &Path) {
    if let Ok(f) = std::fs::File::options().append(true).open(path) {
        let _ = f.set_modified(std::time::SystemTime::now());
    }
}

/// Delete all but the [`KEEP`] most recently used compiled grammars in `dir`.
fn prune(dir: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut bins: Vec<(std::time::SystemTime, PathBuf)> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("grammar-") && n.ends_with(".bin"))
        })
        .filter_map(|p| Some((std::fs::metadata(&p).ok()?.modified().ok()?, p)))
        .collect();
    bins.sort_by(|a, b| b.0.cmp(&a.0));
    for (_, p) in bins.into_iter().skip(KEEP) {
        let _ = std::fs::remove_file(p);
    }
}

struct W(Vec<u8>);

impl W {
    fn u32(&mut self, x: u32) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn u64(&mut self, x: u64) {
        self.0.extend_from_slice(&x.to_le_bytes());
    }
    fn str(&mut self, s: &str) {
        self.u32(s.len() as u32);
        self.0.extend_from_slice(s.as_bytes());
    }
    fn u32s(&mut self, v: &[u32]) {
        self.u32(v.len() as u32);
        for &x in v {
            self.u32(x);
        }
    }
    fn opt_str(&mut self, s: &Option<String>) {
        match s {
            None => self.u32(0),
            Some(s) => {
                self.u32(1);
                self.str(s);
            }
        }
    }
    fn strs(&mut self, v: &[String]) {
        self.u32(v.len() as u32);
        for s in v {
            self.str(s);
        }
    }
    fn chars(&mut self, v: &[char]) {
        self.u32(v.len() as u32);
        for &c in v {
            self.u32(c as u32);
        }
    }
    fn conj(&mut self, c: &Conj) {
        self.u32(c.0.len() as u32);
        for t in &c.0 {
            self.term(t);
        }
    }
    fn term(&mut self, t: &Term) {
        match t {
            Term::Type(s) => {
                self.u32(0);
                self.str(s);
            }
            Term::Str(s) => {
                self.u32(1);
                self.str(s);
            }
            Term::Regex(s) => {
                self.u32(2);
                self.str(s);
            }
            Term::Coref(s) => {
                self.u32(3);
                self.str(s);
            }
            Term::Avm(fvs) => {
                self.u32(4);
                self.u32(fvs.len() as u32);
                for fv in fvs {
                    self.strs(&fv.path);
                    self.conj(&fv.value);
                }
            }
            Term::List { items, open, tail } => {
                self.u32(5);
                self.u32(items.len() as u32);
                for c in items {
                    self.conj(c);
                }
                self.u32(*open as u32);
                match tail {
                    None => self.u32(0),
                    Some(c) => {
                        self.u32(1);
                        self.conj(c);
                    }
                }
            }
            Term::DiffList(items) => {
                self.u32(6);
                self.u32(items.len() as u32);
                for c in items {
                    self.conj(c);
                }
            }
        }
    }
    fn pattern(&mut self, p: &[PatElem]) {
        self.u32(p.len() as u32);
        for e in p {
            match e {
                PatElem::Char(c) => {
                    self.u32(0);
                    self.u32(*c as u32);
                }
                PatElem::Var(c) => {
                    self.u32(1);
                    self.u32(*c as u32);
                }
            }
        }
    }
    fn bits(&mut self, b: &BitSet) {
        self.u32(b.words.len() as u32);
        for &w in b.words.iter() {
            self.u64(w);
        }
    }
    fn dag(&mut self, d: &Dag) {
        self.u32(d.nodes.len() as u32);
        for n in &d.nodes {
            self.u32(n.ty);
            self.u32(n.arc_start);
            self.u32(n.arc_len);
        }
        self.u32(d.arcs.len() as u32);
        for &(f, v) in &d.arcs {
            self.u32(f);
            self.u32(v);
        }
    }
}

struct R<'a> {
    b: &'a [u8],
    i: usize,
}

impl R<'_> {
    fn take(&mut self, n: usize) -> Option<&[u8]> {
        let s = self.b.get(self.i..self.i.checked_add(n)?)?;
        self.i += n;
        Some(s)
    }
    fn u32(&mut self) -> Option<u32> {
        Some(u32::from_le_bytes(self.take(4)?.try_into().ok()?))
    }
    fn u64(&mut self) -> Option<u64> {
        Some(u64::from_le_bytes(self.take(8)?.try_into().ok()?))
    }
    fn len(&mut self) -> Option<usize> {
        let n = self.u32()? as usize;
        // Guard against corrupt lengths before allocating.
        (n <= self.b.len()).then_some(n)
    }
    fn str(&mut self) -> Option<String> {
        let n = self.len()?;
        String::from_utf8(self.take(n)?.to_vec()).ok()
    }
    fn u32s(&mut self) -> Option<Vec<u32>> {
        let n = self.len()?;
        (0..n).map(|_| self.u32()).collect()
    }
    fn opt_str(&mut self) -> Option<Option<String>> {
        match self.u32()? {
            0 => Some(None),
            1 => Some(Some(self.str()?)),
            _ => None,
        }
    }
    fn strs(&mut self) -> Option<Vec<String>> {
        let n = self.len()?;
        (0..n).map(|_| self.str()).collect()
    }
    fn char(&mut self) -> Option<char> {
        char::from_u32(self.u32()?)
    }
    fn chars(&mut self) -> Option<Vec<char>> {
        let n = self.len()?;
        (0..n).map(|_| self.char()).collect()
    }
    fn conj(&mut self) -> Option<Conj> {
        let n = self.len()?;
        Some(Conj((0..n).map(|_| self.term()).collect::<Option<_>>()?))
    }
    fn conjs(&mut self) -> Option<Vec<Conj>> {
        let n = self.len()?;
        (0..n).map(|_| self.conj()).collect()
    }
    fn term(&mut self) -> Option<Term> {
        Some(match self.u32()? {
            0 => Term::Type(self.str()?),
            1 => Term::Str(self.str()?),
            2 => Term::Regex(self.str()?),
            3 => Term::Coref(self.str()?),
            4 => {
                let n = self.len()?;
                Term::Avm(
                    (0..n)
                        .map(|_| {
                            Some(FeatVal {
                                path: self.strs()?,
                                value: self.conj()?,
                            })
                        })
                        .collect::<Option<_>>()?,
                )
            }
            5 => Term::List {
                items: self.conjs()?,
                open: self.u32()? != 0,
                tail: match self.u32()? {
                    0 => None,
                    1 => Some(Box::new(self.conj()?)),
                    _ => return None,
                },
            },
            6 => Term::DiffList(self.conjs()?),
            _ => return None,
        })
    }
    fn pattern(&mut self) -> Option<Vec<PatElem>> {
        let n = self.len()?;
        (0..n)
            .map(|_| match self.u32()? {
                0 => Some(PatElem::Char(self.char()?)),
                1 => Some(PatElem::Var(self.char()?)),
                _ => None,
            })
            .collect()
    }
    fn bits(&mut self) -> Option<BitSet> {
        let n = self.len()?;
        let words: Option<Vec<u64>> = (0..n).map(|_| self.u64()).collect();
        Some(BitSet {
            words: words?.into_boxed_slice(),
        })
    }
    fn dag(&mut self) -> Option<Dag> {
        let n = self.len()?;
        let nodes: Option<Vec<Node>> = (0..n)
            .map(|_| {
                Some(Node {
                    ty: self.u32()?,
                    arc_start: self.u32()?,
                    arc_len: self.u32()?,
                })
            })
            .collect();
        let m = self.len()?;
        let arcs: Option<Vec<(u32, u32)>> =
            (0..m).map(|_| Some((self.u32()?, self.u32()?))).collect();
        Some(Dag {
            nodes: nodes?,
            arcs: arcs?,
        })
    }
}

fn write(g: &Grammar) -> Vec<u8> {
    let mut w = W(Vec::new());
    w.0.extend_from_slice(MAGIC);
    w.u32(VERSION);
    let h = &g.ts.hier;
    w.u32(h.names.len() as u32);
    for n in &h.names {
        w.str(n);
    }
    w.u32(h.n_declared as u32);
    for t in 0..h.names.len() {
        w.bits(&h.codes[t]);
        w.bits(&h.ancestors[t]);
        w.u32s(&h.parents[t]);
        w.u32s(&h.children[t]);
    }
    w.u32(g.ts.string);
    let lits = g.ts.literals.read().unwrap();
    w.u32(lits.values.len() as u32);
    for (k, s) in &lits.values {
        w.u32(match k {
            LiteralKind::Str => 0,
            LiteralKind::Regex => 1,
        });
        w.str(s);
    }
    drop(lits);
    w.u32(g.feats.names.len() as u32);
    for f in &g.feats.names {
        w.str(f);
    }
    let l = &g.lists;
    for x in [
        l.list,
        l.cons,
        l.null,
        l.diff_list,
        l.first,
        l.rest,
        l.list_feat,
        l.last,
    ] {
        w.u32(x);
    }
    w.u32s(&g.intro);
    w.u32(g.constraints.len() as u32);
    for (c, &a) in g.constraints.iter().zip(&g.atomic) {
        w.u32(a as u32);
        w.dag(c);
    }
    w.u32(g.errors.len() as u32);
    for e in &g.errors {
        w.str(&e.what);
        w.str(&e.msg);
    }
    w.u32(g.instances.len() as u32);
    for i in &g.instances {
        w.str(&i.name);
        w.opt_str(&i.status);
        w.conj(&i.body);
        match &i.affix {
            None => w.u32(0),
            Some(a) => {
                w.u32(1);
                w.u32(a.prefix as u32);
                w.u32(a.pairs.len() as u32);
                for (input, output) in &a.pairs {
                    w.pattern(input);
                    w.pattern(output);
                }
            }
        }
        w.strs(&i.docstrings);
    }
    w.u32(g.letter_sets.len() as u32);
    for l in &g.letter_sets {
        w.u32(l.var as u32);
        w.chars(&l.chars);
        w.u32(l.wild as u32);
    }
    w.0
}

fn read(bytes: &[u8]) -> Option<Grammar> {
    let mut r = R { b: bytes, i: 0 };
    if r.take(8)? != MAGIC || r.u32()? != VERSION {
        return None;
    }
    let n = r.len()?;
    let names: Vec<String> = (0..n).map(|_| r.str()).collect::<Option<_>>()?;
    let n_declared = r.u32()? as usize;
    let (mut codes, mut ancestors, mut parents, mut children) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for _ in 0..n {
        codes.push(r.bits()?);
        ancestors.push(r.bits()?);
        parents.push(r.u32s()?);
        children.push(r.u32s()?);
    }
    let index = names
        .iter()
        .enumerate()
        .map(|(i, s)| (s.clone(), i as u32))
        .collect();
    let code_index = codes
        .iter()
        .enumerate()
        .map(|(i, c)| (c.clone(), i as u32))
        .collect();
    let hier = Hierarchy {
        names,
        index,
        n_declared,
        codes,
        code_index,
        ancestors,
        parents,
        children,
        glb_cache: RwLock::new(HashMap::default()),
    };
    let mut ts = TypeSystem::new(hier);
    ts.string = r.u32()?;
    let nl = r.len()?;
    let mut lits = Literals::default();
    for i in 0..nl {
        let kind = match r.u32()? {
            0 => LiteralKind::Str,
            1 => LiteralKind::Regex,
            _ => return None,
        };
        let s: Arc<str> = Arc::from(r.str()?);
        lits.index.insert((kind, s.clone()), i as u32);
        lits.values.push((kind, s));
    }
    *ts.literals.write().unwrap() = lits;
    let nf = r.len()?;
    let mut feats = Features::default();
    for _ in 0..nf {
        feats.intern(&r.str()?);
    }
    let mut v = [0u32; 8];
    for x in &mut v {
        *x = r.u32()?;
    }
    let lists = ListConfig {
        list: v[0],
        cons: v[1],
        null: v[2],
        diff_list: v[3],
        first: v[4],
        rest: v[5],
        list_feat: v[6],
        last: v[7],
    };
    let intro = r.u32s()?;
    let nc = r.len()?;
    let mut constraints = Vec::with_capacity(nc);
    let mut atomic = Vec::with_capacity(nc);
    for _ in 0..nc {
        atomic.push(r.u32()? != 0);
        constraints.push(Arc::new(r.dag()?));
    }
    let ne = r.len()?;
    let errors = (0..ne)
        .map(|_| {
            Some(GrammarError {
                what: r.str()?,
                msg: r.str()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let ni = r.len()?;
    let instances = (0..ni)
        .map(|_| {
            Some(Instance {
                name: r.str()?,
                status: r.opt_str()?,
                body: r.conj()?,
                affix: match r.u32()? {
                    0 => None,
                    1 => Some(Affix {
                        prefix: r.u32()? != 0,
                        pairs: {
                            let n = r.len()?;
                            (0..n)
                                .map(|_| Some((r.pattern()?, r.pattern()?)))
                                .collect::<Option<_>>()?
                        },
                    }),
                    _ => return None,
                },
                docstrings: r.strs()?,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    let nl = r.len()?;
    let letter_sets = (0..nl)
        .map(|_| {
            Some(LetterSet {
                var: r.char()?,
                chars: r.chars()?,
                wild: r.u32()? != 0,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    if r.i != bytes.len() {
        return None;
    }
    let instance_index = instances
        .iter()
        .enumerate()
        .map(|(i, inst): (usize, &Instance)| (inst.name.clone(), i))
        .collect();
    Some(Grammar {
        ts,
        feats,
        lists,
        intro,
        constraints,
        atomic,
        instances,
        instance_index,
        letter_sets,
        errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    #[test]
    fn pruning_keeps_the_most_recently_used_grammars() {
        let dir = std::env::temp_dir().join(format!("emdysi-prune-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let now = SystemTime::now();
        for i in 0..6u64 {
            let p = dir.join(format!("grammar-{i}.bin"));
            std::fs::write(&p, b"x").unwrap();
            std::fs::File::options()
                .append(true)
                .open(&p)
                .unwrap()
                .set_modified(now - Duration::from_secs(60 * (6 - i)))
                .unwrap();
        }
        // The oldest, used again, is kept; other files are left alone.
        touch(&dir.join("grammar-0.bin"));
        std::fs::write(dir.join("sources-x.txt"), b"").unwrap();
        prune(&dir);
        let mut left: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        left.sort();
        assert_eq!(
            left,
            [
                "grammar-0.bin",
                "grammar-3.bin",
                "grammar-4.bin",
                "grammar-5.bin",
                "sources-x.txt"
            ]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
