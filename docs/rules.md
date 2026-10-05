# Rule packs

A rule pack is a TOML file. The built-in packs live in [`packs/`](../packs);
pass your own with `--pack path/to/pack.toml`.

```toml
[pack]
name = "house-style"
description = "Our house style."

[[rule]]
id = "house-style.utilize"
kind = "words"
match = "lemma"
severity = "warning"
words = ["utilize"]
replace = { "utilize" = "use" }
message = "Use '{replacement}' instead of '{match}'."
```

Every rule has an `id` (use the pack name as a prefix), a `kind`, an optional
`severity` (`error`, `warning` (the default) or `suggestion`), a `message`
and an optional `description`. In messages, `{match}` is the text the rule
matched. Some kinds offer more placeholders, listed below.

Optional fields shared by every kind:

- `scope`: where the rule looks: `all` (the default), `heading`, `body`
  (everything but headings), `paragraph`, `list-item`, or `lead` (the first
  paragraph of the document and of each H1 or H2 section, where the main
  point belongs);
- `examples` and `acceptable`: Markdown documents the rule must flag and
  must not flag. `cargo test` runs them for every pack in `packs/`, so a
  rule's behaviour is pinned down by its own test cases;
- `source` and `license`: where a rule or its word list comes from (a style
  guide, another linter) and under which license, for rules adapted from
  elsewhere. `en packs` shows the source.

Only the subset of TOML that packs need is supported: tables, arrays of
tables, strings (basic, literal, multi-line), numbers, booleans, arrays and
inline tables.

## Kinds

### `words`

Words or phrases. `words` lists them; phrases are space-separated words.

- `match = "lemma"` (default): compare dictionary forms from the grammar's
  analysis, so `delve` also matches *delves* and *delving*. Sentences without
  an analysis fall back to surface forms.
- `match = "surface"`: compare the words as written (case-insensitive).
- Overlapping matches of one rule are reported once, for the longest item.
- `replace`: a table from listed items to automatic fixes. The fix keeps the
  capitalization of the matched text. `{replacement}` is available in
  messages.

### `regex`

`pattern` is a regular expression (fancy-regex syntax, with look-around and
back-references) matched against each sentence. With `replace`, matches are
fixed automatically; `$1` and similar refer to capture groups. Sentences that
match `unless` are skipped, e.g. claims that come with a citation.

### `construction`

Nodes of the grammar's derivation tree. The English Resource Grammar names
its constructions, so many syntactic checks are exact:

- `rules`: globs over rule names, e.g. `v_pas*_odlr` (passive participle),
  `vp_sbrd-prd-prp_c` (a trailing participial clause), `*_crd*` (any
  coordination).
- `entries`: globs over lexical entry names, e.g. `there_expl` (expletive
  *there*).
- `types`: globs over lexical types, e.g. `v_np_le`.
- `preceded_by` and `window` (default 3): require one of these words within
  `window` tokens before the match, e.g. a form of *be* for passives.
- `not_parent`: globs over rule names; a match whose parent node (or an
  ancestor reached through lexical rules) is one of them is skipped, e.g.
  `v_j-*` for participles converted to adjectives (*the most underrated
  tool*).
- `except`: matched words to skip (case-insensitive).

To find the names to use, run `en parse --derivations` on an example.

### `coordination`

Coordinated phrases with between `min` (default 3) and `max` conjuncts, e.g.
a three-part list *clear, concise, and simple*.

### `sentence-length`

Sentences longer than `max` words. `{count}` and `{max}` are available in
messages.

### `density`

More than `max` occurrences of any of `chars` (e.g. `["—"]`) or `words` per
`per` words (default 100), over the whole document. Every occurrence is
reported. `{count}`, `{per}` and `{max}` are available in messages.

### `spelling`

