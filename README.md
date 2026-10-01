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
included.

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

- `core`: spelling (with safe automatic fixes) and grammaticality;
- `ai-tells`: vocabulary, stock phrases and constructions over-represented
  in machine-written prose (contrast frames, trailing participial clauses,
  three-part lists, em-dash density, chat-assistant residue);
- `plain-style`: passive voice, long sentences, intensifiers, wordy phrases,
  expletive *there*, repeated words;
- `substance`: hedged, vague and unsupported claims (*seems to*, *may
  potentially*, *studies show* without a citation, *a number of*,
  *clearly*).

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
