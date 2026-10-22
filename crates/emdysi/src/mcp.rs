//! `en mcp`: emdysi as a Model Context Protocol server over stdio, for
//! agents. One JSON-RPC message per line in and out; the grammar is loaded
//! on the first tool call and kept, and parses are cached as for `en
//! check`, so checking a draft again after an edit parses only the
//! sentences that changed.
//!
//! Tools: `check` (diagnostics, as `en check --format json`), `fix` (the
//! text with the automatic fixes applied) and `parse` (sentence analyses,
//! as `en parse --format json`). Each takes `text` or `path`.

use std::io::{BufRead, Write};

use emdysi_check::report::{OutputFormat, ParseDetails, render, render_parses};
use emdysi_check::{Checker, Format, Severity, analyze, apply_fixes};
use emdysi_parse::Erg;
use serde_json::{Value, json};

use crate::{Args, READING_MARGIN, check, load_glossary, load_model, load_packs};

/// The protocol version answered when the client asks for one this server
/// does not know.
const PROTOCOL: &str = "2025-06-18";
const KNOWN_PROTOCOLS: &[&str] = &["2024-11-05", "2025-03-26", "2025-06-18"];

const INSTRUCTIONS: &str = "emdysi checks English prose with a full grammar of English. \
`check` reports problems with exact positions; `core` findings (spelling, grammar errors \
the grammar can prove) are errors or warnings worth fixing, style findings are suggestions \
to weigh. `fix` applies the safe automatic fixes. `parse` shows how each sentence was \
analysed. The first call loads the grammar (a few seconds); unchanged sentences are not \
parsed again, so re-checking a draft after an edit is fast.";

struct Server {
    args: Args,
    erg: Option<Erg>,
    model: Option<Box<dyn emdysi_lm::LanguageModel>>,
    cache: Option<std::path::PathBuf>,
}

pub fn serve(args: Args) -> Result<bool, String> {
    let model = load_model(&args)?;
    let cache = args.cache.then(emdysi_parse::checker_cache_dir).flatten();
    let mut server = Server {
        args,
        erg: None,
        model,
        cache,
    };
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let reply = match serde_json::from_str::<Value>(&line) {
            Ok(msg) => server.handle(&msg),
            Err(e) => Some(error(Value::Null, -32700, &format!("parse error: {e}"))),
        };
        if let Some(r) = reply {
            writeln!(stdout, "{r}").map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?;
        }
    }
    Ok(true)
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn tools() -> Value {
    let source = json!({
        "text": {"type": "string", "description": "The text to read (or give `path`)."},
        "path": {"type": "string", "description": "A file to read instead of `text`."},
        "markdown": {"type": "boolean", "description": "Read the text as Markdown (default: from the file extension; true for `text`)."},
    });
    // The source properties and others.
    let with = |extra: &[&Value]| {
        let mut p = source.clone();
        for e in extra {
            p.as_object_mut()
                .unwrap()
                .extend(e.as_object().unwrap().clone());
        }
        p
    };
    let packs = json!({
        "packs": {"type": "array", "items": {"type": "string"}, "description": "Rule packs (default: core, ai-tells, plain-style, substance, structure, terms; also microsoft, google, elastic, wordlists, equality, languagetool, gitlab, slop, proselint, coverage)."},
        "disable": {"type": "array", "items": {"type": "string"}, "description": "Rule ids to skip, or prefixes ending in '*'."},
    });
    json!([
        {
            "name": "check",
            "title": "Check English prose",
            "description": "Spelling, grammar, style and AI-writing checks. Returns each diagnostic with its rule, severity (error, warning, suggestion), message, byte offsets and lines and columns (from 1, columns in characters, the end exclusive), the text found, and the replacement for that range (`fix`, applied by the fix tool) or other candidates (`suggestions`).",
            "inputSchema": {"type": "object", "properties": with(&[&packs, &json!({
                "min_severity": {"type": "string", "enum": ["error", "warning", "suggestion"], "description": "Report only diagnostics this severe or worse (default: suggestion, all)."},
            })])},
            "annotations": {"readOnlyHint": true},
        },
        {
            "name": "fix",
            "title": "Apply safe fixes",
            "description": "The text with the automatic fixes applied (spelling corrections the checker is sure of, doubled words, and the like), and how many were applied. Nothing is written to disk.",
            "inputSchema": {"type": "object", "properties": with(&[&packs])},
            "annotations": {"readOnlyHint": true},
        },
        {
            "name": "parse",
            "title": "Parse sentences",
            "description": "Each sentence's analysis by the grammar: whether it has a full grammatical reading, the number of readings, the best reading's phrase-structure tree, and how the sentence is ambiguous.",
            "inputSchema": {"type": "object", "properties": with(&[&json!({
                "derivations": {"type": "boolean", "description": "Also give each best reading's derivation."},
                "mrs": {"type": "boolean", "description": "Also give each best reading's semantics (MRS)."},
            })])},
            "annotations": {"readOnlyHint": true},
        },
    ])
}

