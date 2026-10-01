//! Recursive-descent parser over the token stream.

use crate::ast::*;
use crate::error::{Error, Result};
use crate::lexer::{Lexer, Tok, Token};

/// Parse TDL source into statements. `file` is used in error messages and
/// recorded on definitions.
pub fn parse_str(src: &str, file: &str) -> Result<Vec<Statement>> {
    let toks = Lexer::new(src, file).tokenize()?;
    Parser { toks, pos: 0, file }.statements()
}

struct Parser<'a> {
    toks: Vec<Token>,
    pos: usize,
    file: &'a str,
}

impl Parser<'_> {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }

    fn peek2(&self) -> &Tok {
        self.toks.get(self.pos + 1).map_or(&Tok::Eof, |t| &t.tok)
    }

    fn line(&self) -> u32 {
        self.toks[self.pos].line
    }

    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }

    fn err<T>(&self, msg: impl Into<String>) -> Result<T> {
        Err(Error::syntax(self.file, self.line(), msg))
    }

    fn expect(&mut self, want: Tok, what: &str) -> Result<()> {
        if *self.peek() == want {
            self.next();
            Ok(())
        } else {
            self.err(format!("expected {what}, found {:?}", self.peek()))
        }
    }

    fn statements(&mut self) -> Result<Vec<Statement>> {
        let mut out = Vec::new();
        loop {
            match self.peek().clone() {
                Tok::Eof => return Ok(out),
                Tok::LetterSet { var, chars, wild } => {
                    self.next();
                    out.push(Statement::LetterSet { var, chars, wild });
                }
                Tok::Keyword(kw) => {
                    self.next();
                    out.push(self.directive(&kw)?);
                }
                Tok::Ident(_) => out.push(Statement::Def(self.definition()?)),
                t => return self.err(format!("unexpected {t:?} at top level")),
            }
        }
    }

    fn directive(&mut self, kw: &str) -> Result<Statement> {
        let st = match kw {
            "begin" | "end" => {
                let Tok::Keyword(kind) = self.next() else {
                    return self.err(format!(":{kw} must be followed by :type or :instance"));
                };
                if kw == "end" {
                    Statement::End { kind }
                } else {
                    let mut status = None;
                    if *self.peek() == Tok::Keyword("status".into()) {
                        self.next();
                        match self.next() {
                            Tok::Ident(s) => status = Some(s.to_ascii_lowercase()),
                            t => return self.err(format!("expected status name, found {t:?}")),
                        }
                    }
                    Statement::Begin { kind, status }
                }
            }
            "include" => match self.next() {
                Tok::Str(s) => Statement::Include(s),
                t => return self.err(format!("expected file name after :include, found {t:?}")),
            },
            _ => return self.err(format!("unknown directive :{kw}")),
        };
        self.expect(Tok::Dot, "'.' ending directive")?;
        Ok(st)
    }

    fn definition(&mut self) -> Result<Definition> {
        let line = self.line();
        let Tok::Ident(name) = self.next() else {
            unreachable!()
        };
        let op = match self.next() {
            Tok::Define => DefOp::Define,
            Tok::Addendum => DefOp::Addendum,
            Tok::Subsume => DefOp::Subsume,
            t => return self.err(format!("expected ':=' after {name}, found {t:?}")),
        };
        let mut docstrings = Vec::new();
        let mut affix = None;
        loop {
            match self.peek().clone() {
                Tok::DocStr(d) => {
                    self.next();
                    docstrings.push(d);
                }
                Tok::Affix { prefix, pairs } if affix.is_none() => {
                    self.next();
                    affix = Some(Affix { prefix, pairs });
                }
                _ => break,
            }
        }
        let body = if *self.peek() == Tok::Dot {
            Conj::default()
        } else {
            self.conj(&mut docstrings)?
        };
        while let Tok::DocStr(d) = self.peek().clone() {
            self.next();
            docstrings.push(d);
        }
        self.expect(Tok::Dot, "'.' ending definition")?;
        Ok(Definition {
            name,
            op,
            affix,
            body,
            docstrings,
            file: self.file.to_string(),
            line,
        })
    }

    fn conj(&mut self, docs: &mut Vec<String>) -> Result<Conj> {
        let mut terms = vec![self.term(docs)?];
        loop {
            while let Tok::DocStr(d) = self.peek().clone() {
                self.next();
                docs.push(d);
            }
            if *self.peek() != Tok::Amp {
                return Ok(Conj(terms));
            }
            self.next();
            while let Tok::DocStr(d) = self.peek().clone() {
                self.next();
                docs.push(d);
            }
            terms.push(self.term(docs)?);
        }
    }

    fn term(&mut self, docs: &mut Vec<String>) -> Result<Term> {
        Ok(match self.next() {
            Tok::Ident(s) => Term::Type(s),
            Tok::Str(s) => Term::Str(s),
            Tok::Regex(s) => Term::Regex(s),
            Tok::Coref(s) => Term::Coref(s),
            Tok::LBrack => {
                let mut fvs = Vec::new();
                if *self.peek() != Tok::RBrack {
                    loop {
                        let path = self.path()?;
                        let value = self.conj(docs)?;
                        fvs.push(FeatVal { path, value });
                        match self.next() {
                            Tok::Comma => continue,
                            Tok::RBrack => break,
                            t => {
                                return self
                                    .err(format!("expected ',' or ']' in AVM, found {t:?}"));
                            }
                        }
                    }
                } else {
                    self.next();
                }
                Term::Avm(fvs)
            }
            Tok::LAngle => self.list(docs)?,
            Tok::LDiff => {
                let mut items = Vec::new();
                if *self.peek() != Tok::RDiff {
                    loop {
                        items.push(self.conj(docs)?);
                        match self.next() {
                            Tok::Comma => continue,
                            Tok::RDiff => break,
                            t => {
                                return self.err(format!(
                                    "expected ',' or '!>' in diff-list, found {t:?}"
                                ));
                            }
                        }
                    }
                } else {
                    self.next();
                }
                Term::DiffList(items)
            }
            t => return self.err(format!("expected a term, found {t:?}")),
        })
    }

    fn list(&mut self, docs: &mut Vec<String>) -> Result<Term> {
        let mut items = Vec::new();
        let mut open = false;
        let mut tail = None;
        match self.peek() {
            Tok::RAngle => {
                self.next();
                return Ok(Term::List { items, open, tail });
            }
            Tok::Ellipsis => {
                self.next();
                self.expect(Tok::RAngle, "'>'")?;
                return Ok(Term::List {
                    items,
                    open: true,
                    tail,
                });
            }
            _ => {}
        }
        loop {
            items.push(self.conj(docs)?);
            match self.next() {
                Tok::Comma => {
                    if *self.peek() == Tok::Ellipsis {
                        self.next();
                        open = true;
                        self.expect(Tok::RAngle, "'>' after '...'")?;
                        break;
                    }
                }
                Tok::Dot => {
                    tail = Some(Box::new(self.conj(docs)?));
                    self.expect(Tok::RAngle, "'>' after dotted tail")?;
                    break;
                }
                Tok::RAngle => break,
                t => return self.err(format!("expected ',', '.' or '>' in list, found {t:?}")),
            }
        }
        Ok(Term::List { items, open, tail })
    }

    fn path(&mut self) -> Result<Vec<String>> {
        let mut path = Vec::new();
        loop {
            match self.next() {
                Tok::Ident(f) => path.push(f),
                t => return self.err(format!("expected feature name, found {t:?}")),
            }
            if *self.peek() == Tok::Dot && matches!(self.peek2(), Tok::Ident(_)) {
                self.next();
            } else {
                return Ok(path);
            }
        }
    }
}
