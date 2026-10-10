//! The parsing pipeline: REPP tokenization, POS tagging, token mapping,
//! lexical lookup and chart parsing with the English Resource Grammar.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use std::time::Duration;

use emdysi_hpsg::chartmap::{ChartMapper, Lattice, MapRule, compile_rules, string_at};
use emdysi_hpsg::labels::{Labeler, Tree};
use emdysi_hpsg::lexicon::{LexItem, LexPaths, Lexicon};
use emdysi_hpsg::morph::Morphology;
use emdysi_hpsg::parser::{
    Deriv, EdgeKind, ParseResult, Parser, ParserConfig, QuickCheck, Rule, derivation,
};
use emdysi_hpsg::{Dag, Grammar, Unifier};
use emdysi_repp::Repp;

pub mod ambiguity;
mod parse_cache;
pub mod rank;

pub use emdysi_hpsg::mrs;
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

/// Where the measurement tools cache parses (see [`Erg::cache_parses`]):
/// `$EMDYSI_PARSE_CACHE`, else `parses` in the grammar cache directory
/// (`$EMDYSI_CACHE_DIR`, `$XDG_CACHE_HOME/emdysi` or `~/.cache/emdysi`).
/// `None` when `EMDYSI_NO_CACHE` is set.
pub fn parse_cache_dir() -> Option<PathBuf> {
    if std::env::var_os("EMDYSI_NO_CACHE").is_some() {
        return None;
    }
    if let Some(d) = std::env::var_os("EMDYSI_PARSE_CACHE") {
        return Some(PathBuf::from(d));
    }
    emdysi_hpsg::cache::default_dir().map(|d| d.join("parses"))
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
    /// Orthography of each lexical entry, by instance index.
    pub orth: HashMap<usize, String>,
    /// How many of the best readings get a labelled tree.
    pub trees_for: usize,
    /// Keep each reading's feature structure ([`Reading::dag`]).
    pub keep_dags: bool,
    /// Lexical type of each instance, by instance index.
    pub le_types: Vec<String>,
    /// Chart-pruning beam for a first, faster pass (see `parse`).
    pub first_beam: Option<usize>,
    /// How to read semantics (MRS) out of a parse.
    pub mrs: Option<emdysi_hpsg::mrs::MrsConfig>,
    /// Problems found while loading (e.g. rules that could not be built).
    pub warnings: Vec<String>,
    /// The grammar directory.
    pub dir: PathBuf,
    /// The configuration file the grammar was loaded with, relative to
    /// [`Erg::dir`].
    pub config_file: String,
    /// Where parses are cached (see [`Erg::cache_parses`]): the directory
    /// given, and the one for this grammar and parser in it.
    parse_cache: Option<(PathBuf, PathBuf)>,
    /// The grammar-error ("mal-rule") variant of the grammar, loaded on
    /// first use (see [`Erg::mal`]).
    mal: std::sync::OnceLock<Option<Box<Erg>>>,
}

/// The ACE configuration of the ERG's grammar-error variant.
pub const MAL_CONFIG: &str = "ace/config-mal.tdl";

/// Edges kept per chart cell when pruning (see [`ParserConfig::cell_beam`]).
pub const DEFAULT_CELL_BEAM: usize = 40;

/// The parse-ranking model trained on the vendored gold profiles.
pub const DEFAULT_MODEL: &str = include_str!("../data/rank.tsv");

/// Whether a root condition name is the ERG's strict one (formal edited
/// text), in American or British spelling.
pub fn is_strict(root: &str) -> bool {
    root == "root_strict" || root == "root_strict_br"
}

/// Replace `DIALECT us` in the top-level feature structure of a root
/// condition with `DIALECT <to>`. Returns whether there was one.
fn set_dialect(body: &mut emdysi_tdl::Conj, to: &str) -> bool {
    let mut found = false;
    for t in &mut body.0 {
        if let emdysi_tdl::Term::Avm(fvs) = t {
            for fv in fvs {
                if fv.path.len() == 1 && fv.path[0].eq_ignore_ascii_case("DIALECT") {
                    for v in &mut fv.value.0 {
                        if let emdysi_tdl::Term::Type(n) = v {
                            if n.eq_ignore_ascii_case("us") {
                                *n = to.to_string();
                                found = true;
                            }
                        }
                    }
                }
            }
        }
    }
    found
}

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
    /// The derivation, flattened: node 0 is the top.
    pub nodes: Vec<Node>,
    /// The lexical items, in order.
    pub words: Vec<Word>,
    /// The feature structure of the whole sentence, kept only when
    /// [`Erg::keep_dags`] is set: a document's readings would otherwise
    /// hold gigabytes of structures that nothing reads after parsing.
    pub dag: Option<Arc<emdysi_hpsg::Dag>>,
    /// The semantics (Minimal Recursion Semantics) of the reading.
    pub mrs: Option<emdysi_hpsg::mrs::Mrs>,
}

