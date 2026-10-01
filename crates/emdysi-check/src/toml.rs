//! A small TOML subset for rule packs: `[table]` and `[[array-of-tables]]`
//! headers, `key = value` pairs with basic or literal strings (including
//! `"""` multi-line strings), integers, floats, booleans, arrays and inline
//! tables (`{ key = value }`), and `#` comments.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
    Array(Vec<Value>),
    Table(Table),
}

pub type Table = BTreeMap<String, Value>;

impl Value {
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Value]> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }

    pub fn as_table(&self) -> Option<&Table> {
        match self {
            Value::Table(t) => Some(t),
            _ => None,
        }
    }

    /// A string or an array of strings, as a list.
    pub fn as_str_list(&self) -> Option<Vec<String>> {
        match self {
            Value::Str(s) => Some(vec![s.clone()]),
            Value::Array(a) => a.iter().map(|v| v.as_str().map(String::from)).collect(),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct Error {
    pub line: usize,
    pub msg: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "line {}: {}", self.line, self.msg)
    }
}

struct P<'a> {
    s: &'a [u8],
    src: &'a str,
    i: usize,
    line: usize,
}

impl P<'_> {
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, Error> {
        Err(Error {
            line: self.line,
            msg: msg.into(),
        })
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    fn bump(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.i += 1;
        if c == b'\n' {
            self.line += 1;
        }
        Some(c)
    }

    /// Skip spaces, tabs and comments; with `newlines`, also line breaks.
    fn ws(&mut self, newlines: bool) {
        while let Some(c) = self.peek() {
            match c {
                b' ' | b'\t' | b'\r' => {
                    self.bump();
                }
                b'\n' if newlines => {
                    self.bump();
                }
                b'#' => {
                    while self.peek().is_some_and(|c| c != b'\n') {
                        self.bump();
                    }
                }
                _ => break,
            }
        }
    }

    fn key(&mut self) -> Result<String, Error> {
        self.ws(false);
        match self.peek() {
            Some(b'"') | Some(b'\'') => match self.value()? {
                Value::Str(s) => Ok(s),
                _ => self.err("bad key"),
            },
            _ => {
                let start = self.i;
                while self
                    .peek()
                    .is_some_and(|c| c.is_ascii_alphanumeric() || c == b'_' || c == b'-')
                {
                    self.bump();
                }
                if start == self.i {
                    return self.err("expected a key");
                }
                Ok(self.src[start..self.i].to_string())
            }
        }
    }

    /// A dotted key path.
    fn key_path(&mut self) -> Result<Vec<String>, Error> {
        let mut path = vec![self.key()?];
        loop {
            self.ws(false);
            if self.peek() == Some(b'.') {
                self.bump();
                path.push(self.key()?);
            } else {
                return Ok(path);
            }
        }
    }

    fn string(&mut self) -> Result<String, Error> {
        let quote = self.bump().unwrap();
        let multi = self.s[self.i..].starts_with(&[quote, quote]);
        if multi {
            self.bump();
            self.bump();
            // A newline right after the opening delimiter is dropped.
            if self.peek() == Some(b'\n') {
                self.bump();
            }
        }
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return self.err("unterminated string");
            };
            if c == quote {
                if !multi {
                    self.bump();
                    return Ok(out);
                }
                if self.s[self.i..].starts_with(&[quote, quote, quote]) {
                    self.i += 3;
                    return Ok(out);
                }
            }
            if c == b'\n' && !multi {
                return self.err("newline in string");
            }
            if c == b'\\' && quote == b'"' {
                self.bump();
                match self.bump() {
                    Some(b'n') => out.push('\n'),
                    Some(b't') => out.push('\t'),
                    Some(b'"') => out.push('"'),
                    Some(b'\\') => out.push('\\'),
                    Some(b'u') => {
                        let hex = self.src.get(self.i..self.i + 4).unwrap_or("");
                        let ch = u32::from_str_radix(hex, 16).ok().and_then(char::from_u32);
                        match ch {
                            Some(ch) => out.push(ch),
                            None => return self.err("bad \\u escape"),
                        }
                        self.i += 4;
                    }
                    Some(b'\n') => {
                        // Line-ending backslash: skip leading whitespace.
                        while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                            self.bump();
                        }
                    }
                    _ => return self.err("unknown escape"),
                }
                continue;
            }
            // Copy one UTF-8 character.
            let ch = self.src[self.i..].chars().next().unwrap();
            for _ in 0..ch.len_utf8() {
                self.bump();
            }
            out.push(ch);
        }
    }

    fn value(&mut self) -> Result<Value, Error> {
        self.ws(false);
        match self.peek() {
            Some(b'"') | Some(b'\'') => Ok(Value::Str(self.string()?)),
            Some(b'[') => {
                self.bump();
                let mut items = Vec::new();
                loop {
                    self.ws(true);
                    if self.peek() == Some(b']') {
                        self.bump();
                        return Ok(Value::Array(items));
                    }
                    items.push(self.value()?);
                    self.ws(true);
                    match self.bump() {
                        Some(b',') => {}
                        Some(b']') => return Ok(Value::Array(items)),
                        _ => return self.err("expected ',' or ']' in array"),
                    }
                }
            }
            Some(b'{') => {
                self.bump();
                let mut t = Table::new();
                loop {
                    self.ws(false);
                    if self.peek() == Some(b'}') {
                        self.bump();
                        return Ok(Value::Table(t));
                    }
                    let path = self.key_path()?;
                    self.ws(false);
                    if self.bump() != Some(b'=') {
                        return self.err("expected '=' in inline table");
                    }
                    let v = self.value()?;
                    insert(&mut t, &path, v).map_err(|m| Error {
                        line: self.line,
                        msg: m,
                    })?;
                    self.ws(false);
                    match self.bump() {
                        Some(b',') => {}
                        Some(b'}') => return Ok(Value::Table(t)),
                        _ => return self.err("expected ',' or '}' in inline table"),
                    }
                }
            }
            _ => {
                let start = self.i;
                while self.peek().is_some_and(|c| {
                    c.is_ascii_alphanumeric() || matches!(c, b'+' | b'-' | b'.' | b'_')
                }) {
                    self.bump();
                }
                let tok = &self.src[start..self.i];
                match tok {
                    "true" => Ok(Value::Bool(true)),
                    "false" => Ok(Value::Bool(false)),
                    _ => {
                        let clean = tok.replace('_', "");
                        if let Ok(i) = clean.parse::<i64>() {
                            Ok(Value::Int(i))
                        } else if let Ok(f) = clean.parse::<f64>() {
                            Ok(Value::Float(f))
                        } else {
                            self.err(format!("bad value {tok:?}"))
                        }
                    }
                }
            }
        }
    }
}

