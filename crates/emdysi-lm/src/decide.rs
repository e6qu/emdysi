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

pub struct Decider<'a> {
    pub lm: &'a mut dyn LanguageModel,
    /// Calibration temperature ([`fit_temperature`]); 1 keeps the model's
    /// own probabilities.
    pub temperature: f64,
    /// Ask again with the options reversed and average the two answers.
    pub debias: bool,
}

impl<'a> Decider<'a> {
    pub fn new(lm: &'a mut dyn LanguageModel) -> Decider<'a> {
        Decider {
            lm,
            temperature: 1.0,
            debias: true,
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
