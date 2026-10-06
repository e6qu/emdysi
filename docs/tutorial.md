# Tutorial

This tutorial walks through emdysi's command-line tool, `en`, on small
examples: checking a draft, applying fixes, reading the grammar's analysis,
seeing ambiguity, choosing rule packs, declaring a project's terms, and
writing a rule of your own. Every output below is what `en` prints for the
input shown.

## Install

emdysi needs a Rust toolchain (stable). From a clone of the repository:

```sh
cargo build --release
```

The tool is `target/release/en`. To put it on your `PATH`:

```sh
cargo install --path crates/emdysi
```

The first run compiles the English Resource Grammar, which takes a few
seconds, and caches the result outside the repository (in
`$EMDYSI_CACHE_DIR`, `$XDG_CACHE_HOME/emdysi` or `~/.cache/emdysi`). Later
runs load it in about a second.

## Check a draft

Save this as `draft.md`:

```markdown
# Release notes

In today's fast-paced world, it is important to note that our team has
delivered a robust and seamless upgrade. The new version are faster. It
recieves updates automatically.

The cache now stores results on disk, which speeds up repeated runs. Our
users has asked for this feature for a long time.
```

Check it:

```sh
en check draft.md
```

```text
draft.md:3:1: warning [structure.throat-clearing] The opening ('In today's fast-paced world') delays the point; start with the conclusion or the key fact.
    In today's fast-paced world, it is important to note that our team has
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^
draft.md:3:1: suggestion [structure.vague-lead] The opening makes no specific claim (no name, number, date, link or instruction); start with what the reader should know or do.
    In today's fast-paced world, it is important to note that our team has
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
draft.md:3:30: suggestion [ai-tells.signposting] Signposting ('it is important to note') adds words without content.
    In today's fast-paced world, it is important to note that our team has
                                 ^^^^^^^^^^^^^^^^^^^^^^^
draft.md:4:24: warning [ai-tells.vocabulary] 'seamless' is over-used in machine-written prose; prefer a plainer word.
    delivered a robust and seamless upgrade. The new version are faster. It
                           ^^^^^^^^
draft.md:4:58: warning [core.grammar-errors] Your subject doesn't agree in number with the verb 'are'.
    delivered a robust and seamless upgrade. The new version are faster. It
                                                             ^^^
    suggestions: is
draft.md:5:1: warning [core.spelling] Unknown word 'recieves'. Did you mean 'receives'?
    recieves updates automatically.
    ^^^^^^^^
    fix: "receives"
draft.md:8:7: warning [core.grammar-errors] Your subject doesn't agree in number with the verb 'has'.
    users has asked for this feature for a long time.
          ^^^
    suggestions: have
draft.md: 0 errors, 5 warnings, 2 suggestions
```

Each finding gives the file, line and column, its severity, the rule that
found it, a message and the text it refers to. The rule names say where a
finding comes from:

- `core` rules claim errors: an unknown word one edit away from a known
  word, or a grammatical error. They report only what the grammar can
  show. A grammatical error needs a sentence the grammar rejects after a
  complete search, the same error in every best analysis of the ERG's
  grammar-error variant, and a correction that the grammar then accepts.
  The suggestion (*is*, *have*) is that correction.
- `ai-tells` rules flag vocabulary and constructions that are over-used in
  machine-written prose.
- `structure` rules look at the document: here, an opening that delays the
  point and makes no concrete claim.

A run with no finding at the `--fail-on` level or worse (by default
`error`) exits with status 0, so `en check` can gate a CI job:

```sh
en check --fail-on warning draft.md
```

This prints the same report and exits with status 1, because the draft
has warnings.

For a report to paste into a pull request or an issue, ask for Markdown:

```sh
en check --format markdown draft.md
```

```markdown
## `draft.md`

0 errors, 5 warnings, 2 suggestions

- **warning** `structure.throat-clearing` (line 3, column 1): The opening ('In today's fast-paced world') delays the point; start with the conclusion or the key fact. — `In today's fast-paced world`
- **suggestion** `structure.vague-lead` (line 3, column 1): The opening makes no specific claim (no name, number, date, link or instruction); start with what the reader should know or do. — `In today's fast-paced world, it is important to note that our team has delivered a robust and seamless upgrade. The new version are faster. It recieves updates automatically.`
- **suggestion** `ai-tells.signposting` (line 3, column 30): Signposting ('it is important to note') adds words without content. — `it is important to note`
- **warning** `ai-tells.vocabulary` (line 4, column 24): 'seamless' is over-used in machine-written prose; prefer a plainer word. — `seamless`
- **warning** `core.grammar-errors` (line 4, column 58): Your subject doesn't agree in number with the verb 'are'. — `are` (suggestions: is)
- **warning** `core.spelling` (line 5, column 1): Unknown word 'recieves'. Did you mean 'receives'? — `recieves` → `receives`
- **warning** `core.grammar-errors` (line 8, column 7): Your subject doesn't agree in number with the verb 'has'. — `has` (suggestions: have)
```

## Apply fixes

`en fix` prints the text with the automatic fixes applied (the findings
marked `fix:`) and reports how many it made on standard error:

```sh
en fix draft.md > fixed.md
```

```text
draft.md: 1 fix applied
```

```markdown
# Release notes