fn insert(t: &mut Table, path: &[String], v: Value) -> Result<(), String> {
    let (last, parents) = path.split_last().unwrap();
    let mut cur = t;
    for p in parents {
        let entry = cur
            .entry(p.clone())
            .or_insert_with(|| Value::Table(Table::new()));
        cur = match entry {
            Value::Table(t) => t,
            _ => return Err(format!("{p} is not a table")),
        };
    }
    if cur.contains_key(last) {
        return Err(format!("duplicate key {last}"));
    }
    cur.insert(last.clone(), v);
    Ok(())
}

/// The table a header path refers to, creating tables as needed. For an
/// array of tables, a new element is appended.
fn header_table<'t>(
    root: &'t mut Table,
    path: &[String],
    array: bool,
) -> Result<&'t mut Table, String> {
    let mut cur = root;
    for (k, p) in path.iter().enumerate() {
        let last = k + 1 == path.len();
        if last && array {
            let entry = cur
                .entry(p.clone())
                .or_insert_with(|| Value::Array(Vec::new()));
            let Value::Array(a) = entry else {
                return Err(format!("{p} is not an array of tables"));
            };
            a.push(Value::Table(Table::new()));
            let Some(Value::Table(t)) = a.last_mut() else {
                unreachable!()
            };
            return Ok(t);
        }
        let entry = cur
            .entry(p.clone())
            .or_insert_with(|| Value::Table(Table::new()));
        cur = match entry {
            Value::Table(t) => t,
            Value::Array(a) => match a.last_mut() {
                Some(Value::Table(t)) => t,
                _ => return Err(format!("{p} is not a table")),
            },
            _ => return Err(format!("{p} is not a table")),
        };
    }
    Ok(cur)
}

pub fn parse(src: &str) -> Result<Table, Error> {
    let mut root = Table::new();
    let mut current: Vec<String> = Vec::new();
    let mut current_array = false;
    let mut p = P {
        s: src.as_bytes(),
        src,
        i: 0,
        line: 1,
    };
    loop {
        p.ws(true);
        let Some(c) = p.peek() else { break };
        if c == b'[' {
            p.bump();
            let array = p.peek() == Some(b'[');
            if array {
                p.bump();
            }
            let path = p.key_path()?;
            p.ws(false);
            for _ in 0..if array { 2 } else { 1 } {
                if p.bump() != Some(b']') {
                    return p.err("expected ']' closing table header");
                }
            }
            header_table(&mut root, &path, array).map_err(|m| Error {
                line: p.line,
                msg: m,
            })?;
            current = path;
            current_array = array;
            continue;
        }
        let path = p.key_path()?;
        p.ws(false);
        if p.bump() != Some(b'=') {
            return p.err("expected '='");
        }
        let v = p.value()?;
        let line = p.line;
        // Re-resolve the current table without appending a new array element.
        let table = {
            let mut cur = &mut root;
            for (k, seg) in current.iter().enumerate() {
                let last = k + 1 == current.len();
                let entry = cur.get_mut(seg).unwrap();
                cur = match entry {
                    Value::Table(t) => t,
                    Value::Array(a) if last && current_array || !last => match a.last_mut() {
                        Some(Value::Table(t)) => t,
                        _ => unreachable!(),
                    },
                    _ => unreachable!(),
                };
            }
            cur
        };
        insert(table, &path, v).map_err(|m| Error { line, msg: m })?;
        p.ws(false);
        match p.peek() {
            None | Some(b'\n') => {}
            _ => return p.err("expected end of line"),
        }
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn subset() {
        let t = parse(
            r#"
# comment
[pack]
name = "ai-tells"   # trailing
level = 2

[[rule]]
id = 'ai.delve'
words = ["delve", "tapestry",
         "testament"]
fix = { "leverage" = "use" }

[[rule]]
id = "x"
message = """
Multi
line"""
on = true
ratio = 0.5
"#,
        )
        .unwrap();
        assert_eq!(
            t["pack"].as_table().unwrap()["name"].as_str(),
            Some("ai-tells")
        );
        let rules = t["rule"].as_array().unwrap();
        assert_eq!(rules.len(), 2);
        let r0 = rules[0].as_table().unwrap();
        assert_eq!(
            r0["words"].as_str_list().unwrap(),
            vec!["delve", "tapestry", "testament"]
        );
        assert_eq!(
            r0["fix"].as_table().unwrap()["leverage"].as_str(),
            Some("use")
        );
        let r1 = rules[1].as_table().unwrap();
        assert_eq!(r1["message"].as_str(), Some("Multi\nline"));
        assert_eq!(r1["on"].as_bool(), Some(true));
        assert_eq!(r1["ratio"].as_f64(), Some(0.5));
    }
}
