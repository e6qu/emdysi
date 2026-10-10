# Contributing to emdysi

Thank you for helping. This guide covers setting up, the checks every
change must pass, and how to work on each part: rules, the grammar, the
parser and the data. The [tutorial](docs/tutorial.md) shows what the tool
does. [decisions.md](docs/decisions.md) explains why it works the way it
does; read it before changing anything fundamental.

## Set up

You need a stable Rust toolchain with `rustfmt` and `clippy`, and Python
3.11 or later for the scripts. Then:

```sh
git clone https://github.com/e6qu/emdysi
cd emdysi
cargo build --release
cargo test --release
```

Use `--release` for tests too: they parse with the full grammar, which is
slow in a debug build. The first run compiles the grammar into a cache
outside the repository (`$EMDYSI_CACHE_DIR`, `$XDG_CACHE_HOME/emdysi` or
`~/.cache/emdysi`). The cache is keyed by the grammar sources, so editing
a `.tdl` file recompiles it. Set `EMDYSI_NO_CACHE=1` to bypass it.

The optional local-model backend (`--features llama`) builds llama.cpp from
source and needs a C++ toolchain and `libclang`. You only need it to work
on `emdysi-lm` or `emdysi-rewrite`; see [models.md](docs/models.md).

## Checks

CI runs these on every pull request. Run them before you push:

```sh
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --release --all
python3 scripts/check-vendored.py
python3 scripts/import-rules.py --check
```

`cargo deny check licenses bans sources` also runs in CI, if you have
`cargo-deny` installed. `check-vendored.py` verifies the registry of
non-code material and that the generated
[`docs/vendored.md`](docs/vendored.md) and
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) are current.
`import-rules.py --check` verifies that the packs generated from other
linters' data are current.

## Layout

| Crate | Contents |
|---|---|
| `emdysi-tdl` | Reader for TDL, the grammar-definition language of the ERG |
| `emdysi-hpsg` | Type hierarchy, typed feature structures, unification, morphology, the chart parser, MRS |
| `emdysi-repp` | REPP tokenizer with character offsets |
| `emdysi-parse` | The pipeline: tokenizing, tagging, token mapping, lexical lookup, parsing, ranking, ambiguity |
| `emdysi-text` | Markdown and plain-text blocks with source offsets; sentence segmentation |
| `emdysi-check` | Document analysis, rule packs, diagnostics, fixes, reports |
| `emdysi-lm` | Optional local language models: GGUF through llama.cpp, OpenAI-compatible servers |
| `emdysi-rewrite` | Guarded rewriting and typo correction |
| `emdysi` | The command-line tool, `en` |

