//! Regular Expression PreProcessor (REPP), the rule-based tokenizer format
//! used by DELPH-IN grammars (Dridan & Oepen 2012).
//!
//! A REPP module is a sequence of operations:
//!
//! - `!pattern<TAB>replacement`: rewrite every match of `pattern`;
//! - `:pattern`: the pattern separating tokens after all rewrites;
//! - `#N` ... `#`: define internal group `N`; `>N` applies it repeatedly
//!   until the string stops changing;
//! - `>name`: apply external module `name.rpp` if it is active;
//! - `<file`: include another file in place;
//! - `=pattern`: masks (parsed, currently not applied);
//! - `;` comments and `@` metadata lines.
//!
//! Every character of the rewritten string remembers the span of input
//! characters it came from, so tokens carry offsets into the original text.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use fancy_regex::Regex;

#[derive(Debug)]
pub struct Error {
    pub file: String,
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.file, self.line, self.msg)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone)]
enum ReplPart {
    Lit(String),
    Group(usize),
}

#[derive(Debug)]
enum Op {
    Rewrite {
        re: Regex,
        repl: Vec<ReplPart>,
        source: String,
    },
    Group(String),
    Module(String),
}

#[derive(Debug, Default)]
struct Module {
    ops: Vec<Op>,
    groups: HashMap<String, Vec<Op>>,
}

/// A token with its character span `[from, to)` in the original input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub form: String,
    pub from: usize,
    pub to: usize,
}

/// A string under rewriting, with the original span of every character.
/// Inserted characters may carry an empty span.
#[derive(Debug, Clone)]
pub struct Tracked {
    pub chars: Vec<char>,
    pub spans: Vec<(usize, usize)>,
}

impl Tracked {
    pub fn new(s: &str) -> Self {
        let chars: Vec<char> = s.chars().collect();
        let spans = (0..chars.len()).map(|i| (i, i + 1)).collect();
        Tracked { chars, spans }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Original span covered by characters `[a, b)`; an empty range maps to
    /// the boundary position.
    fn span(&self, a: usize, b: usize) -> (usize, usize) {
        let real: Vec<&(usize, usize)> = self.spans[a..b].iter().filter(|s| s.0 < s.1).collect();
        match (real.first(), real.last()) {
            (Some(f), Some(l)) => (f.0, l.1),
            _ => {
                let p = if a < self.spans.len() {
                    self.spans[a].0
                } else {
                    self.spans.last().map_or(0, |s| s.1)
                };
                (p, p)
            }
        }
    }
}

pub struct Repp {
    modules: HashMap<String, Module>,
    main: String,
    tokenizer: Option<Regex>,
    active: HashSet<String>,
}

impl Repp {
    /// Load the main module `main` (e.g. `rpp/tokenizer.rpp`). External
    /// modules are read from `<dir>/<name>.rpp` for each name in `active`.
    pub fn load(main: &Path, active: &[&str]) -> Result<Repp, Error> {
        let dir = main.parent().unwrap_or(Path::new(".")).to_path_buf();
        let mut repp = Repp {
            modules: HashMap::new(),
            main: String::new(),
            tokenizer: None,
            active: active.iter().map(|s| s.to_string()).collect(),
        };
        let name = stem(main);
        repp.load_module(&name, main)?;
        repp.main = name;
        for m in active {
            let path = dir.join(format!("{m}.rpp"));
            repp.load_module(m, &path)?;
        }
        Ok(repp)
    }

    fn load_module(&mut self, name: &str, path: &Path) -> Result<(), Error> {
        let mut module = Module::default();
        let mut groups: Vec<(String, Vec<Op>)> = Vec::new();
        self.read_file(path, &mut module, &mut groups)?;
        if let Some((g, _)) = groups.pop() {
            return Err(Error {
                file: path.display().to_string(),
                line: 0,
                msg: format!("unterminated group #{g}"),
            });
        }
        self.modules.insert(name.to_string(), module);
        Ok(())
    }

