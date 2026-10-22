//! `en lsp`: emdysi as a language server, for editors. Diagnostics are
//! published when a document is opened or saved, and after an edit once
//! typing pauses; each fix or suggestion is a quick fix. The grammar is
//! loaded once and parses are cached, so after the first check of a
//! document only the sentences that changed are parsed again.
//!
//! Three threads: one reads the editor's messages, one checks documents
//! (newest version of each first), and the main one answers requests and
//! decides when a changed document is due, so that the server keeps
//! answering while a long document is parsed.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::sync::mpsc::{self, RecvTimeoutError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use emdysi_check::{Diagnostic, Format, Options, Severity, analyze};
use serde_json::{Value, json};

use crate::{Args, READING_MARGIN, check, load_grammar, make_checker};

/// How long typing must pause before a changed document is checked.
const PAUSE: Duration = Duration::from_millis(800);

/// One JSON-RPC message, framed by a `Content-Length` header; `None` at
/// the end of the input.
fn read_message(r: &mut impl BufRead) -> Option<Value> {
    loop {
        let mut len = None;
        loop {
            let mut line = String::new();
            if r.read_line(&mut line).ok()? == 0 {
                return None;
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(v) = line.strip_prefix("Content-Length:") {
                len = v.trim().parse::<usize>().ok();
            }
        }
        let Some(len) = len else { continue };
        let mut body = vec![0; len];
        r.read_exact(&mut body).ok()?;
        if let Ok(v) = serde_json::from_slice(&body) {
            return Some(v);
        }
    }
}

type Out = Arc<Mutex<std::io::Stdout>>;

fn send(out: &Out, v: &Value) {
    let body = v.to_string();
    let mut w = out.lock().unwrap();
    let _ = write!(w, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = w.flush();
}

/// Where a byte offset is, as LSP counts: the line from 0, and the column
/// in UTF-16 code units (LSP's default) or bytes (`utf8`).
fn position(src: &str, byte: usize, utf8: bool) -> Value {
    let mut byte = byte.min(src.len());
    while !src.is_char_boundary(byte) {
        byte -= 1;
    }
    let line_start = src[..byte].rfind('\n').map_or(0, |i| i + 1);
    let line = src[..line_start].matches('\n').count();
    let text = &src[line_start..byte];
    let character = if utf8 {
        text.len()
    } else {
        text.encode_utf16().count()
    };
    json!({"line": line, "character": character})
}

fn lsp_range(src: &str, r: &std::ops::Range<usize>, utf8: bool) -> Value {
    json!({"start": position(src, r.start, utf8), "end": position(src, r.end, utf8)})
}

/// The byte offset of an LSP position.
fn offset(src: &str, pos: &Value, utf8: bool) -> usize {
    let line = pos["line"].as_u64().unwrap_or(0) as usize;
    let character = pos["character"].as_u64().unwrap_or(0) as usize;
    let start = if line == 0 {
        0
    } else {
        src.match_indices('\n')
            .nth(line - 1)
            .map_or(src.len(), |(i, _)| i + 1)
    };
    let text = &src[start..];
    let text = &text[..text.find('\n').unwrap_or(text.len())];
    if utf8 {
        return start + character.min(text.len());
    }
    let mut units = 0;
    for (i, c) in text.char_indices() {
        if units >= character {
            return start + i;
        }
        units += c.len_utf16();
    }
    start + text.len()
}

fn is_markdown(uri: &str, language: Option<&str>) -> bool {
    matches!(language, Some("markdown" | "mdx"))
        || [".md", ".markdown", ".mdx"]
            .iter()
            .any(|e| uri.to_lowercase().ends_with(e))
}

/// A document as last sent by the editor.
#[derive(Clone)]
struct Doc {
    text: String,
    version: i64,
    markdown: bool,
}

/// The last diagnostics of each document (by URI), with the text they
/// were found in, for code actions.
type Found = Arc<Mutex<HashMap<String, (String, Vec<Diagnostic>)>>>;

enum Job {
    Check(String, Doc),
    Forget(String),
}

/// Checks the documents sent to it, newest version of each, and publishes
/// the diagnostics.
/// What the editor said it can do (`initialize`).
#[derive(Clone, Copy)]
struct Client {
    /// Positions in UTF-8 bytes rather than UTF-16 units.
    utf8: bool,
    /// Progress shown for work the server starts (`window.workDoneProgress`).
    progress: bool,
}

/// The editor's answers to the server's own requests (creating a progress
/// token), from the main thread.
type Responses = mpsc::Receiver<Value>;

/// Shows "emdysi: Checking NAME" in the editor until [`Progress::end`],
/// if the editor shows progress and agrees to the token within a second.
struct Progress<'a> {
    out: &'a Out,
    token: Option<String>,
}

impl<'a> Progress<'a> {
    fn begin(out: &'a Out, client: Client, responses: &Responses, n: usize, uri: &str) -> Self {
        let mut p = Progress { out, token: None };
        if !client.progress {
            return p;
        }
        let token = format!("emdysi-{n}");
        send(
            out,
            &json!({"jsonrpc": "2.0", "id": token, "method": "window/workDoneProgress/create",
                    "params": {"token": token}}),
        );
        let deadline = Instant::now() + Duration::from_secs(1);
        while let Ok(r) = responses.recv_timeout(deadline.saturating_duration_since(Instant::now()))
        {
            if r["id"] == token.as_str() {
                if r.get("error").is_none() {
                    let name = uri.rsplit('/').next().unwrap_or(uri);
                    send(
                        out,
                        &json!({"jsonrpc": "2.0", "method": "$/progress", "params": {"token": token,
                                "value": {"kind": "begin", "title": "emdysi",
                                          "message": format!("Checking {name}")}}}),
                    );
                    p.token = Some(token);
                }
                break;
            }
        }
        p
    }

    fn end(self) {
        if let Some(token) = self.token {
            send(
                self.out,
                &json!({"jsonrpc": "2.0", "method": "$/progress",
                        "params": {"token": token, "value": {"kind": "end"}}}),
            );
        }
    }
}

fn checker(
    args: Args,
    jobs: mpsc::Receiver<Job>,
    responses: Responses,
    out: Out,
    found: Found,
    client: Client,
) {
    let utf8 = client.utf8;
    let mut started = 0;
    let loaded = load_grammar(&args)
        .and_then(|(erg, cache)| Ok((erg, cache, make_checker(&args, &args.packs)?)));
    let (erg, cache, checker) = match loaded {
        Ok(x) => x,
        Err(e) => {
            send(
                &out,
                &json!({"jsonrpc": "2.0", "method": "window/showMessage",
                        "params": {"type": 1, "message": format!("emdysi: {e}")}}),
            );
            return;
        }
    };
    let mut model = crate::load_model(&args).ok().flatten();
    // Documents checked in full at least once.
    let mut checked: std::collections::HashSet<String> = Default::default();
    while let Ok(first) = jobs.recv() {
        // Of the jobs waiting, only the newest for each document counts.
        let mut todo: Vec<Job> = vec![first];
        todo.extend(jobs.try_iter());
        let mut latest: Vec<(String, Option<Doc>)> = Vec::new();
        for j in todo {
            let (uri, doc) = match j {
                Job::Check(u, d) => (u, Some(d)),
                Job::Forget(u) => (u, None),
            };
            latest.retain(|(u, _)| *u != uri);
            latest.push((uri, doc));
        }
        for (uri, doc) in latest {
            let Some(doc) = doc else {
                found.lock().unwrap().remove(&uri);
                checked.remove(&uri);
                continue;
            };
            let format = if doc.markdown {
                Format::Markdown
            } else {
                Format::Plain
            };
            let publish = |diags: &[Diagnostic]| {
                let items: Vec<Value> = diags
                    .iter()
                    .map(|d| {
                        json!({
                            "range": lsp_range(&doc.text, &d.range, utf8),
                            "severity": match d.severity {
                                Severity::Error => 1,
                                Severity::Warning => 2,
                                Severity::Suggestion => 3,
                            },
                            "code": d.rule,
                            "source": "emdysi",
                            "message": d.message,
                        })
                    })
                    .collect();
                send(
                    &out,
                    &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                            "params": {"uri": uri, "version": doc.version, "diagnostics": items}}),
                );
            };
            // The first check of a document parses every sentence, which
            // takes minutes for a long one: the findings that need no parse
            // (spelling, words and phrases) go out first. They are always
            // among the full check's findings (evaluation.md), so nothing
            // shown is taken back. Later checks parse only what changed,
            // and go straight to the full findings.
            let first = checked.insert(uri.clone());
            // A first check parses every sentence: minutes for a long
            // document, so the editor shows that it is under way.
            let progress = if first {
                started += 1;
                Some(Progress::begin(&out, client, &responses, started, &uri))
            } else {
                None
            };
            if args.opts.parse && first {
                let quick = Options {
                    parse: false,
                    ..args.opts.clone()
                };
                let a = analyze(&erg, &doc.text, format, &quick);
                let diags = check(&checker, &erg, &a, None, &args);
                publish(&diags);
                found
                    .lock()
                    .unwrap()
                    .insert(uri.clone(), (doc.text.clone(), diags));
            }
            let mut a = analyze(&erg, &doc.text, format, &args.opts);
            if args.context_readings {
                emdysi_check::decisions::prefer_document_phrases(&mut a, READING_MARGIN);
            }
            let diags = check(&checker, &erg, &a, model.as_mut(), &args);
            publish(&diags);
            if let Some(p) = progress {
                p.end();
            }
            found.lock().unwrap().insert(uri, (doc.text, diags));
        }
        if let Some(c) = &cache {
            emdysi_parse::prune_parse_cache(c, crate::cache_bytes());
        }
    }
}

