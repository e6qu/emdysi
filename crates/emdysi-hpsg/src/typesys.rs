//! Type system: the closed hierarchy plus string literals and feature names.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::types::{Hierarchy, TypeId};

/// Type ids with this bit set denote literals (strings or regexes) rather
/// than hierarchy types.
const LITERAL_BIT: u32 = 1 << 31;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LiteralKind {
    Str,
    /// A `^...$` pattern, used by chart-mapping rules.
    Regex,
}

#[derive(Default)]
pub(crate) struct Literals {
    pub(crate) values: Vec<(LiteralKind, Arc<str>)>,
    pub(crate) index: HashMap<(LiteralKind, Arc<str>), u32>,
}

pub type FeatId = u32;

#[derive(Default, Clone)]
pub struct Features {
    pub(crate) names: Vec<String>,
    pub(crate) index: HashMap<String, FeatId>,
}

impl Features {
    pub fn intern(&mut self, name: &str) -> FeatId {
        let name = name.to_uppercase();
        if let Some(&f) = self.index.get(&name) {
            return f;
        }
        let f = self.names.len() as FeatId;
        self.names.push(name.clone());
        self.index.insert(name, f);
        f
    }

    pub fn get(&self, name: &str) -> Option<FeatId> {
        self.index
            .get(name)
            .or_else(|| self.index.get(&name.to_uppercase()))
            .copied()
    }

    pub fn name(&self, f: FeatId) -> &str {
        &self.names[f as usize]
    }

    pub fn len(&self) -> usize {
        self.names.len()
    }

    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }
}

pub struct TypeSystem {
    pub hier: Hierarchy,
    /// The type every string literal belongs to.
    pub string: TypeId,
    pub(crate) literals: RwLock<Literals>,
    regexes: RwLock<HashMap<TypeId, Option<Arc<fancy_regex::Regex>>>>,
}

impl TypeSystem {
    pub fn new(hier: Hierarchy) -> Self {
        let string = hier.id("string").unwrap_or(crate::types::TOP);
        TypeSystem {
            hier,
            string,
            literals: RwLock::new(Literals::default()),
            regexes: RwLock::new(HashMap::new()),
        }
    }

    pub fn is_literal(t: TypeId) -> bool {
        t & LITERAL_BIT != 0
    }

    pub fn literal(&self, kind: LiteralKind, s: &str) -> TypeId {
        let key: Arc<str> = Arc::from(s);
        if let Some(&i) = self
            .literals
            .read()
            .unwrap()
            .index
            .get(&(kind, key.clone()))
        {
            return i | LITERAL_BIT;
        }
        let mut lits = self.literals.write().unwrap();
        if let Some(&i) = lits.index.get(&(kind, key.clone())) {
            return i | LITERAL_BIT;
        }
        let i = lits.values.len() as u32;
        lits.values.push((kind, key.clone()));
        lits.index.insert((kind, key), i);
        i | LITERAL_BIT
    }

    pub fn string_literal(&self, s: &str) -> TypeId {
        self.literal(LiteralKind::Str, s)
    }

    /// The kind and text of a literal type.
    pub fn literal_value(&self, t: TypeId) -> Option<(LiteralKind, Arc<str>)> {
        if !Self::is_literal(t) {
            return None;
        }
        let lits = self.literals.read().unwrap();
        let (k, s) = &lits.values[(t & !LITERAL_BIT) as usize];
        Some((*k, s.clone()))
    }

    /// The hierarchy type a type belongs to: `string` for literals.
    pub fn base(&self, t: TypeId) -> TypeId {
        if Self::is_literal(t) { self.string } else { t }
    }

    pub fn glb(&self, a: TypeId, b: TypeId) -> Option<TypeId> {
        if a == b {
            return Some(a);
        }
        match (Self::is_literal(a), Self::is_literal(b)) {
            (false, false) => self.hier.glb(a, b),
            (true, true) => self.glb_literals(a, b),
            (true, false) => self.hier.subsumed_by(self.string, b).then_some(a),
            (false, true) => self.hier.subsumed_by(self.string, a).then_some(b),
        }
    }

    /// Two distinct literals unify only when one is a regex that matches the
    /// other, a string.
    fn glb_literals(&self, a: TypeId, b: TypeId) -> Option<TypeId> {
        let (ka, sa) = self.literal_value(a)?;
        let (kb, sb) = self.literal_value(b)?;
        match (ka, kb) {
            (LiteralKind::Regex, LiteralKind::Str) => self.regex_matches(a, &sb).then_some(b),
            (LiteralKind::Str, LiteralKind::Regex) => self.regex_matches(b, &sa).then_some(a),
            _ => None,
        }
    }

    /// The compiled form of a regex literal, matching whole strings. `None`
    /// if the literal is not a valid regex.
    pub fn regex(&self, t: TypeId) -> Option<Arc<fancy_regex::Regex>> {
        if let Some(r) = self.regexes.read().unwrap().get(&t) {
            return r.clone();
        }
        let compiled = match self.literal_value(t) {
            Some((LiteralKind::Regex, src)) => compile_anchored(&src).map(Arc::new),
            _ => None,
        };
        self.regexes.write().unwrap().insert(t, compiled.clone());
        compiled
    }

    fn regex_matches(&self, re: TypeId, s: &str) -> bool {
        self.regex(re)
            .is_some_and(|r| r.is_match(s).unwrap_or(false))
    }

    /// `a` is equal to or more specific than `b`.
    pub fn subsumed_by(&self, a: TypeId, b: TypeId) -> bool {
        if a == b {
            return true;
        }
        match (Self::is_literal(a), Self::is_literal(b)) {
            (false, false) => self.hier.subsumed_by(a, b),
            (true, false) => self.hier.subsumed_by(self.string, b),
            _ => false,
        }
    }

    pub fn name(&self, t: TypeId) -> String {
        match self.literal_value(t) {
            Some((LiteralKind::Str, s)) => format!("{s:?}"),
            Some((LiteralKind::Regex, s)) => s.to_string(),
            None => self.hier.name(t).to_string(),
        }
    }
}

/// Compile a `^...$` TDL pattern so that it must match the whole string,
/// even when the pattern contains top-level alternation.
pub fn compile_anchored(src: &str) -> Option<fancy_regex::Regex> {
    let inner = src.strip_prefix('^').unwrap_or(src);
    let inner = inner.strip_suffix('$').unwrap_or(inner);
    // POSIX classes are Unicode-aware in the grammar's intended semantics.
    let mut inner = inner.to_string();
    for (posix, unicode) in [
        ("[:upper:]", "\\p{Lu}"),
        ("[:lower:]", "\\p{Ll}"),
        ("[:alpha:]", "\\p{L}"),
        ("[:digit:]", "\\p{Nd}"),
        ("[:alnum:]", "\\p{L}\\p{N}"),
        ("[:punct:]", "\\p{P}\\p{S}"),
        ("[:space:]", "\\s"),
    ] {
        inner = inner.replace(posix, unicode);
    }
    fancy_regex::Regex::new(&format!("^(?:{inner})$")).ok()
}
