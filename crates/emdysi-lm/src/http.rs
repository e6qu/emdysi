//! A model behind an OpenAI-compatible HTTP API (feature `http`): MLX's
//! `mlx_lm.server` on Apple silicon, LM Studio (MLX or GGUF), Ollama,
//! llama.cpp's `llama-server`, vLLM and others, usually on localhost.
//!
//! Generation uses `POST /v1/chat/completions`. Decisions use the same
//! endpoint with `logprobs` and `top_logprobs` for the first token of the
//! reply, which `llama-server`, `mlx_lm.server` and OpenAI-compatible
//! servers in general return. Scoring text uses `POST /v1/completions`
//! with `echo` and `logprobs`; not every server returns the log-probabilities
//! of the echoed prompt (`llama-server` and `mlx_lm.server` do not), and
//! then candidates are ranked by their order instead of the model's
//! likelihood.

use std::time::Duration;

use serde_json::{Value, json};

use crate::LanguageModel;

pub struct HttpLm {
    /// Base URL, e.g. `http://127.0.0.1:8080`.
    pub base: String,
    /// Model name to send, if the server needs one.
    pub model: Option<String>,
    /// How many top tokens to ask for (OpenAI-compatible servers allow up
    /// to 20; some fewer, and the request is retried with 10).
    pub top_logprobs: usize,
    agent: ureq::Agent,
}

impl HttpLm {
    pub fn new(base: &str, model: Option<String>) -> HttpLm {
        let agent = ureq::Agent::config_builder()
            .timeout_global(Some(Duration::from_secs(120)))
            .http_status_as_error(false)
            .build()
            .new_agent();
        HttpLm {
            base: base.trim_end_matches('/').to_string(),
            model,
            top_logprobs: 20,
            agent,
        }
    }

    fn post(&self, path: &str, mut body: Value) -> Option<Value> {
        if let Some(m) = &self.model {
            body["model"] = json!(m);
        }
        let mut resp = self
            .agent
            .post(&format!("{}{path}", self.base))
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let text = resp.body_mut().read_to_string().ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// A token as servers spell it, without the marks byte-level BPE
/// (`Ġ`, `Ċ`) and SentencePiece (`▁`) vocabularies use for spaces and
/// newlines, and without surrounding white space.
fn plain_token(t: &str) -> String {
    t.replace(['Ġ', '▁'], " ")
        .replace('Ċ', "\n")
        .trim()
        .to_string()
}

/// The top tokens and log-probabilities of the first generated token in a
/// chat completion response.
fn top_tokens(v: &Value) -> Option<Vec<(String, f64)>> {
    let first = &v["choices"][0]["logprobs"]["content"][0];
    let top = first["top_logprobs"].as_array()?;
    let mut out: Vec<(String, f64)> = top
        .iter()
        .filter_map(|t| Some((t["token"].as_str()?.to_string(), t["logprob"].as_f64()?)))
        .collect();
    if let (Some(t), Some(l)) = (first["token"].as_str(), first["logprob"].as_f64()) {
        if !out.iter().any(|(k, _)| k == t) {
            out.push((t.to_string(), l));
        }
    }
    (!out.is_empty()).then_some(out)
}

impl LanguageModel for HttpLm {
    fn first_token(
        &mut self,
        system: &str,
        user: &str,
        labels: &[&str],
    ) -> Option<Vec<Option<f64>>> {
        let ask = |top: usize| {
            self.post(
                "/v1/chat/completions",
                json!({
                    "messages": [
                        {"role": "system", "content": system},
                        {"role": "user", "content": user},
                    ],
                    "max_tokens": 1,
                    "temperature": 0,
                    "logprobs": true,
                    "top_logprobs": top,
                    "stream": false,
                }),
            )
            .as_ref()
            .and_then(top_tokens)
        };
        let tokens = match ask(self.top_logprobs) {
            Some(t) => t,
            None if self.top_logprobs > 10 => {
                let t = ask(10)?;
                self.top_logprobs = 10;
                t
            }
            None => return None,
        };
        // Spellings of a label with and without a leading space are one
        // answer.
        Some(
            labels
                .iter()
                .map(|l| {
                    tokens
                        .iter()
                        .filter(|(t, _)| plain_token(t) == *l)
                        .map(|&(_, lp)| lp)
                        .reduce(crate::log_add)
                })
                .collect(),
        )
    }

    fn logprob(&mut self, prefix: &str, text: &str) -> Option<(f64, usize)> {
        let v = self.post(
            "/v1/completions",
            json!({
                "prompt": format!("{prefix}{text}"),
                "max_tokens": 1,
                "temperature": 0,
                "echo": true,
                "logprobs": 1,
            }),
        )?;
        let lp = &v["choices"][0]["logprobs"];
        let tokens = lp["token_logprobs"].as_array()?;
        let offsets = lp["text_offset"].as_array();
        let end = prefix.len() + text.len();
        let (mut sum, mut n) = (0.0, 0);
        for (i, t) in tokens.iter().enumerate() {
            let Some(l) = t.as_f64() else { continue };
            let at = offsets
                .and_then(|o| o.get(i))
                .and_then(Value::as_u64)
                .map(|x| x as usize);
            match at {
                // Only the tokens of `text`, not the prefix or the one
                // generated token.
                Some(a) if a >= prefix.len() && a < end => {}
                Some(_) => continue,
                None if i + 1 == tokens.len() => continue,
                None => {}
            }
            sum += l;
            n += 1;
        }
        (n > 0).then_some((sum, n))
    }

    fn generate(
        &mut self,
        system: &str,
        user: &str,
        max_tokens: usize,
        temperature: f32,
        seed: u32,
    ) -> Option<String> {
        let v = self.post(
            "/v1/chat/completions",
            json!({
                "messages": [
                    {"role": "system", "content": system},
                    {"role": "user", "content": user},
                ],
                "max_tokens": max_tokens,
                "temperature": temperature,
                "seed": seed,
                "stream": false,
            }),
        )?;
        let content = v["choices"][0]["message"]["content"].as_str()?;
        let line = content.lines().map(str::trim).find(|l| !l.is_empty())?;
        Some(line.to_string())
    }
}
