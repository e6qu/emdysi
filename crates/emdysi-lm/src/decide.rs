//! Decisions with typed probabilities (decision D12).
//!
//! In the manner of decision-only models such as Jev, a question gets a
//! probability for each of a fixed set of answers instead of a written
//! reply. Three kinds of question:
//!
//! - [`Decider::choose`]: which of up to ten options;
//! - [`Decider::holds`]: the probability that a statement is true;
//! - [`Decider::score`]: a distribution over the points of a small scale.
//!
//! Any [`LanguageModel`] that implements [`LanguageModel::first_token`] can
//! answer. The options are labelled with letters and the model is asked
//! for one letter; the probabilities of the letters as the first token of
//! its reply, renormalised over the options, are the answer. That needs
//! only the probabilities of generated tokens, which llama.cpp in-process,
//! `llama-server`, `mlx_lm.server` and other OpenAI-compatible servers all
//! give (servers only for their top tokens: an option outside them gets
//! the probability of the least likely token returned, an upper bound).
//!
//! Models prefer some positions; with [`Decider::debias`] the question is
//! asked again with the options in reverse order and the two answers are
//! averaged. Their probabilities are also often overconfident:
//! [`fit_temperature`] finds the temperature that calibrates them on
//! examples with known answers, and [`report`] measures accuracy and
//! calibration.

use crate::LanguageModel;

/// Option labels; a question has at most this many options.
pub const LETTERS: [&str; 10] = ["A", "B", "C", "D", "E", "F", "G", "H", "I", "J"];

/// The system message of every decision.
pub const SYSTEM: &str = "You make decisions. Reply with only the letter of your choice.";

/// The user message for a question: the context (if any), the question,
/// and the labelled options, one per line.
pub fn prompt(context: &str, question: &str, options: &[&str]) -> String {
    let mut lines: Vec<String> = Vec::new();
    if !context.trim().is_empty() {
        lines.push(context.trim().to_string());
        lines.push(String::new());
    }
    lines.push(format!("Question: {question}"));
    for (l, o) in LETTERS.iter().zip(options) {
        lines.push(format!("{l}. {o}"));
    }
    lines.join("\n")
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecideError {
    /// Fewer than two options.
    TooFewOptions,
    /// More options than [`LETTERS`].
    TooManyOptions(usize),
    /// The model gives no token probabilities.
    Unsupported,
    /// None of the options' labels got a probability.
    Unscored,
}

impl std::fmt::Display for DecideError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DecideError::TooFewOptions => f.write_str("a decision needs at least two options"),
            DecideError::TooManyOptions(n) => {
                write!(f, "{n} options; at most {} are supported", LETTERS.len())
            }
            DecideError::Unsupported => f.write_str("the model gives no token probabilities"),
            DecideError::Unscored => f.write_str("the model gave no option a probability"),
        }
    }
}

impl std::error::Error for DecideError {}

/// The model's raw label log-probabilities for one question, once per
/// order the options were asked in, each mapped back to the original
/// order of the options.
#[derive(Debug, Clone, PartialEq)]
pub struct Raw {
    pub orders: Vec<Vec<f64>>,
}

/// Probabilities of the options from raw log-probabilities at a
/// calibration temperature: a softmax per order, averaged over orders.
pub fn probabilities(raw: &Raw, temperature: f64) -> Vec<f64> {
    let n = raw.orders.first().map_or(0, Vec::len);
    let mut out = vec![0.0; n];
    for lp in &raw.orders {
        let scaled: Vec<f64> = lp.iter().map(|l| l / temperature).collect();
        let max = scaled.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let sum: f64 = scaled.iter().map(|s| (s - max).exp()).sum();
        for (o, s) in out.iter_mut().zip(&scaled) {
            *o += (s - max).exp() / sum / raw.orders.len() as f64;
        }
    }
    out
}

/// The surprise (negative log-probability) of each of `spans` of `src`,
/// from scored tokens: a token counts for the span its first non-space
/// character is in.
fn tally(tokens: Vec<(usize, f64)>, src: &str, spans: &[(usize, usize)]) -> Vec<f64> {
    let mut out = vec![0.0; spans.len()];
    for (at, lp) in tokens {
        let rest = src.get(at..).unwrap_or("");
        let at = at + rest.len() - rest.trim_start().len();
        if let Some(k) = spans.iter().position(|&(f, t)| f <= at && at < t) {
            out[k] -= lp;
        }
    }
    out
}

/// The surprise of each of `words` alone: scored as a list, one per line.
pub fn alone(lm: &mut dyn LanguageModel, words: &[&str]) -> Option<Vec<f64>> {
    let list = words.join("\n");
    let mut at = 0;
    let spans: Vec<(usize, usize)> = words
        .iter()
        .map(|w| {
            let span = (at, at + w.len());
            at += w.len() + 1;
            span
        })
        .collect();
    Some(tally(lm.token_logprobs("", &list)?, &list, &spans))
}

