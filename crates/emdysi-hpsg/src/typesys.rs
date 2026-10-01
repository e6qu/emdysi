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
struct Literals {
    values: Vec<(LiteralKind, Arc<str>)>,
    index: HashMap<(LiteralKind, Arc<str>), u32>,
}

pub type FeatId = u32;

#[derive(Default, Clone)]
pub struct Features {
    names: Vec<String>,
    index: HashMap<String, FeatId>,
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
    literals: RwLock<Literals>,
}

impl TypeSystem {
    pub fn new(hier: Hierarchy) -> Self {
        let string = hier.id("string").unwrap_or(crate::types::TOP);
        TypeSystem {
            hier,
            string,
            literals: RwLock::new(Literals::default()),
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
            (true, true) => None,
            (true, false) => self.hier.subsumed_by(self.string, b).then_some(a),
            (false, true) => self.hier.subsumed_by(self.string, a).then_some(b),
        }
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
