//! The HTTP backend against a minimal stand-in for an OpenAI-compatible
//! server such as MLX's `mlx_lm.server`.
#![cfg(feature = "http")]

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;

use emdysi_rewrite::LanguageModel;
use emdysi_rewrite::http::HttpLm;

/// Serve `n` requests, answering by path, and return the request bodies.
fn serve(n: usize) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let mut bodies = Vec::new();
        for stream in listener.incoming().take(n) {
            let mut stream = stream.unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut request_line = String::new();
            reader.read_line(&mut request_line).unwrap();
            let mut len = 0;
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                if h == "\r\n" || h.is_empty() {
                    break;
                }
                if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                    len = v.trim().parse().unwrap();
                }
            }
            let mut body = vec![0; len];
            reader.read_exact(&mut body).unwrap();
            bodies.push(String::from_utf8(body).unwrap());
            let reply = if request_line.contains("/v1/chat/completions") {
                r#"{"choices":[{"message":{"role":"assistant","content":"He goes to school every day.\nExtra line."}}]}"#
            } else {
                // "He goes" after the prefix "A: ": offsets 0 (prefix), 3, 6,
                // and one generated token at the end.
                r#"{"choices":[{"text":"A: He goes.","logprobs":{"tokens":["A:"," He"," goes","."],"token_logprobs":[null,-1.5,-0.5,-9.0],"text_offset":[0,3,6,11]}}]}"#
            };
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{reply}",
                reply.len()
            )
            .unwrap();
        }
        bodies
    });
    (url, handle)
}

#[test]
fn openai_compatible_server() {
    let (url, handle) = serve(2);
    let mut lm = HttpLm::new(&url, Some("mlx-community/test-model".into()));
    let out = lm.generate("system", "user", 32, 0.0, 1).unwrap();
    assert_eq!(out, "He goes to school every day.");
    let (lp, n) = lm.logprob("A: ", "He goes").unwrap();
    assert_eq!(n, 2);
    assert!((lp - -2.0).abs() < 1e-9, "{lp}");
    let bodies = handle.join().unwrap();
    assert!(
        bodies[0].contains("\"model\":\"mlx-community/test-model\""),
        "{}",
        bodies[0]
    );
    assert!(bodies[0].contains("\"role\":\"system\""));
    assert!(bodies[1].contains("\"echo\":true"));
}
