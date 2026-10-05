# Evaluation

## Grammaticality judgments on minimal pairs

`cargo run --release -p emdysi-check --example minimal_pairs -- FILE.tsv`
parses both sentences of each pair with the standard grammar. A sentence
is *accepted* when it has a strict analysis. A pair is correct when the
acceptable sentence is accepted and the unacceptable one is not. The
samples are in [`corpora/blimp`](../corpora/blimp/SOURCE.md) and
[`corpora/zorro`](../corpora/zorro/SOURCE.md).

The grammar is a precise judge of morphosyntax (agreement, verb forms,
case), but it accepts many sentences that are unacceptable for semantic or
pragmatic reasons (binding, islands, NPI licensing, animacy, quantifier
restrictions): the ERG licenses their syntax, and their oddness lies in
meaning. Those paradigms are the target of the statistical and local-model
layers (decision D10), which compare the two sentences instead of judging
each alone.

### BLiMP sample

2010 pairs: acceptable sentences accepted 95.7%, unacceptable rejected 29.7%, pairs correct 27.3%.

| Paradigm | Pairs | Acceptable accepted | Unacceptable rejected | Pair correct |
|---|---|---|---|---|
| adjunct_island | 30 | 100.0% | 0.0% | 0.0% |
| anaphor_gender_agreement | 30 | 100.0% | 0.0% | 0.0% |
| anaphor_number_agreement | 30 | 96.7% | 3.3% | 0.0% |
| animate_subject_passive | 30 | 100.0% | 0.0% | 0.0% |
| animate_subject_trans | 30 | 100.0% | 0.0% | 0.0% |
| causative | 30 | 93.3% | 50.0% | 50.0% |
| complex_NP_island | 30 | 100.0% | 10.0% | 10.0% |
| coordinate_structure_constraint_complex_left_branch | 30 | 76.7% | 3.3% | 0.0% |
| coordinate_structure_constraint_object_extraction | 30 | 100.0% | 66.7% | 66.7% |
| determiner_noun_agreement_1 | 30 | 96.7% | 90.0% | 86.7% |
| determiner_noun_agreement_2 | 30 | 93.3% | 73.3% | 66.7% |
| determiner_noun_agreement_irregular_1 | 30 | 100.0% | 80.0% | 80.0% |
| determiner_noun_agreement_irregular_2 | 30 | 100.0% | 80.0% | 80.0% |
| determiner_noun_agreement_with_adj_2 | 30 | 96.7% | 63.3% | 60.0% |
| determiner_noun_agreement_with_adj_irregular_1 | 30 | 100.0% | 56.7% | 56.7% |
| determiner_noun_agreement_with_adj_irregular_2 | 30 | 100.0% | 63.3% | 63.3% |
| determiner_noun_agreement_with_adjective_1 | 30 | 100.0% | 73.3% | 73.3% |
| distractor_agreement_relational_noun | 30 | 100.0% | 70.0% | 70.0% |
| distractor_agreement_relative_clause | 30 | 90.0% | 53.3% | 43.3% |
| drop_argument | 30 | 100.0% | 56.7% | 56.7% |
| ellipsis_n_bar_1 | 30 | 90.0% | 10.0% | 0.0% |
| ellipsis_n_bar_2 | 30 | 73.3% | 10.0% | 0.0% |
| existential_there_object_raising | 30 | 100.0% | 0.0% | 0.0% |
| existential_there_quantifiers_1 | 30 | 100.0% | 0.0% | 0.0% |
| existential_there_quantifiers_2 | 30 | 93.3% | 0.0% | 0.0% |
| existential_there_subject_raising | 30 | 90.0% | 40.0% | 33.3% |
| expletive_it_object_raising | 30 | 100.0% | 3.3% | 3.3% |
| inchoative | 30 | 96.7% | 30.0% | 30.0% |
| intransitive | 30 | 96.7% | 60.0% | 56.7% |
| irregular_past_participle_adjectives | 30 | 100.0% | 86.7% | 86.7% |
| irregular_past_participle_verbs | 30 | 100.0% | 96.7% | 96.7% |
| irregular_plural_subject_verb_agreement_1 | 30 | 100.0% | 76.7% | 76.7% |
| irregular_plural_subject_verb_agreement_2 | 30 | 96.7% | 83.3% | 80.0% |
| left_branch_island_echo_question | 30 | 100.0% | 43.3% | 43.3% |
| left_branch_island_simple_question | 30 | 100.0% | 40.0% | 40.0% |
| matrix_question_npi_licensor_present | 30 | 96.7% | 0.0% | 0.0% |
| npi_present_1 | 30 | 96.7% | 3.3% | 0.0% |
| npi_present_2 | 30 | 93.3% | 3.3% | 0.0% |
| only_npi_licensor_present | 30 | 100.0% | 0.0% | 0.0% |
| only_npi_scope | 30 | 100.0% | 0.0% | 0.0% |
| passive_1 | 30 | 100.0% | 10.0% | 10.0% |
| passive_2 | 30 | 100.0% | 30.0% | 30.0% |
| principle_A_c_command | 30 | 100.0% | 0.0% | 0.0% |
| principle_A_case_1 | 30 | 100.0% | 50.0% | 50.0% |
| principle_A_case_2 | 30 | 100.0% | 23.3% | 23.3% |
| principle_A_domain_1 | 30 | 100.0% | 0.0% | 0.0% |
| principle_A_domain_2 | 30 | 96.7% | 3.3% | 0.0% |
| principle_A_domain_3 | 30 | 100.0% | 0.0% | 0.0% |
| principle_A_reconstruction | 30 | 26.7% | 26.7% | 0.0% |
| regular_plural_subject_verb_agreement_1 | 30 | 96.7% | 90.0% | 90.0% |
| regular_plural_subject_verb_agreement_2 | 30 | 100.0% | 100.0% | 100.0% |
| sentential_negation_npi_licensor_present | 30 | 100.0% | 0.0% | 0.0% |
| sentential_negation_npi_scope | 30 | 90.0% | 10.0% | 0.0% |
| sentential_subject_island | 30 | 66.7% | 36.7% | 3.3% |
| superlative_quantifiers_1 | 30 | 96.7% | 3.3% | 0.0% |
| superlative_quantifiers_2 | 30 | 100.0% | 0.0% | 0.0% |
| tough_vs_raising_1 | 30 | 100.0% | 40.0% | 40.0% |
| tough_vs_raising_2 | 30 | 96.7% | 3.3% | 0.0% |
| transitive | 30 | 96.7% | 16.7% | 16.7% |
| wh_island | 30 | 90.0% | 10.0% | 10.0% |
| wh_questions_object_gap | 30 | 100.0% | 13.3% | 13.3% |
| wh_questions_subject_gap | 30 | 100.0% | 13.3% | 13.3% |
| wh_questions_subject_gap_long_distance | 30 | 96.7% | 3.3% | 0.0% |
| wh_vs_that_no_gap | 30 | 96.7% | 13.3% | 10.0% |
| wh_vs_that_no_gap_long_distance | 30 | 100.0% | 10.0% | 10.0% |
| wh_vs_that_with_gap | 30 | 96.7% | 0.0% | 0.0% |
| wh_vs_that_with_gap_long_distance | 30 | 96.7% | 3.3% | 0.0% |