Lower-case words that neither the bundled word list (ESDB/SCOWL, size 60,
American and British spellings, both *-ise* and *-ize*), nor the rarer
words of size 70 and the software terms of the cspell dictionaries
(accepted, never suggested), nor the grammar's lexicon and inflection rules
know. Names, acronyms, numbers and inline code are skipped. A word is
reported only when it looks like a typo: a known word is one edit away,
and the word occurs once in the document (a word used twice or more is one
of the document's terms, such as *kubelet*). Options are `min_length` (default 3) and
`ignore` (a list of words). `{suggestion}` is available in messages. A
suggestion is applied automatically only when it is clearly the most likely
correction: a transposed or doubled letter, or the only known word one edit
away.

### `semantics`

Checks on the semantics of the best strict analysis (the ERG's Minimal
Recursion Semantics, see `en parse --mrs`). `check` selects one:

- `missing-comparand`: the main predication is a comparative whose
  standard is not expressed (*this approach is better*); attributive
  comparatives (*a larger kitchen*) and quantities (*more desks*) are not
  flagged.
- `agentless-passive`: a tensed passive verb whose actor is not expressed
  (*the report was written*, not *... by Kim*).
- `stacked-negation`: two or more negations in a sentence (*did not see
  nothing*). `{detail}` gives the count.
- `bare-demonstrative`: a sentence opening with *this* or *that* as a whole
  noun phrase (*This shows ...*).
- `tense-shift`: in a paragraph with at least three declarative sentences,
  a sentence in the past (or present) tense while at least two thirds of
  the others are in the present (or past). `{detail}` explains.

### `grammar-errors`

Specific grammatical errors named by the grammar-error ("mal-rule")
variant of the English Resource Grammar: subject-verb agreement (*He go*),
wrong verb forms (*buyed*), missing determiners (*went to house*), *a*/*an*,
plural first nouns in compounds and others, about 800 error codes in all.
Sentences without a strict analysis, or whose analysis relies on an
unknown word, are re-parsed with that variant (loaded on first use); the
whole-sentence analyses that assume the fewest errors are used.
`{feedback}` (the ERG's own message) and `{code}` (the rule or entry that
names the error) are available in messages.

An error is a claim, so it is reported only when the grammar can show it
(decision D13): a complete search (no pruning, no time limit) found no
strict or informal analysis of the sentence; every best analysis of the
grammar-error variant names the same error; and correcting it the way its
kind suggests (another inflection of the word, a finite one for
agreement; a removed article; the subject or object form of a pronoun;
*a*/*an*; *fewer*/*less*; deleting a doubled word, ...) gives a sentence the
grammar analyses strictly. That correction is the suggestion. Corrections
are kept to ones that can only repair the error, not build another phrase:
no *-ing* form for an agreement error, no possessive for a pronoun. Errors
of other kinds, and errors in names and numbers, are not reported; nor is
an agreement error on the first word of a sentence (an imperative has no
subject), nor a missing article (a bare noun is often right in edited
text: a mass use, or headline style, which the grammar cannot tell from an
error). The analyses must not lean on generic entries for unknown words,
except for an error on the unknown word itself (*buyed*). A correct sentence the grammar does not
cover stays unanalysable after such a small change, so it is not flagged.
See [evaluation.md](evaluation.md#false-flags-on-edited-text).

Two further filters apply. When the best
analysis needs more than two corrections, nothing is reported: that is the
grammar-error variant making the best of a sentence the grammar could not
analyse (long sentences, constructions it lacks), not a list of real
errors. And a capital letter is not reported as wrong at the start of a
sentence, after a colon or a line break (list labels, verse), or next to
another capitalized word (titles such as *Your Majesty*). See
[evaluation.md](evaluation.md#grammar-errors-on-real-documents).

### `grammar`

Sentences for which the grammar finds no analysis, or only a fragment or
informal one, and for which `grammar-errors` names no specific error. `{reason}` is available in messages. Headings and table cells
are exempt. Not a claim of error: most such sentences are correct English
the grammar does not cover. Used by the opt-in `coverage` pack
(`coverage.grammar`) to show which sentences no grammar-based rule could
check.

### `articles`

*a* before a vowel sound or *an* before a consonant sound (*a alpha-level
field*, *an cluster*). The sound is read from the spelling only where the
spelling settles it: *a* and *e*, *i*, *o* words take *an*, except *one*,
*once*, *eu-* and *ewe*; *u* words take *a* when they start with a
*you* sound (*unique*, *unit*, *use*, *usual*, *utility*) and *an* when they
start with *un-* (not *uni-*), *up-*, *um-*, *ul-*, *ug-*, *ut-* or *ur-*;
*hour*, *honest*, *honor* and *heir* take *an*. Words starting with *h*, *x*
or *uni-* otherwise, single letters, acronyms (also in lower case, *an
mri*), numbers, names and function words (*options a and b*) are skipped;
the next word must be in the word list, and the article must be a
determiner in the best analysis. `{fix}` and `{word}` are available in
messages. The fix is applied automatically.

### `repeated-word`

A function word written twice in a row (*the the*, *to to*, *are are*) or,
with `articles = true`, two different articles in a row (*a the*, *the
an*). English allows some doublings (*That that is is that that is not is
not*, *Will Will will Will Will's will?*, *He had had enough*), so a
doubling is reported only in a sentence the grammar has no full analysis
for, only for words in a fixed list (articles, prepositions,
conjunctions, auxiliaries and pronouns that are never doubled in
running text), and not in runs of three or more (*Can can can can can
can*). The fix drops the second word.

### `consistency`

American and British spellings mixed in one document, e.g. *color* and
*colour*, or, in British spelling, *-ise* and *-ize* (*organise*,
*organize*). Either variety is accepted on its own; when both occur, the one
used more often wins (the first one used, on a tie) and the other words are
fixed to it. `prefer` (`"us"` or `"gb"`) and `prefer_suffix` (`"ise"` or
`"ize"`) fix the choice instead. `{variety}`, `{dominant}` and
`{replacement}` are available in messages. The variant pairs come from ESDB
(see [`data/scowl/SOURCE.md`](../data/scowl/SOURCE.md)); words that are
standard in both varieties in some sense (*tire*, *program*, *check*) are
not flagged.

### `structure`

Document structure, from the block tree of a Markdown document. `check` is
one of:

- `heading-increment`: a heading more than one level below the previous one
  (markdownlint MD001); `{from}`, `{to}`;
- `single-h1`: more than one top-level heading (MD025); `{count}`;
- `empty-section`: a heading with nothing under it before the next heading
  of the same or a higher level;
- `stacked-headings`: a heading followed directly by a subheading; `{next}`;
- `lone-subsection`: the only subsection of a section;
- `depth`: headings deeper than `max` (default 4); `{level}`, `{max}`;
- `paragraph-length`: paragraphs over `max_words` words (default 150) or
  `max_sentences` sentences; `{count}`, `{sentences}`;
- `wall-of-text`: a run of paragraphs with no heading, list, table or code
  block between them, over `max_words` words (default 450) or
  `max_paragraphs` paragraphs (default 5); `{count}`, `{paragraphs}`;
- `conclusion-at-end`: the last of two or more H2 sections is a conclusion
  or summary (heading matching `pattern`), so the main point comes last;
- `vague-lead`: an opening paragraph (of the document or of an H1 or H2
  section) in which no sentence states anything a reader can check or act
  on (a name, number, date or time in the semantics of its analysis, a
  digit, link, quotation or inline code, or an instruction) and which sets
  a generic scene (`pattern`: *today*, *landscape*, *businesses*, *more than
  ever*, ...);
- `unnumbered-steps`: a bulleted list of three or more instructions with
  words of sequence (`pattern`: *then*, *next*, *finally*, *steps*, ...) in
  its items or the paragraph before it; `{count}`.

### `parallel`

Sibling headings (`of = "headings"`: same level, same parent) or the items
of one list (`of = "list-items"`) that cannot be read in the form most of
their siblings share: an instruction (imperative), an *-ing* phrase, a
to-infinitive, a question, a full sentence, a noun phrase, or a *label:
description* item. Short fragments are often ambiguous (*Test corpora* is
an instruction or a noun phrase), so each item counts for every form it can
be read as: the form of its best analysis, plus a noun phrase or an
instruction when the grammar's lexicon has such an entry for its first word
(a noun reading only when no determiner follows: *Start the service* is
not a noun phrase). Groups of fewer than `min_items` (default 3) are
skipped; the most common form must fit at least `majority` (default 0.75)
of the group. With `form` (`imperative`, `gerund`, `infinitive`,
`question`, `statement`, `noun-phrase` or `labelled`), items are checked
against that form instead of the most common one, provided it fits at
least `majority` of the group; `ordered = true` restricts the rule to
numbered lists. An instruction is recognized from the semantics, so a
fronted condition does not hide it (*If prompted, enter your password*).
`{form}` (the item's best reading) and `{majority}` are available in
messages. See [evaluation.md](evaluation.md#headings-and-list-items)
for how this behaves on real documents.

### `acronyms`

Acronyms and initialisms (two or more capitals, no run of three lower-case
letters: *API*, *IaaS*, *PhD*; plural *s* stripped). A definition is found
by Schwartz–Hearst alignment of the letters with the words before a
parenthesized short form (*infrastructure as a service (IaaS)*), with a
parenthesized long form after it (*IaaS (infrastructure as a service)*), or
from *stands for* / *is short for*. `check` is one of `undefined`,
`defined-after-use` (`{line}` of the definition), `used-once` (defined but
not used again; `{long}`), `redefined` (`{long}`, `{first}`) and
`first-use-in-heading`. Acronyms listed in `known`, in the glossary, or in
the word list at least as common as `known_tier` (40 general audience, 50
technical (the default), 60 expert) need no definition. Roman numerals,
words in capitals for emphasis (*NOT*) and units or standards next to a
number (*5 GB*, *RFC 9110*) are not acronyms.

### `glossary`

Terms of the glossary: `check = "deprecated"` flags deprecated or
superseded terms with a fix to the concept's preferred term (inflected forms
are found through the grammar's lemmas and get a suggestion instead);
`check = "casing"` flags allowed terms written with other capitalization.
`{preferred}` and `{concept}` are available in messages. See
[Glossaries](#glossaries).

### `variants`

One term written in several ways in one document: spellings that differ only
in hyphens, spaces or case (*e-mail* / *email*, *data set* / *dataset*,
*front-end* / *frontend*), for words of at least `min_length` letters
(default 5). The spelling the glossary prefers wins, else the one used most,
else the first. Spellings to which the grammar gives different parts of
speech (the verb *set up*, the noun *setup*) are different words and are left
alone. `{preferred}` is available in messages.

### `coined-words`

Words that neither the grammar nor the word list knows, made of a known word
and a productive affix (*promptability*, *agentification*,
*hyperpersonalize*): `{base}`, `{affix}` and `{count}` (uses in the
document). Only the first use is reported. Words defined in the document
(*we call this X*, *X is a ...*, *the term X*, *"X"*), in the glossary or in
`ignore` are not reported. The `spelling` kind leaves such words to this one
when it has no spelling suggestion for them.

### `concept-names`

Capitalized names made of common words and a framework-like head noun from
`heads` (*the Clarity Loop*, *the Trust Tax*) that are not defined, linked or
in the glossary, and not in `except`. Names whose words are not common words
(*the Pareto Principle*) are left alone.

### `hyphen-chain`

Hyphen chains of three or more parts that the grammar does not know as one
word: a modifier fused onto its noun (*decision-making-framework*, fixed to
*decision-making framework*), a chain used as a noun (*the
single-source-of-truth*, suggested *single source of truth*), or a chain of
four or more parts before a noun. Chains in `except` and numbers
(*twenty-one*) are skipped. `{kind}` and `{fix}` are available in messages.

### `ly-hyphen`

A hyphen after an *-ly* adverb (*highly-available*, fixed to *highly
available*), using the grammar's analysis to tell adverbs from *-ly* nouns
and adjectives (*family-owned*, *early-stage*). `{fix}`.

### `noun-stack`

Noun-noun compounds of `min` (default 3) to `max` nouns (*customer data
platform integration strategy*), from the grammar's compound rules. Proper
names, runs of capitalized nouns and multiword glossary terms count as one
noun; words the grammar does not know are not counted. `{count}`.

### `existence`

Any of a list of patterns (Vale's `existence` rules): `tokens` are joined
into one regular expression, matched between word boundaries unless
`nonword = true`, case-insensitively with `ignorecase = true`. Matches equal
to one of `exceptions` are skipped. Inline code is never matched.

### `substitution`

Patterns with preferred replacements (Vale's `substitution` rules): a
`[rule.swap]` table maps each pattern to its replacement, with alternatives
separated by `|`. `{replacement}` is available in messages. Replacements
are offered as suggestions; with `fix = true`, a single replacement is
applied by `en fix`. `ignorecase` and `nonword` work as for `existence`.

### `adjective-stack`

Nouns that carry `min` (default 3) or more adjectives or participles used as
adjectives (*the vibrant, confident and independent women*), counted along
the grammar's chain of modifiers for the noun. `{count}`.

### `modifier-density`

Sentences in which adjectives and descriptive adverbs (not negation, focus
or degree words) number at least `min` (default 5) and make up at least
`ratio` (default 0.3) of the words. `{count}`, `{percent}`.

Neither kind is used by a built-in pack: on the Beemo sample they fire as
often on the experts' edits as on the models' output (see
[evaluation.md](evaluation.md#modifiers-purple-prose)), so they do not
mark machine-written text. Use them in a house style that limits
modifiers.

## Imported packs

Five opt-in packs are generated by `scripts/import-rules.py` from rule data
vendored under `data/` (see [`VENDORED.toml`](../VENDORED.toml)); load them
with `--pack NAME`:

| Pack | Source | License |
|---|---|---|
| `microsoft` | Vale package errata-ai/Microsoft (Microsoft Writing Style Guide) | MIT; style guide CC BY 4.0 |
| `google` | Vale package errata-ai/Google (Google developer documentation style guide) | MIT; style guide CC BY 4.0 |
| `elastic` | Vale package elastic/vale-rules (Elastic style guide) | Apache-2.0, with NOTICE |
| `wordlists` | words/hedges, words/weasels, words/fillers | MIT |
| `equality` | retext-equality (the data behind alex): insensitive wording, `type: basic` entries only | MIT |

Only Vale's `existence` and `substitution` rules are converted, with each
rule's link kept as its `source`; capitalization, sequence, readability and
similar rules are left out, since emdysi has its own grammar-based rules for
those concerns. Do not edit the generated packs: change the converter and
run `python3 scripts/import-rules.py` (CI runs it with `--check`).

Some rule data is deliberately not imported:

- proselint (BSD-3-Clause) reads a list derived from After the Deadline,
  which is GPL-licensed;
- the Red Hat Vale package (MIT) includes rules adapted from the IBM Style
  Guide, all rights to which "belong to IBM";
- GitLab's Vale rules are CC BY-SA 4.0, as are lists derived from
  Wikipedia's "Signs of AI writing";
- LanguageTool's rules are LGPL, with GPL portions from After the Deadline.

## Glossaries

A glossary lists concepts and their terms, following the TBX-Basic model of
terminology management. Put it in its own file and load it with
`--glossary FILE`, or add `[[concept]]` tables to a pack:

```toml
[[concept]]
id = "sign-in"
definition = "Authenticating to an account."
term = [
  { text = "sign in", pos = "verb", status = "preferred" },
  { text = "log in", pos = "verb", status = "deprecated" },
  { text = "sign-in", pos = "noun", status = "preferred" },
]

[[concept]]
id = "javascript"
term = [{ text = "JavaScript" }]
```

`status` is `preferred` (the default), `admitted`, `deprecated` or
`superseded`; `pos` (`noun`, `verb`, `adjective` or `adverb`) is checked
against the grammar's analysis; `case` is `exact` (the default when the term
has a capital letter) or `any`. Every loaded glossary is used by every rule.
Allowed terms are known words for the `spelling`, `coined-words` and
`concept-names` kinds, count as one noun for `noun-stack`, need no
definition as acronyms, and decide which spelling `variants` keeps. This is
how a project declares its established jargon: once *customer data platform*
is in the glossary, it is no longer a noun stack.

Glossaries can also come from other tools. `--glossary` reads TBX files
(`.tbx` or `.xml`, the ISO 30042 TermBase eXchange format of terminology
tools, in both the TBX v3 and the older TBX-Basic layout; English language
sections only, with the administrative status and part of speech mapped as
above) and Vale vocabularies (a directory with `accept.txt` and
`reject.txt`, or one of those files: accepted entries become preferred
terms with Vale's case-sensitive matching, rejected entries deprecated
terms with no replacement; entries that are regular expressions, other than
a first letter in both cases such as `[Pp]ython`, are reported and
skipped). `en glossary --to toml FILE...` and `en glossary --to tbx
FILE...` convert any of these and print the result; the case policy is not
part of TBX and is lost in that direction.

## Disabling rules

`--disable RULE` skips a rule. `--disable 'plain-style.*'` skips a whole
pack's rules by prefix.