In today's fast-paced world, it is important to note that our team has
delivered a robust and seamless upgrade. The new version are faster. It
receives updates automatically.

The cache now stores results on disk, which speeds up repeated runs. Our
users has asked for this feature for a long time.
```

Only the spelling was fixed. The grammar corrections are `suggestions:`:
the grammar accepts them, but whether *is* or *are* is right depends on
what the writer meant (*The new versions are faster*), so emdysi does not
choose. Rewording the opening is the writer's job too.

## Read the analysis

`en parse` shows how the grammar analyses each sentence: the number of
readings it found and the best one as a labelled tree.

```sh
echo "The cache stores results on disk." > one.txt
en parse one.txt
```

```text
[1] line 1: The cache stores results on disk.
    5 readings
    (S (NP (DET The) (N cache)) (VP (V (V stores) (NP (N results))) (PP (P on) (NP (N (N disk) (PT .))))))
```

`--derivations` adds the grammar's derivation (each rule and lexical
entry, with the span of words it covers), and `--mrs` the meaning in
Minimal Recursion Semantics: who does what to whom (`_store_v_cause` with
the cache as ARG1 and the results as ARG2), tense, number and scope.

```sh
en parse --derivations --mrs one.txt
```

```text
[1] line 1: The cache stores results on disk.
    5 readings
    (S (NP (DET The) (N cache)) (VP (V (V stores) (NP (N results))) (PP (P on) (NP (N (N disk) (PT .))))))
    (sb-hd_mc_c 0 7 (sp-hd_n_c 0 2 (the_1 0 1 ("the")) (n_sg_ilr 1 2 (cache_n1 1 2 ("cache")))) (hd-cmp_u_c 2 7 (hd-cmp_u_c 2 4 (v_3s-fin_olr 2 3 (store_v2 2 3 ("stores"))) (hdn_bnp_c 3 4 (hdn_optcmp_c 3 4 (n_pl_olr 3 4 (result_n1 3 4 ("results")))))) (hd-cmp_u_c 4 7 (on 4 5 ("on")) (hdn_bnp-sg-nomod_c 5 7 (hd-pct_c 5 7 (n_sg_ilr 5 6 (disk_i_n1 5 6 ("disk"))) (period_pct 6 7 (".")))))))
    [ LTOP: h0 INDEX: e2 [ e SF: prop TENSE: pres MOOD: indicative PROG: - PERF: - ] RELS: < [ _the_q<0:3> LBL: h4 ARG0: x3 [ x PERS: 3 NUM: sg IND: + ] RSTR: h5 BODY: h6 ] [ _cache_n_1<4:9> LBL: h7 ARG0: x3 ] [ _store_v_cause<10:16> LBL: h1 ARG0: e2 ARG1: x3 ARG2: x8 [ x PERS: 3 NUM: pl IND: + ] ARG3: h9 ] [ udef_q<17:24> LBL: h10 ARG0: x8 RSTR: h11 BODY: h12 ] [ _result_n_of<17:24> LBL: h13 ARG0: x8 ARG1: i14 ] [ _on_p_loc<25:27> LBL: h15 ARG0: e16 [ e SF: prop TENSE: untensed MOOD: indicative PROG: - PERF: - ] ARG1: x8 ARG2: x17 [ x PERS: 3 NUM: sg IND: + ] ] [ idiom_q_i<28:33> LBL: h18 ARG0: x17 RSTR: h19 BODY: h20 ] [ _disk_n_1<28:32> LBL: h21 ARG0: x17 ] > HCONS: < h0 qeq h1 h5 qeq h7 h9 qeq h15 h11 qeq h13 h19 qeq h21 > ICONS: < > ]
