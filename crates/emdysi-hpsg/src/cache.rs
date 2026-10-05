//! A cache of the expensive part of grammar compilation: the closed type
//! hierarchy and the expanded type constraints.
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
use crate::grammar::{Grammar, GrammarError};
use crate::types::Hierarchy;
use crate::typesys::{Features, LiteralKind, Literals, TypeSystem};

/// Bump when the serialized layout or the compilation algorithm changes.
const VERSION: u32 = 2;
const MAGIC: &[u8; 8] = b"EMDYSIGC";

/// `$EMDYSI_CACHE_DIR`, else `$XDG_CACHE_HOME/emdysi`, else
/// `~/.cache/emdysi`.
pub fn default_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("EMDYSI_CACHE_DIR") {
        return Some(PathBuf::from(d));
    }
    if let Some(d) = std::env::var_os("XDG_CACHE_HOME") {
        return Some(PathBuf::from(d).join("emdysi"));
    }
    std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache/emdysi"))
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
                if let Some(mut g) = read(&bytes) {
                    g.letter_sets = loaded.letter_sets.clone();
                    g.add_instances(loaded);
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
            if std::fs::write(&tmp, bytes).is_ok() {
                let _ = std::fs::rename(&tmp, p);
            }
        }
        Ok(g)
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
    if r.i != bytes.len() {
        return None;
    }
    Some(Grammar {
        ts,
        feats,
        lists,
        intro,
        constraints,
        atomic,
        instances: Vec::new(),
        instance_index: HashMap::new(),
        letter_sets: Vec::new(),
        errors,
    })
}
