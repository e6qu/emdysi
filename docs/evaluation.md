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