```

The style rules read this analysis instead of matching patterns: a passive
is the grammar's passive rule, a noun stack is a run of nouns in a
compound, and *set up* the verb is told apart from *setup* the noun.

## See ambiguity

English is often ambiguous, and emdysi says so. Readings with the same
meaning (the same predicate-argument dependencies) are grouped together,
and a sentence whose second meaning keeps at least 5% of the probability
is reported as ambiguous, with what differs:

```sh
echo "I saw the man with the telescope." > amb.txt
en parse amb.txt
```

```text
[1] line 1: I saw the man with the telescope.
    5 readings
    (S (NP I) (VP (VP (V saw) (NP (DET the) (N man))) (PP (P with) (NP (DET the) (N (N telescope) (PT .))))))
    ambiguous: 3 interpretations (50%, 40%, 10%)
      2. 40%: with(man, telescope) instead of with(saw, telescope)
      3. 10%: saw(pron, man, with), with(man, telescope) instead of saw(pron, man), with(saw, telescope)
```

In the best reading the telescope is what the seeing was done with
(*with(saw, telescope)*); in the second the man has it (*with(man,
telescope)*). Ambiguity is never reported as an error. To see it in a
check, load the opt-in `coverage` pack, which reports sentences whose
second meaning keeps at least 25%, and sentences the grammar could only
analyse as fragments:

```sh
en check --pack coverage amb.txt
```

```text
amb.txt:1:1: suggestion [coverage.ambiguity] This sentence can be read 2 ways (50%, 40%); the second reading has with(man, telescope) instead of with(saw, telescope).
    I saw the man with the telescope.
    ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
amb.txt: 0 errors, 0 warnings, 1 suggestion
```

## Choose rule packs

`en packs` lists the rules of the packs in use. It starts:

```text
core: Spelling and grammaticality, from the English Resource Grammar's lexicon and rules. Every rule here claims an error, so each reports only what the grammar can establish: a complete search that found no analysis, an error every best analysis of the grammar-error variant names, a word one edit from a known word.
  core.spelling (warning)
  core.grammar-errors (warning)
  core.consistency (warning)
  core.articles (warning)
  core.repeated-word (warning)
  core.double-article (warning)
ai-tells: Vocabulary and constructions that are over-represented in machine-written prose.
  ai-tells.vocabulary (warning)
  ai-tells.stock-phrases (warning)
  ai-tells.signposting (suggestion)
  ai-tells.chatbot (error)
  ai-tells.not-x-but-y (suggestion)
  ai-tells.participial-tail (suggestion)
  ai-tells.rule-of-three (suggestion)
  ai-tells.em-dash (suggestion)
plain-style: Plain-language conventions: active voice, short sentences, concrete wording.
  plain-style.passive (suggestion)
  plain-style.sentence-length (suggestion)
  plain-style.intensifiers (suggestion)
```

The default packs are `core`, `ai-tells`, `plain-style`, `substance`,
`structure` and `terms`. Opt-in packs bring other style guides' word
choices, converted from their Vale packages: `microsoft`, `google` and
`elastic`, as well as `wordlists` (hedges, weasel words, fillers) and
`equality` (insensitive wording). Naming a pack with `--pack` replaces
the defaults:

```sh
echo "Please utilize the dashboard to leverage the data." > ms.txt
en check --pack microsoft ms.txt
```

```text
ms.txt:1:8: suggestion [microsoft.wordiness] Consider using 'use' instead of 'utilize'.
    Please utilize the dashboard to leverage the data.
           ^^^^^^^
    suggestions: use
ms.txt:1:33: suggestion [microsoft.jargon] Consider using 'take advantage of' instead of the jargon 'leverage'.
    Please utilize the dashboard to leverage the data.
                                    ^^^^^^^^
    suggestions: take advantage of
ms.txt: 0 errors, 0 warnings, 2 suggestions
```

To use the defaults and more, name them all (`--pack core --pack
plain-style --pack microsoft`). `--disable RULE` skips one rule, and
`--disable 'plain-style.*'` a whole pack.

## Declare your terms

Projects have their own vocabulary: product names, acronyms, terms that
must be spelled one way. Save this as `guide.md`:

```markdown
# Set up the CLI

Log in to the dashboard with your account. The CDP syncs customer data
every hour, so the customer data platform dashboard stays current.
```

```sh
en check guide.md
```

```text
guide.md:1:14: warning [terms.undefined-acronym] 'CLI' is not defined; spell it out at first use, followed by the acronym in parentheses.
    # Set up the CLI
                 ^^^
guide.md:1:14: suggestion [terms.acronym-in-heading] 'CLI' first appears in a heading; spell it out in the heading or define it in the text first.
    # Set up the CLI
                 ^^^
guide.md:3:48: warning [terms.undefined-acronym] 'CDP' is not defined; spell it out at first use, followed by the acronym in parentheses.
    Log in to the dashboard with your account. The CDP syncs customer data
                                                   ^^^
guide.md:4:20: warning [terms.noun-stack] 'customer data platform dashboard' stacks 4 nouns; open it up with prepositions ('the strategy for integrating ...').
    every hour, so the customer data platform dashboard stays current.
                       ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
