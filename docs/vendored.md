# Vendored resources and their licenses

<!-- Generated from VENDORED.toml by scripts/check-vendored.py
     --update; do not edit by hand. -->

Everything in this repository that is not emdysi's own Rust code is
listed here: the grammar, word lists, rule data, model weights and
the corpora used to test and evaluate the tool. The registry
[`VENDORED.toml`](../VENDORED.toml) is the source of this page;
[`VENDORED.sha256`](../VENDORED.sha256) pins the content of every
file, and `python3 scripts/check-vendored.py` (run in CI) verifies
both, that every file under `grammar/`, `corpora/` and `data/` belongs
to exactly one entry, and that this page is current.

Policy ([`docs/dependencies.md`](dependencies.md)): only licenses
compatible with emdysi's MIT license are allowed (MIT, Apache-2.0,
BSD, ISC, CC0, CC BY, public domain, the SCOWL notice). Share-alike
material (CC BY-SA) is allowed only as test data in its own
directory and never shapes what the tool ships. Non-commercial,
no-derivatives, LDC and unclear licenses are not allowed. Material
marked *shipped* is compiled into or loaded by `en`; material
marked *test* is used only by tests and evaluations. Rust crate
dependencies are covered separately, by `cargo deny` and
[`docs/dependencies.md`](dependencies.md).

## Summary

