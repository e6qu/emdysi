//! `en mcp` end to end: the protocol's handshake, the tool list, and a
//! check, a fix and an error, over the binary's stdin and stdout.

use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

#[test]
fn mcp_session() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_en"))
        .args(["mcp", "--no-parse"])
        .env("EMDYSI_NO_CACHE", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    let mut ask = |msg: &str| -> String {
        writeln!(stdin, "{msg}").unwrap();
        let mut line = String::new();
        stdout.read_line(&mut line).unwrap();
        line
    };
    let init = ask(
        r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"0"}}}"#,
    );
    assert!(init.contains(r#""protocolVersion":"2025-06-18""#), "{init}");
    assert!(init.contains(r#""tools":{}"#), "{init}");
    // A notification gets no answer: the next line answers the next request.
    let tools = ask(
        "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}",
    );
    assert!(
        tools.starts_with(r#"{"id":2,"#) || tools.contains(r#""id":2"#),
        "{tools}"
    );
    for name in ["check", "fix", "parse"] {
        assert!(tools.contains(&format!(r#""name":"{name}""#)), "{tools}");
    }
    let check = ask(
        r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"check","arguments":{"text":"We recieved the files.\n"}}}"#,
    );
    assert!(check.contains(r#""isError":false"#), "{check}");
    assert!(check.contains(r#""rule":"core.spelling""#), "{check}");
    assert!(check.contains(r#""fix":"received""#), "{check}");
    let fix = ask(
        r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"fix","arguments":{"text":"We recieved the files.\n"}}}"#,
    );
    assert!(
        fix.contains(r#""text":"We received the files.\n""#),
        "{fix}"
    );
    let missing = ask(
        r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"check","arguments":{}}}"#,
    );
    assert!(missing.contains(r#""isError":true"#), "{missing}");
    let unknown = ask(r#"{"jsonrpc":"2.0","id":6,"method":"resources/list"}"#);
    assert!(unknown.contains(r#""code":-32601"#), "{unknown}");
    drop(stdin);
    assert!(child.wait().unwrap().success());
}