Data lives outside the crates: the grammar in `grammar/` (the vendored ERG
in `grammar/erg`, emdysi's extensions in `grammar/emdysi`), rule packs in
`packs/`, word lists and rule data in `data/`, and test corpora in
`corpora/`.

## Ground rules

- **Error claims must be provable** (decision D13). A rule that says
  something is wrong (spelling, a grammatical error) reports it only on
  deterministic evidence. The aim is at most one false flag per 1,000
  sentences of edited text. Ranking, taggers and models never decide an
  error claim. A change that makes the checker claim more must show that
  it does not flag edited text (see [Measure](#measure)).
- **Ambiguity is information, not an error.** English is ambiguous; emdysi
  reports it and never turns it into an error claim.
- **Licenses.** Crates and data must be under an allowed license, verified
  from the primary source (see [Vendored data](#vendored-data) and
  [dependencies.md](docs/dependencies.md)). Never vendor anything whose
  terms are unclear.
- **No compiled artifacts or model weights** in the repository, apart
  from the ranker's weights (`crates/emdysi-parse/data/rank.tsv`), which
  are trained here from vendored data and registered like any other data.
- **No ERG file is changed.** Extensions go in `grammar/emdysi` (decision
  D14).
- US and UK spellings are both correct (decision D8).

## Add or change a rule

Rules are TOML in `packs/`; [rules.md](docs/rules.md) describes every
kind. Each rule should have:

- an `id` prefixed with its pack's name, and a message that says what to
  do;
- `examples`: documents it must flag;
- `acceptable`: documents it must not flag, especially near misses;
- a `source` if it follows a style guide or another tool, with its
  `license` if it reuses their data.

`cargo test --release -p emdysi-check --test examples` runs every pack's
examples. Prefer the grammar's analysis (the `construction`, `semantics`
and lemma-matching `words` kinds) to surface patterns wherever grammar
decides the matter (decision D11). If a rule needs new Rust code, add a
kind in `crates/emdysi-check/src/rules.rs`, document it in `rules.md`, and
test it.

The `microsoft`, `google`, `elastic`, `wordlists` and `equality` packs are
generated: change `scripts/import-rules.py` and run it, never the TOML.

## Extend the grammar

When a well-formed sentence gets no full analysis, find out why first:

```sh
echo "The park, established in 1879, was large." | cargo run --release -p emdysi-parse --example parse
```

The example prints how many readings the sentence has, how many are
strict (full and formal), and the best three. It reads these environment
variables:

| Variable | Effect |
|---|---|
| `TIMEOUT=60` | the time limit in seconds |
| `MAX_NODES=N` | the node budget of the chart |
| `NO_BEAM=1` | turn off the per-cell beam |
| `NO_PACKING=1` | turn off ambiguity packing |
| `ROOT=root_strict` | show readings under one root |
| `CONFIG=ace/config-mal.tdl` | parse with the grammar-error variant |

A sentence that parses once shortened ran out of search; one that does
not is a grammar gap. To find which word is the problem, run `culprits`.
It replaces each word in turn by a plain word of the same shape and
reports the replacements that give a full analysis:

```sh
cargo run --release -p emdysi-parse --example culprits < sentences.txt
```

For the grammar itself:

- `EMDYSI_CHART=1` prints the chart.
- `EMDYSI_CHART_PATH="SYNSEM LOCAL CONJ"` adds a feature path to each
  edge.
- `EMDYSI_TRACE_RULE=<rule>` traces one rule's attempts and root checks.
- `cargo run --release -p emdysi-parse --example grammar_check -- <type|rule|word>...`
  reports inconsistent type constraints.

A fix goes in a file under `grammar/emdysi`. It can be a new lexical entry
in the form of the ERG's own, a rule on the ERG's types, or a formal
counterpart of a rule the ERG marks informal. Then:

1. Add the sentences it must analyse to `corpora/stress/stress.tsv`, or a
   test, and run `cargo test --release -p emdysi-check --test stress`.
2. Check that it does not create spurious ambiguity:
   `cargo run --release -p emdysi-parse --example interpretations -- "It is unusual."`
3. Measure it on the gold profiles (see [Measure](#measure)). An extension
   must not cost coverage or ranking accuracy.
4. If it adds a rule or entry that stands in for an ERG one, map it in
   `EQUIVALENT` in `crates/emdysi-parse/src/rank.rs`. The ranker was
   trained on the ERG's names.
5. Describe it in [`grammar/emdysi/README.md`](grammar/emdysi/README.md)
   and run `python3 scripts/check-vendored.py --update`.

A native lexical entry replaces the ERG's generic entries for an unknown
word. If you add a noun for a word the ERG lacks, also add its other
common uses (its adjective, its verb); otherwise they are lost.

## Measure

Accuracy is measured, not assumed. These examples measure it; each
documents itself at the top of its source file.

| Command | Measures |
|---|---|
| `cargo run --release -p emdysi-parse --example eval -- corpora/erg-gold/<profile>` | Coverage, gold tree found and gold tree ranked first on the ERG's gold profiles (`csli`, `esd`, `control`, `sh-spec`, `mrs`, `ccs`). `ITEMS=out.tsv` writes per-item results, so two runs can be compared item by item |
| `cargo run --release -p emdysi-check --example false_flags -- out.tsv` | Error claims on edited text, per 1,000 sentences. `SET=dev2` and `SET=heldout` choose the second development set and the held-out set |
| `cargo run --release -p emdysi-check --example real_errors -- out.tsv` | How many real errors from documentation histories are caught |
| `cargo run --release -p emdysi-check --example minimal_pairs -- corpora/blimp/sample.tsv` | Grammaticality judgments on minimal pairs (also `corpora/zorro/sample.tsv`) |
| `cargo run --release -p emdysi-check --example treebank_eval` | How often the best reading is the hand-judged one, on machine-written prose |
| `cargo run --release -p emdysi-check --example rule_rates -- corpora/beemo/sample.tsv 3,4` | How often each rule fires on a corpus's texts (here Beemo's model outputs, column 3, against their expert edits, column 4) |

These tools cache every parse they make, in `parses` in the grammar
cache directory (or `$EMDYSI_PARSE_CACHE`). A parse is cached under a hash
of everything its result depends on: the grammar's files, the parser's
settings, the source code of the parsing crates (`emdysi-parse`,
`emdysi-hpsg`, `emdysi-repp`, `emdysi-tdl`) and the sentence. A change to
the checks after parsing, such as a rule or an error correction, reruns
in minutes; a change to the grammar or the parser parses afresh, as it
must. A parse cut short by the time limit is reused as it is, so a rerun
reports the same results. Each version of the grammar and the parser has
its own directory, and only the four most recently used are kept, so even
a formatting change in the parsing crates starts a cold run. The cache of
both development sets takes a few gigabytes; delete the directory to
reclaim it, or set `EMDYSI_NO_CACHE=1` to parse without it.

The held-out set (`corpora/edited/heldout-*`) is for final measurements
only: do not tune against it. Record results that change what the
project claims in [evaluation.md](docs/evaluation.md), with the date and
the numbers before and after.

The parse ranker can be retrained with the `train` example (see its
source). That changes `rank.tsv`, which must then be re-registered.

## Vendored data

Everything under `grammar/`, `corpora/` and `data/`, and every data file
compiled into a crate, belongs to exactly one entry in
[`VENDORED.toml`](VENDORED.toml). To add or update material:

1. Verify its license from the primary source: the upstream license file
   at the exact version, not a listing. Allowed: MIT, Apache-2.0, BSD, ISC,
   CC0, CC BY, public domain, the SCOWL notice. CC BY-SA only as test data
   in its own directory. Not allowed: NC, ND, LDC, GPL-family data, or
   anything unclear. For machine-generated text, also check the
   generating model's terms on its output.
2. Copy the license text alongside the material, and write a `SOURCE.md`:
   the upstream URL, version, what was taken, how it was converted (keep
   the conversion script beside it) and why its license is clean.
3. Add an entry to `VENDORED.toml`. The fields are documented at the top
   of the file.
4. Run `python3 scripts/check-vendored.py --update`. This pins every
   file's hash in `VENDORED.sha256` and regenerates `docs/vendored.md` and
   `THIRD_PARTY_NOTICES.md`. Review the diff.

New crates follow [dependencies.md](docs/dependencies.md): verify the
license from the crate's `Cargo.toml` and its license files, add a row to
the table, and keep `deny.toml` passing.

## Pull requests

- Keep a pull request to one change, with its tests and its measurements.
- Say in the description what changed, why, and what you measured
  (coverage, false flags, gold trees), with numbers before and after.
- Pull requests are squash-merged. The commit is a single line of at most
  80 characters, without trailers.
- Write documentation and messages in plain English: lead with the point
  and use concrete words. `en check` on your own Markdown is a good test.

## License

emdysi is MIT-licensed. By contributing, you agree that your contributions
are licensed under the same terms ([`LICENSE`](LICENSE)).