    fn read_file(
        &mut self,
        path: &Path,
        module: &mut Module,
        groups: &mut Vec<(String, Vec<Op>)>,
    ) -> Result<(), Error> {
        let file = path.display().to_string();
        let src = std::fs::read_to_string(path).map_err(|e| Error {
            file: file.clone(),
            line: 0,
            msg: e.to_string(),
        })?;
        for (i, line) in src.lines().enumerate() {
            let err = |msg: String| Error {
                file: file.clone(),
                line: i + 1,
                msg,
            };
            let Some(first) = line.chars().next() else {
                continue;
            };
            let rest = &line[first.len_utf8()..];
            let op = match first {
                ';' | '@' => continue,
                '!' => {
                    let (pat, repl) = match rest.find('\t') {
                        Some(p) => (&rest[..p], rest[p..].trim_start_matches('\t')),
                        None => return Err(err("rewrite rule without a tab separator".into())),
                    };
                    let re =
                        Regex::new(pat).map_err(|e| err(format!("bad pattern {pat:?}: {e}")))?;
                    Op::Rewrite {
                        re,
                        repl: parse_replacement(repl),
                        source: line.to_string(),
                    }
                }
                ':' => {
                    let re =
                        Regex::new(rest).map_err(|e| err(format!("bad pattern {rest:?}: {e}")))?;
                    self.tokenizer = Some(re);
                    continue;
                }
                '=' => continue,
                '#' => {
                    let id = rest.trim();
                    if id.is_empty() {
                        match groups.pop() {
                            Some((g, ops)) => {
                                module.groups.insert(g, ops);
                            }
                            None => return Err(err("'#' closes no group".into())),
                        }
                    } else {
                        groups.push((id.to_string(), Vec::new()));
                    }
                    continue;
                }
                '>' => {
                    let id = rest.trim();
                    if id.chars().all(|c| c.is_ascii_digit()) {
                        Op::Group(id.to_string())
                    } else {
                        Op::Module(id.to_string())
                    }
                }
                '<' => {
                    let inc = path.parent().unwrap_or(Path::new(".")).join(rest.trim());
                    self.read_file(&inc, module, groups)?;
                    continue;
                }
                c if c.is_whitespace() => {
                    if line.trim().is_empty() {
                        continue;
                    }
                    return Err(err(format!("unexpected line {line:?}")));
                }
                _ => return Err(err(format!("unknown operator {first:?}"))),
            };
            match groups.last_mut() {
                Some((_, ops)) => ops.push(op),
                None => module.ops.push(op),
            }
        }
        Ok(())
    }

    /// Apply all rewrites to `input`.
    pub fn rewrite(&self, input: &str) -> Tracked {
        let mut t = Tracked::new(input);
        self.apply_module(&self.main, &mut t, None);
        t
    }

    /// Like [`Repp::rewrite`], recording each rule that changed the string.
    pub fn trace(&self, input: &str) -> (Tracked, Vec<(String, String)>) {
        let mut t = Tracked::new(input);
        let mut trace = Vec::new();
        self.apply_module(&self.main, &mut t, Some(&mut trace));
        (t, trace)
    }

    /// Rewrite and split `input` into tokens.
    pub fn tokenize(&self, input: &str) -> Vec<Token> {
        let t = self.rewrite(input);
        self.split(&t)
    }

    fn split(&self, t: &Tracked) -> Vec<Token> {
        let text = t.text();
        let byte_to_char = byte_to_char_map(&text);
        let mut tokens = Vec::new();
        let mut start = 0;
        let mut push = |a: usize, b: usize| {
            if a < b {
                let (from, to) = t.span(a, b);
                tokens.push(Token {
                    form: t.chars[a..b].iter().collect(),
                    from,
                    to,
                });
            }
        };
        if let Some(re) = &self.tokenizer {
            for m in re.find_iter(&text).flatten() {
                let (a, b) = (byte_to_char[m.start()], byte_to_char[m.end()]);
                push(start, a);
                start = b;
            }
        }
        push(start, t.chars.len());
        tokens
    }

    fn apply_module(
        &self,
        name: &str,
        t: &mut Tracked,
        mut trace: Option<&mut Vec<(String, String)>>,
    ) {
        let Some(module) = self.modules.get(name) else {
            return;
        };
        for op in &module.ops {
            self.apply_op(module, op, t, trace.as_deref_mut());
        }
    }

