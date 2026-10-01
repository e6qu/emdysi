//! Tokenizer for TDL source text.

use crate::ast::PatElem;
use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Ident(String),
    /// `:begin`, `:end`, `:include`, `:type`, `:instance`, `:status`, ...
    Keyword(String),
    Str(String),
    DocStr(String),
    Regex(String),
    Coref(String),
    /// `:=`
    Define,
    /// `:+`
    Addendum,
    /// `:<`
    Subsume,
    Amp,
    LBrack,
    RBrack,
    LAngle,
    RAngle,
    LDiff,
    RDiff,
    Comma,
    Dot,
    Ellipsis,
    LetterSet {
        var: char,
        chars: Vec<char>,
        wild: bool,
    },
    Affix {
        prefix: bool,
        pairs: Vec<(Vec<PatElem>, Vec<PatElem>)>,
    },
    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    pub line: u32,
}

/// Characters that may not appear in an identifier.
fn is_ident_char(c: char) -> bool {
    !(c.is_whitespace() || "!\"#$%&'(),./:;<=>[]^|".contains(c))
}

pub struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: u32,
    file: &'a str,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &str, file: &'a str) -> Self {
        Lexer {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            file,
        }
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T> {
        Err(Error::syntax(self.file, self.line, msg))
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, off: usize) -> Option<char> {
        self.chars.get(self.pos + off).copied()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
        }
        Some(c)
    }

    fn starts_with(&self, s: &str) -> bool {
        s.chars()
            .enumerate()
            .all(|(i, c)| self.peek_at(i) == Some(c))
    }

    fn skip_trivia(&mut self) -> Result<()> {
        loop {
            match self.peek() {
                Some(c) if c.is_whitespace() => {
                    self.bump();
                }
                Some(';') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.bump();
                    }
                }
                Some('#') if self.peek_at(1) == Some('|') => {
                    let start = self.line;
                    self.pos += 2;
                    loop {
                        if self.starts_with("|#") {
                            self.pos += 2;
                            break;
                        }
                        if self.bump().is_none() {
                            return Err(Error::syntax(
                                self.file,
                                start,
                                "unterminated block comment",
                            ));
                        }
                    }
                }
                _ => return Ok(()),
            }
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut out = Vec::new();
        loop {
            self.skip_trivia()?;
            let line = self.line;
            let Some(c) = self.peek() else {
                out.push(Token {
                    tok: Tok::Eof,
                    line,
                });
                return Ok(out);
            };
            let tok = match c {
                '"' if self.starts_with("\"\"\"") => self.docstring()?,
                '"' => Tok::Str(self.string()?),
                '^' => self.regex()?,
                '#' => {
                    self.bump();
                    let name = self.ident_chars();
                    if name.is_empty() {
                        return self.err("empty coreference tag");
                    }
                    Tok::Coref(name)
                }
                ':' => {
                    self.bump();
                    match self.peek() {
                        Some('=') => {
                            self.bump();
                            Tok::Define
                        }
                        Some('+') => {
                            self.bump();
                            Tok::Addendum
                        }
                        Some('<') => {
                            self.bump();
                            Tok::Subsume
                        }
                        _ => {
                            let kw = self.ident_chars();
                            if kw.is_empty() {
                                return self.err("stray ':'");
                            }
                            Tok::Keyword(kw.to_ascii_lowercase())
                        }
                    }
                }
                '%' => self.percent()?,
                '&' => {
                    self.bump();
                    Tok::Amp
                }
                '[' => {
                    self.bump();
                    Tok::LBrack
                }
                ']' => {
                    self.bump();
                    Tok::RBrack
                }
                '<' => {
                    self.bump();
                    if self.peek() == Some('!') {
                        self.bump();
                        Tok::LDiff
                    } else {
                        Tok::LAngle
                    }
                }
                '!' if self.peek_at(1) == Some('>') => {
                    self.pos += 2;
                    Tok::RDiff
                }
                '>' => {
                    self.bump();
                    Tok::RAngle
                }
                ',' => {
                    self.bump();
                    Tok::Comma
                }
                '.' => {
                    if self.starts_with("...") {
                        self.pos += 3;
                        Tok::Ellipsis
                    } else {
                        self.bump();
                        Tok::Dot
                    }
                }
                c if is_ident_char(c) => Tok::Ident(self.ident_chars()),
                c => return self.err(format!("unexpected character {c:?}")),
            };
            out.push(Token { tok, line });
        }
    }

    fn ident_chars(&mut self) -> String {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if !is_ident_char(c) {
                break;
            }
            s.push(c);
            self.bump();
        }
        s
    }

    fn string(&mut self) -> Result<String> {
        let start = self.line;
        self.bump();
        let mut s = String::new();
        loop {
            match self.bump() {
                None => return Err(Error::syntax(self.file, start, "unterminated string")),
                Some('"') => return Ok(s),
                Some('\\') => match self.bump() {
                    Some(c) => s.push(c),
                    None => return Err(Error::syntax(self.file, start, "unterminated string")),
                },
                Some(c) => s.push(c),
            }
        }
    }

    fn docstring(&mut self) -> Result<Tok> {
        let start = self.line;
        self.pos += 3;
        let mut s = String::new();
        loop {
            if self.starts_with("\"\"\"") {
                self.pos += 3;
                return Ok(Tok::DocStr(s.trim().to_string()));
            }
            match self.bump() {
                None => return Err(Error::syntax(self.file, start, "unterminated docstring")),
                Some('\\') => {
                    if let Some(c) = self.bump() {
                        s.push(c);
                    }
                }
                Some(c) => s.push(c),
            }
        }
    }

    /// A regular expression literal `^...$`, kept verbatim (anchors included).
    fn regex(&mut self) -> Result<Tok> {
        let start = self.line;
        let mut s = String::new();
        s.push(self.bump().unwrap());
        loop {
            match self.bump() {
                None => return Err(Error::syntax(self.file, start, "unterminated regex")),
                Some('\\') => {
                    s.push('\\');
                    if let Some(c) = self.bump() {
                        s.push(c);
                    }
                }
                Some('$') => {
                    s.push('$');
                    return Ok(Tok::Regex(s));
                }
                Some(c) => s.push(c),
            }
        }
    }

    fn percent(&mut self) -> Result<Tok> {
        self.bump();
        if self.peek() == Some('(') {
            self.bump();
            let kind = self.ident_chars().to_ascii_lowercase();
            let wild = match kind.as_str() {
                "letter-set" => false,
                "wild-card" => true,
                _ => return self.err(format!("unknown %({kind} ...) directive")),
            };
            self.skip_ws();
            if self.bump() != Some('(') {
                return self.err("expected '(' in letter set");
            }
            let marker = if wild { '?' } else { '!' };
            if self.bump() != Some(marker) {
                return self.err(format!("expected '{marker}' variable in letter set"));
            }
            let Some(var) = self.bump() else {
                return self.err("unterminated letter set");
            };
            self.skip_ws();
            let mut chars = Vec::new();
            loop {
                match self.bump() {
                    None => return self.err("unterminated letter set"),
                    Some('\\') => match self.bump() {
                        Some(c) => chars.push(c),
                        None => return self.err("unterminated letter set"),
                    },
                    Some(')') => break,
                    Some(c) => chars.push(c),
                }
            }
            self.skip_ws();
            if self.bump() != Some(')') {
                return self.err("expected ')' closing letter set");
            }
            return Ok(Tok::LetterSet { var, chars, wild });
        }
        let kind = self.ident_chars().to_ascii_lowercase();
        let prefix = match kind.as_str() {
            "suffix" => false,
            "prefix" => true,
            _ => return self.err(format!("unknown %{kind} directive")),
        };
        let mut pairs = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() != Some('(') {
                break;
            }
            self.bump();
            // Split the pair into whitespace-separated words, honouring escapes.
            let mut words: Vec<Vec<PatElem>> = Vec::new();
            let mut cur: Option<Vec<PatElem>> = None;
            loop {
                match self.bump() {
                    None => return self.err("unterminated affix pattern"),
                    Some(')') => break,
                    Some(c) if c.is_whitespace() => {
                        if let Some(w) = cur.take() {
                            words.push(w);
                        }
                    }
                    Some('\\') => match self.bump() {
                        Some(c) => cur.get_or_insert_with(Vec::new).push(PatElem::Char(c)),
                        None => return self.err("unterminated affix pattern"),
                    },
                    Some('!') | Some('?') => match self.bump() {
                        Some(c) => cur.get_or_insert_with(Vec::new).push(PatElem::Var(c)),
                        None => return self.err("unterminated affix pattern"),
                    },
                    // `*` stands for the empty string.
                    Some('*') => {
                        cur.get_or_insert_with(Vec::new);
                    }
                    Some(c) => cur.get_or_insert_with(Vec::new).push(PatElem::Char(c)),
                }
            }
            if let Some(w) = cur.take() {
                words.push(w);
            }
            if words.len() != 2 {
                return self.err("affix pattern must have exactly two sides");
            }
            let rhs = words.pop().unwrap();
            let lhs = words.pop().unwrap();
            pairs.push((lhs, rhs));
        }
        Ok(Tok::Affix { prefix, pairs })
    }

    fn skip_ws(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.bump();
        }
    }
}
