# emdysi

An MIT-licensed English text analyzer written in Rust. It aims to:

- parse English into phrase (constituency) and sentence structure,
- check spelling and apply safe automatic fixes,
- run semi-deterministic style and substance checks, especially on
  AI-generated prose, and enforce configurable style guides.

It runs the [English Resource Grammar](https://github.com/delph-in/erg) (ERG),
a broad-coverage HPSG grammar, with its own Rust implementation of the
DELPH-IN processing stack: TDL reader, typed feature structures and
unification, REPP tokenizer, chart mapping, morphology and a packing chart
parser. On the ERG's own test suites it parses 98% of the grammatical items
and finds the gold analysis among its readings for 98.5% of them. A parse
ranker trained on license-clean gold trees (2,626 items of all lengths)
picks the gold analysis first for 81.7% of held-out sentences, long ones
included. Each analysis comes with its semantics as Minimal Recursion
Semantics (MRS, `en parse --mrs`), identical to the ERG's gold MRS for all
2,966 gold analyses we reproduce. Sentences the grammar rejects are
re-parsed with the ERG's grammar-error ("mal-rule") variant, which names
the error (agreement, verb forms, articles, ...).

Status: working prototype. See [`docs/plan.md`](docs/plan.md) for progress,
[`docs/prior-art.md`](docs/prior-art.md) for the survey of existing tools and
data, and [`docs/decisions.md`](docs/decisions.md) for the choices made.

## Usage

```sh
cargo build --release
target/release/en check README.md             # report problems
target/release/en check --format markdown notes.txt
target/release/en fix draft.md > fixed.md     # apply safe fixes
target/release/en parse --derivations essay.md
target/release/en packs                       # list rules
```

Input is plain text or Markdown (chosen from the file extension, or with
`--input`); output is plain text or Markdown (`--format`). Rule packs are
TOML files; see [`docs/rules.md`](docs/rules.md). The built-in packs are:

- `core`: spelling (with safe automatic fixes), named grammatical errors,
  grammaticality and US/GB spelling consistency;
- `ai-tells`: vocabulary, stock phrases and constructions over-represented
  in machine-written prose (contrast frames, trailing participial clauses,
  three-part lists, em-dash density, chat-assistant residue);
- `plain-style`: passive voice, long sentences, intensifiers, wordy phrases,
  expletive *there*, repeated words;
- `substance`: hedged, vague and unsupported claims (*seems to*, *may
  potentially*, *studies show* without a citation, *a number of*,
  *clearly*).
- `structure`: the main point first (no throat-clearing or vague
  openings, no conclusion held back to the end), a clean heading hierarchy
  (no skipped levels, empty or lone sections), short paragraphs, specific
  headings, parallel headings and list items, and procedures written as
  numbered instructions;
- `terms`: acronyms defined at first use, one spelling per term, glossary
  terms (`--glossary FILE`) instead of deprecated ones, no coined words or
  concept names, no hyphen chains (*decision-making-framework*), no hyphen
  after *-ly* adverbs and no noun stacks.

Opt-in packs converted from other linters' rule data (see
[`docs/rules.md`](docs/rules.md#imported-packs)): `microsoft`, `google` and
`elastic` (word choice from those companies' style guides, via their Vale
packages), `wordlists` (hedges, weasel words and fillers) and `equality`
(insensitive wording, from the data behind alex). Load them with
`--pack microsoft` and so on.

`en rewrite` prints the text with guarded rewrites: the automatic fixes,
spelling corrections and a small local language model's rewrites of
sentences the rules cannot fix: a GGUF file with `--model FILE.gguf` (a
build with `--features llama`), or any model served over the
OpenAI-compatible API with `--server URL`, such as an MLX model on Apple
silicon (`mlx_lm.server --model mlx-community/...`), LM Studio or Ollama. A candidate is kept only if the grammar gives it a
strict analysis, its semantics keeps the original's content, and it
introduces no new problem; the model's likelihood then picks among the
survivors. Model weights are never bundled.

The first run compiles the grammar (about five seconds) and caches the
result in `$EMDYSI_CACHE_DIR`, `$XDG_CACHE_HOME/emdysi` or `~/.cache/emdysi`
(never in the repository); later runs load in about a second. The cache is
keyed by the grammar sources, so editing them recompiles. Set
`EMDYSI_NO_CACHE=1` to bypass it. Parsing takes from tens of milliseconds to
a few seconds per sentence.

## License

MIT. See [`LICENSE`](LICENSE). Bundled third-party data will carry its own
notices in `THIRD_PARTY_NOTICES` once added.

## Layout

| Path | Contents |
|---|---|
| `crates/emdysi-tdl` | Reader for TDL, the grammar-definition language of the ERG |
| `crates/emdysi-hpsg` | Type hierarchy, typed feature structures, unification, grammar compilation |
| `crates/emdysi-repp` | REPP tokenizer with character offsets |
| `crates/emdysi-text` | Markdown and plain-text prose blocks with source offsets; sentence segmentation |
| `crates/emdysi-check` | Document analysis, rule packs, diagnostics, fixes, reports |
| `crates/emdysi` | Command-line tool, installed as `en` |
| `crates/emdysi-parse` | Pipeline: tokenizing, tagging, token mapping, lexical lookup, parsing with the ERG |
| `grammar/erg/` | Vendored English Resource Grammar (MIT), see `SOURCE.md` |
| `packs/` | Built-in rule packs |
| `data/scowl/` | English word list, American and British spellings (ESDB/SCOWL size 60) |
| `corpora/` | Test-only corpora, each with its own license |
| `docs/` | Prior art, decisions, plan, dependency policy |
