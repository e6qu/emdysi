//! Abstract syntax for TDL.

/// One element of an orthographic affix pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatElem {
    Char(char),
    /// A letter-set (`!x`) or wild-card (`?x`) variable.
    Var(char),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Term {
    /// A type name.
    Type(String),
    Str(String),
    /// A regular expression including its `^`/`$` anchors.
    Regex(String),
    /// A coreference tag, without the `#`.
    Coref(String),
    Avm(Vec<FeatVal>),
    List {
        items: Vec<Conj>,
        /// `< a, ... >`: the list may continue.
        open: bool,
        /// `< a . b >`: explicit tail.
        tail: Option<Box<Conj>>,
    },
    DiffList(Vec<Conj>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct FeatVal {
    pub path: Vec<String>,
    pub value: Conj,
}

/// A conjunction of terms joined by `&`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Conj(pub Vec<Term>);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DefOp {
    /// `:=`
    Define,
    /// `:+`
    Addendum,
    /// `:<`
    Subsume,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Affix {
    pub prefix: bool,
    /// (input, output) pattern pairs.
    pub pairs: Vec<(Vec<PatElem>, Vec<PatElem>)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Definition {
    pub name: String,
    pub op: DefOp,
    pub affix: Option<Affix>,
    pub body: Conj,
    pub docstrings: Vec<String>,
    pub file: String,
    pub line: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Def(Definition),
    LetterSet {
        var: char,
        chars: Vec<char>,
        wild: bool,
    },
    /// `:begin :type.` / `:begin :instance :status foo.`
    Begin {
        kind: String,
        status: Option<String>,
    },
    End {
        kind: String,
    },
    Include(String),
}