### Zorro sample

690 pairs: acceptable sentences accepted 98.6%, unacceptable rejected 46.5%, pairs correct 45.2%.

| Paradigm | Pairs | Acceptable accepted | Unacceptable rejected | Pair correct |
|---|---|---|---|---|
| agreement_determiner_noun-across_1_adjective | 30 | 100.0% | 86.7% | 86.7% |
| agreement_determiner_noun-between_neighbors | 30 | 100.0% | 100.0% | 100.0% |
| agreement_subject_verb-across_prepositional_phrase | 30 | 100.0% | 100.0% | 100.0% |
| agreement_subject_verb-across_relative_clause | 30 | 100.0% | 100.0% | 100.0% |
| agreement_subject_verb-in_question_with_aux | 30 | 100.0% | 90.0% | 90.0% |
| agreement_subject_verb-in_simple_question | 30 | 100.0% | 70.0% | 70.0% |
| anaphor_agreement-pronoun_gender | 30 | 100.0% | 0.0% | 0.0% |
| argument_structure-dropped_argument | 30 | 100.0% | 16.7% | 16.7% |
| argument_structure-swapped_arguments | 30 | 100.0% | 80.0% | 80.0% |
| argument_structure-transitive | 30 | 100.0% | 33.3% | 33.3% |
| binding-principle_a | 30 | 100.0% | 20.0% | 20.0% |
| case-subjective_pronoun | 30 | 100.0% | 70.0% | 70.0% |
| ellipsis-n_bar | 30 | 96.7% | 0.0% | 0.0% |
| filler-gap-wh_question_object | 30 | 100.0% | 40.0% | 40.0% |
| filler-gap-wh_question_subject | 30 | 100.0% | 36.7% | 36.7% |
| irregular-verb | 30 | 100.0% | 96.7% | 96.7% |
| island-effects-adjunct_island | 30 | 100.0% | 0.0% | 0.0% |
| island-effects-coordinate_structure_constraint | 30 | 100.0% | 56.7% | 56.7% |
| local_attractor-in_question_with_aux | 30 | 90.0% | 23.3% | 13.3% |
| npi_licensing-matrix_question | 30 | 80.0% | 20.0% | 0.0% |
| npi_licensing-only_npi_licensor | 30 | 100.0% | 0.0% | 0.0% |
| quantifiers-existential_there | 30 | 100.0% | 30.0% | 30.0% |
| quantifiers-superlative | 30 | 100.0% | 0.0% | 0.0% |

## Machine-generated text samples

Samples of machine-generated prose for evaluating the style packs are in
[`corpora/beemo`](../corpora/beemo/SOURCE.md) (open-model outputs with
expert edits), [`corpora/cheat`](../corpora/cheat/SOURCE.md)
(ChatGPT-written abstracts) and
[`corpora/hh-rlhf`](../corpora/hh-rlhf/SOURCE.md) (assistant turns). Each
`sample.tsv` has one text per row, with backslash, tab, newline and
carriage return escaped as `\\`, `\t`, `\n`, `\r`. `cargo run --release -p emdysi-check --example rule_rates -- FILE.tsv
COLUMNS [ROWS]` checks the texts of the given columns with the built-in
packs and prints, per rule, the number of diagnostics and the rate per
1,000 words.