pub struct Decider<'a> {
    pub lm: &'a mut dyn LanguageModel,
    /// Calibration temperature ([`fit_temperature`]); 1 keeps the model's
    /// own probabilities.
    pub temperature: f64,
    /// Ask again with the options reversed and average the two answers.
    pub debias: bool,
    /// Score each option by the model's probability of its text after the
    /// context, instead of asking for a letter: for options that are
    /// versions of one sentence. Needs a backend that scores text
    /// ([`LanguageModel::logprob`]); the question is not shown.
    pub likelihood: bool,
    /// With `likelihood`, the options' log-probabilities are divided by
    /// this before they become probabilities: a version of a sentence is
    /// often a few nats more likely than the writer's without being what
    /// they meant (|need to have| against |need to save|), so a choice
    /// should take a large margin. At 6 (2026-10-20), 0.8 against a single
    /// alternative takes about 8.3 nats: on 558 questions from real-error
    /// paragraphs, 5 of 527 correct sentences changed against 9 at 5, for
    /// one correction of 19 fewer.
    pub likelihood_temperature: f64,
}

/// The default [`Decider::likelihood_temperature`].
pub const LIKELIHOOD_TEMPERATURE: f64 = 6.0;

impl<'a> Decider<'a> {
    pub fn new(lm: &'a mut dyn LanguageModel) -> Decider<'a> {
        Decider {
            lm,
            temperature: 1.0,
            debias: true,
            likelihood: false,
            likelihood_temperature: LIKELIHOOD_TEMPERATURE,
        }
    }

    /// The raw label log-probabilities of a question.
    pub fn raw(
        &mut self,
        context: &str,
        question: &str,
        options: &[&str],
    ) -> Result<Raw, DecideError> {
        let n = options.len();
        if n < 2 {
            return Err(DecideError::TooFewOptions);
        }
        if n > LETTERS.len() {
            return Err(DecideError::TooManyOptions(n));
        }
        if self.likelihood {
            let prefix = if context.trim().is_empty() {
                String::new()
            } else {
                format!("{} ", context.trim())
            };
            let t = self.likelihood_temperature;
            let lp = options
                .iter()
                .map(|o| self.lm.logprob(&prefix, o.trim()).map(|(lp, _)| lp / t))
                .collect::<Option<Vec<f64>>>()
                .ok_or(DecideError::Unsupported)?;
            return Ok(Raw { orders: vec![lp] });
        }
        let identity: Vec<usize> = (0..n).collect();
        let mut orders = vec![identity.clone()];
        if self.debias {
            orders.push(identity.into_iter().rev().collect());
        }
        let mut out = Vec::new();
        for order in orders {
            let asked: Vec<&str> = order.iter().map(|&i| options[i]).collect();
            let user = prompt(context, question, &asked);
            let lp = self
                .lm
                .first_token(SYSTEM, &user, &LETTERS[..n])
                .ok_or(DecideError::Unsupported)?;
            let known: Vec<f64> = lp.iter().flatten().copied().collect();
            if known.is_empty() {
                return Err(DecideError::Unscored);
            }
            // An unscored label was less likely than every token returned.
            let floor = known.iter().copied().fold(f64::INFINITY, f64::min);
            let mut back = vec![0.0; n];
            for (pos, &i) in order.iter().enumerate() {
                back[i] = lp[pos].unwrap_or(floor);
            }
            out.push(back);
        }
        Ok(Raw { orders: out })
    }

    /// The probability of each option, in the order given.
    pub fn choose(
        &mut self,
        context: &str,
        question: &str,
        options: &[&str],
    ) -> Result<Vec<f64>, DecideError> {
        let raw = self.raw(context, question, options)?;
        Ok(probabilities(&raw, self.temperature))
    }

    /// The probability of each of `versions` of one sentence: by their
    /// likelihood after the context when the model scores text (much more
    /// accurate for small models than a letter, which they choose largely
    /// by position), else as the answer to `question`.
    pub fn versions(
        &mut self,
        context: &str,
        question: &str,
        versions: &[&str],
    ) -> Result<Vec<f64>, DecideError> {
        let letters = !self.likelihood;
        self.likelihood = true;
        let p = self.choose(context, question, versions);
        self.likelihood = !letters;
        match p {
            Err(DecideError::Unsupported) if letters => self.choose(context, question, versions),
            p => p,
        }
    }

    /// How surprising each of `spans` (byte ranges of words in `text`) is
    /// where it stands, after `context`: the negative log-probability of
    /// its tokens in the text, less that of the same word among the text's
    /// words listed one per line. A rare word is surprising anywhere and
    /// scores about 0; a word that does not fit where it stands (a doubled
    /// *the*, *allows to*) scores high, and so does the word after a gap.
    /// Needs a backend that scores tokens ([`LanguageModel::token_logprobs`]).
    pub fn surprise(
        &mut self,
        context: &str,
        text: &str,
        spans: &[(usize, usize)],
    ) -> Result<Vec<f64>, DecideError> {
        let prefix = if context.trim().is_empty() {
            String::new()
        } else {
            format!("{} ", context.trim())
        };
        let here = self
            .lm
            .token_logprobs(&prefix, text)
            .ok_or(DecideError::Unsupported)?;
        let here = tally(here, text, spans);
        let words: Vec<&str> = spans.iter().map(|&(f, t)| &text[f..t]).collect();
        let alone = alone(&mut *self.lm, &words).ok_or(DecideError::Unsupported)?;
        Ok(here.iter().zip(alone).map(|(h, a)| h - a).collect())
    }

    /// The probability that `statement` is true.
    pub fn holds(&mut self, context: &str, statement: &str) -> Result<f64, DecideError> {
        let q = format!("Is this true? {statement}");
        Ok(self.choose(context, &q, &["yes", "no"])?[0])
    }

    /// A distribution over the points `low..=high` of a scale.
    pub fn score(
        &mut self,
        context: &str,
        question: &str,
        low: i32,
        high: i32,
    ) -> Result<Vec<(i32, f64)>, DecideError> {
        let points: Vec<i32> = (low..=high).collect();
        let labels: Vec<String> = points.iter().map(i32::to_string).collect();
        let options: Vec<&str> = labels.iter().map(String::as_str).collect();
        let p = self.choose(context, question, &options)?;
        Ok(points.into_iter().zip(p).collect())
    }
}

/// A question with a known answer, for calibration and evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct Sample {
    pub raw: Raw,
    /// Index of the right option.
    pub right: usize,
}

