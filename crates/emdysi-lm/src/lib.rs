//! Pluggable local language models (decisions D10 and D12).
//!
//! A [`LanguageModel`] scores text, generates continuations and, for the
//! decision engine ([`decide`]), gives the probabilities of the first token
//! of its reply. Backends:
//!
//! - [`llama`] (feature `llama`): a GGUF file run in-process by llama.cpp;
//!   exact probabilities for every token.
//! - [`http`] (feature `http`, on by default): any server with the
//!   OpenAI-compatible API, such as llama.cpp's `llama-server` (GGUF),
//!   `mlx_lm.server` (MLX, Apple silicon), Ollama or LM Studio; the
//!   probabilities of the top tokens the server returns.
//! - [`script`]: a scripted stand-in, for tests and for trying the pipeline
//!   without a model.
//!
//! Weights are never bundled: a model is a file or a server the user
//! points to ([`open`]).

pub mod decide;
#[cfg(feature = "http")]
pub mod http;
#[cfg(feature = "llama")]
pub mod llama;
pub mod script;

/// A language model: scores text and generates continuations.
pub trait LanguageModel {
    /// Log-probability (natural log) of `text` following `prefix`, and the
    /// number of tokens of `text`. `None` if the backend cannot score text.
    fn logprob(&mut self, prefix: &str, text: &str) -> Option<(f64, usize)>;
    /// Generate a continuation of a chat prompt: `system` instructions and
    /// a `user` message. `temperature` 0 is greedy; `seed` makes sampling
    /// reproducible. Generation stops at the end of a line.
    fn generate(
        &mut self,
        system: &str,
        user: &str,
        max_tokens: usize,
        temperature: f32,
        seed: u32,
    ) -> Option<String>;
    /// Log-probabilities that the reply to a chat (`system`, `user`) starts
    /// with each of `labels`: short strings such as "A" and "B" that are a
    /// single token in most vocabularies. An entry is `None` when the
    /// backend could not score that label (e.g. it was not among the top
    /// tokens a server returned); the result is `None` when the backend
    /// gives no probabilities at all.
    fn first_token(
        &mut self,
        _system: &str,
        _user: &str,
        _labels: &[&str],
    ) -> Option<Vec<Option<f64>>> {
        None
    }
}

/// Open a model from a specification:
///
/// - `PATH.gguf` or `gguf:PATH`: a GGUF file through llama.cpp (needs the
///   `llama` feature);
/// - `http://HOST:PORT` (optionally `#MODEL` for the model name to request):
///   an OpenAI-compatible server;
/// - `script:PATH`: a scripted stand-in ([`script::Script`]).
pub fn open(spec: &str) -> Result<Box<dyn LanguageModel>, String> {
    if let Some(path) = spec.strip_prefix("script:") {
        let src = std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))?;
        return Ok(Box::new(script::Script::parse(&src)?));
    }
    if spec.starts_with("http://") || spec.starts_with("https://") {
        return open_http(spec);
    }
    let path = spec.strip_prefix("gguf:").unwrap_or(spec);
    open_gguf(std::path::Path::new(path))
}

#[cfg(feature = "http")]
fn open_http(spec: &str) -> Result<Box<dyn LanguageModel>, String> {
    let (url, model) = match spec.split_once('#') {
        Some((u, m)) => (u, Some(m.to_string())),
        None => (spec, None),
    };
    Ok(Box::new(http::HttpLm::new(url, model)))
}

#[cfg(not(feature = "http"))]
fn open_http(_: &str) -> Result<Box<dyn LanguageModel>, String> {
    Err("a model server needs a build with the `http` feature".into())
}

#[cfg(feature = "llama")]
fn open_gguf(path: &std::path::Path) -> Result<Box<dyn LanguageModel>, String> {
    llama::LlamaLm::load(path, 4096)
        .map(|m| Box::new(m) as Box<dyn LanguageModel>)
        .map_err(|e| format!("loading model: {e}"))
}

#[cfg(not(feature = "llama"))]
fn open_gguf(_: &std::path::Path) -> Result<Box<dyn LanguageModel>, String> {
    Err(
        "a GGUF model needs a build with the `llama` feature: cargo install --features llama \
         (or serve the model, e.g. with llama-server or MLX, and give its URL)"
            .into(),
    )
}

/// `ln(exp(a) + exp(b))`.
pub(crate) fn log_add(a: f64, b: f64) -> f64 {
    if a == f64::NEG_INFINITY {
        return b;
    }
    if b == f64::NEG_INFINITY {
        return a;
    }
    let m = a.max(b);
    m + ((a - m).exp() + (b - m).exp()).ln()
}
