//! A model behind an OpenAI-compatible HTTP API (feature `http`): MLX's
//! `mlx_lm.server` on Apple silicon, LM Studio (MLX or GGUF), Ollama,
//! llama.cpp's `llama-server`, vLLM and others, usually on localhost.
//!
//! Generation uses `POST /v1/chat/completions`. Scoring uses `POST
//! /v1/completions` with `echo` and `logprobs`, which not every server
//! supports; without it, candidates are ranked by their order instead of
//! the model's likelihood.

use std::time::Duration;

use serde_json::{Value, json};

use crate::LanguageModel;

pub struct HttpLm {
    /// Base URL, e.g. `http://127.0.0.1:8080`.
    pub base: String,
    /// Model name to send, if the server needs one.
    pub model: Option<String>,
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

impl LanguageModel for HttpLm {
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