### Beemo: model output against its expert edit

The first 60 rows of the Beemo sample (columns 3 and 4: the raw output of
zephyr-7b-beta, Mistral-7B or Mixtral-8x7B, and the same text after an
expert annotator edited it); count (per 1,000 words):

| Rule | model_output | expert_edited |
|---|---|---|
| words | 17113 | 14908 |
| `ai-tells.chatbot` | 1 (0.1) | 0 (0.0) |
| `ai-tells.not-x-but-y` | 11 (0.6) | 9 (0.6) |
| `ai-tells.participial-tail` | 45 (2.6) | 36 (2.4) |
| `ai-tells.rule-of-three` | 45 (2.6) | 41 (2.8) |
| `ai-tells.signposting` | 4 (0.2) | 2 (0.1) |
| `ai-tells.stock-phrases` | 0 (0.0) | 1 (0.1) |
| `ai-tells.vocabulary` | 14 (0.8) | 14 (0.9) |
| `core.consistency` | 3 (0.2) | 1 (0.1) |
| `core.grammar` | 244 (14.3) | 215 (14.4) |
| `core.grammar-errors` | 723 (42.2) | 549 (36.8) |
| `core.spelling` | 23 (1.3) | 22 (1.5) |
| `plain-style.double-space` | 0 (0.0) | 6 (0.4) |
| `plain-style.expletive-there` | 21 (1.2) | 17 (1.1) |
| `plain-style.intensifiers` | 31 (1.8) | 33 (2.2) |
| `plain-style.passive` | 64 (3.7) | 70 (4.7) |
| `plain-style.repeated-word` | 1 (0.1) | 1 (0.1) |
| `plain-style.sentence-length` | 22 (1.3) | 10 (0.7) |
| `plain-style.stacked-negation` | 4 (0.2) | 3 (0.2) |
| `plain-style.tense-shift` | 6 (0.4) | 4 (0.3) |
| `plain-style.wordy` | 3 (0.2) | 2 (0.1) |
| `structure.long-paragraph` | 2 (0.1) | 2 (0.1) |
| `structure.parallel-list` | 20 (1.2) | 12 (0.8) |
| `structure.wall-of-text` | 30 (1.8) | 29 (1.9) |
| `substance.agentless-passive` | 30 (1.8) | 30 (2.0) |
| `substance.bare-demonstrative` | 5 (0.3) | 4 (0.3) |
| `substance.certainty` | 3 (0.2) | 3 (0.2) |
| `substance.hedge-adverb` | 1 (0.1) | 1 (0.1) |
| `substance.hedge-verb` | 7 (0.4) | 3 (0.2) |
| `substance.vague-quantity` | 2 (0.1) | 2 (0.1) |
| `terms.acronym-used-once` | 1 (0.1) | 2 (0.1) |
| `terms.coined-word` | 4 (0.2) | 2 (0.1) |
| `terms.hyphen-chain` | 1 (0.1) | 1 (0.1) |
| `terms.ly-hyphen` | 1 (0.1) | 1 (0.1) |
| `terms.noun-stack` | 26 (1.5) | 14 (0.9) |
| `terms.noun-string` | 29 (1.7) | 33 (2.2) |
| `terms.undefined-acronym` | 4 (0.2) | 4 (0.3) |
| `terms.variant-spelling` | 0 (0.0) | 1 (0.1) |

Most prompts are creative writing (stories, poems, letters), not
documents, so few structure and terminology rules apply. What the table
shows:

- Rules whose rate drops after expert editing point at what editors fix:
  noun stacks of four or more nouns (1.5 to 0.9 per 1,000 words),
  non-parallel list items (1.2 to 0.8), long sentences (1.3 to 0.7), named
  grammatical errors, hedging verbs and coined words.
- Rules with the same rate on both sides (wall of text, rule of three,
  participial tails, agentless passives) describe the genre, not the
  machine: editors kept them. They stay at suggestion level. `wall-of-text`
  in particular fires on half of these stories, which is right for
  documents meant to be scanned but not for fiction; disable it
  (`--disable structure.wall-of-text`) for narrative text.
- `core.grammar-errors` and `core.grammar` fire often on both sides (about
  40 and 14 per 1,000 words): dialogue, poetry and informal punctuation
  defeat the strict grammar. These rates, not the structure rules, are the
  main source of noise on creative text.

## Headings and list items

`cargo run --release -p emdysi-check --example forms -- FILE.md...` prints
the forms the `parallel` rules assign to each heading and list item. On the
repository's own documentation (README, `docs/`, the `SOURCE.md` files;
70 headings and 165 list items, written by people):

| | Strict analysis | Informal root | Fragment root | Robust root or a cover of partial analyses |
|---|---|---|---|---|
| Headings | 0 | 41 | 27 | 2 |
| List items | 91 | 30 | 17 | 27 |