/// Quick fixes for the diagnostics that touch `range`: the fix `en fix`
/// would apply first, then the other suggestions.
fn code_actions(found: &Found, uri: &str, range: &Value, utf8: bool) -> Value {
    let found = found.lock().unwrap();
    let Some((text, diags)) = found.get(uri) else {
        return json!([]);
    };
    let from = offset(text, &range["start"], utf8);
    let to = offset(text, &range["end"], utf8);
    let mut actions = Vec::new();
    for d in diags {
        if d.range.start > to || d.range.end < from {
            continue;
        }
        let fixes = d.replacement.iter().chain(&d.suggestions);
        let mut seen = Vec::new();
        for (i, rep) in fixes.enumerate() {
            if seen.contains(&rep) {
                continue;
            }
            seen.push(rep);
            actions.push(json!({
                "title": format!("Replace with '{rep}' ({})", d.rule),
                "kind": "quickfix",
                "isPreferred": i == 0 && d.replacement.is_some(),
                "edit": {"changes": {uri: [{"range": lsp_range(text, &d.range, utf8), "newText": rep}]}},
            }));
        }
    }
    Value::Array(actions)
}

pub fn serve(args: Args) -> Result<bool, String> {
    let out: Out = Arc::new(Mutex::new(std::io::stdout()));
    let (msgs_tx, msgs) = mpsc::channel::<Value>();
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        let mut input = stdin.lock();
        while let Some(m) = read_message(&mut input) {
            if msgs_tx.send(m).is_err() {
                break;
            }
        }
    });
    let found: Found = Arc::default();
    let (responses_tx, responses) = mpsc::channel::<Value>();
    let mut responses = Some(responses);
    let mut jobs: Option<mpsc::Sender<Job>> = None;
    let mut utf8 = false;
    let mut docs: HashMap<String, Doc> = HashMap::new();
    // Documents to check, and when they are due.
    let mut due: HashMap<String, Instant> = HashMap::new();
    let mut exit_ok = false;
    loop {
        let now = Instant::now();
        for uri in due
            .iter()
            .filter(|(_, t)| **t <= now)
            .map(|(u, _)| u.clone())
            .collect::<Vec<_>>()
        {
            due.remove(&uri);
            if let (Some(d), Some(j)) = (docs.get(&uri), &jobs) {
                let _ = j.send(Job::Check(uri, d.clone()));
            }
        }
        let wait = due.values().min().map_or(Duration::from_secs(3600), |t| {
            t.saturating_duration_since(now)
        });
        let msg = match msgs.recv_timeout(wait) {
            Ok(m) => m,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => break,
        };
        // (The time the message came, not the time the wait began.)
        let now = Instant::now();
        // An answer to one of the server's own requests (it has an id and
        // no method) goes to the checker; it is never answered.
        let Some(method) = msg.get("method").and_then(Value::as_str) else {
            let _ = responses_tx.send(msg);
            continue;
        };
        let id = msg.get("id").cloned();
        let params = msg.get("params").cloned().unwrap_or(json!({}));
        let reply = |result: Value| {
            if let Some(id) = &id {
                send(&out, &json!({"jsonrpc": "2.0", "id": id, "result": result}));
            }
        };
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        match method {
            "initialize" => {
                utf8 = params
                    .pointer("/capabilities/general/positionEncodings")
                    .and_then(Value::as_array)
                    .is_some_and(|a| a.iter().any(|e| e == "utf-8"));
                let client = Client {
                    utf8,
                    progress: params.pointer("/capabilities/window/workDoneProgress")
                        == Some(&Value::Bool(true)),
                };
                // The checker starts now, so that the grammar loads while
                // the editor opens its documents.
                if let Some(responses) = responses.take() {
                    let (tx, rx) = mpsc::channel();
                    let (args, out, found) = (args.clone(), out.clone(), found.clone());
                    std::thread::spawn(move || checker(args, rx, responses, out, found, client));
                    jobs = Some(tx);
                }
                reply(json!({
                    "capabilities": {
                        "positionEncoding": if utf8 { "utf-8" } else { "utf-16" },
                        "textDocumentSync": {"openClose": true, "change": 1, "save": {"includeText": false}},
                        "codeActionProvider": {"codeActionKinds": ["quickfix"]},
                    },
                    "serverInfo": {"name": "emdysi", "version": env!("CARGO_PKG_VERSION")},
                }));
            }
            "shutdown" => {
                exit_ok = true;
                reply(Value::Null)
            }
            "exit" => break,
            "textDocument/didOpen" => {
                let td = &params["textDocument"];
                docs.insert(
                    uri.clone(),
                    Doc {
                        text: td["text"].as_str().unwrap_or("").to_string(),
                        version: td["version"].as_i64().unwrap_or(0),
                        markdown: is_markdown(&uri, td["languageId"].as_str()),
                    },
                );
                due.insert(uri, now);
            }
            "textDocument/didChange" => {
                if let Some(d) = docs.get_mut(&uri) {
                    // Full sync: the last change holds the whole text.
                    if let Some(t) = params["contentChanges"]
                        .as_array()
                        .and_then(|c| c.last())
                        .and_then(|c| c["text"].as_str())
                    {
                        d.text = t.to_string();
                    }
                    d.version = params["textDocument"]["version"]
                        .as_i64()
                        .unwrap_or(d.version);
                    due.insert(uri, now + PAUSE);
                }
            }
            "textDocument/didSave" => {
                if docs.contains_key(&uri) {
                    due.insert(uri, now);
                }
            }
            "textDocument/didClose" => {
                docs.remove(&uri);
                due.remove(&uri);
                if let Some(j) = &jobs {
                    let _ = j.send(Job::Forget(uri.clone()));
                }
                send(
                    &out,
                    &json!({"jsonrpc": "2.0", "method": "textDocument/publishDiagnostics",
                            "params": {"uri": uri, "diagnostics": []}}),
                );
            }
            "textDocument/codeAction" => reply(code_actions(&found, &uri, &params["range"], utf8)),
            _ => {
                // An unknown request gets an error; an unknown
                // notification, nothing.
                if let Some(id) = &id {
                    send(
                        &out,
                        &json!({"jsonrpc": "2.0", "id": id,
                                "error": {"code": -32601, "message": format!("unknown method {method:?}")}}),
                    );
                }
            }
        }
    }
    Ok(exit_ok)
}