/// Character span of a sequence of lattice tokens.
type TokenSpan<'a> = dyn Fn(&[usize]) -> Option<(usize, usize)> + 'a;

/// A node of a derivation tree.
#[derive(Debug, Clone)]
pub struct Node {
    /// Rule name, or lexical entry name for leaves.
    pub name: String,
    pub leaf: bool,
    /// Character span in the sentence.
    pub from: usize,
    pub to: usize,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
}

/// A word of a reading: a lexical entry with the lexical rules applied to it.
#[derive(Debug, Clone)]
pub struct Word {
    pub surface: String,
    /// Lexical entry name, e.g. `dog_n1`.
    pub entry: String,
    /// Dictionary form, e.g. `dog` for *dogs*.
    pub lemma: String,
    /// Lexical type, e.g. `n_-_c_le`.
    pub le_type: String,
    /// A generic entry for a word the grammar does not know.
    pub generic: bool,
    /// Lexical rules applied, innermost first (e.g. `n_pl_olr`).
    pub rules: Vec<String>,
    pub from: usize,
    pub to: usize,
    /// Index of the leaf in [`Reading::nodes`].
    pub node: usize,
}

#[derive(Debug, Clone)]
pub struct Parse {
    pub tokens: Vec<InputToken>,
    pub readings: Vec<Reading>,
    pub edges: usize,
    pub lexical_items: usize,
    pub exhausted: bool,
    /// Whether the search was complete: no chart pruning and no time or
    /// size limit cut it short, so no analysis was missed. A sentence
    /// without an analysis after a complete search is outside the grammar.
    pub complete: bool,
    pub elapsed: Duration,
    /// The ranking model's temperature: the probability of a reading among
    /// `readings` is proportional to `exp(score / temperature)`.
    pub temperature: f64,
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
    /// Load the grammar with its default parsing configuration
    /// (`ace/config.tdl`).
    pub fn load(dir: &Path) -> Result<Erg, Error> {
        Self::load_config(dir, "ace/config.tdl")
    }