Headings are fragments almost by definition, and the best analysis of a
short fragment is often an imperative: *Rule packs*, *Test corpora* and
*Open questions* all have verb readings. A heading's form is therefore the
set of forms it can be read as: the best analysis's form plus, when no
determiner follows the first word, a noun phrase if the grammar's lexicon
has a non-verb entry for that word (and an instruction if it has a verb
entry). An item is flagged only if none of its forms is the one most of its
siblings share. *label: description* items (and items ending in a colon,
before a nested list) form their own class.

With these refinements the `parallel-headings` and `parallel-list` rules
flag 15 of the 235 headings and items. Checked by hand, 14 are genuine
mixtures (*label: description* items next to full sentences, a question
among noun phrases, a statement among instructions); one was a long label
before a nested list, now recognized. The first version, which used only
the best analysis, flagged 21, several of them noun-phrase headings read as
imperatives.

### Opening paragraphs and procedures

On the same documents, `structure.vague-lead` flags no opening paragraph,
and on the AI-style samples in `corpora/ai-prose` it flags the one generic
opening (*In today's fast-paced world, the way we work is changing. ...*)
and not the plainly written control. `structure.procedure-steps` first
flagged two items of a numbered list of open questions: the grammar gives
unknown words (*Apache*, *Tagger*) a default verb entry, so *Apache-2.0
dependencies.* read as a command. A command reading now counts as an
instruction only when the clause starts with a verb the grammar knows or
with a fronted condition or phrase (*If prompted, enter your password.*).
The one remaining flag was a genuine statement among numbered
instructions in `corpora/README.md`, since rewritten.

## Grammar errors on real documents

On the first 120 rows of the Beemo sample, `core.grammar-errors` first
fired about 45 times per 1,000 words on the models' output and 39 times on
the experts' edits: roughly once every three sentences, mostly false.
Checking the diagnostics on 25 edited texts by hand showed two causes:

- capitals reported as wrong after a colon (*Keep going: Staying
  motivated ...*), at the start of a sentence after a line break, and in
  titles (*Your Majesty*);
- long sentences the grammar could not analyse, for which the grammar-error
  variant chose a reading with several made-up corrections (*Add an article
  before 'our beloved kingdom'*).

Capitals in those positions are no longer reported, and no named error is
reported when the best reading needs more than two corrections. On the 25
edited texts the diagnostics fell from 287 to 83; most of those left are
real (*On perfectly clear afternoon*, *Can you could start*). On the ERG's
`csli` test suite nothing changed: 104 of 388 ungrammatical items still
get a named error, and 17 of 965 grammatical ones a false one. Sentences
that lose their named errors fall back to the generic `core.grammar`
suggestion (*possibly ungrammatical*), which says only that the grammar
found no strict analysis.

### Beemo: rule rates after the change

The same 120 rows, with the default packs and `wordlists`; count (per
1,000 words):

| Rule | model_output | expert_edited |
|---|---|---|
| words | 31112 | 27281 |
| `ai-tells.chatbot` | 1 (0.0) | 0 (0.0) |
| `ai-tells.em-dash` | 0 (0.0) | 6 (0.2) |
| `ai-tells.not-x-but-y` | 17 (0.5) | 15 (0.5) |
| `ai-tells.participial-tail` | 76 (2.4) | 68 (2.5) |
| `ai-tells.rule-of-three` | 94 (3.0) | 86 (3.2) |
| `ai-tells.signposting` | 5 (0.2) | 2 (0.1) |
| `ai-tells.stock-phrases` | 1 (0.0) | 1 (0.0) |
| `ai-tells.vocabulary` | 25 (0.8) | 24 (0.9) |
| `core.consistency` | 5 (0.2) | 3 (0.1) |
| `core.grammar` | 734 (23.6) | 622 (22.8) |
| `core.grammar-errors` | 349 (11.2) | 277 (10.2) |
| `core.spelling` | 43 (1.4) | 38 (1.4) |
| `plain-style.double-space` | 0 (0.0) | 10 (0.4) |
| `plain-style.expletive-there` | 37 (1.2) | 29 (1.1) |
| `plain-style.intensifiers` | 36 (1.2) | 44 (1.6) |
| `plain-style.passive` | 107 (3.4) | 108 (4.0) |
| `plain-style.repeated-word` | 8 (0.3) | 3 (0.1) |
| `plain-style.sentence-length` | 42 (1.3) | 31 (1.1) |
| `plain-style.stacked-negation` | 5 (0.2) | 8 (0.3) |
| `plain-style.tense-shift` | 8 (0.3) | 5 (0.2) |
| `plain-style.wordy` | 4 (0.1) | 4 (0.1) |
| `structure.long-paragraph` | 3 (0.1) | 6 (0.2) |
| `structure.parallel-list` | 14 (0.4) | 11 (0.4) |
| `structure.procedure-steps` | 4 (0.1) | 2 (0.1) |
| `structure.vague-lead` | 1 (0.0) | 1 (0.0) |
| `structure.wall-of-text` | 55 (1.8) | 50 (1.8) |
| `substance.agentless-passive` | 50 (1.6) | 47 (1.7) |
| `substance.bare-demonstrative` | 7 (0.2) | 7 (0.3) |
| `substance.certainty` | 7 (0.2) | 5 (0.2) |
| `substance.hedge-adverb` | 2 (0.1) | 1 (0.0) |
| `substance.hedge-verb` | 12 (0.4) | 5 (0.2) |
| `substance.stacked-hedge` | 1 (0.0) | 0 (0.0) |
| `substance.vague-quantity` | 9 (0.3) | 6 (0.2) |
| `terms.acronym-used-once` | 1 (0.0) | 2 (0.1) |
| `terms.coined-word` | 9 (0.3) | 4 (0.1) |
| `terms.hyphen-chain` | 4 (0.1) | 5 (0.2) |
| `terms.ly-hyphen` | 1 (0.0) | 1 (0.0) |
| `terms.noun-stack` | 45 (1.4) | 28 (1.0) |
| `terms.noun-string` | 44 (1.4) | 45 (1.6) |
| `terms.undefined-acronym` | 5 (0.2) | 6 (0.2) |
| `terms.variant-spelling` | 1 (0.0) | 1 (0.0) |
| `wordlists.fillers` | 493 (15.8) | 469 (17.2) |
| `wordlists.hedges` | 1079 (34.7) | 952 (34.9) |
| `wordlists.weasels` | 1641 (52.7) | 1417 (51.9) |

- Rules that fire more on the models' output than on the edits point at
  what editors fix: noun stacks of four or more nouns (1.5 against 1.0 per
  1,000 words), coined words (0.3 against 0.1), hedging verbs (0.4 against
  0.2), long sentences, repeated words and vague quantities.
- Rules with the same rate on both sides (wall of text, rule of three,
  participial tails, three-noun strings, `ai-tells.vocabulary`) describe
  the genre or the prompt, not the machine; passives and intensifiers are
  even more frequent in the edits.
- The imported word lists (`wordlists`: weasels, hedges, fillers) fire
  16 to 53 times per 1,000 words on both sides alike. Bare word lists do
  not tell edited from unedited text; they stay opt-in.

## Modifiers (purple prose)

Professional editors of machine-written paragraphs often cut piled-up
adjectives and ornate description (*purple prose* in the LAMP study,
Chakrabarty et al. 2025). Two rule kinds measure it from the grammar's
analysis: adjectives on one noun (`adjective-stack`) and the share of
adjectives and descriptive adverbs in a sentence (`modifier-density`). On
the first 120 rows of the Beemo sample; count (per 1,000 words):

| Probe | model_output | expert_edited |
|---|---|---|
| 2 or more adjectives on a noun | 192 (6.2) | 171 (6.3) |
| 3 or more adjectives on a noun | 33 (1.1) | 27 (1.0) |
| modifiers at least 25% of a sentence (4 or more) | 71 (2.3) | 70 (2.6) |
| modifiers at least 33% of a sentence (5 or more) | 11 (0.4) | 14 (0.5) |

None of them separates the models' output from the edits, so no built-in
pack uses them; the kinds remain for house styles that limit modifiers.
Beemo's editors corrected facts, structure and phrasing more than
adjectives, and its prompts are mostly creative writing, so a corpus of
edited explanatory prose could still show a difference.

## False flags on edited text

The checker's error claims (the `core` pack: spelling, named grammar
errors, spelling consistency) should be right. The target is at most one
false flag per 1,000 sentences of edited English, whatever the genre.
[`corpora/edited`](../corpora/edited/SOURCE.md) and
[`corpora/edited-by-sa`](../corpora/edited-by-sa/SOURCE.md) hold published,
edited text in five genres: a novel (Austen, public domain), US government
guidance (plainlanguage.gov, CC0), technical documentation and blog posts
(the Rust book and blog, MIT/Apache; Kubernetes, CC BY 4.0), and news and
encyclopedia sentences (UD English PUD, CC BY-SA 3.0). Every claim of an
error in them is counted:
`cargo run --release -p emdysi-check --example false_flags -- OUT.tsv`
(2026-10-04, 2,498 sentences; flags per 1,000 sentences):