    fn apply_op(
        &self,
        module: &Module,
        op: &Op,
        t: &mut Tracked,
        mut trace: Option<&mut Vec<(String, String)>>,
    ) -> bool {
        match op {
            Op::Rewrite { re, repl, source } => {
                let changed = rewrite_once(re, repl, t);
                if changed {
                    if let Some(tr) = trace {
                        tr.push((source.clone(), t.text()));
                    }
                }
                changed
            }
            Op::Group(g) => {
                let Some(ops) = module.groups.get(g) else {
                    return false;
                };
                let mut any = false;
                // Iterate to a fixpoint, with a safety bound.
                for _ in 0..100 {
                    let mut changed = false;
                    for op in ops {
                        changed |= self.apply_op(module, op, t, trace.as_deref_mut());
                    }
                    any |= changed;
                    if !changed {
                        break;
                    }
                }
                any
            }
            Op::Module(m) => {
                if self.active.contains(m) {
                    let before = t.chars.clone();
                    self.apply_module(m, t, trace);
                    before != t.chars
                } else {
                    false
                }
            }
        }
    }
}

fn stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn byte_to_char_map(s: &str) -> Vec<usize> {
    let mut map = vec![0; s.len() + 1];
    let mut ci = 0;
    for (bi, c) in s.char_indices() {
        for k in 0..c.len_utf8() {
            map[bi + k] = ci;
        }
        ci += 1;
    }
    map[s.len()] = ci;
    map
}

fn parse_replacement(s: &str) -> Vec<ReplPart> {
    let mut parts = Vec::new();
    let mut lit = String::new();
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some(d) if d.is_ascii_digit() => {
                    let n = d.to_digit(10).unwrap() as usize;
                    chars.next();
                    if !lit.is_empty() {
                        parts.push(ReplPart::Lit(std::mem::take(&mut lit)));
                    }
                    parts.push(ReplPart::Group(n));
                }
                Some(&d) => {
                    chars.next();
                    lit.push(d);
                }
                None => lit.push('\\'),
            }
        } else {
            lit.push(c);
        }
    }
    if !lit.is_empty() {
        parts.push(ReplPart::Lit(lit));
    }
    parts
}

/// Apply one rewrite rule to every non-overlapping match. Returns whether
/// the string changed.
fn rewrite_once(re: &Regex, repl: &[ReplPart], t: &mut Tracked) -> bool {
    let text = t.text();
    let b2c = byte_to_char_map(&text);
    let mut out_chars = Vec::with_capacity(t.chars.len() + 8);
    let mut out_spans = Vec::with_capacity(t.chars.len() + 8);
    let mut last = 0;
    let mut matched = false;
    for caps in re.captures_iter(&text) {
        let Ok(caps) = caps else { break };
        let m = caps.get(0).unwrap();
        let (ms, me) = (b2c[m.start()], b2c[m.end()]);
        matched = true;
        out_chars.extend_from_slice(&t.chars[last..ms]);
        out_spans.extend_from_slice(&t.spans[last..ms]);
        let group_range = |n: usize| caps.get(n).map(|g| (b2c[g.start()], b2c[g.end()]));
        for (i, part) in repl.iter().enumerate() {
            match part {
                ReplPart::Group(n) => {
                    if let Some((a, b)) = group_range(*n) {
                        out_chars.extend_from_slice(&t.chars[a..b]);
                        out_spans.extend_from_slice(&t.spans[a..b]);
                    }
                }
                ReplPart::Lit(s) => {
                    // A literal stands for the input between the groups
                    // around it in the replacement.
                    let prev = repl[..i].iter().rev().find_map(|p| match p {
                        ReplPart::Group(n) => group_range(*n).map(|r| r.1),
                        _ => None,
                    });
                    let next = repl[i + 1..].iter().find_map(|p| match p {
                        ReplPart::Group(n) => group_range(*n).map(|r| r.0),
                        _ => None,
                    });
                    let a = prev.unwrap_or(ms);
                    let b = next.unwrap_or(me).max(a);
                    let span = t.span(a, b);
                    for c in s.chars() {
                        out_chars.push(c);
                        out_spans.push(span);
                    }
                }
            }
        }
        last = me;
    }
    if !matched {
        return false;
    }
    out_chars.extend_from_slice(&t.chars[last..]);
    out_spans.extend_from_slice(&t.spans[last..]);
    let changed = out_chars != t.chars;
    t.chars = out_chars;
    t.spans = out_spans;
    changed
}

/// The ERG's default REPP configuration (from `ace/config.tdl`).
pub fn erg(grammar_dir: &Path) -> Result<Repp, Error> {
    let main: PathBuf = grammar_dir.join("rpp/tokenizer.rpp");
    Repp::load(
        &main,
        &["xml", "ascii", "lgt", "quotes", "wiki", "gml", "html"],
    )
}