/// Mean negative log-likelihood of the right answers.
fn nll(samples: &[Sample], t: f64) -> f64 {
    let sum: f64 = samples
        .iter()
        .map(|s| -probabilities(&s.raw, t)[s.right].max(1e-12).ln())
        .sum();
    sum / samples.len().max(1) as f64
}

/// The temperature in [0.05, 20] that minimises the negative
/// log-likelihood of the right answers (golden-section search on its
/// logarithm).
pub fn fit_temperature(samples: &[Sample]) -> f64 {
    if samples.is_empty() {
        return 1.0;
    }
    let f = |x: f64| nll(samples, x.exp());
    let (mut a, mut b) = (0.05f64.ln(), 20f64.ln());
    let g = (5f64.sqrt() - 1.0) / 2.0;
    let mut c = b - g * (b - a);
    let mut d = a + g * (b - a);
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..60 {
        if fc < fd {
            b = d;
            d = c;
            fd = fc;
            c = b - g * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + g * (b - a);
            fd = f(d);
        }
    }
    ((a + b) / 2.0).exp()
}

/// Accuracy and calibration on samples with known answers.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    pub n: usize,
    /// Share of samples whose most probable option is the right one.
    pub accuracy: f64,
    /// Mean negative log-likelihood of the right answers.
    pub nll: f64,
    /// Mean Brier score (squared error over all options).
    pub brier: f64,
    /// Expected calibration error: the gap between confidence and accuracy,
    /// over ten bins of the top probability.
    pub ece: f64,
}

