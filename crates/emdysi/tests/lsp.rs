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

/// `en lsp` with `args`: its stdin, the messages it sends, and the process.
fn start(
    args: &[&str],
) -> (
    std::process::ChildStdin,
    mpsc::Receiver<Value>,
    std::process::Child,
) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_en"))
        .arg("lsp")
        .args(args)
        .env("EMDYSI_NO_CACHE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let stdin = child.stdin.take().unwrap();
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
    (stdin, rx, child)
}

#[test]
fn lsp_session() {
    let (mut stdin, rx, mut child) = start(&["--no-parse"]);
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

#[test]
fn quick_findings_first_then_the_full_ones_with_progress() {
    let (mut stdin, rx, mut child) = start(&[]);
    let mut send = |v: Value| stdin.write_all(&frame(&v)).unwrap();
    let next = || -> Value {
        rx.recv_timeout(Duration::from_secs(120))
            .expect("no message")
    };
    // An editor that shows progress.
    send(json!({"jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": {"capabilities": {"window": {"workDoneProgress": true}}}}));
    assert_eq!(next()["id"], 1);
    send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didOpen", "params": {"textDocument":
        {"uri": "file:///a.txt", "languageId": "plaintext", "version": 1,
         "text": "The new version are faster. We recieved it.\n"}}}),
    );
    // The server asks for a progress token; the answer is not answered.
    let create = next();
    assert_eq!(
        create["method"], "window/workDoneProgress/create",
        "{create}"
    );
    send(json!({"jsonrpc": "2.0", "id": create["id"], "result": null}));
    let token = create["params"]["token"].clone();
    // Then: progress begins, the quick findings, the full ones, progress ends.
    let mut seen = Vec::new();
    loop {
        let m = next();
        assert!(m.get("error").is_none(), "{m}");
        if m["method"] == "$/progress" {
            assert_eq!(m["params"]["token"], token);
            let kind = m["params"]["value"]["kind"].as_str().unwrap().to_string();
            seen.push((kind.clone(), Vec::new()));
            if kind == "end" {
                break;
            }
        } else if m["method"] == "textDocument/publishDiagnostics" {
            let rules: Vec<String> = m["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .map(|d| d["code"].as_str().unwrap().to_string())
                .collect();
            seen.push(("publish".to_string(), rules));
        }
    }
    let kinds: Vec<&str> = seen.iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kinds, ["begin", "publish", "publish", "end"]);
    // Spelling needs no parse; the agreement error does.
    let (quick, full) = (&seen[1].1, &seen[2].1);
    assert!(quick.contains(&"core.spelling".to_string()), "{quick:?}");
    assert!(
        !quick.contains(&"core.grammar-errors".to_string()),
        "{quick:?}"
    );
    assert!(
        full.contains(&"core.grammar-errors".to_string()),
        "{full:?}"
    );
    assert!(quick.iter().all(|r| full.contains(r)), "{quick:?} {full:?}");
    // A later check goes straight to the full findings, without progress.
    send(
        json!({"jsonrpc": "2.0", "method": "textDocument/didSave", "params": {
        "textDocument": {"uri": "file:///a.txt"}}}),
    );
    let again = next();
    assert_eq!(
        again["method"], "textDocument/publishDiagnostics",
        "{again}"
    );
    assert!(again.to_string().contains("core.grammar-errors"), "{again}");
    send(json!({"jsonrpc": "2.0", "id": 2, "method": "shutdown"}));
    send(json!({"jsonrpc": "2.0", "method": "exit"}));
    assert!(child.wait().unwrap().success());
}