    /// Load the grammar with an ACE configuration file, relative to `dir`
    /// (e.g. `ace/config-mal.tdl` for the grammar-error variant of the ERG).
    pub fn load_config(dir: &Path, config: &str) -> Result<Erg, Error> {
        let config_file = config.to_string();
        let config_path = dir.join(config);
        let config_src = std::fs::read_to_string(&config_path).map_err(err)?;
        let config_dir = config_path.parent().unwrap_or(dir).to_path_buf();
        let setting_path = |key: &str| -> Option<PathBuf> {
            ace_setting(&config_src, key)
                .first()
                .map(|f| config_dir.join(f.trim_matches('"')))
        };
        let top = setting_path("grammar-top").unwrap_or_else(|| dir.join("english.tdl"));
        let mut loaded = emdysi_tdl::load(&top, emdysi_tdl::Env::Type).map_err(err)?;
        // emdysi's extensions of the grammar (grammar/emdysi/*.tdl, e.g.
        // comparative correlatives), loaded after the ERG's own files.
        let ext_dir = dir.join("../emdysi");
        let mut exts: Vec<PathBuf> = std::fs::read_dir(&ext_dir)
            .map(|d| {
                d.filter_map(|e| e.ok().map(|e| e.path()))
                    .filter(|p| p.extension().is_some_and(|x| x == "tdl"))
                    .collect()
            })
            .unwrap_or_default();
        exts.sort();
        for p in exts {
            let ext = emdysi_tdl::load(&p, emdysi_tdl::Env::Type).map_err(err)?;
            loaded.entries.extend(ext.entries);
            loaded.letter_sets.extend(ext.letter_sets);
            loaded.files.extend(ext.files);
        }
        // The compiled type system is cached outside the source tree
        // (EMDYSI_CACHE_DIR, XDG_CACHE_HOME or ~/.cache); EMDYSI_NO_CACHE
        // turns the cache off.
        let cache_dir = if std::env::var_os("EMDYSI_NO_CACHE").is_some() {
            None
        } else {
            emdysi_hpsg::cache::default_dir()
        };
        let grammar = Grammar::compile_cached(&loaded, cache_dir.as_deref()).map_err(err)?;
        let repp = match setting_path("preprocessor") {
            Some(main) => {
                let modules: Vec<String> = ace_setting(&config_src, "preprocessor-modules")
                    .iter()
                    .filter_map(|m| {
                        Path::new(m.trim_matches('"'))
                            .file_stem()
                            .map(|s| s.to_string_lossy().to_string())
                    })
                    .collect();
                let refs: Vec<&str> = modules.iter().map(String::as_str).collect();
                emdysi_repp::Repp::load(&main, &refs).map_err(err)?
            }
            None => emdysi_repp::erg(dir).map_err(err)?,
        };
        let mut u = Unifier::new();
        let token_mapping =
            compile_rules(&grammar, "token-mapping-rule", &mut u).map_err(|e| err(e.0))?;
        let lexical_filtering =
            compile_rules(&grammar, "lexical-filtering-rule", &mut u).map_err(|e| err(e.0))?;

        let mut morph = Morphology::from_grammar(&grammar);
        let irregs: Vec<PathBuf> = ace_setting(&config_src, "irregular-forms")
            .iter()
            .map(|f| config_dir.join(f.trim_matches('"')))
            .collect();
        for f in if irregs.is_empty() {
            vec![dir.join("irregs.tab")]
        } else {
            irregs
        } {
            morph.load_irregs(&std::fs::read_to_string(f).map_err(err)?);
        }
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
            &std::fs::read_to_string(
                setting_path("quickcheck-code").unwrap_or_else(|| dir.join("ace/ace-erg-qc.txt")),
            )
            .map_err(err)?,
        );
        let args = grammar.feat("ARGS").ok_or_else(|| err("no ARGS feature"))?;
        let spanning: HashSet<String> = ace_setting(&config_src, "spanning-only-rules")
            .into_iter()
            .collect();
        // emdysi's extensions declare their own spanning-only rules (rules
        // that apply only to the whole input) in grammar/emdysi/settings.cfg.
        let spanning: HashSet<String> = spanning
            .into_iter()
            .chain(
                std::fs::read_to_string(dir.join("../emdysi/settings.cfg"))
                    .map(|src| ace_setting(&src, "spanning-only-rules"))
                    .unwrap_or_default(),
            )
            .collect();
        let mut rules = Vec::new();
        // Types whose constraints are inconsistent (e.g. in an extension).
        let mut warnings: Vec<String> = grammar
            .errors
            .iter()
            .map(|e| format!("type {}: {}", e.what, e.msg))
            .collect();
        for inst in &grammar.instances {
            let lexical = match inst.status.as_deref() {
                Some("rule") => false,
                Some("lex-rule") => true,
                _ => continue,
            };
            let dag = match grammar.expand(&inst.body, &mut u) {
                Ok(d) => d,
                Err(e) => {
                    // A rule the processor cannot build is reported and left
                    // out, as ACE does, rather than failing the whole load.
                    warnings.push(format!("rule {}: {e}", inst.name));
                    continue;
                }
            };
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
            // British spelling is as acceptable as American: a root that
            // requires `DIALECT us` gets a `_br` twin requiring `br`.
            let mut br = inst.body.clone();
            let twin = set_dialect(&mut br, "br");
            roots.push((name.clone(), Arc::new(dag)));
            if twin {
                let dag = grammar.expand(&br, &mut u).map_err(Error)?;
                roots.push((format!("{name}_br"), Arc::new(dag)));
            }
        }
        // Readings under the leading whole-sentence roots are unpacked
        // first, so that a cap on readings does not crowd them out with
        // fragments: the strict roots of the grammar, and the sentence
        // roots of its grammar-error variant (an error needs an analysis of
        // the whole sentence).
        let sentence_root = |n: &str| {
            is_strict(n)
                || matches!(
                    n.trim_end_matches("_br"),
                    "root_decl"
                        | "root_question"
                        | "root_command"
                        | "root_robust_ques"
                        | "root_robust_s"
                )
        };
        let preferred_roots = roots.iter().take_while(|(n, _)| sentence_root(n)).count();
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
            max_edges: 100_000,
            // Nodes of the structures the chart keeps (released ones are not
            // counted): about 1 GB per parse at most, measured, so that
            // several threads parsing long sentences stay within a few
            // gigabytes.
            max_nodes: 25_000_000,
            timeout: Duration::from_secs(60),
            packing_restrictor,
            max_readings: 1000,
            packing_top_type: ace_setting(&config_src, "generalize-edge-top-types")
                .first()
                .filter(|v| v.as_str() == "enabled")
                .and_then(|_| grammar.ts.hier.id("sign")),
            cell_beam: Some(DEFAULT_CELL_BEAM),
            cell_beam_from: 20,
            preferred_roots,
            unpack_beam: 100,
            fragments: true,
        };
        let lexicon = Lexicon::new(&grammar, morph, lex_paths);
        let labeler = Labeler::new(&grammar, &mut u).ok_or_else(|| err("no parse-node labels"))?;
        let orth = lexicon
            .entries
            .iter()
            .map(|e| (e.inst, e.orth.join(" ")))
            .collect();
        let le_types = (0..grammar.instances.len())
            .map(|i| rank::lexical_type(&grammar, i))
            .collect();
        let mrs = ace_setting(&config_src, "variable-property-mapping")
            .first()
            .map(|f| config_dir.join(f.trim_matches('"')))
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|src| {
                let vpm = emdysi_hpsg::vpm::Vpm::parse(&grammar, &src);
                emdysi_hpsg::mrs::MrsConfig::new(
                    &grammar,
                    vpm,
                    &ace_setting(&config_src, "mrs-deleted-roles"),
                )
            });
        Ok(Erg {
            dir: dir.to_path_buf(),
            config_file,
            parse_cache: None,
            mal: std::sync::OnceLock::new(),
            warnings,
            mrs,
            first_beam: Some(20),
            le_types,
            orth,
            model: rank::Model::parse(DEFAULT_MODEL),
            trees_for: 3,
            keep_dags: false,
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

    /// The grammar-error variant of the grammar: the ERG with its
    /// "mal-rules" and robust lexical entries (`ace/config-mal.tdl`), which
    /// analyse common errors (agreement, wrong verb forms, missing
    /// determiners, a/an, ...) and name them. Loaded on first use; `None`
    /// if the grammar does not provide it.
    pub fn mal(&self) -> Option<&Erg> {
        self.mal
            .get_or_init(|| {
                if !self.dir.join(MAL_CONFIG).exists() {
                    return None;
                }
                let mut m = Erg::load_config(&self.dir, MAL_CONFIG).ok()?;
                if let Some((dir, _)) = &self.parse_cache {
                    m.cache_parses(dir);
                }
                Some(Box::new(m))
            })
            .as_deref()
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
        self.parse_with(text, &self.config)
    }

    /// Parse one sentence with a different time limit.
    pub fn parse_with_timeout(&self, text: &str, timeout: Duration) -> Result<Parse, Error> {
        let mut config = self.config.clone();
        config.timeout = timeout;
        self.parse_with(text, &config)
    }

    /// Parse one sentence with a time limit and at most `max_readings`
    /// readings (the best-scoring ones are unpacked first).
    pub fn parse_limited(
        &self,
        text: &str,
        timeout: Duration,
        max_readings: usize,
    ) -> Result<Parse, Error> {
        let mut config = self.config.clone();
        config.timeout = timeout;
        config.max_readings = max_readings;
        self.parse_with(text, &config)
    }

    /// Cache parses in `dir`, and reuse the ones cached there (also for
    /// the grammar-error variant, [`Erg::mal`]). For measurement runs,
    /// which parse the same sentences each time: a parse is keyed by the
    /// grammar's files, the parser's settings and source code, and the
    /// sentence, so a change to any of them parses afresh. Parses cut
    /// short by the time limit are reused as they are. Not used while
    /// [`Erg::keep_dags`] is set, as the cache holds no structures; a
    /// replaced ranking model ([`Erg::model`]) must use another `dir`.
    pub fn cache_parses(&mut self, dir: &Path) {
        let version = parse_cache::open(
            dir,
            &[self.dir.clone(), self.dir.join("../emdysi")],
            &self.config_file,
        );
        self.parse_cache = Some((dir.to_path_buf(), version));
    }

    fn parse_with(&self, text: &str, config: &ParserConfig) -> Result<Parse, Error> {
        let Some((_, dir)) = self.parse_cache.as_ref().filter(|_| !self.keep_dags) else {
            return self.parse_uncached(text, config);
        };
        let key = parse_cache::key(config, self.first_beam, self.trees_for, text);
        if let Some(p) = parse_cache::load(dir, &key) {
            return Ok(p);
        }
        let p = self.parse_uncached(text, config)?;
        parse_cache::store(dir, &key, &p);
        Ok(p)
    }

    fn parse_uncached(&self, text: &str, config: &ParserConfig) -> Result<Parse, Error> {
        let t0 = std::time::Instant::now();
        let tokens = self.tokens(text);
        let (lat, _) = self.map_tokens(&tokens, false)?;
        if std::env::var_os("EMDYSI_CHART").is_some() {
            let tags: Vec<String> = tokens
                .iter()
                .map(|t| {
                    format!(
                        "{}/{}",
                        t.form,
                        t.tags.first().map_or("", |g| g.tag.as_str())
                    )
                })
                .collect();
            eprintln!("tokens: {}", tags.join(" "));
        }
        let mut u = Unifier::new();
        let items = self.lexicon.instantiate(&self.grammar, &lat, &mut u);
        let n_items = items.len();
        let scorer = rank::ChartScorer {
            grammar: &self.grammar,
            rules: &self.rules,
            model: &self.model,
            le_types: &self.le_types,
        };
        let run = |config: &ParserConfig, items: Vec<LexItem>| -> ParseResult {
            Parser::new(
                &self.grammar,
                &self.rules,
                &self.qc,
                config,
                &self.lexical_filtering,
            )
            .with_scorer(&scorer)
            .parse(&lat, items)
        };
        let strict = |r: &ParseResult| r.readings.iter().any(|r| is_strict(&r.root));
        let complete = |r: &ParseResult| r.readings.iter().any(|r| r.root != "fragment");
        // With chart pruning, a narrow beam first; the configured one only
        // when that finds no strict analysis and time remains.
        let result = match (self.first_beam, config.cell_beam) {
            (Some(first), Some(beam)) if first < beam => {
                let narrow = ParserConfig {
                    cell_beam: Some(first),
                    ..config.clone()
                };
                let r = run(&narrow, items.clone());
                let left = config.timeout.saturating_sub(t0.elapsed());
                if strict(&r)
                    || left < Duration::from_secs(1)
                    || r.positions <= config.cell_beam_from
                {
                    r
                } else {
                    let wide = ParserConfig {
                        timeout: left,
                        ..config.clone()
                    };
                    let r2 = run(&wide, items.clone());
                    if strict(&r2) || (complete(&r2) && !complete(&r)) {
                        r2
                    } else {
                        r
                    }
                }
            }
            _ => run(config, items.clone()),
        };
        // Pruning can make a full analysis of a long sentence impossible:
        // with no complete analysis, try again without pruning in the time
        // left (on the AI-prose treebank, 13 of 17 sentences that got only
        // fragments then get a full analysis within 10 seconds).
        let result = if !complete(&result)
            && config.cell_beam.is_some()
            && result.positions > config.cell_beam_from
            && config.timeout.saturating_sub(t0.elapsed()) >= Duration::from_secs(1)
        {
            let exhaustive = ParserConfig {
                cell_beam: None,
                timeout: config.timeout.saturating_sub(t0.elapsed()),
                ..config.clone()
            };
            let r = run(&exhaustive, items);
            if complete(&r) { r } else { result }
        } else {
            result
        };
        // EMDYSI_CHART=1 prints every edge of the chart (for grammar work).
        if std::env::var_os("EMDYSI_CHART").is_some() {
            for (i, e) in result.chart.iter().enumerate() {
                let name = match &e.kind {
                    EdgeKind::Lex { inst, .. } => self.grammar.instances[*inst].name.clone(),
                    EdgeKind::Rule(r) => self.rules[*r].name.clone(),
                    EdgeKind::Cover => "cover".into(),
                };
                // EMDYSI_CHART_PATH="SYNSEM LOCAL CONJ" adds the type there.
                let at = std::env::var("EMDYSI_CHART_PATH")
                    .ok()
                    .and_then(|p| self.grammar.path(&p))
                    .and_then(|p| e.dag.follow(0, &p))
                    .map(|n| self.grammar.ts.hier.name(e.dag.ty(n)).to_string())
                    .unwrap_or_default();
                eprintln!(
                    "edge {i} {}-{} {name} {:?} {:?} {at}",
                    e.start, e.end, e.daughters, e.state
                );
            }
        }
        let form_path = self.lexicon.paths.token_form.clone();
        let forms = |toks: &[usize]| -> String {
            toks.iter()
                .filter_map(|&t| string_at(&self.grammar.ts, &lat.entries[t].dag, &form_path))
                .collect::<Vec<_>>()
                .join(" ")
        };
        let chars: Vec<char> = text.chars().collect();
        let (from_path, to_path) = (self.grammar.path("+FROM"), self.grammar.path("+TO"));
        let token_span = |tokens: &[usize]| -> Option<(usize, usize)> {
            let span = |t: usize, p: &Option<Vec<emdysi_hpsg::FeatId>>| {
                string_at(&self.grammar.ts, &lat.entries[t].dag, p.as_ref()?)?
                    .parse::<usize>()
                    .ok()
            };
            Some((
                span(*tokens.first()?, &from_path)?,
                span(*tokens.last()?, &to_path)?,
            ))
        };
        let surface = |d: &Deriv| -> Option<String> {
            let EdgeKind::Lex { tokens, .. } = &d.kind else {
                return None;
            };
            let (from, to) = token_span(tokens)?;
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
            .map(|(i, (score, features, r))| {
                let (nodes, words) = self.flatten(&r.deriv, &token_span, &chars);
                Reading {
                    nodes,
                    words,
                    dag: self.keep_dags.then(|| r.dag.clone()),
                    mrs: self
                        .mrs
                        .as_ref()
                        .and_then(|c| emdysi_hpsg::mrs::extract(&self.grammar, c, &r.dag)),
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
                }
            })
            .collect();
        Ok(Parse {
            tokens,
            readings,
            edges: result.chart.len(),
            lexical_items: n_items,
            exhausted: result.exhausted,
            complete: !result.exhausted && result.stats.pruned == 0,
            elapsed: t0.elapsed(),
            temperature: self.model.temperature,
        })
    }

    /// Whether the grammar knows a word form, directly or through its
    /// orthographic rules (e.g. *dogs*, *stopped*).
    pub fn known_word(&self, word: &str) -> bool {
        let w = word.to_lowercase();
        if self.lexicon.has_first_word(&w) {
            return true;
        }
        // Derivational prefixes (re-, co-, un-, ...) chain freely in the
        // morphology and are filtered by unification when parsing; for
        // spelling, allow at most one, on a stem of four letters or more.
        self.lexicon
            .morph
            .analyze(&w, &|s| self.lexicon.is_stem(s), 3)
            .iter()
            .any(|a| {
                let derivational = a.rules.iter().filter(|r| r.ends_with("_dlr")).count();
                // One inflection at most: "chosing" is not "chose" + "-ing".
                let inflectional = a.rules.iter().filter(|r| r.ends_with("_olr")).count();
                inflectional <= 1
                    && (derivational == 0 || derivational == 1 && a.stem.chars().count() >= 4)
            })
    }

    /// Lexical types of the one-word entries for a word form: its own
    /// entries and those of its stems under inflection (not derivation),
    /// e.g. `n_-_c_le` and `v_np_le` for *test* or *tests*.
    pub fn word_types(&self, word: &str) -> Vec<&str> {
        let w = word.to_lowercase();
        let mut stems = vec![w.clone()];
        for a in self
            .lexicon
            .morph
            .analyze(&w, &|s| self.lexicon.is_stem(s), 2)
        {
            if !a.rules.iter().any(|r| r.ends_with("_dlr")) && !stems.contains(&a.stem) {
                stems.push(a.stem);
            }
        }
        let mut out = Vec::new();
        for st in &stems {
            for &e in self.lexicon.entries_starting(st) {
                let entry = &self.lexicon.entries[e];
                if entry.orth.len() == 1 {
                    let t = self.le_types[entry.inst].as_str();
                    if !out.contains(&t) {
                        out.push(t);
                    }
                }
            }
        }
        out
    }

    /// Other inflected forms of the word's known stems (not derivations):
    /// regular ones (-s, -es, -ed, -d, -ing) that the grammar knows, and
    /// irregular ones from its table. *buyed* gives *buy*, *buys*,
    /// *bought*, *buying*; *go* gives *goes*, *went*, *gone*, ...
    pub fn inflections(&self, word: &str) -> Vec<String> {
        let w = word.to_lowercase();
        let mut stems: Vec<String> = Vec::new();
        if self.lexicon.is_stem(&w) {
            stems.push(w.clone());
        }
        for a in self
            .lexicon
            .morph
            .analyze(&w, &|s| self.lexicon.is_stem(s), 2)
        {
            if !a.rules.iter().any(|r| r.ends_with("_dlr")) && !stems.contains(&a.stem) {
                stems.push(a.stem);
            }
        }
        // The stem by spelling: a regular past of an irregular verb
        // (*buyed*), or a misspelled inflection (*runing*).
        for suffix in ["ed", "d", "ing", "s", "es"] {
            if let Some(st) = w.strip_suffix(suffix) {
                if self.lexicon.is_stem(st) && !stems.iter().any(|s| s == st) {
                    stems.push(st.to_string());
                }
            }
        }
        let mut out: Vec<String> = Vec::new();
        for st in &stems {
            let regular = [
                st.clone(),
                format!("{st}s"),
                format!("{st}es"),
                format!("{st}ed"),
                format!("{st}d"),
                format!("{st}ing"),
                st.strip_suffix('e')
                    .map(|b| format!("{b}ing"))
                    .unwrap_or_default(),
                st.strip_suffix('y')
                    .map(|b| format!("{b}ies"))
                    .unwrap_or_default(),
                // Doubled final consonant: "running", "stopped".
                st.chars()
                    .last()
                    .map(|c| format!("{st}{c}ing"))
                    .unwrap_or_default(),
                st.chars()
                    .last()
                    .map(|c| format!("{st}{c}ed"))
                    .unwrap_or_default(),
            ];
            for f in regular
                .into_iter()
                .chain(self.lexicon.morph.irregular_forms(st))
            {
                if !f.is_empty() && f != w && !out.contains(&f) && (self.known_word(&f)) {
                    out.push(f);
                }
            }
        }
        out
    }

    /// The forms of `lemma` with the same inflection as `like`: *use* like
    /// *utilizes* is *uses*, like *utilized* *used*, like *utilize* *use*.
    /// The grammar's spelling rules also analyse misspellings (*useed* as
    /// *use* plus -ed), so callers should check the forms against a word
    /// list.
    pub fn inflect_like(&self, lemma: &str, like: &str) -> Vec<String> {
        let lemma = lemma.to_lowercase();
        let endings = |form: &str, stem: Option<&str>| -> Vec<Vec<String>> {
            self.lexicon
                .morph
                .analyze(form, &|s| self.lexicon.is_stem(s), 2)
                .into_iter()
                .filter(|a| !a.rules.iter().any(|r| r.ends_with("_dlr")))
                .filter(|a| stem.is_none_or(|st| a.stem == st))
                .map(|a| a.rules)
                .collect()
        };
        let wanted = endings(&like.to_lowercase(), None);
        std::iter::once(lemma.clone())
            .chain(self.inflections(&lemma))
            .filter(|f| {
                endings(f, Some(&lemma))
                    .iter()
                    .any(|rules| wanted.contains(rules))
            })
            .collect()
    }

    /// The forms of the verbs `word` is a form of that the grammar makes
    /// with inflection rule `rule` (`None`: the base form): *go* for
    /// *goes* and none, *going* for *go* and `v_prp_olr`.
    pub fn forms_by_rule(&self, word: &str, rule: Option<&str>) -> Vec<String> {
        let w = word.to_lowercase();
        let analyses = |form: &str| {
            self.lexicon
                .morph
                .analyze(form, &|s| self.lexicon.is_stem(s), 2)
                .into_iter()
                .filter(|a| !a.rules.iter().any(|r| r.ends_with("_dlr")))
                .collect::<Vec<_>>()
        };
        let mut stems: Vec<String> = Vec::new();
        for a in analyses(&w) {
            if !stems.contains(&a.stem) {
                stems.push(a.stem);
            }
        }
        let wanted: Vec<String> = rule.into_iter().map(String::from).collect();
        let mut out: Vec<String> = Vec::new();
        for st in &stems {
            for f in std::iter::once(st.clone()).chain(self.inflections(st)) {
                if f != w
                    && !out.contains(&f)
                    && analyses(&f)
                        .iter()
                        .any(|a| a.stem == *st && a.rules == wanted)
                {
                    out.push(f);
                }
            }
        }
        out
    }

    /// The irregular past participles of the verb `word` is a form of:
    /// *gone* for *went*, *broken* for *broke*.
    pub fn irregular_participles(&self, word: &str) -> Vec<String> {
        let w = word.to_lowercase();
        let mut out: Vec<String> = Vec::new();
        for a in self
            .lexicon
            .morph
            .analyze(&w, &|s| self.lexicon.is_stem(s), 1)
        {
            for f in self
                .lexicon
                .morph
                .irregular_forms_by(&a.stem, &["v_psp_olr"])
            {
                if f != w && !out.contains(&f) {
                    out.push(f);
                }
            }
        }
        out
    }

    /// The irregular past forms of the verb a wrongly regular form
    /// belongs to: *bought* for *buyed*, *ran* for *runned*, *went* and
    /// *gone* for *goed*.
    pub fn irregular_pasts(&self, word: &str) -> Vec<String> {
        let w = word.to_lowercase();
        let mut stems: Vec<String> = Vec::new();
        let mut add = |st: String| {
            if self.lexicon.is_stem(&st) && !stems.contains(&st) {
                stems.push(st);
            }
        };
        for suffix in ["ed", "d"] {
            if let Some(st) = w.strip_suffix(suffix) {
                add(st.to_string());
                // A doubled final consonant: |runned|, |stopped|.
                let cs: Vec<char> = st.chars().collect();
                if cs.len() > 2 && cs[cs.len() - 1] == cs[cs.len() - 2] {
                    add(cs[..cs.len() - 1].iter().collect());
                }
                // A final y made i: |buyed| has none, |flied| from |fly|.
                if let Some(b) = st.strip_suffix('i') {
                    add(format!("{b}y"));
                }
            }
        }
        let mut out = Vec::new();
        for st in &stems {
            for f in self
                .lexicon
                .morph
                .irregular_forms_by(st, &["v_pst_olr", "v_psp_olr"])
            {
                if !out.contains(&f) {
                    out.push(f);
                }
            }
        }
        out
    }

    /// Spelling suggestions for an unknown word: known words within edit
    /// distance 1, or 2 if there are none, best first.
    pub fn spelling_suggestions(&self, word: &str, max: usize) -> Vec<String> {
        let lower = word.to_lowercase();
        let mut found: Vec<(usize, String)> = Vec::new();
        let mut seen = HashSet::new();
        for e in edits(&lower) {
            if seen.insert(e.clone()) && self.known_word(&e) {
                found.push((1, e));
            }
        }
        if found.is_empty() && lower.chars().count() <= 12 {
            let first: Vec<String> = seen.iter().cloned().collect();
            for e1 in first {
                for e2 in edits(&e1) {
                    if seen.insert(e2.clone()) && self.known_word(&e2) {
                        found.push((2, e2));
                    }
                }
            }
        }
        // Prefer suggestions that keep the first letter, then by length
        // difference, then alphabetically for determinism.
        let first = lower.chars().next();
        let len = lower.chars().count() as i64;
        found.sort_by_key(|(d, w)| {
            (
                *d,
                w.chars().next() != first,
                (w.chars().count() as i64 - len).abs(),
                w.clone(),
            )
        });
        let restore_case = |w: &str| -> String {
            if word.chars().next().is_some_and(char::is_uppercase) {
                let mut c = w.chars();
                c.next()
                    .map(|f| f.to_uppercase().chain(c).collect())
                    .unwrap_or_default()
            } else {
                w.to_string()
            }
        };
        found
            .into_iter()
            .take(max)
            .map(|(_, w)| restore_case(&w))
            .collect()
    }

    /// Flatten a derivation into nodes and words with character spans.
    fn flatten(&self, d: &Deriv, token_span: &TokenSpan, chars: &[char]) -> (Vec<Node>, Vec<Word>) {
        let mut nodes = Vec::new();
        let mut words = Vec::new();
        self.flatten_into(d, None, token_span, chars, &mut nodes, &mut words);
        (nodes, words)
    }

    fn flatten_into(
        &self,
        d: &Deriv,
        parent: Option<usize>,
        token_span: &TokenSpan,
        chars: &[char],
        nodes: &mut Vec<Node>,
        words: &mut Vec<Word>,
    ) -> usize {
        let id = nodes.len();
        let (name, leaf) = match &d.kind {
            EdgeKind::Lex { inst, .. } => (self.grammar.instances[*inst].name.clone(), true),
            EdgeKind::Rule(ri) => (self.rules[*ri].name.clone(), false),
            EdgeKind::Cover => ("fragment".to_string(), false),
        };
        nodes.push(Node {
            name,
            leaf,
            from: 0,
            to: 0,
            parent,
            children: Vec::new(),
        });
        if let EdgeKind::Lex { inst, tokens } = &d.kind {
            let (from, to) = token_span(tokens).unwrap_or((0, 0));
            nodes[id].from = from;
            nodes[id].to = to;
            let surface: String = chars
                .get(from..to)
                .map(|c| c.iter().collect())
                .unwrap_or_default();
            let generic =
                self.grammar.instances[*inst].status.as_deref() == Some("generic-lex-entry");
            // Lexical rules applied above this leaf, innermost first.
            let mut rules = Vec::new();
            let mut p = parent;
            while let Some(pi) = p {
                let rule = &nodes[pi].name;
                if !self.rules.iter().any(|r| r.lexical && r.name == *rule) {
                    break;
                }
                rules.push(rule.clone());
                p = nodes[pi].parent;
            }
            rules.reverse();
            words.push(Word {
                lemma: if generic {
                    surface.to_lowercase()
                } else {
                    self.orth
                        .get(inst)
                        .cloned()
                        .unwrap_or_else(|| surface.to_lowercase())
                },
                surface,
                entry: self.grammar.instances[*inst].name.clone(),
                le_type: rank::lexical_type(&self.grammar, *inst),
                generic,
                rules,
                from,
                to,
                node: id,
            });
            return id;
        }
        let mut span: Option<(usize, usize)> = None;
        for k in &d.daughters {
            let c = self.flatten_into(k, Some(id), token_span, chars, nodes, words);
            nodes[id].children.push(c);
            let (f, t) = (nodes[c].from, nodes[c].to);
            span = Some(span.map_or((f, t), |(a, b)| (a.min(f), b.max(t))));
        }
        let (f, t) = span.unwrap_or((0, 0));
        nodes[id].from = f;
        nodes[id].to = t;
        id
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

/// All strings within one edit (deletion, transposition, substitution,
/// insertion) of `w`, over lower-case letters, hyphen and apostrophe.
pub fn edits(w: &str) -> Vec<String> {
    const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz-'";
    let c: Vec<char> = w.chars().collect();
    let mut out = Vec::new();
    let s = |v: &[char]| v.iter().collect::<String>();
    for i in 0..c.len() {
        let mut d = c.clone();
        d.remove(i);
        out.push(s(&d));
        if i + 1 < c.len() {
            let mut t = c.clone();
            t.swap(i, i + 1);
            out.push(s(&t));
        }
        for a in ALPHABET.chars() {
            if a != c[i] {
                let mut r = c.clone();
                r[i] = a;
                out.push(s(&r));
            }
        }
    }
    for i in 0..=c.len() {
        for a in ALPHABET.chars() {
            let mut n = c.clone();
            n.insert(i, a);
            out.push(s(&n));
        }
    }
    out
}