impl Server {
    fn handle(&mut self, msg: &Value) -> Option<Value> {
        // Notifications (no id) get no answer; nor do answers to requests
        // this server never sends (no method).
        let id = msg.get("id")?.clone();
        let method = msg.get("method").and_then(Value::as_str)?;
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let result = match method {
            "initialize" => {
                let asked = params.get("protocolVersion").and_then(Value::as_str);
                let version = asked
                    .filter(|v| KNOWN_PROTOCOLS.contains(v))
                    .unwrap_or(PROTOCOL);
                json!({
                    "protocolVersion": version,
                    "capabilities": {"tools": {}},
                    "serverInfo": {"name": "emdysi", "version": env!("CARGO_PKG_VERSION")},
                    "instructions": INSTRUCTIONS,
                })
            }
            "ping" => json!({}),
            "tools/list" => json!({"tools": tools()}),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let empty = json!({});
                let arguments = params.get("arguments").unwrap_or(&empty);
                match self.call(name, arguments) {
                    Ok(v) => json!({
                        "content": [{"type": "text", "text": v.to_string()}],
                        "structuredContent": v,
                        "isError": false,
                    }),
                    Err(e) => json!({
                        "content": [{"type": "text", "text": e}],
                        "isError": true,
                    }),
                }
            }
            _ => return Some(error(id, -32601, &format!("unknown method {method:?}"))),
        };
        Some(json!({"jsonrpc": "2.0", "id": id, "result": result}))
    }

    fn call(&mut self, name: &str, arguments: &Value) -> Result<Value, String> {
        if !matches!(name, "check" | "fix" | "parse") {
            return Err(format!("unknown tool {name:?}"));
        }
        let str_list = |key: &str| -> Vec<String> {
            arguments
                .get(key)
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect()
                })
                .unwrap_or_default()
        };
        let (label, src, mut format) = match (
            arguments.get("text").and_then(Value::as_str),
            arguments.get("path").and_then(Value::as_str),
        ) {
            (Some(t), _) => ("<text>".to_string(), t.to_string(), Format::Markdown),
            (None, Some(p)) => {
                let path = std::path::Path::new(p);
                let src = std::fs::read_to_string(path).map_err(|e| format!("{p}: {e}"))?;
                (p.to_string(), src, Format::from_path(path))
            }
            (None, None) => return Err("give `text` or `path`".into()),
        };
        if let Some(md) = arguments.get("markdown").and_then(Value::as_bool) {
            format = if md { Format::Markdown } else { Format::Plain };
        }
        if self.erg.is_none() {
            let dir = match &self.args.grammar {
                Some(d) => d.clone(),
                None => {
                    emdysi_parse::grammar_dir().map_err(|e| format!("finding the grammar: {e}"))?
                }
            };
            let mut erg = Erg::load(&dir).map_err(|e| format!("loading grammar: {e}"))?;
            if let Some(c) = &self.cache {
                erg.cache_parses(c);
            }
            self.erg = Some(erg);
        }
        let erg = self.erg.as_ref().unwrap();
        let mut opts = self.args.opts.clone();
        if name == "parse" {
            opts.meaning = true;
        }
        let mut a = analyze(erg, &src, format, &opts);
        if self.args.context_readings {
            emdysi_check::decisions::prefer_document_phrases(&mut a, READING_MARGIN);
        }
        let out = if name == "parse" {
            let show = ParseDetails {
                derivations: arguments.get("derivations").and_then(Value::as_bool) == Some(true),
                mrs: arguments.get("mrs").and_then(Value::as_bool) == Some(true),
            };
            serde_json::from_str(&render_parses(&a, OutputFormat::Json, &show))
                .map_err(|e| e.to_string())?
        } else {
            let packs = str_list("packs");
            let mut packs = load_packs(if packs.is_empty() {
                &self.args.packs
            } else {
                &packs
            })?;
            for g in &self.args.glossaries {
                packs.push(load_glossary(g)?);
            }
            let mut checker = Checker::new(packs);
            checker.disabled = self.args.disabled.clone();
            checker.disabled.extend(str_list("disable"));
            let mut args = self.args.clone();
            if let Some(s) = arguments.get("min_severity").and_then(Value::as_str) {
                args.min_severity =
                    Severity::parse(s).ok_or("min_severity needs error, warning or suggestion")?;
            }
            let diags = check(&checker, erg, &a, self.model.as_mut(), &args);
            if name == "fix" {
                let (fixed, n) = apply_fixes(&src, &diags);
                json!({"text": fixed, "applied": n})
            } else {
                serde_json::from_str(&render(&a, &diags, &label, OutputFormat::Json))
                    .map_err(|e| e.to_string())?
            }
        };
        if let Some(c) = &self.cache {
            emdysi_parse::prune_parse_cache(c, crate::cache_bytes());
        }
        Ok(out)
    }
}
