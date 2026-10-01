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
- `replace`: a table from listed items to automatic fixes. The fix keeps the
  capitalization of the matched text. `{replacement}` is available in
  messages.

### `regex`

`pattern` is a regular expression (fancy-regex syntax, with look-around and
back-references) matched against each sentence. With `replace`, matches are
fixed automatically; `$1` and similar refer to capture groups.

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
American and British spellings, both *-ise* and *-ize*)
nor the grammar's lexicon and inflection rules know. Names, acronyms, numbers
and inline code are skipped. Options are `min_length` (default 3) and
`ignore` (a list of words). `{suggestion}` is available in messages. A
suggestion is applied automatically only when it is clearly the most likely
correction: a transposed or doubled letter, or the only known word one edit
away.

### `grammar`

Sentences for which the grammar finds no analysis, or only a fragment or
informal one. `{reason}` is available in messages. Headings and table cells
are exempt.

## Disabling rules

`--disable RULE` skips a rule. `--disable 'plain-style.*'` skips a whole
pack's rules by prefix.