guide.md: 0 errors, 3 warnings, 1 suggestion
```

A glossary declares the project's terms once. Save this as
`glossary.toml`:

```toml
[[concept]]
id = "sign-in"
term = [
  { text = "sign in", pos = "verb", status = "preferred" },
  { text = "log in", pos = "verb", status = "deprecated" },
]

[[concept]]
id = "cdp"
definition = "Customer data platform: the service that stores customer profiles."
term = [
  { text = "customer data platform", status = "preferred" },
  { text = "CDP", status = "admitted" },
]
```

```sh
en check --glossary glossary.toml guide.md
```

```text
guide.md:1:14: warning [terms.undefined-acronym] 'CLI' is not defined; spell it out at first use, followed by the acronym in parentheses.
    # Set up the CLI
                 ^^^
guide.md:1:14: suggestion [terms.acronym-in-heading] 'CLI' first appears in a heading; spell it out in the heading or define it in the text first.
    # Set up the CLI
                 ^^^
guide.md:3:1: warning [terms.deprecated] 'Log in' is a deprecated term (sign-in): use 'sign in' instead.
    Log in to the dashboard with your account. The CDP syncs customer data
    ^^^^^^
    fix: "Sign in"
guide.md: 0 errors, 2 warnings, 1 suggestion
```

*CDP* is now a known acronym, *customer data platform* counts as one term
rather than a stack of nouns, and the deprecated *log in* has an automatic
fix. `pos = "verb"` is checked against the grammar's analysis: in *Open
the log in page*, where *log in* modifies a noun, it is not flagged.
Glossaries can also be TBX files
from terminology tools or Vale vocabularies; `en glossary --to tbx
glossary.toml` converts this one to TBX. See
[rules.md](rules.md#glossaries).

## Write a rule

A rule pack is a TOML file. Save this as `house.toml`:

```toml
[pack]
name = "house"
description = "Our house style."

[[rule]]
id = "house.utilize"
kind = "words"
match = "lemma"
severity = "warning"
words = ["utilize"]
replace = { "utilize" = "use" }
message = "Use '{replacement}' instead of '{match}'."
examples = ["We utilized the cache."]
acceptable = ["We used the cache."]
```

`match = "lemma"` matches every form of the word through the grammar's
analysis, and the fix takes the same form:

```sh
echo "The service utilizes a cache, and we utilized it in every test." > note.txt
en check --pack house.toml note.txt
```

```text
note.txt:1:13: warning [house.utilize] Use 'uses' instead of 'utilizes'.
    The service utilizes a cache, and we utilized it in every test.
                ^^^^^^^^
    fix: "uses"
note.txt:1:38: warning [house.utilize] Use 'used' instead of 'utilized'.
    The service utilizes a cache, and we utilized it in every test.
                                         ^^^^^^^^
    fix: "used"
note.txt: 0 errors, 2 warnings, 0 suggestions
```

```sh
en fix --pack house.toml note.txt
```

```text
The service uses a cache, and we used it in every test.
note.txt: 2 fixes applied
```

`examples` and `acceptable` are documents the rule must and must not flag.
For the packs in `packs/`, `cargo test` runs them. Besides word lists,
rules can match regular expressions, the grammar's constructions (a
passive, a trailing participial clause, a coordination), its semantics
(an agentless passive, a comparison without a comparand), and document
structure. [rules.md](rules.md) describes every kind.

## Local models

The default checks never need a language model. Two commands can use an
optional local one, which emdysi never bundles: a GGUF file run in-process
through llama.cpp (a build with `--features llama`), or any model behind
an OpenAI-compatible server (`llama-server`, `mlx_lm.server`, Ollama, LM
Studio).

- `en rewrite --server http://127.0.0.1:8080 draft.md` prints the text
  with guarded rewrites. A model's rewrite of a sentence is kept only if
  the grammar accepts it, its meaning (MRS) keeps the original's content,
  and no rule finds a new problem.
- `en decide` asks a model a question with fixed answers and prints the
  probability of each. The opt-in `decisions` pack uses it for checks that
  patterns cannot settle, and `--decide-readings` uses it to settle close
  calls between readings.

See [models.md](models.md) for the setup, the backends and how the
answers are calibrated.

## Where next

- [rules.md](rules.md): every rule kind and the built-in packs.
- [evaluation.md](evaluation.md): how accurate the parser and the checks
  are, measured on gold treebanks, edited text and real errors.
- [decisions.md](decisions.md): why emdysi works the way it does.
- [CONTRIBUTING.md](../CONTRIBUTING.md): building, testing and extending
  emdysi.
