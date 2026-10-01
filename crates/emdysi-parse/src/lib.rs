//! The parsing pipeline: REPP tokenization, POS tagging, token mapping,
//! lexical lookup and chart parsing with the English Resource Grammar.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::time::Duration;

use emdysi_hpsg::chartmap::{ChartMapper, Lattice, MapRule, compile_rules, string_at};
use emdysi_hpsg::labels::{Labeler, Tree};
use emdysi_hpsg::lexicon::{LexPaths, Lexicon};
use emdysi_hpsg::morph::Morphology;
use emdysi_hpsg::parser::{
    Deriv, EdgeKind, ParseResult, Parser, ParserConfig, QuickCheck, Rule, derivation,
};
use emdysi_hpsg::{Dag, Grammar, Unifier};
use emdysi_repp::Repp;

pub mod rank;
pub mod tagger;

#[derive(Debug)]
pub struct Error(pub String);

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err(e: impl std::fmt::Display) -> Error {
    Error(e.to_string())
}

/// Location of the vendored ERG inside this repository.
pub fn default_grammar_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../grammar/erg")
}

pub struct Erg {
    pub grammar: Grammar,
    pub repp: Repp,
    pub token_mapping: Vec<MapRule>,
    pub lexical_filtering: Vec<MapRule>,
    pub lexicon: Lexicon,
    pub rules: Vec<Rule>,
    pub qc: QuickCheck,
    pub config: ParserConfig,
    pub labeler: Labeler,
    pub model: rank::Model,
    /// How many of the best readings get a labelled tree.
    pub trees_for: usize,
}

/// The parse-ranking model trained on the vendored gold profiles.
pub const DEFAULT_MODEL: &str = include_str!("../data/rank.tsv");

/// Read a `key := value value ... .` setting from an ACE configuration file.
fn ace_setting(src: &str, key: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut active = false;
    for line in src.lines() {
        let line = line.split(';').next().unwrap_or("").trim();
        if !active {
            if let Some(rest) = line.strip_prefix(key) {
                if let Some(v) = rest.trim_start().strip_prefix(":=") {
                    active = true;
                    let v = v.trim();
                    let done = v.ends_with('.');
                    out.extend(v.trim_end_matches('.').split_whitespace().map(String::from));
                    if done {
                        break;
                    }
                }
            }
        } else {
            let done = line.ends_with('.');
            out.extend(
                line.trim_end_matches('.')
                    .split_whitespace()
                    .map(String::from),
            );
            if done {
                break;
            }
        }
    }
    out
}

/// One analysis of a sentence.
#[derive(Debug, Clone)]
pub struct Reading {
    pub root: String,
    pub derivation: String,
    /// Labelled phrase-structure tree (for the best readings only).
    pub tree: Option<Tree>,
    /// Ranking features and model score.
    pub features: Vec<String>,
    pub score: f64,
}

#[derive(Debug, Clone)]
pub struct Parse {
    pub tokens: Vec<InputToken>,
    pub readings: Vec<Reading>,
    pub edges: usize,
    pub lexical_items: usize,
    pub exhausted: bool,
    pub elapsed: Duration,
}

/// A part-of-speech hypothesis for a token (Penn Treebank tag set).
#[derive(Debug, Clone)]
pub struct Tag {
    pub tag: String,
    pub prob: f64,
}

/// An input token after REPP, with POS hypotheses.
#[derive(Debug, Clone)]
pub struct InputToken {
    pub form: String,
    pub from: usize,
    pub to: usize,
    pub tags: Vec<Tag>,
}

