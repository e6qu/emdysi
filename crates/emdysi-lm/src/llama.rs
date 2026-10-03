//! A local model through llama.cpp (feature `llama`): any GGUF model file,
//! e.g. a small instruction-tuned model such as Gemma 4 E2B/E4B, Qwen3.5
//! or Phi-4-mini. Weights are never bundled: pass a file path, and pin its
//! SHA-256 if you want the load to fail on a different file.

use std::num::NonZeroU32;
use std::path::Path;

use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{LlamaChatMessage, LlamaModel};
use llama_cpp_2::sampling::LlamaSampler;
use llama_cpp_2::token::LlamaToken;

use crate::LanguageModel;

pub struct LlamaLm {
    backend: LlamaBackend,
    model: LlamaModel,
    n_ctx: u32,
    threads: i32,
}

#[derive(Debug)]
pub struct LoadError(pub String);

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for LoadError {}

impl LlamaLm {
    /// Load a GGUF model. `n_ctx` bounds prompt plus output length (at most
    /// the length the model was trained with).
    pub fn load(path: &Path, n_ctx: u32) -> Result<LlamaLm, LoadError> {
        let backend = LlamaBackend::init().map_err(|e| LoadError(e.to_string()))?;
        let model = LlamaModel::load_from_file(&backend, path, &LlamaModelParams::default())
            .map_err(|e| LoadError(format!("{}: {e}", path.display())))?;
        let threads = std::thread::available_parallelism().map_or(4, |n| n.get() as i32);
        let trained = model.n_ctx_train();
        let n_ctx = if trained > 0 {
            n_ctx.min(trained)
        } else {
            n_ctx
        };
        Ok(LlamaLm {
            backend,
            model,
            n_ctx,
            threads,
        })
    }

    fn context(&self) -> Option<llama_cpp_2::context::LlamaContext<'_>> {
        let params = LlamaContextParams::default()
            .with_n_ctx(NonZeroU32::new(self.n_ctx))
            .with_n_batch(self.n_ctx)
            .with_n_threads(self.threads);
        self.model.new_context(&self.backend, params).ok()
    }

    fn tokenize(&self, text: &str, bos: bool) -> Vec<LlamaToken> {
        self.model.vocab().tokenize(text.as_bytes(), bos, true)
    }

    fn piece(&self, t: LlamaToken) -> String {
        String::from_utf8_lossy(&self.model.vocab().token_to_piece(t, false, None)).to_string()
    }

    /// The chat prompt in the model's own template, or a plain fallback.
    fn chat(&self, system: &str, user: &str) -> String {
        let messages = [
            LlamaChatMessage::new("system".into(), system.into()),
            LlamaChatMessage::new("user".into(), user.into()),
        ];
        if let (Ok(tmpl), [Ok(s), Ok(u)]) = (self.model.chat_template(None), messages) {
            if let Ok(p) = self.model.apply_chat_template(&tmpl, &[s, u], true) {
                return p;
            }
        }
        format!("{system}\n\n{user}\n")
    }
}

fn log_softmax_at(logits: &[f32], i: usize) -> f64 {
    let max = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max) as f64;
    let sum: f64 = logits.iter().map(|&l| (l as f64 - max).exp()).sum();
    logits[i] as f64 - max - sum.ln()
}

impl LanguageModel for LlamaLm {
    fn first_token(
        &mut self,
        system: &str,
        user: &str,
        labels: &[&str],
    ) -> Option<Vec<Option<f64>>> {
        let prompt = self.chat(system, user);
        let tokens = self.tokenize(&prompt, true);
        if tokens.is_empty() || tokens.len() as u32 >= self.n_ctx {
            return None;
        }
        let mut ctx = self.context()?;
        let mut batch = LlamaBatch::new(tokens.len(), 1);
        let last = tokens.len() - 1;
        for (i, &t) in tokens.iter().enumerate() {
            batch.add(t, i as i32, &[0], i == last).ok()?;
        }
        ctx.decode(&mut batch).ok()?;
        let logits = ctx.get_logits_ith(last as i32);
        // A label is the first token of its spelling, with or without a
        // leading space; both count.
        Some(
            labels
                .iter()
                .map(|l| {
                    let mut ids: Vec<LlamaToken> = Vec::new();
                    for form in [l.to_string(), format!(" {l}")] {
                        if let Some(&t) = self.tokenize(&form, false).first() {
                            if self.piece(t).trim() == *l && !ids.contains(&t) {
                                ids.push(t);
                            }
                        }
                    }
                    ids.iter()
                        .map(|t| log_softmax_at(logits, t.0 as usize))
                        .reduce(crate::log_add)
                })
                .collect(),
        )
    }

    fn logprob(&mut self, prefix: &str, text: &str) -> Option<(f64, usize)> {
        let head = self.tokenize(prefix, true);
        let all = self.tokenize(&format!("{prefix}{text}"), true);
        if all.len() <= head.len() || all.len() as u32 >= self.n_ctx {
            return None;
        }
        let mut ctx = self.context()?;
        let mut batch = LlamaBatch::new(all.len(), 1);
        for (i, &t) in all.iter().enumerate() {
            batch.add(t, i as i32, &[0], true).ok()?;
        }
        ctx.decode(&mut batch).ok()?;
        let mut sum = 0.0;
        // Each token of `text` is predicted by the logits at the previous
        // position; the first token after BOS needs a predecessor.
        let start = head.len().max(1);
        for (i, t) in all.iter().enumerate().skip(start) {
            let logits = ctx.get_logits_ith(i as i32 - 1);
            sum += log_softmax_at(logits, t.0 as usize);
        }
        Some((sum, all.len() - start))
    }

    fn generate(
        &mut self,
        system: &str,
        user: &str,
        max_tokens: usize,
        temperature: f32,
        seed: u32,
    ) -> Option<String> {
        let prompt = self.chat(system, user);
        let tokens = self.tokenize(&prompt, true);
        if tokens.len() + max_tokens >= self.n_ctx as usize {
            return None;
        }
        let mut ctx = self.context()?;
        let mut batch = LlamaBatch::new(self.n_ctx as usize, 1);
        let last = tokens.len() - 1;
        for (i, &t) in tokens.iter().enumerate() {
            batch.add(t, i as i32, &[0], i == last).ok()?;
        }
        ctx.decode(&mut batch).ok()?;
        let mut sampler = if temperature <= 0.0 {
            LlamaSampler::greedy()
        } else {
            LlamaSampler::chain_simple([
                LlamaSampler::top_p(0.95, 1),
                LlamaSampler::temp(temperature),
                LlamaSampler::dist(seed),
            ])
        };
        let mut out = String::new();
        for pos in (tokens.len() as i32..).take(max_tokens) {
            let t = sampler.sample(&ctx, batch.n_tokens() - 1);
            sampler.accept(t);
            if self.model.vocab().is_eog(t) {
                break;
            }
            let piece = self.piece(t);
            if piece.contains('\n') && !out.trim().is_empty() {
                out.push_str(piece.split('\n').next().unwrap_or(""));
                break;
            }
            out.push_str(&piece);
            batch.clear();
            batch.add(t, pos, &[0], true).ok()?;
            ctx.decode(&mut batch).ok()?;
        }
        Some(out.trim().to_string())
    }
}
