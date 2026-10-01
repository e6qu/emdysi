# Prior art and reuse survey

Survey date: 2026-10-01. Licenses were read from primary sources (LICENSE/README
files, crate sources, model metadata) where reachable. Items marked
**(unverified)** came from search snippets or secondary sources and must be
re-checked before any data is bundled.

Goal of the project: an **MIT-licensed Rust** library and CLI that

1. parses English into phrase (constituency) and sentence structure,
2. spell-checks and auto-fixes,
3. runs semi-deterministic style and substance checks, with AI-generated prose
   and style-guide enforcement as the main use case.

Legend: **REUSE**: depend on it or bundle it (keep notices). **PORT**: reimplement
the algorithm or rules ourselves. **IDEA**: design inspiration only. **AVOID**:
license or provenance incompatible with an MIT deliverable.

---

## TL;DR

- **Nothing existing covers the whole goal.** The closest is
  [Harper](https://github.com/Automattic/harper) (Rust, Apache-2.0). It has
  tokenizing, an FST dictionary, a Brill POS tagger, NP chunking and about 700
  grammar/style rules. It has **no constituency or dependency tree**, and its
  tagger and chunker weights are trained on **NonCommercial** UD treebanks
  (GUM, LinES). We should borrow its design, not its models.
- **There is no usable Rust constituency parser.** The phrase-structure layer
  is new work, and it is the main thing that makes this project different.
- **The safest grammar and lexicon source is the
  [English Resource Grammar](https://github.com/delph-in/erg) (MIT).** We can
  harvest its lexicon, lexical types and constructions. A full HPSG engine is
  not needed for v1.
- **Biggest trap:** "MIT" or "Apache" pretrained models that were trained on
  LDC data (Penn Treebank, OntoNotes) or NC-licensed treebanks. Examples are
  the NLTK/TextBlob perceptron, Brill's original rules, benepar, Stanza,
  CoreNLP, UDPipe models and Harper's tagger. Reimplement the algorithms and
  train on clean data.
- **There is no permissively licensed English treebank.** UD English-EWT is
  CC BY-SA 4.0, and the rest are BY-SA, BY-NC-SA or LDC. See
  [Open question 2](#open-questions).
- **Style rules:** borrow Vale's packaging and scoping model, and Harper Weir's
  and LanguageTool's pattern-and-inline-test ideas. Import
  MIT/BSD/Apache/CC-BY word lists (proselint, write-good, alex, slop-forensics,
  Kobak's excess-words list, vale-ai-tells, Google style guide) with
  attribution.

---

## 1. Rust crates

| Crate / project | License | Activity | Verdict | Notes |
|---|---|---|---|---|
| **harper-core**, harper-brill, harper-pos-utils ([repo](https://github.com/Automattic/harper)) | Apache-2.0 (code) | v2.11.0, 2026-09-16; very active | **IDEA** (dependency possible) | Hunspell-like `.dict` plus `annotations.json` compiled to `fst`. Suggestions come from Levenshtein automata. The `Expr` combinators and the **Weir** rule DSL keep tests inline. Markdown parsing via pulldown-cmark. Data: the dictionary is SCOWL (OK) and the thesaurus is Moby (PD), but the Brill tagger is trained on **GUM (CC BY-NC-SA)** and the chunker on **GUM + EWT + LinES**, per [harper#4468](https://github.com/Automattic/harper/issues/4468). Do not ship those weights. |
| nlprule | MIT/Apache code; **`.bin` data LGPL-2.1** (derived from LanguageTool) | Unmaintained since 2021 | IDEA | A LanguageTool rule engine in Rust. |
| fst + levenshtein_automata | Unlicense/MIT; MIT | Stable | **REUSE** | Core of a fast dictionary and suggestion engine. |
| symspell | MIT | 2026-03 | **REUSE** (algorithm) | Symmetric-delete suggestions and word segmentation. Its frequency dictionaries' provenance is unverified, so build our own. |
| typos / typos-dict | MIT OR Apache-2.0 | Very active | **REUSE** (verify) | A curated misspelling-to-fix table, ideal for confident auto-fix. Check the provenance of words.csv against codespell, which is GPL-2.0 (unverified). |
| spellbook | MPL-2.0 | Active | REUSE possible | Pure-Rust Hunspell. File-level copyleft is fine as an unmodified dependency, but we likely don't need it. |
| zspell | Apache-2.0 | Slow | IDEA | Hunspell-compatible. |
| hunspell-rs / -sys | FFI to C++ Hunspell (MPL/LGPL/GPL tri-license) | Stale | AVOID | |
| unicode-segmentation | MIT/Apache | Active | **REUSE** | UAX#29 words and sentences, used as the base layer. |
| sentencex (Wikimedia) | MIT | Very active | **REUSE** | Rule-based sentence splitting with abbreviation lists. |
| srx | MIT/Apache engine | 2023 | IDEA | Common rule files (LanguageTool's) are LGPL. |
| punkt | MIT/Apache | Stale | IDEA | |
| rs_conllu | MIT/Apache | | **REUSE** | CoNLL-U reading for training and evaluation. |
| whatlang | MIT | 2025 | **REUSE** | Language ID, to skip non-English spans. |
| lingua | Apache-2.0 | Active | optional | More accurate, but its models are large. |
| candle | MIT/Apache | Active | optional | For any future neural component. Weights need their own review. |
| rust-bert | Apache-2.0 | Slowing; needs libtorch | AVOID | Heavy. |
| syntaxdot | MIT/Apache | Dormant | IDEA | |
| arc-eager, crfrs, crftag, english-pos-tagger, verbora-* | MIT | New in 2026, low adoption | evaluate | |
| pos-tagger, cg3 | **GPL-3.0** | | AVOID | |
| earlgrey (Earley parser) | not checked | 2024 | evaluate | A general CFG engine. |

No Rust Link Grammar bindings exist, and upstream is LGPL anyway.

## 2. Parsers, grammars and pipelines (non-Rust)

| Resource | License | Verdict | Notes |
|---|---|---|---|
| **English Resource Grammar (ERG)**, DELPH-IN | **MIT** | **REUSE / PORT** | A broad-coverage HPSG grammar. Its lexicon (about 40k entries with lexical types), inflectional rules and constructions are the best MIT-clean grammar knowledge available. |
| ACE parser | MIT (unverified) | IDEA | A C reference for chart parsing, packing and unification. It can also produce silver training data offline. |
| Link Grammar 5.x | **LGPL-2.1** (code and English dictionary) | AVOID | Pre-5.0 was a BSD variant (unverified). |
| Stanford CoreNLP | GPL-3.0 | AVOID | |
| Stanza | Apache-2.0 code; models trained on PTB/LDC | AVOID models | |
| benepar | MIT code; models trained on WSJ (LDC) | AVOID models | Also too large. |
| spaCy en_core_web_* | MIT, with model meta listing OntoNotes (commercially licensed by Explosion) | IDEA | Statistical and dependency-only. Its tokenizer exception tables are MIT. |
| UDPipe | MPL-2.0 code; models **CC BY-NC-SA** | AVOID models | |
| Trankit | Apache-2.0 code | AVOID models | |
| XTAG English grammar | GPL (unverified) | IDEA (from papers) | |

## 3. Treebanks (training and evaluation data)

| Treebank | License | Use |
|---|---|---|
| Penn Treebank, OntoNotes | LDC | AVOID |
| UD English-EWT | CC BY-SA 4.0 | Evaluation. Training is possible, but weights are arguably share-alike, so ship them in a separate crate if at all. |
| UD English-PUD, Atis, ESLSpok, CTeTex, Pronouns | CC BY-SA 3.0/4.0 | Evaluation |
| UD English-GUM, GENTLE, ParTUT, LinES | CC BY-NC-SA | AVOID (not even training) |

Keep share-alike fixtures in a separately licensed directory, excluded from
the published crate.

## 4. Taggers and segmenters

| Resource | License | Verdict |
|---|---|---|
| Averaged perceptron tagger (Honnibal; NLTK) | MIT algorithm; **weights trained on WSJ** | PORT the algorithm, retrain |
| Brill tagger | MIT-style; rules learned from WSJ/Brown | PORT the algorithm, regenerate the rules |
| TnT | Research-only | IDEA (from the paper) |
| Punkt | Apache-2.0 code; pickle provenance unclear | PORT the algorithm |
| **pySBD / Pragmatic Segmenter "Golden Rules"** | MIT | **PORT** the rules and the test suite |
| LanguageTool `segment.srx` | LGPL-2.1 | AVOID |

## 5. Lexicons and dictionaries

| Resource | License | Verdict | Use |
|---|---|---|---|
| **SCOWL / en_US Hunspell** (en-wl/wordlist) | Permissive Atkinson notice plus component notices | **REUSE** (ship the Copyright file) | Spelling lexicon with US/UK/CA/AU variants and size tiers. |
| **AGID** inflection DB | Atkinson notice plus WordNet notice | **REUSE** | Morphology: lemma ↔ inflected forms with POS. |
| **Moby** (POS, thesaurus, hyphenation) | Public domain | **REUSE** | POS candidates (233k, noisy) and syllables. |
| Princeton WordNet / **Open English WordNet** | WordNet License / CC BY 4.0 | **REUSE** (attribution) | POS, lemmas, senses, nominalization links. |
| **CMUdict** | BSD-2-Clause | **REUSE** | Pronunciation for a/an, syllables for readability. |
| **VerbNet** | BSD-like (unverified text) | **REUSE** | Verb subcategorization frames for the parser. |
| Wiktionary / Wiktextract | Data CC BY-SA + GFDL | AVOID bundling | Offline cross-checking only. |
| COMLEX | LDC | AVOID | |

## 6. Style and prose linters

| Tool | License | Take |
|---|---|---|
| **Vale** (Go) | MIT; packages mostly MIT | The **rule packaging model**: one rule per file, styles as directories, `.vale.ini` cascade, markup-aware scopes, levels, `vale test` fixtures. Check kinds: existence, substitution, occurrence, repetition, consistency, conditional, capitalization, metric, readability, spelling, sequence, script. Lessons: regex-in-YAML gets brittle, syntax support (`sequence`) was bolted on with a weak tagger, and users escape to `script`. |
| Vale packages: Google, Microsoft, proselint, write-good, alex, Readability | MIT / BSD-3 | Import wordlists and formulas with attribution. |
| proselint | BSD-3 | Port its curated, sourced checks (keep the notice). |
| write-good | MIT | Wordlists. Its regex passive check is the example of why syntax is needed. |
| alex / retext / nlcst / cuss | MIT | The nlcst position-preserving tree (paragraph → sentence → word); small focused plugins; inclusive-language data. |
| textlint / prh | MIT | Filter rules (suppression) and the prh substitution-dictionary format. |
| **LanguageTool** | **LGPL-2.1+**, rule XML included | **IDEA only.** The most mature syntax-aware pattern DSL: POS regex, skip, inflection, antipatterns, back-referenced suggestions, inline examples, a separate disambiguation stage. |
| After the Deadline | GPL | IDEA |
| **Harper Weir** | Apache-2.0 | The best modern DSL reference: expressions, message, kind, replacement, inline `test`. |

## 7. AI-prose ("slop") signals

| Source | License | Use |
|---|---|---|
| sam-paech/slop-forensics | MIT | Method and lists for over-represented words and n-grams. |
| sam-paech/slop-score | MIT code; frequency blob **CC BY-SA** (avoid) | Its "not X but Y" detector needs a **POS-tagged stream**, which shows syntax is required. |
| sam-paech/antislop-sampler | Apache-2.0 | Banned-phrase lists. |
| berenslab/llm-excess-vocab (Kobak et al. 2025) | MIT | About 900 evidence-based excess words. |
| mandakan/llm-slop-detector | MIT | About 500 patterns, plus invisible Unicode and dash/quote checks. |
| tbhb/vale-ai-tells | MIT | 137 Vale rules: pseudo-clefts, shell nouns, abstract metaphors and more. |
| Wikipedia "Signs of AI writing" | CC BY-SA | **Taxonomy only**, restated in our own words. |

## 8. Which checks need syntax

**Tokens and regex are enough for:**
- banned and substituted words, clichés, inclusive language, profanity;
- punctuation and Unicode checks (em-dash density, curly quotes, invisible characters);
- repeated words, spelling variant consistency, sentence and paragraph length;
- readability formulas (with a syllable counter);
- n-gram density and document-level occurrence limits.

**These need POS tags and lemmas, and a phrase parser does better:**
- passive voice (versus adjectival participles);
- nominalizations and zombie nouns;
- shell nouns without an antecedent;
- not-X-but-Y and negative parallelism (X and Y must be parallel phrases);
- the rule of three (coordination of three same-category phrases);
- participial tails (", highlighting …");
- pseudo-clefts, dangling modifiers, subject–verb agreement;
- lemma-aware matching (delve, delves, delving) and homograph disambiguation;
- tense and person consistency.

This list is the case for building the parser rather than another regex
linter.

## 9. Licenses of style guides themselves

- **Google developer documentation style guide:** CC BY 4.0, so it can be
  encoded and quoted with attribution.
- **Microsoft, Chicago, AP:** proprietary. Encode conventions in our own words
  only, as user or community packs.
- **GOV.UK** (OGL) and **plainlanguage.gov** (US public domain): likely
  reusable (unverified).

---

## Proposed architecture (first cut)

```
text ─▶ markup layer (plain / Markdown via pulldown-cmark; byte spans kept)
     ─▶ segmentation (unicode-segmentation + sentencex; Golden Rules tests)
     ─▶ tokenization (contractions, clitics, numbers, URLs; spaCy-style exception tables)
     ─▶ lexicon lookup (FST built from SCOWL + AGID + Moby + OEWN + ERG lexicon)
     ─▶ spelling (FST + Levenshtein automata; typos-style confident-fix table)
     ─▶ POS tagging (lexicon candidates + rule disambiguation; optional averaged perceptron)
     ─▶ phrase parsing (feature-CFG chart parser, Earley/CKY, ERG-informed grammar;
                        partial parses / chunks when a full parse fails)
     ─▶ rules engine (declarative packs: token, lemma, POS and phrase patterns;
                      aggregate/metric checks; scopes; typed fixes; inline tests)
     ─▶ diagnostics (JSON / SARIF / human), fixes applied by span
```

Design principles:

- **Deterministic by default.** Every diagnostic is traceable to a rule and a
  span, and identical input gives identical output. Any statistical component
  is optional, behind a feature flag, and versioned.
- **Robust partial analysis.** AI prose is mostly grammatical, but rules must
  still work on fragments, lists and headings. A failed parse falls back to
  chunks.
- **Data and code licensed separately.** Third-party data goes in `data/`
  with a `THIRD_PARTY_NOTICES` file, and a build step compiles it to FSTs.
- **Rule DSL:** TOML metadata plus a small pattern language:
  - token atoms: word, `<lemma>`, `POS`, `@wordlist`, regex;
  - phrase atoms: `NP`, `VP`, `CLAUSE`, `COORD[3]`;
  - operators: `skip{0,3}`, negation, antipatterns, a marked report span;
  - fixes: `replace`, `remove`, `suggest[]` with case preservation;
  - mandatory inline `test`/`pass` examples, with CI failing on untested
    rules.

## Open questions

1. **Apache-2.0 dependencies.** Is it acceptable for the MIT crate to depend on
   Apache-2.0 crates (lingua, possibly harper-core)? Legally this is fine; it
   is a policy choice.
2. **Tagger training data.** There is no permissive English treebank. Options:
   - (a) a rule-only tagger from lexicon candidates plus hand-written
     disambiguation;
   - (b) train on UD EWT and ship the weights in a separate, CC BY-SA-labelled
     crate;
   - (c) **produce silver data** by parsing public-domain text (for example
     Project Gutenberg) with ERG/ACE, then train on it. This keeps everything
     MIT-clean.

   Recommendation: (a) for v1, (c) later.
3. **Grammar depth.** Should we use a hand-written feature CFG covering the
   constructions the rules need, or a fuller ERG port? Recommendation: CFG
   first, using ERG as the lexicon and coverage reference.
4. **Input formats beyond plain text and Markdown?** For example HTML,
   reStructuredText, or code comments.
