//! `en lsp` end to end: diagnostics on open, at UTF-16 positions; two
//! quick edits checked once, as the newest version; a quick fix; and a
//! clean shutdown.

use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::Duration;

use serde_json::{Value, json};

fn frame(v: &Value) -> Vec<u8> {
    let body = v.to_string();
    format!("Content-Length: {}\r\n\r\n{body}", body.len()).into_bytes()
}

#[test]
fn lsp_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_en"))
        .args(["lsp", "--no-parse"])
        .env("EMDYSI_NO_CACHE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let (tx, rx) = mpsc::channel::<Value>();
    std::thread::spawn(move || {
        let mut r = BufReader::new(stdout);
        loop {
            let mut len = 0;
            loop {
                let mut line = String::new();
                if r.read_line(&mut line).unwrap_or(0) == 0 {
                    return;
                }
                if line.trim().is_empty() {
                    break;
                }
                if let Some(n) = line.strip_prefix("Content-Length:") {
                    len = n.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; len];
            r.read_exact(&mut body).unwrap();
            if tx.send(serde_json::from_slice(&body).unwrap()).is_err() {
                return;
            }
        }
    });
    let mut send = |v: Value| stdin.write_all(&frame(&v)).unwrap();
    let next = |pred: &dyn Fn(&Value) -> bool| -> Value {
        loop {
            let m = rx
                .recv_timeout(Duration::from_secs(60))
                .expect("no message");
            if pred(&m) {
                return m;
            }
        }
    };
    send(
        json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {"capabilities": {}}}),
    );
    let init = next(&|m| m["id"] == 1);
    assert_eq!(init["result"]["capabilities"]["positionEncoding"], "utf-16");
    send(json!({"jsonrpc": "2.0", "method": "initialized", "params": {}}));

    let uri = "file:///notes.md";
    // "é" and "𝒳" are one and two UTF-16 units (two and four bytes):
    // "recieved" starts at unit 11, byte 15.
    let text = "Caf\u{e9} \u{1d4b3} we recieved it.\n";
    send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument":
        {"uri": uri, "languageId": "markdown", "version": 1, "text": text}}}),
    );
    let published = |m: &Value| m["method"] == "textDocument/publishDiagnostics";
    let open = next(&published);
    assert_eq!(open["params"]["version"], 1);
    let spelling = open["params"]["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == "core.spelling")
        .expect("a spelling diagnostic")
        .clone();
    assert_eq!(
        spelling["range"],
        json!({"start": {"line": 0, "character": 11}, "end": {"line": 0, "character": 19}})
    );

    // Two quick edits: one check, of the second.
    for (v, t) in [(2, "We recieved it.\n"), (3, "We recieved them.\n")] {
        send(
            json!({"jsonrpc": "2.0", "method": "textDocument/didChange", "params": {
            "textDocument": {"uri": uri, "version": v}, "contentChanges": [{"text": t}]}}),
        );
    }
    let changed = next(&published);
    assert_eq!(changed["params"]["version"], 3);

    send(
        json!({"jsonrpc": "2.0", "id": 2, "method": "textDocument/codeAction", "params": {
        "textDocument": {"uri": uri},
        "range": {"start": {"line": 0, "character": 4}, "end": {"line": 0, "character": 5}},
        "context": {"diagnostics": []}}}),
    );
    let actions = next(&|m| m["id"] == 2);
    let fix = &actions["result"][0];
    assert_eq!(fix["isPreferred"], true);
    assert_eq!(
        fix["edit"]["changes"][uri][0],
        json!({"range": {"start": {"line": 0, "character": 3}, "end": {"line": 0, "character": 11}},
               "newText": "received"})
    );

    send(json!({"jsonrpc": "2.0", "id": 3, "method": "shutdown"}));
    next(&|m| m["id"] == 3);
    send(json!({"jsonrpc": "2.0", "method": "exit"}));
    assert!(child.wait().unwrap().success());
}