fn tdl_string(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        if c == '"' || c == '\\' {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('"');
    out
}

impl Erg {
    pub fn load(dir: &Path) -> Result<Erg, Error> {
        let loaded =
            emdysi_tdl::load(&dir.join("english.tdl"), emdysi_tdl::Env::Type).map_err(err)?;
        let grammar = Grammar::compile(&loaded).map_err(err)?;
        let repp = emdysi_repp::erg(dir).map_err(err)?;
        let mut u = Unifier::new();
        let token_mapping =
            compile_rules(&grammar, "token-mapping-rule", &mut u).map_err(|e| err(e.0))?;
        let lexical_filtering =
            compile_rules(&grammar, "lexical-filtering-rule", &mut u).map_err(|e| err(e.0))?;
        let config_src = std::fs::read_to_string(dir.join("ace/config.tdl")).map_err(err)?;

        let mut morph = Morphology::from_grammar(&grammar);
        morph.load_irregs(&std::fs::read_to_string(dir.join("irregs.tab")).map_err(err)?);
        let path = |p: &str| {
            grammar
                .path(p)
                .ok_or_else(|| err(format!("unknown path {p}")))
        };
        let lex_paths = LexPaths {
            tokens_list: path("TOKENS +LIST")?,
            tokens_last: path("TOKENS +LAST")?,
            token_form: path("+FORM")?,
        };

        let qc = QuickCheck::parse_ace(
            &grammar,
            &std::fs::read_to_string(dir.join("ace/ace-erg-qc.txt")).map_err(err)?,
        );
        let args = grammar.feat("ARGS").ok_or_else(|| err("no ARGS feature"))?;
        let spanning: HashSet<String> = ace_setting(&config_src, "spanning-only-rules")
            .into_iter()
            .collect();
        let mut rules = Vec::new();
        for inst in &grammar.instances {
            let lexical = match inst.status.as_deref() {
                Some("rule") => false,
                Some("lex-rule") => true,
                _ => continue,
            };
            let dag = grammar
                .expand(&inst.body, &mut u)
                .map_err(|e| err(format!("{}: {e}", inst.name)))?;
            let mut rule = Rule::new(
                &grammar,
                &inst.name,
                Arc::new(dag),
                lexical,
                morph.is_orth_rule(&inst.name),
                &qc,
                args,
            );
            rule.spanning_only = spanning.contains(&inst.name);
            rules.push(rule);
        }

        let mut roots = Vec::new();
        for name in ace_setting(&config_src, "parsing-roots") {
            let inst = grammar
                .instance(&name)
                .ok_or_else(|| err(format!("no root {name}")))?;
            let dag = grammar.expand(&inst.body, &mut u).map_err(Error)?;
            roots.push((name, Arc::new(dag)));
        }
        let deleted_daughters = ace_setting(&config_src, "deleted-daughters")
            .iter()
            .filter_map(|f| grammar.feat(f))
            .collect();
        let packing_restrictor = Some(
            ace_setting(&config_src, "parsing-packing-restrictor")
                .iter()
                .filter_map(|f| grammar.feat(f))
                .collect(),
        );
        let config = ParserConfig {
            deleted_daughters,
            roots,
            max_edges: 200_000,
            timeout: Duration::from_secs(60),
            packing_restrictor,
            max_readings: 1000,
            packing_top_type: ace_setting(&config_src, "generalize-edge-top-types")
                .first()
                .filter(|v| v.as_str() == "enabled")
                .and_then(|_| grammar.ts.hier.id("sign")),
        };
        let lexicon = Lexicon::new(&grammar, morph, lex_paths);
        let labeler = Labeler::new(&grammar, &mut u).ok_or_else(|| err("no parse-node labels"))?;
        Ok(Erg {
            model: rank::Model::parse(DEFAULT_MODEL),
            trees_for: 3,
            labeler,
            grammar,
            repp,
            token_mapping,
            lexical_filtering,
            lexicon,
            rules,
            qc,
            config,
        })
    }

    /// Tokenize and tag a sentence.
    pub fn tokens(&self, text: &str) -> Vec<InputToken> {
        self.repp
            .tokenize(text)
            .into_iter()
            .enumerate()
            .map(|(i, t)| InputToken {
                tags: tagger::tag(&t.form, i == 0),
                form: t.form,
                from: t.from,
                to: t.to,
            })
            .collect()
    }

    /// Parse one sentence.
    pub fn parse(&self, text: &str) -> Result<Parse, Error> {
        let t0 = std::time::Instant::now();
        let tokens = self.tokens(text);
        let (lat, _) = self.map_tokens(&tokens, false)?;
        let mut u = Unifier::new();
        let items = self.lexicon.instantiate(&self.grammar, &lat, &mut u);
        let n_items = items.len();
        let parser = Parser::new(
            &self.grammar,
            &self.rules,
            &self.qc,
            &self.config,
            &self.lexical_filtering,
        );
        let result: ParseResult = parser.parse(&lat, items);
        let form_path = self.lexicon.paths.token_form.clone();
        let forms = |toks: &[usize]| -> String {
            toks.iter()
                .filter_map(|&t| string_at(&self.grammar.ts, &lat.entries[t].dag, &form_path))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let chars: Vec<char> = text.chars().collect();
        let (from_path, to_path) = (self.grammar.path("+FROM"), self.grammar.path("+TO"));
        let surface = |d: &Deriv| -> Option<String> {
            let EdgeKind::Lex { tokens, .. } = &d.kind else {
                return None;
            };
            let span = |t: usize, p: &Option<Vec<emdysi_hpsg::FeatId>>| {
                string_at(&self.grammar.ts, &lat.entries[t].dag, p.as_ref()?)?
                    .parse::<usize>()
                    .ok()
            };
            let from = span(*tokens.first()?, &from_path)?;
            let to = span(*tokens.last()?, &to_path)?;
            Some(chars.get(from..to)?.iter().collect())
        };
        let mut scored: Vec<(f64, Vec<String>, &emdysi_hpsg::parser::Reading)> = result
            .readings
            .iter()
            .map(|r| {
                let features = rank::features(&self.grammar, &self.rules, &r.deriv, &r.root);
                (self.model.score(&features), features, r)
            })
            .collect();
        scored.sort_by(|a, b| b.0.total_cmp(&a.0));
        let readings = scored
            .into_iter()
            .enumerate()
            .map(|(i, (score, features, r))| Reading {
                root: r.root.clone(),
                derivation: derivation(&self.grammar, &self.rules, &r.deriv, &forms),
                // Labelling costs unifications; only the best readings get trees.
                tree: (i < self.trees_for).then(|| {
                    self.labeler
                        .tree(&self.grammar, &mut u, &r.deriv, &surface)
                        .collapse()
                }),
                score,
                features,
            })
            .collect();
        Ok(Parse {
            tokens,
            readings,
            edges: result.chart.len(),
            lexical_items: n_items,
            exhausted: result.exhausted,
            elapsed: t0.elapsed(),
        })
    }

    /// The feature structure of an input token, as the token-mapping rules
    /// expect it.
    pub fn token_dag(&self, i: usize, t: &InputToken, u: &mut Unifier) -> Result<Dag, Error> {
        let tags: Vec<String> = t.tags.iter().map(|x| tdl_string(&x.tag)).collect();
        let prbs: Vec<String> = t
            .tags
            .iter()
            .map(|x| tdl_string(&format!("{:.4}", x.prob)))
            .collect();
        let src = format!(
            "t := token & [ +FORM {}, +FROM {}, +TO {}, +ID <! {} !>, +TNT [ +TAGS < {} >, +PRBS < {} > ] ].",
            tdl_string(&t.form),
            tdl_string(&t.from.to_string()),
            tdl_string(&t.to.to_string()),
            tdl_string(&i.to_string()),
            tags.join(", "),
            prbs.join(", ")
        );
        let loaded =
            emdysi_tdl::load_str(&src, "token", emdysi_tdl::Env::Instance(None)).map_err(err)?;
        self.grammar
            .expand(&loaded.entries[0].def.body, u)
            .map_err(Error)
    }

    /// Build the initial token lattice and apply the token-mapping rules.
    pub fn map_tokens(
        &self,
        tokens: &[InputToken],
        trace: bool,
    ) -> Result<(Lattice, Vec<String>), Error> {
        let mut u = Unifier::new();
        let mut lat = Lattice::default();
        let mut prev = lat.add_vertex(0.0);
        for (i, t) in tokens.iter().enumerate() {
            let next = lat.add_vertex((i + 1) as f64);
            let dag = self.token_dag(i, t, &mut u)?;
            lat.add(Arc::new(dag), prev, next);
            prev = next;
        }
        let mut mapper = ChartMapper::new(&self.grammar, &self.token_mapping);
        if trace {
            mapper.trace = Some(Vec::new());
        }
        mapper.apply(&mut lat);
        Ok((lat, mapper.trace.unwrap_or_default()))
    }
}