| Resource | License | Use | Paths |
|---|---|---|---|
| [English Resource Grammar (ERG), 2025 release line](#erg) | MIT | shipped | [`grammar/erg`](../grammar/erg) |
| [emdysi's extensions of the ERG (constructions, lexical entries and formal counterparts of informal rules)](#erg-extensions) | MIT | shipped | [`grammar/emdysi`](../grammar/emdysi) |
| [ERG gold treebank profiles built from constructed test suites](#erg-gold-constructed) | MIT | test | [`corpora/erg-gold/mrs`](../corpora/erg-gold/mrs), [`corpora/erg-gold/csli`](../corpora/erg-gold/csli), [`corpora/erg-gold/esd`](../corpora/erg-gold/esd), [`corpora/erg-gold/control`](../corpora/erg-gold/control), [`corpora/erg-gold/ccs`](../corpora/erg-gold/ccs) |
| [ERG gold treebank profile of 'The Adventure of the Speckled Band'](#erg-gold-sh-spec) | MIT | test | [`corpora/erg-gold/sh-spec`](../corpora/erg-gold/sh-spec) |
| [Provenance note and license text for the ERG gold profiles](#erg-gold-docs) | MIT | test | [`corpora/erg-gold/SOURCE.md`](../corpora/erg-gold/SOURCE.md), [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE) |
| [English 'Golden Rules' sentence-segmentation test cases](#golden-rules) | MIT | test | [`corpora/golden-rules`](../corpora/golden-rules) |
| [AI-style prose samples with expected diagnostics](#ai-prose) | MIT | test | [`corpora/ai-prose`](../corpora/ai-prose) |
| [Grammatical stress sentences (buffalo, garden paths, ambiguity)](#stress) | MIT | test | [`corpora/stress`](../corpora/stress) |
| [AI-prose treebank: sentences with the right ERG reading chosen by hand](#ai-treebank) (machine-generated text) | MIT | test | [`corpora/ai-treebank`](../corpora/ai-treebank) |
| [Corpus policy](#corpora-readme) | MIT | test | [`corpora/README.md`](../corpora/README.md) |
| [English Speller Database (ESDB/SCOWL) word list, size 60 (and the size-70 additions), and US/GB variant pairs](#scowl) | LicenseRef-SCOWL | shipped | [`data/scowl/en-60.tsv`](../data/scowl/en-60.tsv), [`data/scowl/en-70-extra.txt`](../data/scowl/en-70-extra.txt), [`data/scowl/variants.tsv`](../data/scowl/variants.tsv), [`data/scowl/Copyright`](../data/scowl/Copyright), [`data/scowl/SOURCE.md`](../data/scowl/SOURCE.md) |
| [Generator for the US/GB variant table](#scowl-tools) | MIT | test | [`data/scowl/gen-variants.py`](../data/scowl/gen-variants.py) |
| [Parse-ranking model weights](#rank-model) | MIT | shipped | [`crates/emdysi-parse/data/rank.tsv`](../crates/emdysi-parse/data/rank.tsv) |
| [ERG grammar-error codes and feedback texts](#erg-errors) | MIT | shipped | [`data/erg-errors/errors.tsv`](../data/erg-errors/errors.tsv), [`data/erg-errors/SOURCE.md`](../data/erg-errors/SOURCE.md) |
| [Converter for the ERG error table](#erg-errors-tools) | MIT | test | [`data/erg-errors/gen-errors.py`](../data/erg-errors/gen-errors.py) |
| [BLiMP minimal pairs (first 30 pairs of each of the 67 paradigms)](#blimp-sample) | CC-BY-4.0 | test | [`corpora/blimp/sample.tsv`](../corpora/blimp/sample.tsv), [`corpora/blimp/LICENSE`](../corpora/blimp/LICENSE), [`corpora/blimp/SOURCE.md`](../corpora/blimp/SOURCE.md) |
| [Zorro acceptability minimal pairs (first 30 pairs of each of the 23 paradigms)](#zorro-sample) | MIT | test | [`corpora/zorro/sample.tsv`](../corpora/zorro/sample.tsv), [`corpora/zorro/LICENSE`](../corpora/zorro/LICENSE), [`corpora/zorro/SOURCE.md`](../corpora/zorro/SOURCE.md) |
| [Pride and Prejudice, chapters 1 to 6 (development) and 7 to 30 (held out) (edited fiction sample)](#edited-fiction) | LicenseRef-PublicDomain | test | [`corpora/edited/fiction.tsv`](../corpora/edited/fiction.tsv), [`corpora/edited/heldout-fiction.tsv`](../corpora/edited/heldout-fiction.tsv), [`corpora/edited/dev2-fiction.tsv`](../corpora/edited/dev2-fiction.tsv), [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md) |
| [plainlanguage.gov guideline pages (edited government text sample)](#edited-government) | CC0-1.0 | test | [`corpora/edited/government.tsv`](../corpora/edited/government.tsv), [`corpora/edited/heldout-government.tsv`](../corpora/edited/heldout-government.tsv), [`corpora/edited/dev2-government.tsv`](../corpora/edited/dev2-government.tsv), [`corpora/edited/LICENSE-plainlanguage.md`](../corpora/edited/LICENSE-plainlanguage.md) |
| [The Rust Programming Language (four chapters) and six Rust blog posts (edited technical and blog sample)](#edited-rust) | MIT | test | [`corpora/edited/rust.tsv`](../corpora/edited/rust.tsv), [`corpora/edited/heldout-rust.tsv`](../corpora/edited/heldout-rust.tsv), [`corpora/edited/dev2-rust.tsv`](../corpora/edited/dev2-rust.tsv), [`corpora/edited/LICENSE-rust-book-MIT`](../corpora/edited/LICENSE-rust-book-MIT), [`corpora/edited/LICENSE-rust-book-APACHE`](../corpora/edited/LICENSE-rust-book-APACHE), [`corpora/edited/LICENSE-rust-blog-MIT`](../corpora/edited/LICENSE-rust-blog-MIT), [`corpora/edited/LICENSE-rust-blog-APACHE`](../corpora/edited/LICENSE-rust-blog-APACHE) |
| [Kubernetes concept documentation, four pages (edited technical sample)](#edited-kubernetes) | CC-BY-4.0 | test | [`corpora/edited/kubernetes.tsv`](../corpora/edited/kubernetes.tsv), [`corpora/edited/heldout-kubernetes.tsv`](../corpora/edited/heldout-kubernetes.tsv), [`corpora/edited/dev2-kubernetes.tsv`](../corpora/edited/dev2-kubernetes.tsv), [`corpora/edited/LICENSE-kubernetes-website`](../corpora/edited/LICENSE-kubernetes-website) |
| [UD English PUD sentences, all 1,000 (edited news and Wikipedia sample, share-alike; first 500 development, last 500 held out)](#edited-pud) | CC-BY-SA-3.0 | test | [`corpora/edited-by-sa`](../corpora/edited-by-sa) |
| [Software-term word lists from the cspell dictionaries (software-terms, rust, k8s, fullstack)](#cspell-software-terms) | MIT | shipped | [`data/cspell`](../data/cspell) |
| [Typo, spelling and grammar fixes from the commit history of The Rust Programming Language](#real-errors-rust-book) | MIT | test | [`corpora/real-errors/rust-book.tsv`](../corpora/real-errors/rust-book.tsv), [`corpora/real-errors/LICENSE-rust-book-MIT`](../corpora/real-errors/LICENSE-rust-book-MIT), [`corpora/real-errors/LICENSE-rust-book-APACHE`](../corpora/real-errors/LICENSE-rust-book-APACHE), [`corpora/real-errors/SOURCE.md`](../corpora/real-errors/SOURCE.md) |
| [Typo, spelling and grammar fixes from the commit history of the Kubernetes documentation](#real-errors-kubernetes) | CC-BY-4.0 | test | [`corpora/real-errors/kubernetes.tsv`](../corpora/real-errors/kubernetes.tsv), [`corpora/real-errors/LICENSE-kubernetes-website`](../corpora/real-errors/LICENSE-kubernetes-website) |
| [Converters for the corpus samples (minimal pairs and machine-generated text)](#pair-sample-tools) | MIT | test | [`corpora/blimp/sample.py`](../corpora/blimp/sample.py), [`corpora/zorro/sample.py`](../corpora/zorro/sample.py), [`corpora/beemo/sample.py`](../corpora/beemo/sample.py), [`corpora/cheat/sample.py`](../corpora/cheat/sample.py), [`corpora/hh-rlhf/sample.py`](../corpora/hh-rlhf/sample.py), [`corpora/edited/sample.py`](../corpora/edited/sample.py), [`corpora/real-errors/mine.py`](../corpora/real-errors/mine.py) |
| [Beemo machine-generated texts with expert edits (first 200 Generation/Open QA rows from Apache-2.0/MIT models)](#beemo-sample) (machine-generated text) | MIT | test | [`corpora/beemo/sample.tsv`](../corpora/beemo/sample.tsv), [`corpora/beemo/LICENSE`](../corpora/beemo/LICENSE), [`corpora/beemo/SOURCE.md`](../corpora/beemo/SOURCE.md) |
| [CHEAT ChatGPT-generated abstracts (first 200 records of the generation file)](#cheat-sample) (machine-generated text) | MIT | test | [`corpora/cheat/sample.tsv`](../corpora/cheat/sample.tsv), [`corpora/cheat/LICENSE`](../corpora/cheat/LICENSE), [`corpora/cheat/SOURCE.md`](../corpora/cheat/SOURCE.md) |
| [Anthropic HH-RLHF helpful-base: final assistant turn of the chosen dialogue (first 200 test records)](#hh-rlhf-sample) (machine-generated text) | MIT | test | [`corpora/hh-rlhf/sample.tsv`](../corpora/hh-rlhf/sample.tsv), [`corpora/hh-rlhf/LICENSE`](../corpora/hh-rlhf/LICENSE), [`corpora/hh-rlhf/SOURCE.md`](../corpora/hh-rlhf/SOURCE.md) |
| [Vale package for the Microsoft Writing Style Guide, and the pack generated from it](#vale-microsoft) | MIT | shipped | [`data/vale/microsoft`](../data/vale/microsoft), [`packs/microsoft.toml`](../packs/microsoft.toml) |
| [Vale package for the Google developer documentation style guide, and the pack generated from it](#vale-google) | MIT | shipped | [`data/vale/google`](../data/vale/google), [`packs/google.toml`](../packs/google.toml) |
| [Vale package for the Elastic style guide, and the pack generated from it](#vale-elastic) | Apache-2.0 | shipped | [`data/vale/elastic`](../data/vale/elastic), [`packs/elastic.toml`](../packs/elastic.toml) |
| [Hedge, weasel and filler word lists (words/hedges, words/weasels, words/fillers), and the pack generated from them](#words-lists) | MIT | shipped | [`data/words`](../data/words), [`packs/wordlists.toml`](../packs/wordlists.toml) |
| [retext-equality English word lists (alex), and the pack generated from them](#retext-equality) | MIT | shipped | [`data/equality`](../data/equality), [`packs/equality.toml`](../packs/equality.toml) |

Licenses: MIT License (26), Creative Commons Attribution 4.0 (3), SCOWL/ESDB permissive notice (1), Public domain (1), CC0 1.0 (public domain dedication) (1), Creative Commons Attribution-ShareAlike 3.0 (1), Apache License 2.0 (1).

## Details

<a id="erg"></a>

### English Resource Grammar (ERG), 2025 release line

- **Paths:** [`grammar/erg`](../grammar/erg)
- **Upstream:** <https://github.com/delph-in/erg>
- **Version:** 13b615ab607374ed7cafe753830e23043a22b71d
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`grammar/erg/LICENSE`](../grammar/erg/LICENSE)
- **Copyright:** Dan Flickinger, Rob Malouf, Emily M. Bender, Stephan Oepen and the DELPH-IN contributors (ERG file headers and METADATA)
- **Provenance:** Hand-written grammar; METADATA declares LICENSE="MIT". Files were grepped for GPL/LGPL/CC/all-rights-reserved notices; none found.
- **Modifications:** Subset of files only, including the grammar-error (mal-rule) variant in educ/ and mal.tdl (see SOURCE.md); SOURCE.md added. No file content changed.
- **Use:** shipped
- **Details:** [`grammar/erg/SOURCE.md`](../grammar/erg/SOURCE.md)

<a id="erg-extensions"></a>

### emdysi's extensions of the ERG (constructions, lexical entries and formal counterparts of informal rules)

- **Paths:** [`grammar/emdysi`](../grammar/emdysi)
- **Upstream:** this repository
- **Version:** written 2026-10-05 to 2026-10-06
- **Retrieved:** 2026-10-06
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Written for this project in TDL on the ERG's types; no ERG file is changed.
- **Modifications:** n/a
- **Use:** shipped
- **Details:** [`grammar/emdysi/README.md`](../grammar/emdysi/README.md)

<a id="erg-gold-constructed"></a>

### ERG gold treebank profiles built from constructed test suites

- **Paths:** [`corpora/erg-gold/mrs`](../corpora/erg-gold/mrs), [`corpora/erg-gold/csli`](../corpora/erg-gold/csli), [`corpora/erg-gold/esd`](../corpora/erg-gold/esd), [`corpora/erg-gold/control`](../corpora/erg-gold/control), [`corpora/erg-gold/ccs`](../corpora/erg-gold/ccs)
- **Upstream:** <https://github.com/delph-in/erg> (tsdb/gold)
- **Version:** 13b615ab607374ed7cafe753830e23043a22b71d
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE)
- **Copyright:** ERG developers (item authors oe, danf); annotations by the ERG treebankers
- **Provenance:** Sentences constructed by the grammar developers as test suites (not excerpts of third-party text); annotations are part of the MIT-licensed ERG repository.
- **Modifications:** none (profiles copied verbatim)
- **Use:** test
- **Details:** [`corpora/erg-gold/SOURCE.md`](../corpora/erg-gold/SOURCE.md)

<a id="erg-gold-sh-spec"></a>

### ERG gold treebank profile of 'The Adventure of the Speckled Band'

- **Paths:** [`corpora/erg-gold/sh-spec`](../corpora/erg-gold/sh-spec)
- **Upstream:** <https://github.com/delph-in/erg> (tsdb/gold/sh-spec)
- **Version:** 13b615ab607374ed7cafe753830e23043a22b71d
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE)
- **Copyright:** Text: public domain (Arthur Conan Doyle, 1892; author died 1930). Annotations: ERG developers (MIT).
- **Provenance:** Public-domain short story; annotations from the ERG repository.
- **Modifications:** none (profile copied verbatim)
- **Use:** test
- **Details:** [`corpora/erg-gold/SOURCE.md`](../corpora/erg-gold/SOURCE.md)

<a id="erg-gold-docs"></a>

### Provenance note and license text for the ERG gold profiles

- **Paths:** [`corpora/erg-gold/SOURCE.md`](../corpora/erg-gold/SOURCE.md), [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE)
- **Upstream:** this repository (LICENSE copied from https://github.com/delph-in/erg)
- **Version:** 13b615ab607374ed7cafe753830e23043a22b71d
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE)
- **Copyright:** emdysi contributors (SOURCE.md); ERG developers (LICENSE)
- **Provenance:** Documentation.
- **Modifications:** n/a
- **Use:** test
- **Details:** [`corpora/erg-gold/SOURCE.md`](../corpora/erg-gold/SOURCE.md)

<a id="golden-rules"></a>

### English 'Golden Rules' sentence-segmentation test cases

- **Paths:** [`corpora/golden-rules`](../corpora/golden-rules)
- **Upstream:** <https://github.com/nipunsadvilkar/pySBD> (tests/lang/test_english.py), originating in https://github.com/diasks2/pragmatic_segmenter
- **Version:** 5905f13be4fc95f407b98392e0ec303617a33d86
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`corpora/golden-rules/LICENSE`](../corpora/golden-rules/LICENSE)
- **Copyright:** Copyright (c) 2019 Nipun Sadvilkar
- **Provenance:** 47 short constructed test cases from the Pragmatic Segmenter (MIT).
- **Modifications:** Converted from Python test cases to TSV (en.tsv); one pytest.param case omitted.
- **Use:** test
- **Details:** [`corpora/golden-rules/SOURCE.md`](../corpora/golden-rules/SOURCE.md)

<a id="ai-prose"></a>

### AI-style prose samples with expected diagnostics

- **Paths:** [`corpora/ai-prose`](../corpora/ai-prose)
- **Upstream:** this repository
- **Version:** written 2026-10-01
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Written for this project; no text copied from other sources; names and figures invented. expected/ is generated by crates/emdysi-check/tests/corpus.rs.
- **Generated by:** written for this project in the style of machine-written prose (not sampled from a model's output corpus); covered by the repository's MIT license
- **Modifications:** n/a
- **Use:** test
- **Details:** [`corpora/ai-prose/README.md`](../corpora/ai-prose/README.md)

<a id="stress"></a>

### Grammatical stress sentences (buffalo, garden paths, ambiguity)

- **Paths:** [`corpora/stress`](../corpora/stress)
- **Upstream:** this repository (well-known example sentences from the linguistics literature and folklore)
- **Version:** written 2026-10-05 to 2026-10-06
- **Retrieved:** 2026-10-06
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors (list and notes); the example sentences are short, widely quoted linguistic examples, attributed in README.md where the author is known
- **Provenance:** Compiled for this project from well-known examples; one sentence of Pride and Prejudice (public domain).
- **Modifications:** n/a
- **Use:** test
- **Details:** [`corpora/stress/README.md`](../corpora/stress/README.md)

<a id="ai-treebank"></a>

### AI-prose treebank: sentences with the right ERG reading chosen by hand

- **Paths:** [`corpora/ai-treebank`](../corpora/ai-treebank)
- **Upstream:** this repository (sentences from corpora/ai-prose and corpora/beemo)
- **Version:** judged 2026-10-03
- **Retrieved:** 2026-10-03
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE), [`corpora/beemo/LICENSE`](../corpora/beemo/LICENSE)
- **Copyright:** Annotations and ai-prose sentences: Copyright (c) 2026 emdysi contributors; Beemo sentences: Copyright (c) 2024 Toloka
- **Provenance:** 160 sentences (36 from corpora/ai-prose, 124 from the model outputs in corpora/beemo); for each, the derivation skeleton of the reading judged right among the six best, by crates/emdysi-parse/examples/judge.rs and hand judgement.
- **Generated by:** Beemo sentences: HuggingFaceH4/zephyr-7b-beta (MIT), mistralai/Mistral-7B-Instruct-v0.1 (Apache-2.0), mistralai/Mixtral-8x7B-Instruct-v0.1 (Apache-2.0); ai-prose sentences written for this project; judgements made for this project
- **Modifications:** Sentences copied unchanged; derivation skeletons and notes added.
- **Use:** test
- **Details:** [`corpora/ai-treebank/README.md`](../corpora/ai-treebank/README.md)

<a id="corpora-readme"></a>

### Corpus policy

- **Paths:** [`corpora/README.md`](../corpora/README.md)
- **Upstream:** this repository
- **Version:** n/a
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Documentation.
- **Modifications:** n/a
- **Use:** test
- **Details:** [`corpora/README.md`](../corpora/README.md)

<a id="scowl"></a>

### English Speller Database (ESDB/SCOWL) word list, size 60 (and the size-70 additions), and US/GB variant pairs

- **Paths:** [`data/scowl/en-60.tsv`](../data/scowl/en-60.tsv), [`data/scowl/en-70-extra.txt`](../data/scowl/en-70-extra.txt), [`data/scowl/variants.tsv`](../data/scowl/variants.tsv), [`data/scowl/Copyright`](../data/scowl/Copyright), [`data/scowl/SOURCE.md`](../data/scowl/SOURCE.md)
- **Upstream:** <https://github.com/en-wl/wordlist>
- **Version:** 1e5b7d3a72f47a71da5d28686c1dd4b397178485
- **Retrieved:** 2026-10-01
- **License:** LicenseRef-SCOWL (SCOWL/ESDB permissive notice)
- **License text:** [`data/scowl/Copyright`](../data/scowl/Copyright)
- **Copyright:** Copyright 2000-2026 by Kevin Atkinson (ESDB); see data/scowl/Copyright for the sources ESDB is derived from
- **Provenance:** Generated with the upstream tools (word-list 60 A,B,Z 1; no Australian spellings, which carry an additional notice). Per the Copyright file, for a generated list of size 80 or below that is not Australian English, no additional copyright applies beyond the notice before the ===.
- **Modifications:** Generated lists, not upstream files: en-60.tsv adds each word's smallest ESDB size; en-70-extra.txt lists the size-70 words not in en-60.tsv; variants.tsv is produced by data/scowl/gen-variants.py from the same database.
- **Use:** shipped
- **Details:** [`data/scowl/SOURCE.md`](../data/scowl/SOURCE.md)

<a id="scowl-tools"></a>

### Generator for the US/GB variant table

- **Paths:** [`data/scowl/gen-variants.py`](../data/scowl/gen-variants.py)
- **Upstream:** this repository
- **Version:** n/a
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Own code.
- **Modifications:** n/a
- **Use:** test
- **Details:** [`data/scowl/SOURCE.md`](../data/scowl/SOURCE.md)

<a id="rank-model"></a>

### Parse-ranking model weights

- **Paths:** [`crates/emdysi-parse/data/rank.tsv`](../crates/emdysi-parse/data/rank.tsv)
- **Upstream:** this repository (trained by crates/emdysi-parse/examples/train.rs)
- **Version:** trained 2026-10-01 on corpora/erg-gold
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE), [`corpora/erg-gold/LICENSE`](../corpora/erg-gold/LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors; derived only from the MIT-licensed ERG gold profiles listed above
- **Provenance:** Averaged-perceptron weights over derivation features, and a temperature that calibrates their scores, fitted on a held-out 10% of the same items (2026-10-05); training data is only corpora/erg-gold (no LDC or NC data).
- **Modifications:** n/a
- **Use:** shipped
- **Details:** [`docs/decisions.md`](../docs/decisions.md)

<a id="erg-errors"></a>

### ERG grammar-error codes and feedback texts

- **Paths:** [`data/erg-errors/errors.tsv`](../data/erg-errors/errors.tsv), [`data/erg-errors/SOURCE.md`](../data/erg-errors/SOURCE.md)
- **Upstream:** <https://github.com/delph-in/erg> (educ/ParserErrorCodes.xlsx)
- **Version:** 13b615ab607374ed7cafe753830e23043a22b71d
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`grammar/erg/LICENSE`](../grammar/erg/LICENSE)
- **Copyright:** Dan Flickinger and the ERG developers (educ/METADATA declares LICENSE="MIT")
- **Provenance:** Feedback texts and example sentences written by the ERG developers for the grammar-error (EPGY/Redbird) deployment.
- **Modifications:** Converted from the vendored grammar/erg/educ/ParserErrorCodes.xlsx by data/erg-errors/gen-errors.py: English feedback, error class and examples only (Chinese texts and comments dropped); one row per error code.
- **Use:** shipped
- **Details:** [`data/erg-errors/SOURCE.md`](../data/erg-errors/SOURCE.md)

<a id="erg-errors-tools"></a>

### Converter for the ERG error table

- **Paths:** [`data/erg-errors/gen-errors.py`](../data/erg-errors/gen-errors.py)
- **Upstream:** this repository
- **Version:** n/a
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Own code.
- **Modifications:** n/a
- **Use:** test
- **Details:** [`data/erg-errors/SOURCE.md`](../data/erg-errors/SOURCE.md)

<a id="blimp-sample"></a>

### BLiMP minimal pairs (first 30 pairs of each of the 67 paradigms)

- **Paths:** [`corpora/blimp/sample.tsv`](../corpora/blimp/sample.tsv), [`corpora/blimp/LICENSE`](../corpora/blimp/LICENSE), [`corpora/blimp/SOURCE.md`](../corpora/blimp/SOURCE.md)
- **Upstream:** <https://github.com/alexwarstadt/blimp> (data/*.jsonl)
- **Version:** 3e56b06fcabca9b30822fc66435fca6b1aa40bb1
- **Retrieved:** 2026-10-01
- **License:** CC-BY-4.0 (Creative Commons Attribution 4.0)
- **License text:** [`corpora/blimp/LICENSE`](../corpora/blimp/LICENSE)
- **Copyright:** Alex Warstadt, Alicia Parrish, Haokun Liu, Anhad Mohananey, Wei Peng, Sheng-Fu Wang, Samuel R. Bowman (TACL 2020, doi:10.1162/tacl_a_00321)
- **Provenance:** Sentences generated by the authors from templates and a hand-built vocabulary. License stated in the upstream README (no LICENSE file); LICENSE here is the CC BY 4.0 text from the SPDX License List (spdx/license-list-data 31ba1a50e5397e00a304dbadc76531740e89ee48, text/CC-BY-4.0.txt).
- **Modifications:** Sample of 2,010 pairs converted from JSON Lines to TSV by corpora/blimp/sample.py; fields other than UID, field, linguistics_term, sentence_good and sentence_bad dropped; no sentence changed.
- **Use:** test
- **Details:** [`corpora/blimp/SOURCE.md`](../corpora/blimp/SOURCE.md)

<a id="zorro-sample"></a>

### Zorro acceptability minimal pairs (first 30 pairs of each of the 23 paradigms)

- **Paths:** [`corpora/zorro/sample.tsv`](../corpora/zorro/sample.tsv), [`corpora/zorro/LICENSE`](../corpora/zorro/LICENSE), [`corpora/zorro/SOURCE.md`](../corpora/zorro/SOURCE.md)
- **Upstream:** <https://github.com/phueb/Zorro> (sentences/babyberta/*.txt)
- **Version:** 687ba433e91a3815e8204a8fc0cf0577303b1660
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`corpora/zorro/LICENSE`](../corpora/zorro/LICENSE)
- **Copyright:** Copyright (c) 2020 Philip Huebner
- **Provenance:** Sentences generated from templates and curated word lists (words chosen by frequency in child-directed speech, Newsela and Wikipedia; no sentences copied).
- **Modifications:** Sample of 690 pairs converted to TSV by corpora/zorro/sample.py; no sentence changed.
- **Use:** test
- **Details:** [`corpora/zorro/SOURCE.md`](../corpora/zorro/SOURCE.md)

<a id="edited-fiction"></a>

### Pride and Prejudice, chapters 1 to 6 (development) and 7 to 30 (held out) (edited fiction sample)

- **Paths:** [`corpora/edited/fiction.tsv`](../corpora/edited/fiction.tsv), [`corpora/edited/heldout-fiction.tsv`](../corpora/edited/heldout-fiction.tsv), [`corpora/edited/dev2-fiction.tsv`](../corpora/edited/dev2-fiction.tsv), [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)
- **Upstream:** <https://github.com/GITenberg/Pride-and-Prejudice_1342> (1342-0.txt, Project Gutenberg eBook #1342)
- **Version:** 81db45c9c48c592f0b77f01fc59e677ad0a5634e
- **Retrieved:** 2026-10-04
- **License:** LicenseRef-PublicDomain (Public domain)
- **License text:** [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)
- **Copyright:** none (Jane Austen, 1813; public domain)
- **Provenance:** Public-domain novel as transcribed by Project Gutenberg; the Project Gutenberg header and footer (and with them its trademark and license terms) are not included.
- **Modifications:** All 61 chapters (1 to 6 and 31 to 61 development, 7 to 30 held out); line breaks within paragraphs joined; converted to escaped TSV by corpora/edited/sample.py.
- **Use:** test
- **Details:** [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)

<a id="edited-government"></a>

### plainlanguage.gov guideline pages (edited government text sample)

- **Paths:** [`corpora/edited/government.tsv`](../corpora/edited/government.tsv), [`corpora/edited/heldout-government.tsv`](../corpora/edited/heldout-government.tsv), [`corpora/edited/dev2-government.tsv`](../corpora/edited/dev2-government.tsv), [`corpora/edited/LICENSE-plainlanguage.md`](../corpora/edited/LICENSE-plainlanguage.md)
- **Upstream:** <https://github.com/GSA/plainlanguage.gov> (_pages/guidelines/)
- **Version:** fd7694740f19c0ed20ed71c2dd1dc92699e920fa
- **Retrieved:** 2026-10-04
- **License:** CC0-1.0 (CC0 1.0 (public domain dedication))
- **License text:** [`corpora/edited/LICENSE-plainlanguage.md`](../corpora/edited/LICENSE-plainlanguage.md)
- **Copyright:** none (work of the United States Government, GSA; public domain in the US, CC0 1.0 worldwide)
- **Provenance:** Plain-language guidelines written by the US federal Plain Language Action and Information Network, published by GSA.
- **Modifications:** 41 pages (18 development, 23 held out); YAML front matter, Liquid tags, HTML tags and comments removed by corpora/edited/sample.py; converted to escaped TSV.
- **Use:** test
- **Details:** [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)

<a id="edited-rust"></a>

### The Rust Programming Language (four chapters) and six Rust blog posts (edited technical and blog sample)

- **Paths:** [`corpora/edited/rust.tsv`](../corpora/edited/rust.tsv), [`corpora/edited/heldout-rust.tsv`](../corpora/edited/heldout-rust.tsv), [`corpora/edited/dev2-rust.tsv`](../corpora/edited/dev2-rust.tsv), [`corpora/edited/LICENSE-rust-book-MIT`](../corpora/edited/LICENSE-rust-book-MIT), [`corpora/edited/LICENSE-rust-book-APACHE`](../corpora/edited/LICENSE-rust-book-APACHE), [`corpora/edited/LICENSE-rust-blog-MIT`](../corpora/edited/LICENSE-rust-blog-MIT), [`corpora/edited/LICENSE-rust-blog-APACHE`](../corpora/edited/LICENSE-rust-blog-APACHE)
- **Upstream:** <https://github.com/rust-lang/book> (src/) and https://github.com/rust-lang/blog.rust-lang.org (content/, commit c7510dd334a8690c207a08e3e4b6a17a8b675bf2)
- **Version:** 1500248d8f230566e4ec9f27fcbb8fe9e2898ab1 (book); c7510dd334a8690c207a08e3e4b6a17a8b675bf2 (blog)
- **Retrieved:** 2026-10-04
- **License:** MIT (MIT License)
- **License text:** [`corpora/edited/LICENSE-rust-book-MIT`](../corpora/edited/LICENSE-rust-book-MIT), [`corpora/edited/LICENSE-rust-blog-MIT`](../corpora/edited/LICENSE-rust-blog-MIT)
- **Copyright:** The Rust Project Developers (book by Steve Klabnik, Carol Nichols, Chris Krycho and contributors; blog posts by their named authors)
- **Provenance:** Both repositories are dual-licensed MIT OR Apache-2.0 (book: COPYRIGHT; blog: README, 'the blog is licensed MIT/Apache 2.0'); used here under MIT, both license texts kept.
- **Modifications:** Book chapters and blog posts as listed by sample.py (development: 4 chapters plus chapters 10 to 16, and 31 posts; held out: 10 chapters and 20 posts); front matter, template tags and HTML removed by corpora/edited/sample.py; converted to escaped TSV.
- **Use:** test
- **Details:** [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)

<a id="edited-kubernetes"></a>

### Kubernetes concept documentation, four pages (edited technical sample)

- **Paths:** [`corpora/edited/kubernetes.tsv`](../corpora/edited/kubernetes.tsv), [`corpora/edited/heldout-kubernetes.tsv`](../corpora/edited/heldout-kubernetes.tsv), [`corpora/edited/dev2-kubernetes.tsv`](../corpora/edited/dev2-kubernetes.tsv), [`corpora/edited/LICENSE-kubernetes-website`](../corpora/edited/LICENSE-kubernetes-website)
- **Upstream:** <https://github.com/kubernetes/website> (content/en/docs/concepts/)
- **Version:** 77db41e9c776b614fdb31de4cc6c8e9a70673817
- **Retrieved:** 2026-10-04
- **License:** CC-BY-4.0 (Creative Commons Attribution 4.0)
- **License text:** [`corpora/edited/LICENSE-kubernetes-website`](../corpora/edited/LICENSE-kubernetes-website)
- **Copyright:** The Kubernetes Authors
- **Provenance:** Documentation written by the Kubernetes contributors; the repository LICENSE is CC BY 4.0.
- **Modifications:** 58 pages (34 development, 24 held out); front matter, Hugo shortcodes, HTML tags and comments removed by corpora/edited/sample.py; converted to escaped TSV.
- **Use:** test
- **Details:** [`corpora/edited/SOURCE.md`](../corpora/edited/SOURCE.md)

<a id="edited-pud"></a>

### UD English PUD sentences, all 1,000 (edited news and Wikipedia sample, share-alike; first 500 development, last 500 held out)

- **Paths:** [`corpora/edited-by-sa`](../corpora/edited-by-sa)
- **Upstream:** <https://github.com/UniversalDependencies/UD_English-PUD> (en_pud-ud-test.conllu)
- **Version:** f16eba4ae7f3d161870ed320676c5088b8fa476c
- **Retrieved:** 2026-10-04
- **License:** CC-BY-SA-3.0 (Creative Commons Attribution-ShareAlike 3.0)
- **License text:** [`corpora/edited-by-sa/LICENSE-UD_English-PUD.txt`](../corpora/edited-by-sa/LICENSE-UD_English-PUD.txt)
- **Copyright:** Wikipedia contributors (text); Google and the UD English PUD contributors (treebank)
- **Provenance:** Randomly selected Wikipedia sentences, which Google makes available under CC BY-SA 3.0 (upstream README, 'Licenses and terms-of-use'). Only the sentence text is used, not the annotations.
- **Modifications:** The '# text' lines of the first 500 sentences (pud.tsv) and of the other 500 (heldout-pud.tsv), grouped by '# newdoc', converted to escaped TSV by corpora/edited/sample.py; no text changed.
- **Use:** test
- **Details:** [`corpora/edited-by-sa/SOURCE.md`](../corpora/edited-by-sa/SOURCE.md)

<a id="cspell-software-terms"></a>

### Software-term word lists from the cspell dictionaries (software-terms, rust, k8s, fullstack)

- **Paths:** [`data/cspell`](../data/cspell)
- **Upstream:** <https://github.com/streetsidesoftware/cspell-dicts> (dictionaries/*/src)
- **Version:** a69283e74295a9fed0ca16648266f4decfd95573
- **Retrieved:** 2026-10-04
- **License:** MIT (MIT License)
- **License text:** [`data/cspell/LICENSE-software-terms`](../data/cspell/LICENSE-software-terms), [`data/cspell/LICENSE-rust`](../data/cspell/LICENSE-rust), [`data/cspell/LICENSE-k8s`](../data/cspell/LICENSE-k8s), [`data/cspell/LICENSE-fullstack`](../data/cspell/LICENSE-fullstack)
- **Copyright:** Copyright (c) 2017-2025 Street Side Software; Rust dictionary also Copyright (c) 2017-2020 Alexander Andreev
- **Provenance:** Word lists compiled by the cspell maintainers and contributors. Each dictionary package declares MIT in its LICENSE and package.json (the repository root is GPL-3.0; only these MIT packages' word lists are copied).
- **Modifications:** none (six src/*.txt files copied verbatim; SOURCE.md and the four LICENSE files added)
- **Use:** shipped
- **Details:** [`data/cspell/SOURCE.md`](../data/cspell/SOURCE.md)

<a id="real-errors-rust-book"></a>

### Typo, spelling and grammar fixes from the commit history of The Rust Programming Language

- **Paths:** [`corpora/real-errors/rust-book.tsv`](../corpora/real-errors/rust-book.tsv), [`corpora/real-errors/LICENSE-rust-book-MIT`](../corpora/real-errors/LICENSE-rust-book-MIT), [`corpora/real-errors/LICENSE-rust-book-APACHE`](../corpora/real-errors/LICENSE-rust-book-APACHE), [`corpora/real-errors/SOURCE.md`](../corpora/real-errors/SOURCE.md)
- **Upstream:** <https://github.com/rust-lang/book> (git history of src/)
- **Version:** 1500248d8f230566e4ec9f27fcbb8fe9e2898ab1
- **Retrieved:** 2026-10-05
- **License:** MIT (MIT License)
- **License text:** [`corpora/real-errors/LICENSE-rust-book-MIT`](../corpora/real-errors/LICENSE-rust-book-MIT)
- **Copyright:** The Rust Project Developers and the book's contributors
- **Provenance:** Paragraphs of the book before and after commits that fix typos, spelling or grammar; both versions are part of the MIT OR Apache-2.0 repository's history (used under MIT).
- **Modifications:** Selected and aligned by corpora/real-errors/mine.py (122 paragraph pairs); no text changed.
- **Use:** test
- **Details:** [`corpora/real-errors/SOURCE.md`](../corpora/real-errors/SOURCE.md)

<a id="real-errors-kubernetes"></a>

### Typo, spelling and grammar fixes from the commit history of the Kubernetes documentation

- **Paths:** [`corpora/real-errors/kubernetes.tsv`](../corpora/real-errors/kubernetes.tsv), [`corpora/real-errors/LICENSE-kubernetes-website`](../corpora/real-errors/LICENSE-kubernetes-website)
- **Upstream:** <https://github.com/kubernetes/website> (git history of content/en/)
- **Version:** 27a415c72e2ca106a48145ea3ffa620486ccda14
- **Retrieved:** 2026-10-05
- **License:** CC-BY-4.0 (Creative Commons Attribution 4.0)
- **License text:** [`corpora/real-errors/LICENSE-kubernetes-website`](../corpora/real-errors/LICENSE-kubernetes-website)
- **Copyright:** The Kubernetes Authors
- **Provenance:** Paragraphs of the documentation before and after the 400 most recent commits that fix typos, spelling or grammar; both versions are part of the CC BY 4.0 repository's history.
- **Modifications:** Selected and aligned by corpora/real-errors/mine.py (255 paragraph pairs); no text changed.
- **Use:** test
- **Details:** [`corpora/real-errors/SOURCE.md`](../corpora/real-errors/SOURCE.md)

<a id="pair-sample-tools"></a>

### Converters for the corpus samples (minimal pairs and machine-generated text)

- **Paths:** [`corpora/blimp/sample.py`](../corpora/blimp/sample.py), [`corpora/zorro/sample.py`](../corpora/zorro/sample.py), [`corpora/beemo/sample.py`](../corpora/beemo/sample.py), [`corpora/cheat/sample.py`](../corpora/cheat/sample.py), [`corpora/hh-rlhf/sample.py`](../corpora/hh-rlhf/sample.py), [`corpora/edited/sample.py`](../corpora/edited/sample.py), [`corpora/real-errors/mine.py`](../corpora/real-errors/mine.py)
- **Upstream:** this repository
- **Version:** n/a
- **Retrieved:** 2026-10-01
- **License:** MIT (MIT License)
- **License text:** [`LICENSE`](../LICENSE)
- **Copyright:** Copyright (c) 2026 emdysi contributors
- **Provenance:** Own code.
- **Modifications:** n/a
- **Use:** test
- **Details:** [`corpora/README.md`](../corpora/README.md)

<a id="beemo-sample"></a>

### Beemo machine-generated texts with expert edits (first 200 Generation/Open QA rows from Apache-2.0/MIT models)

- **Paths:** [`corpora/beemo/sample.tsv`](../corpora/beemo/sample.tsv), [`corpora/beemo/LICENSE`](../corpora/beemo/LICENSE), [`corpora/beemo/SOURCE.md`](../corpora/beemo/SOURCE.md)
- **Upstream:** <https://github.com/Toloka/beemo> (dataset.parquet)
- **Version:** cca3b817eb6d0bf7a60f885f3022696553eabb63
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`corpora/beemo/LICENSE`](../corpora/beemo/LICENSE)
- **Copyright:** Copyright (c) 2024 Toloka
- **Provenance:** Outputs of instruction-tuned LLMs for No Robots prompts, and the same outputs edited by Toloka's expert annotators. Upstream README: expert-edited texts MIT unless the generator's terms say otherwise; model outputs subject to the generator's terms; No Robots prompts and human texts CC BY-NC 4.0 (excluded).
- **Generated by:** HuggingFaceH4/zephyr-7b-beta (MIT), mistralai/Mistral-7B-Instruct-v0.1 (Apache-2.0), mistralai/Mixtral-8x7B-Instruct-v0.1 (Apache-2.0); expert edits by Toloka annotators (MIT)
- **Modifications:** Rows: only zephyr-7b-beta, Mistral-7B-Instruct-v0.1 and Mixtral-8x7B-Instruct-v0.1, only categories Generation and Open QA (others restate No Robots input text), first 200 by id. Columns: id, model, category, model_output, human_edits only (prompt, prompt_id, human_output and the Llama-3.1/GPT-4o edits dropped). Converted to escaped TSV by corpora/beemo/sample.py; no text changed.
- **Use:** test
- **Details:** [`corpora/beemo/SOURCE.md`](../corpora/beemo/SOURCE.md)

<a id="cheat-sample"></a>

### CHEAT ChatGPT-generated abstracts (first 200 records of the generation file)

- **Paths:** [`corpora/cheat/sample.tsv`](../corpora/cheat/sample.tsv), [`corpora/cheat/LICENSE`](../corpora/cheat/LICENSE), [`corpora/cheat/SOURCE.md`](../corpora/cheat/SOURCE.md)
- **Upstream:** <https://github.com/botianzhe/CHEAT> (data/ieee-chatgpt-generation.jsonl)
- **Version:** 99bdfc2842963a5f3ba8f0a8439b0e1c6df82059
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`corpora/cheat/LICENSE`](../corpora/cheat/LICENSE)
- **Copyright:** Copyright (c) 2023 security
- **Provenance:** Abstracts written by ChatGPT from the title and keywords of IEEE papers (first-pass generation). The init, polish and fusion files (IEEE abstracts or derivatives of them) are excluded.
- **Generated by:** OpenAI ChatGPT (2023; version not stated upstream); OpenAI terms assign output rights to the user; released by the dataset authors under MIT
- **Modifications:** First 200 records in file order; only id and the generated abstract kept (IEEE titles and keywords dropped). Converted to escaped TSV by corpora/cheat/sample.py; whitespace trimmed, no text changed.
- **Use:** test
- **Details:** [`corpora/cheat/SOURCE.md`](../corpora/cheat/SOURCE.md)

<a id="hh-rlhf-sample"></a>

### Anthropic HH-RLHF helpful-base: final assistant turn of the chosen dialogue (first 200 test records)

- **Paths:** [`corpora/hh-rlhf/sample.tsv`](../corpora/hh-rlhf/sample.tsv), [`corpora/hh-rlhf/LICENSE`](../corpora/hh-rlhf/LICENSE), [`corpora/hh-rlhf/SOURCE.md`](../corpora/hh-rlhf/SOURCE.md)
- **Upstream:** <https://github.com/anthropics/hh-rlhf> (helpful-base/test.jsonl.gz, Git LFS)
- **Version:** c72f5cee8eb7b4d2ea5617657f4430d5e333af07
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`corpora/hh-rlhf/LICENSE`](../corpora/hh-rlhf/LICENSE)
- **Copyright:** Copyright (c) 2022 Anthropic
- **Provenance:** Responses of Anthropic's context-distilled 52B assistant models in conversations with crowdworkers (Bai et al. 2022, arXiv:2204.05862). Only model text is kept; helpful-base only, no red-team or harmless data.
- **Generated by:** Anthropic context-distilled 52B language models (2022); released by Anthropic under MIT
- **Modifications:** First 200 records of helpful-base/test.jsonl with a non-empty final assistant turn; only that turn of the chosen dialogue kept, with the line number as id (human turns and rejected dialogue dropped). Converted to escaped TSV by corpora/hh-rlhf/sample.py; whitespace trimmed, no text changed.
- **Use:** test
- **Details:** [`corpora/hh-rlhf/SOURCE.md`](../corpora/hh-rlhf/SOURCE.md)

<a id="vale-microsoft"></a>

### Vale package for the Microsoft Writing Style Guide, and the pack generated from it

- **Paths:** [`data/vale/microsoft`](../data/vale/microsoft), [`packs/microsoft.toml`](../packs/microsoft.toml)
- **Upstream:** <https://github.com/errata-ai/Microsoft>
- **Version:** ea5fa73c530a8413f39701b01856258c0223f101
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`data/vale/microsoft/LICENSE`](../data/vale/microsoft/LICENSE), [`data/vale/microsoft/LICENSE-style-guide`](../data/vale/microsoft/LICENSE-style-guide)
- **Copyright:** Copyright (c) 2018 - 2019 Joseph Kato (package); Microsoft Corporation (Microsoft Writing Style Guide, CC BY 4.0)
- **Provenance:** Rule files written by the package authors from the Microsoft Writing Style Guide (CC BY 4.0, verified in MicrosoftDocs/microsoft-style-guide at c6945c32294e845a84b192a094fb1b7c2c452a6a). No other sources noted.
- **Modifications:** data/vale/microsoft: verbatim (Microsoft/, LICENSE), plus LICENSE-style-guide and SOURCE.md. packs/microsoft.toml: generated by scripts/import-rules.py (existence and substitution rules only; messages' %s mapped to {match}/{replacement}; one generated example per rule).
- **Use:** shipped
- **Details:** [`data/vale/microsoft/SOURCE.md`](../data/vale/microsoft/SOURCE.md)

<a id="vale-google"></a>

### Vale package for the Google developer documentation style guide, and the pack generated from it

- **Paths:** [`data/vale/google`](../data/vale/google), [`packs/google.toml`](../packs/google.toml)
- **Upstream:** <https://github.com/errata-ai/Google>
- **Version:** 9e2483e2f09e7f623bacbbc154adb9627bed563b
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`data/vale/google/LICENSE`](../data/vale/google/LICENSE)
- **Copyright:** Copyright (c) 2018 - 2019 Joseph Kato (package); Google LLC (style guide, CC BY 4.0 per the upstream README, not verified at the primary source)
- **Provenance:** Rule files written by the package authors from the Google developer documentation style guide. No other sources noted.
- **Modifications:** data/vale/google: verbatim (Google/, LICENSE), plus SOURCE.md. packs/google.toml: generated by scripts/import-rules.py (existence and substitution rules only).
- **Use:** shipped
- **Details:** [`data/vale/google/SOURCE.md`](../data/vale/google/SOURCE.md)

<a id="vale-elastic"></a>

### Vale package for the Elastic style guide, and the pack generated from it

- **Paths:** [`data/vale/elastic`](../data/vale/elastic), [`packs/elastic.toml`](../packs/elastic.toml)
- **Upstream:** <https://github.com/elastic/vale-rules>
- **Version:** edbc6fab94cf62728753d6cc55c30109b8aee65a
- **Retrieved:** 2026-10-02
- **License:** Apache-2.0 (Apache License 2.0)
- **License text:** [`data/vale/elastic/LICENSE`](../data/vale/elastic/LICENSE), [`data/vale/elastic/NOTICE.txt`](../data/vale/elastic/NOTICE.txt)
- **Copyright:** Copyright 2025 Elasticsearch B.V.
- **Provenance:** Rule files based on the Elastic style guide (upstream README). No other sources noted.
- **Modifications:** data/vale/elastic: styles/Elastic/*.yml (as Elastic/), LICENSE and NOTICE.txt verbatim, plus SOURCE.md. packs/elastic.toml: generated by scripts/import-rules.py (existence and substitution rules only).
- **Use:** shipped
- **Details:** [`data/vale/elastic/SOURCE.md`](../data/vale/elastic/SOURCE.md)

<a id="words-lists"></a>

### Hedge, weasel and filler word lists (words/hedges, words/weasels, words/fillers), and the pack generated from them

- **Paths:** [`data/words`](../data/words), [`packs/wordlists.toml`](../packs/wordlists.toml)
- **Upstream:** <https://github.com/words/hedges,> https://github.com/words/weasels, https://github.com/words/fillers
- **Version:** hedges 0e972f034671bc9fde086abd701a8a55adfd4e0c; weasels cf8212ec0df90730571b6891e511e0d19f6fc738; fillers d1924b589e6b76d7cd8c4ee3c6d4859aadae8fc7
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`data/words/hedges/license`](../data/words/hedges/license), [`data/words/weasels/license`](../data/words/weasels/license), [`data/words/fillers/license`](../data/words/fillers/license)
- **Copyright:** Copyright (c) 2014 Titus Wormer
- **Provenance:** Hand-made lists by the author.
- **Modifications:** data/words: data.txt and license verbatim, plus SOURCE.md. packs/wordlists.toml: generated by scripts/import-rules.py.
- **Use:** shipped
- **Details:** [`data/words/SOURCE.md`](../data/words/SOURCE.md)

<a id="retext-equality"></a>

### retext-equality English word lists (alex), and the pack generated from them

- **Paths:** [`data/equality`](../data/equality), [`packs/equality.toml`](../packs/equality.toml)
- **Upstream:** <https://github.com/retextjs/retext-equality>
- **Version:** 192deddf13bc2823540b0d9a0a6e17fb6d995bcd
- **Retrieved:** 2026-10-02
- **License:** MIT (MIT License)
- **License text:** [`data/equality/license`](../data/equality/license)
- **Copyright:** Copyright (c) 2015 Titus Wormer
- **Provenance:** Entries written by the project's contributors; some notes paraphrase linked style guides. No list copied from a third party.
- **Modifications:** data/equality: en/*.yml and license verbatim, plus SOURCE.md. packs/equality.toml: generated by scripts/import-rules.py from the `type: basic` entries only (one substitution or existence rule per category file).
- **Use:** shipped
- **Details:** [`data/equality/SOURCE.md`](../data/equality/SOURCE.md)