| Step | Blog | Fiction | Government | News and wiki | Technical | All |
|---|---|---|---|---|---|---|
| Before (with `core.grammar`, "possibly ungrammatical") | 463 | 375 | 276 | 302 | 437 | 374 |
| `core.grammar` moved to the opt-in `coverage` pack; named errors only after a complete search proved the sentence outside the grammar, and only those every best analysis names; unknown words only with a known word one edit away and used once | 53 | 27 | 30 | 46 | 41 | 39 |
| Named errors only with a correction the grammar accepts | 16 | 10 | 9 | 16 | 15 | 14 |
| Rare words (ESDB size 70) and software terms (cspell) accepted; code-like tokens masked before parsing; no corrections of names or numbers | 4 | 10 | 9 | 6 | 3 | 6.0 |

Of the 15 flags left, at least six are right: *neices* (a typo in the
novel's text), *offense* among British spellings, *labour* and *centered*
against the rest of their documents, "Its police includes" and "Literal
include things like 1". The others are an archaic construction ("there was
no enduring him"), "Affectation of candour", a list of bare noun phrases
read as sentences once an article is added ("Remote moderated usability
testing"), "Google open sourced", a sentence-initial name ("Pod is ..."),
and two words outside every list (*permittee*, *async*). About 3.6 false
flags per 1,000 sentences remain.

After two more gates (no grammar claims on bold labels, or on words the
document uses as names), the development set is at 4.8 flags per 1,000
sentences (12 flags, at least six of them real errors in the source).

With the changes of 2026-10-05 (spelling suggestions from the word list
first, sentence-initial typos, a/an and repeated words, the grammar
extensions, the grammar-error variant's types and the tagger), the
development set is at 3.6 flags per 1,000 sentences (9 flags). Three of
them were sentence-initial names read as typos (*Krug, Steve.*, *Cuaron,
whose ...*, *Isner, who ...*); a capitalized first word followed by a
comma or by another capitalized word was then taken as a name. With the
later changes (a capitalized first word is a typo candidate only from
seven letters, short ones being mostly names; agreement errors repaired
only within the tense; contractions formal), the development set is at
2.0 flags per 1,000 sentences (5 flags, measured): *neices* (a real typo
in the novel), *offense* and *centered* against the rest of their
documents, and two words outside every list, *permittee* and *async*.

A held-out test set (`heldout-*.tsv`: other chapters, pages and posts of
the same sources, 8,371 sentences, never inspected while developing) gives
the honest estimate (`SET=heldout`, 2026-10-04):

| Genre | Sentences | Flags per 1,000 | Consistency | Grammar errors | Spelling |
|---|---|---|---|---|---|
| Blog | 1,241 | 11.3 | 0.0 | 3.2 | 8.1 |
| Fiction | 2,068 | 11.1 | 5.3 | 2.9 | 2.9 |
| Government | 822 | 1.2 | 0.0 | 0.0 | 1.2 |
| News and wiki | 500 | 18.0 | 0.0 | 6.0 | 12.0 |
| Technical | 3,740 | 5.1 | 1.9 | 1.9 | 1.3 |
| All | 8,371 | 7.9 | 2.2 | 2.4 | 3.3 |

These are flags, not false flags: the held-out flags are not read, so how
many are real errors in the source is not known. Spelling generalizes
least (2 flags on the development set, 3.3 per 1,000 here): correct but
rare words are an open set. The development set is too small to show such
gaps; the next step is a larger development set from other documents of
the same sources, keeping the held-out set untouched.

A second, larger development set (`dev2-*.tsv`, 13,442 sentences: the
rest of the novel, more book chapters, blog posts and concept pages) showed
gaps the first could not:

| Step | Flags per 1,000 sentences |
|---|---|
| Gates above | 13.1 |
| No claim leaning on an analysis with a generic entry for an unknown word (except an error on that word, *buyed*); a typo must be one edit from a listed word, not one the morphology derives; words of four letters or more | 8.6 |
| Corrections must repair, not rebuild: finite forms only for agreement, none on a sentence's first word; subject/object swaps only for pronouns; no missing-article claims. A listed base plus an affix is a coinage unless a real inflection is one edit away; names skipped by the consistency rule | 6.4 |

What is left is mostly spelling (3.4 per 1,000: jargon such as *libs*,
*rootfs*, *async*; names and handles; the novel's own spellings such as
*expence* and *dropt*; and real typos in the sources: *durnig*, *anyeone*,
*righly*, *reseearchers*), spelling consistency within a document (1.7),
and named grammar errors (1.0, mostly archaic constructions in the novel).

The price is recall. On the `csli` test suite, named errors now catch 33
of 388 ungrammatical sentences (104 before) and wrongly flag 3 of 965
grammatical ones (17 before): the checker names fewer errors, and nearly
all it names are real. A grammatical error is reported only when the
grammar can show it: the sentence is outside the grammar after a complete
search, every best analysis of the grammar-error variant names the same
error, and correcting it the way the error's kind suggests (another form
of the word, an added or removed article, another case of a pronoun, a/an,
fewer/less, ...) gives a sentence the grammar accepts. The verified
correction is offered as the suggestion.

## Recall on real errors

Precision alone is easy to get by flagging nothing.
[`corpora/real-errors`](../corpora/real-errors/SOURCE.md) holds 377
paragraphs from the Rust book and the Kubernetes documentation as they
were before and after a commit that fixed a typo, a spelling or a grammar
error. An error counts as caught when a `core` diagnostic overlaps the
changed words; a diagnostic on the same place in the fixed paragraph is a
false flag:
`cargo run --release -p emdysi-check --example real_errors -- OUT.tsv`
(2026-10-05):

| Step | Spelling (178) | Function words (123) | Inflections (76) | Flagged after the fix |
|---|---|---|---|---|
| Gates of the previous section | 129 | 0 | 1 | 0 |
| Listed words suggested first; a capitalized first word checked in lower case; a listed base plus an affix is a typo when another common word is one edit away; at most one inflection per word | 145 | 0 | 1 | 0 |
| Deterministic a/an, repeated-word and double-article rules | 145 | 26 | 1 | 0 |
| Repeated words claimed only in sentences the grammar rejects (*that that is is*, *Will Will will* are English) | 145 | 22 | 1 | 0 |
| Precision first: no "common word one edit away" guess for coinages (*destructures*, *liveness* are words), sentence-initial typos only from seven letters, contractions formal | 139 | 22 | 1 | 0 |

The 33 spelling misses left are mostly words two edits from the intended
one (*certicate*, *neccesary*, *admistrators*), capitalized words after
the start of a sentence, which are taken as names (*Mananger*, *Servies*),
words of three letters (*tha*, *wil*, *Nex*), abbreviations (*perf*,
*langs*), and forms the grammar's morphology derives (*informations*,
*stucked*, *returing*). Most of the grammar
pairs are rewordings rather than errors (*a* to *the*, *use* to *we use*);
the errors among them that no rule catches are mostly wrong verb forms in
sentences the grammar still analyses (*adds* for *add*, *is granted* for
*granted*) and missing words.

## Stress sentences and ambiguity

[`corpora/stress`](../corpora/stress/README.md) has 66 grammatical
sentences that are hard for parsers and checkers: the buffalo and police
sentences, *had had*, *That that is is ...*, *Will Will will Will Will's
will?*, garden paths (*The horse raced past the barn fell*, *The old man
the boat*, *The prime number few*), center embedding, famous ambiguities,
and comparative correlatives (*The more you read, the more you know*,
*The bigger they are, the harder they fall*, *The more, the merrier*).
`crates/emdysi-check/tests/stress.rs` (2026-10-05): the `core` pack
claims no error in any of them, and all 66 get a full analysis.

With the ERG alone, 54 of the first 58 did. The comparative correlatives
had no analysis at all, and *The prime number few* (and *the brave*,
*the free*, *the meek*) none with the adjective as a plural noun for
people: both are now covered by emdysi's grammar extensions
([`grammar/emdysi`](../grammar/emdysi/README.md), D14). *Can can can can
can can* (a bare singular count noun as subject, not standard English)
was replaced by *Cans can can cans* and *Cans cans can can can cans*,
which the grammar analyses.

Loading the grammar-error variant also showed that seven of its types
(bare nouns, unknown proper names, partitive determiners, quasi-modals
such as *ought to*, ...) had inconsistent constraints, leaving their
lexical entries unusable: the variant redefines types of the standard
grammar, and emdysi merged the two definitions where ACE lets the later
one replace the earlier. A later definition now replaces the earlier one,
and a test checks that both grammars load without an inconsistent type
or a rule that cannot be built (`crates/emdysi-parse/tests/grammar_loads.rs`).

With those types working, the grammar-error variant read a sentence-initial
*He* as an unknown name (*He go* then agrees, as a plural name would): the
tagger, which only guessed from suffixes, tagged closed-class words such as
*He*, *The* and *to* as nouns, and the ERG makes a sentence-initial
capitalized noun a candidate name. Closed-class words now get their own
tags (pronouns, determiners, prepositions, modals, ...), as a statistical
tagger would give them, so no sentence starting with *He*, *The* or *We*
gets a spurious name reading; and the grammar-error check no longer counts
an analysis built from fragments, or one with a generic entry for a word
the lexicon knows, as a whole-sentence analysis.

On the ERG's gold profiles (`examples/eval.rs`, the code before these
changes against after, 2026-10-05), the extensions and the tagger change
cost no gold analysis and reject more ungrammatical items:

| Profile | Grammatical parsed | Ungrammatical parsed (lower is better) | Gold tree found | Gold ranked first |
|---|---|---|---|---|
| csli | 939 → 938 of 960 | 270 → 254 of 388 | 907 → 908 of 921 | 884 → 885 |
| esd | 62 → 61 of 62 | | 58 → 58 of 59 | 57 → 57 |
| control | 1,294 → 1,292 of 1,305 | 498 → 497 of 527 | 1,494 → 1,494 of 1,582 | 1,384 → 1,384 |
| sh-spec | 525 → 522 of 599 | | 358 → 360 of 576 | 314 → 313 |
| mrs, ccs | unchanged | | unchanged | unchanged |

The 16 ungrammatical items no longer accepted (*We works*, *Her hired
him*, *Him hired her*, *Whom hired Browne?*, *Every programmers were
hired*, ...) had been "parsed" by reading the capitalized first word as a
name. The grammatical items lost had the same kind of analysis: none of
them had the gold tree among its readings before either (*Did or will
Abrams interview Browne?*, *Because.*, three long sentences of the
Sherlock Holmes story whose analyses began with a name reading of
*From* or *The*). Parsing time is unchanged.

Two more extensions followed from the sentences of the development set
that get no full analysis (found with
`cargo run --release -p emdysi-parse --example culprits`, which replaces
each word of such a sentence in turn by a plain word and reports the words
whose replacement gives a full analysis): contracted auxiliaries
(*we'll*, *it's*), which the ERG marks informal to steer its generator,
now count as formal, and a sentence may start with a coordinating
conjunction (*And his sisters are charming women.*, *But I saw nothing.*),
which the ERG analyses as a fragment (`cl_cnj-frg_c`; emdysi's rule, a
strict counterpart, applies only to the whole input). On the gold profiles
neither changes coverage, the gold trees found, or the gold trees ranked
first (sh-spec: 360 found, 313 first, with or without them; the eval
compares emdysi's rule under the name of the ERG rule it stands for).

The ranker's scores (an averaged perceptron) are not probabilities. A
temperature fitted on the held-out 10% of the gold items (5.0, by
maximum likelihood of the gold reading;
`NO_PARSE=1 CALIBRATE_ONLY=1 cargo run --release -p emdysi-parse --example train -- CACHE`)
makes them calibrated: on those 273 items,

| Probability of the best reading | Items | Mean probability | Best reading right |
|---|---|---|---|
| 0-20% | 9 | 17% | 22% |
| 20-40% | 10 | 33% | 30% |
| 40-60% | 37 | 52% | 54% |
| 60-80% | 66 | 71% | 79% |
| 80-100% | 151 | 96% | 97% |

Readings with the same predicate-argument dependencies mean the same, so
they are grouped into interpretations
([`ambiguity.rs`](../crates/emdysi-parse/src/ambiguity.rs)), and a
sentence is reported as ambiguous when a second interpretation keeps at
least 5% of the probability (`cargo run --release -p emdysi-parse
--example interpretations -- SENTENCE` shows them all):

| Sentence | Interpretations (probability) |
|---|---|
| I saw the man with the telescope. | *with(saw, telescope)* 50%; *with(man, telescope)* 40%; *saw(I, man, with)* 10% |
| The chicken is ready to eat. | the chicken eats 50%; the chicken is eaten 49% |
| Time flies like an arrow. | *time flies* (insects) *like* an arrow 62%; time *flies* like an arrow 17%; a third reading 13% (the ranker prefers the joke reading) |
| They are cooking apples. | *cooking(they, apples)* 70%; *are(they, apples)* with *cooking apples* 21% |
| Visiting relatives can be boring. | the visiting is boring 92%; the relatives are boring 5% |
| We painted the wall with cracks. | *with(painted, cracks)* 82%; *with(wall, cracks)* 16% (two readings) |

## Parse readings of machine-written prose

[`corpora/ai-treebank`](../corpora/ai-treebank/README.md) has 160 sentences
from `corpora/ai-prose` and Beemo's model outputs, with the right reading
of the ERG chosen by hand (judged as English, not by agreement with another
parser) for the 97 that have one among their six best readings.
`cargo run --release -p emdysi-check --example treebank_eval` analyses each
sentence in its document, as `en check` does, and counts how often the best
reading is the right one (2026-10-03):

| | Best reading right (of 97) |
|---|---|
| Ranker alone | 57 |
| Phrases the document uses elsewhere, margin 1, 2 or 4 | 58 (one fixed, none broken) |
| Same, any margin | 57 (one fixed, one broken) |

The phrase preference only counts phrases of two or more content words;
with all phrases, common ones such as *of them* broke two right readings.
It changes the best reading of 14 sentences of these documents at the
default margin (2), mostly in sentences that were not judged. Most of the
40 wrong best readings differ from the right one in ways nothing else in
the document can speak to: the rule for a fronted adverb (*First,
employees save time ...*), a past participle read as a past tense, or an
attachment the document never repeats.

Training the ranker on the treebank as well does not help: in 5-fold cross
validation, the ranker trained on the gold profiles gets 38 of 96 right on
held-out folds, and 39, 35 and 38 with the other folds added once, three
times and ten times (`TREEBANK_FOLDS=5 TREEBANK_WEIGHT=n cargo run --release
-p emdysi-parse --example train -- ... treebank:corpora/ai-treebank/treebank.tsv`;
these figures parse sentences alone with the trainer's settings, so they are
lower than in-document ones). The treebank is kept for evaluation.

Other rankers, compared in documents (`treebank_eval` also counts the
sentences whose right reading is among the readings at all):

| Ranker | Held out (gold profiles) | Best reading right | Right reading among the readings |
|---|---|---|---|
| Shipped: averaged perceptron, training parses of 2026-10-01 | 81.7% | 57 | 96 |
| Log-linear (`RANKER=maxent`, L2 = 1), same parses | 84.8% | 48 | 84 |
| Averaged perceptron, the items parsed again with today's parser | 82.6% | 40 | |
| Log-linear, the items parsed again | 83.7% | 38 | 84 |
| Perceptron with head-word features (head lexical entries of each headed phrase, one side backed off to a lexical type) | 84.3% | 35 of 96 (5-fold) | |

Better scores on the held-out gold profiles do not carry over to
machine-written prose. Most of the loss is not in ranking: long sentences
are parsed with chart pruning guided by the ranker's local weights, and
with another model the right reading falls out of the chart for 12 of the
97 sentences. Among the sentences that keep it, the log-linear ranker
picks it about as often as the shipped one (57% against 59%). The
comparison also favours the shipped model, because each right reading was
chosen from its six best. Head-word features fit the short constructed
sentences of the test suites and do worse on the treebank. The shipped
model stays; a fair comparison needs right readings judged independently
of any one ranker, and pruning that does not depend on the model being
evaluated.

