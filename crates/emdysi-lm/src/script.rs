//! A scripted stand-in for a language model, for tests and for trying the
//! pipeline without one (`script:FILE`).
//!
//! A script is a list of tab-separated directives, tried in order; the
//! first whose pattern occurs in the input applies (an empty pattern
//! matches everything). Lines starting with `#` are comments.
//!
//! ```text
//! first<TAB>Which spelling is American English?<TAB>A=0.2 B=0.8
//! generate<TAB>wordy<TAB>The results were clear.
//! logprob<TAB>clear<TAB>-0.5
//! ```
//!
//! - `first`: probabilities of the first token of the reply, by label;
//!   labels not listed are unscored (as when a server's top tokens leave
//!   them out).
//! - `generate`: the reply to a chat whose user message contains the
//!   pattern.
//! - `logprob`: the log-probability per token of a text containing the
//!   pattern (tokens are words here).

use crate::LanguageModel;

#[derive(Debug, Clone, PartialEq)]
enum Directive {
    First(String, Vec<(String, f64)>),
    Generate(String, String),
    Logprob(String, f64),
}

#[derive(Debug, Clone, Default)]
pub struct Script {
    directives: Vec<Directive>,
}

impl Script {
    pub fn parse(src: &str) -> Result<Script, String> {
        let mut directives = Vec::new();
        for (i, line) in src.lines().enumerate() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let err = |m: &str| format!("script line {}: {m}", i + 1);
            let parts: Vec<&str> = line.split('\t').collect();
            let [kind, pattern, value] = parts[..] else {
                return Err(err("expected three tab-separated fields"));
            };
            let pattern = pattern.to_string();
            directives.push(match kind {
                "first" => {
                    let mut probs = Vec::new();
                    for item in value.split_whitespace() {
                        let (label, p) = item
                            .split_once('=')
                            .ok_or_else(|| err("expected LABEL=PROBABILITY"))?;
                        let p: f64 = p.parse().map_err(|_| err("bad probability"))?;
                        if !(0.0..=1.0).contains(&p) {
                            return Err(err("probability outside 0..1"));
                        }
                        probs.push((label.to_string(), p));
                    }
                    Directive::First(pattern, probs)
                }
                "generate" => Directive::Generate(pattern, value.to_string()),
                "logprob" => Directive::Logprob(
                    pattern,
                    value.parse().map_err(|_| err("bad log-probability"))?,
                ),
                k => return Err(err(&format!("unknown directive {k:?}"))),
            });
        }
        Ok(Script { directives })
    }
}

impl LanguageModel for Script {
    fn logprob(&mut self, prefix: &str, text: &str) -> Option<(f64, usize)> {
        let _ = prefix;
        let n = text.split_whitespace().count().max(1);
        self.directives.iter().find_map(|d| match d {
            Directive::Logprob(p, lp) if text.contains(p.as_str()) => Some((lp * n as f64, n)),
            _ => None,
        })
    }

    fn generate(&mut self, _: &str, user: &str, _: usize, _: f32, _: u32) -> Option<String> {
        self.directives.iter().find_map(|d| match d {
            Directive::Generate(p, reply) if user.contains(p.as_str()) => Some(reply.clone()),
            _ => None,
        })
    }

    fn first_token(
        &mut self,
        _system: &str,
        user: &str,
        labels: &[&str],
    ) -> Option<Vec<Option<f64>>> {
        let probs = self.directives.iter().find_map(|d| match d {
            Directive::First(p, probs) if user.contains(p.as_str()) => Some(probs),
            _ => None,
        })?;
        Some(
            labels
                .iter()
                .map(|l| {
                    probs
                        .iter()
                        .find(|(k, _)| k == l)
                        .map(|(_, p)| if *p > 0.0 { p.ln() } else { f64::NEG_INFINITY })
                })
                .collect(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn directives() {
        let mut s = Script::parse(
            "# comment\nfirst\tAmerican\tA=0.25 B=0.75\ngenerate\twordy\tShort.\nlogprob\t\t-1\n",
        )
        .unwrap();
        let lp = s
            .first_token("", "Which is American?", &["A", "B", "C"])
            .unwrap();
        assert!((lp[0].unwrap() - 0.25f64.ln()).abs() < 1e-12);
        assert!(lp[2].is_none());
        assert!(s.first_token("", "other", &["A"]).is_none());
        assert_eq!(
            s.generate("", "too wordy", 10, 0.0, 0).as_deref(),
            Some("Short.")
        );
        assert_eq!(s.logprob("", "two words"), Some((-2.0, 2)));
        assert!(Script::parse("first\tx\tA=2\n").is_err());
    }
}