pub fn report(samples: &[Sample], temperature: f64) -> Report {
    let n = samples.len();
    let mut right = 0;
    let mut brier = 0.0;
    let mut bins = [(0usize, 0.0f64, 0usize); 10];
    for s in samples {
        let p = probabilities(&s.raw, temperature);
        let best = (0..p.len())
            .max_by(|&i, &j| p[i].total_cmp(&p[j]))
            .unwrap_or(0);
        let hit = best == s.right;
        right += hit as usize;
        brier += p
            .iter()
            .enumerate()
            .map(|(i, q)| (q - (i == s.right) as u8 as f64).powi(2))
            .sum::<f64>();
        let b = ((p[best] * 10.0) as usize).min(9);
        bins[b].0 += 1;
        bins[b].1 += p[best];
        bins[b].2 += hit as usize;
    }
    let ece = bins
        .iter()
        .map(|&(_, conf, hits)| (conf - hits as f64).abs() / n.max(1) as f64)
        .sum();
    Report {
        n,
        accuracy: right as f64 / n.max(1) as f64,
        nll: nll(samples, temperature),
        brier: brier / n.max(1) as f64,
        ece,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Script;

    #[test]
    fn prompt_format() {
        assert_eq!(
            prompt("Some text.", "Which?", &["one", "two"]),
            "Some text.\n\nQuestion: Which?\nA. one\nB. two"
        );
        assert_eq!(
            prompt("", "Which?", &["x", "y"]),
            "Question: Which?\nA. x\nB. y"
        );
    }

    #[test]
    fn choose_with_debiasing() {
        // The scripted model always prefers A at 0.6: position bias only.
        let mut lm = Script::parse("first\t\tA=0.6 B=0.4\n").unwrap();
        let mut d = Decider::new(&mut lm);
        let p = d.choose("", "Which?", &["x", "y"]).unwrap();
        assert!((p[0] - 0.5).abs() < 1e-9 && (p[1] - 0.5).abs() < 1e-9);
        d.debias = false;
        let p = d.choose("", "Which?", &["x", "y"]).unwrap();
        assert!((p[0] - 0.6).abs() < 1e-9);
    }

    #[test]
    fn choose_by_likelihood() {
        // The letters favour A, the texts favour the second option.
        let mut lm =
            Script::parse("first\t\tA=0.9 B=0.1\nlogprob\twere\t-1\nlogprob\twas\t-2\n").unwrap();
        let mut d = Decider::new(&mut lm);
        d.likelihood = true;
        d.likelihood_temperature = 1.0;
        let p = d
            .choose("", "Which?", &["They was here.", "They were here."])
            .unwrap();
        // Three words each: -6 against -3.
        assert!((p[1] - 1.0 / (1.0 + (-3.0f64).exp())).abs() < 1e-9, "{p:?}");
        // At the default temperature the margin counts for less.
        d.likelihood_temperature = LIKELIHOOD_TEMPERATURE;
        let q = d
            .choose("", "Which?", &["They was here.", "They were here."])
            .unwrap();
        let t = LIKELIHOOD_TEMPERATURE;
        assert!(
            (q[1] - 1.0 / (1.0 + (-3.0 / t).exp())).abs() < 1e-9,
            "{q:?}"
        );
        d.likelihood_temperature = 1.0;
        // Versions of a sentence are scored by likelihood without asking.
        d.likelihood = false;
        let v = d
            .versions("", "Which?", &["They was here.", "They were here."])
            .unwrap();
        assert_eq!(v, p);
        assert!(!d.likelihood);
        // A backend that cannot score text cannot answer, except with
        // letters.
        let mut lm = Script::parse("first\t\tA=0.9 B=0.1\n").unwrap();
        let mut d = Decider::new(&mut lm);
        d.debias = false;
        let v = d.versions("", "Which?", &["x", "y"]).unwrap();
        assert!((v[0] - 0.9).abs() < 1e-9);
        d.likelihood = true;
        assert_eq!(
            d.choose("", "Which?", &["x", "y"]),
            Err(DecideError::Unsupported)
        );
    }

    #[test]
    fn surprise_of_words() {
        // The second |the| is unlikely after the first; alone, every word
        // has the default -1.
        let mut lm = Script::parse("logprob\tthe the\t-6\nlogprob\tkubelet\t-9\n").unwrap();
        let mut d = Decider::new(&mut lm);
        let text = "Restart the the kubelet.";
        let spans = [(0, 7), (8, 11), (12, 15), (16, 24)];
        let s = d.surprise("", text, &spans).unwrap();
        // |kubelet.| does not end with |kubelet|: -1 like the rest.
        assert_eq!(s, vec![0.0, 0.0, 5.0, 0.0]);
    }

    #[test]
    fn unscored_labels_get_the_floor() {
        let mut lm = Script::parse("first\t\tA=0.5 B=0.25\n").unwrap();
        let mut d = Decider::new(&mut lm);
        d.debias = false;
        let p = d.choose("", "Which?", &["x", "y", "z"]).unwrap();
        // C is unscored and gets B's log-probability.
        assert!((p[1] - p[2]).abs() < 1e-9 && p[0] > p[1]);
        assert_eq!(
            d.choose("", "Which?", &["x"]),
            Err(DecideError::TooFewOptions)
        );
    }

    #[test]
    fn calibration() {
        // Overconfident: 0.9 for the predicted option, right 70% of the time.
        let raw = |first: bool| Raw {
            orders: vec![if first {
                vec![0.9f64.ln(), 0.1f64.ln()]
            } else {
                vec![0.1f64.ln(), 0.9f64.ln()]
            }],
        };
        let samples: Vec<Sample> = (0..100)
            .map(|i| Sample {
                raw: raw(true),
                right: if i < 70 { 0 } else { 1 },
            })
            .collect();
        let t = fit_temperature(&samples);
        let p = probabilities(&samples[0].raw, t);
        assert!((p[0] - 0.7).abs() < 0.01, "{p:?} at {t}");
        let r = report(&samples, t);
        assert!((r.accuracy - 0.7).abs() < 1e-9 && r.ece < 0.01);
        assert!(report(&samples, 1.0).ece > 0.15);
    }
}
