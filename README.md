# emdysi

emdysi is an English parser and prose checker, written in Rust and
MIT-licensed. Its command-line tool, `en`:

- parses English into phrase structure and meaning, and says when a
  sentence is ambiguous;
- checks spelling and grammar, claiming an error only when the grammar
  can prove it, and applies safe fixes;
- enforces style guides: plain language, document structure, consistent
  terminology, and the tells of machine-written prose.

```sh
cargo build --release
target/release/en check draft.md
```

```text
draft.md:4:58: warning [core.grammar-errors] Your subject doesn't agree in number with the verb 'are'.
    delivered a robust and seamless upgrade. The new version are faster. It
                                                             ^^^
    suggestions: is
```

The [tutorial](docs/tutorial.md) walks through checking, fixing, parsing,
rule packs, glossaries and custom rules, with real output.

## How it works

emdysi runs the [English Resource Grammar](https://github.com/delph-in/erg)
(ERG), a broad-coverage HPSG grammar of English, with its own Rust
implementation of the DELPH-IN processing stack: a TDL reader, typed
feature structures and unification, the REPP tokenizer, chart mapping,
morphology, and a packing chart parser. On the ERG's gold treebanks
(dated measurements in [evaluation.md](docs/evaluation.md)):

- it parses 97% of the grammatical items (3,011 of 3,096);
- it finds the gold analysis among its readings for 96% of the
  constructed test suites' items, and 63% of the sentences of a Sherlock
  Holmes story;
- each analysis comes with its meaning as Minimal Recursion Semantics
  (MRS), identical to the ERG's gold MRS for every gold analysis checked
  (2,966).

A parse ranker trained only on license-clean gold trees picks the gold
analysis first for 81.7% of held-out sentences. Its scores are calibrated
probabilities, which lets emdysi report ambiguity: readings are grouped by
meaning, and when a second meaning keeps at least 5% of the probability,
the sentence is reported as ambiguous, with what differs (*I saw the man
with the telescope*: *with(saw, telescope)* 50%, *with(man, telescope)*
40%).

Famous stress sentences get full analyses and no error claims: *Buffalo
buffalo Buffalo buffalo buffalo buffalo Buffalo buffalo*, *had had had*,
garden paths, center embedding ([`corpora/stress`](corpora/stress/README.md)).
Constructions and words the ERG lacks are added in emdysi's own grammar
files ([`grammar/emdysi`](grammar/emdysi/README.md)), such as comparative
correlatives (*The more you read, the more you know*) and US-style dates.

Sentences the grammar rejects are parsed again with the ERG's
grammar-error variant, which names the error (agreement, verb form,
article, ...). An error is claimed only when every best analysis of that
variant names it and a correction makes the sentence grammatical; on
edited text in five genres the checker claims about 2 errors per 1,000
sentences of the development set
([evaluation](docs/evaluation.md#false-flags-on-edited-text)).

## Usage

```sh
en check draft.md
en check --format markdown notes.txt
en fix draft.md > fixed.md
en parse --derivations --mrs essay.md
en packs
```

Input is plain text or Markdown (chosen from the file extension, or with
`--input`); reports are plain text or Markdown (`--format`). `en --help`
lists every option.

The default rule packs are:

- `core`: spelling (with safe automatic fixes), grammatical errors named
  by the grammar, US/GB spelling consistency, *a*/*an*, repeated words.
  Every rule here claims an error, so each reports only what it can show.
- `ai-tells`: vocabulary, stock phrases and constructions over-represented
  in machine-written prose (contrast frames, trailing participial clauses,
  three-part lists, em-dash density, chat-assistant residue).
- `plain-style`: passive voice, long sentences, intensifiers, wordy
  phrases, expletive *there*, tense shifts.
- `substance`: hedged, vague and unsupported claims (*seems to*, *studies
  show* without a citation, *a number of*, *clearly*).
- `structure`: the main point first, a clean heading hierarchy, short
  paragraphs, specific and parallel headings and list items, procedures
  as numbered instructions.
- `terms`: acronyms defined at first use, one spelling per term, glossary
  terms instead of deprecated ones (`--glossary FILE`: TOML, TBX or a Vale
  vocabulary), no coined words, hyphen chains or noun stacks.

Opt-in packs (`--pack NAME`): `microsoft`, `google` and `elastic` (word
choice from those style guides, via their Vale packages), `wordlists`
(hedges, weasel words, fillers), `equality` (insensitive wording, from the
data behind alex), `coverage` (sentences the grammar could not analyse
fully, and ambiguous ones) and `decisions` (checks that ask a local model).
Write your own packs in TOML: see [rules.md](docs/rules.md).

Optional local models, never bundled: `en rewrite` proposes rewrites of
sentences the rules cannot fix, kept only if the grammar accepts them and
their meaning holds, and `en decide` asks a model a question with fixed
answers. They work with a GGUF file through llama.cpp (a build with
`--features llama`) or any OpenAI-compatible server (`llama-server`,
`mlx_lm.server`, Ollama, LM Studio). See [models.md](docs/models.md).

The first run compiles the grammar (a few seconds) and caches it in
`$EMDYSI_CACHE_DIR`, `$XDG_CACHE_HOME/emdysi` or `~/.cache/emdysi`, never
in the repository; later runs load it in about a second. Parsing takes
from tens of milliseconds to a few seconds per sentence.

## Documentation

| Document | Contents |
|---|---|
| [docs/tutorial.md](docs/tutorial.md) | A walk through the tool, with real output |
| [docs/rules.md](docs/rules.md) | Rule packs, every rule kind, glossaries |
| [docs/models.md](docs/models.md) | Optional local models: backends, decisions, rewriting |
| [docs/evaluation.md](docs/evaluation.md) | Measured accuracy: gold treebanks, edited text, real errors, minimal pairs |
| [docs/decisions.md](docs/decisions.md) | The design decisions and why |
| [docs/plan.md](docs/plan.md) | Milestones and status |
| [docs/prior-art.md](docs/prior-art.md) | Survey of existing tools and data |
| [docs/vendored.md](docs/vendored.md) | Every vendored resource, with its source, version and license |
| [docs/dependencies.md](docs/dependencies.md) | Rust dependencies and their verified licenses |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Building, testing, measuring and extending emdysi |

## Layout

| Path | Contents |
|---|---|
| `crates/emdysi-tdl` | Reader for TDL, the grammar-definition language of the ERG |
| `crates/emdysi-hpsg` | Type hierarchy, typed feature structures, unification, morphology, chart parser, MRS |
| `crates/emdysi-repp` | REPP tokenizer with character offsets |
| `crates/emdysi-parse` | The pipeline: tokenizing, tagging, token mapping, lexical lookup, parsing, ranking, ambiguity |
| `crates/emdysi-text` | Markdown and plain-text blocks with source offsets; sentence segmentation |
| `crates/emdysi-check` | Document analysis, rule packs, diagnostics, fixes, reports |
| `crates/emdysi-lm` | Optional local language models |
| `crates/emdysi-rewrite` | Guarded rewriting and typo correction |
| `crates/emdysi` | The command-line tool, installed as `en` |
| `grammar/erg` | The vendored English Resource Grammar (MIT), unchanged |
| `grammar/emdysi` | emdysi's extensions of the ERG |
| `packs/` | Built-in rule packs |
| `data/` | Word lists (SCOWL, cspell), ERG error texts, rule data from other linters |
| `corpora/` | Test and evaluation corpora, each with its license |
| `scripts/` | The vendoring check, the rule importer, the tiny test model |

## License

emdysi is MIT-licensed ([`LICENSE`](LICENSE)). It includes third-party
material under its own licenses, all compatible with MIT: see
[`THIRD_PARTY_NOTICES.md`](THIRD_PARTY_NOTICES.md) and
[docs/vendored.md](docs/vendored.md).
