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

With the changes of 2026-10-05 (a/an and repeated words, grammar
extensions, the grammar-error variant's types, the tagger, and the
precision fixes that followed), dev2 is at 6.4 flags per 1,000 sentences
(86 flags; 2026-10-06): spelling 3.7, consistency 1.6, repeated words
0.4, articles 0.3, named grammar errors 0.3. About 16 of the 86 are real
errors in the sources: all six repeated words (*this is is*, *the the*,
*and and*, *this this*), *a overflow*, and the typos *reseearchers*,
*anyeone*, *durnig*, *righly*, *consided*, *santisers*. What is left is
mostly jargon and handles of the Rust blog (*libc*, *libs*, *iirc*,
*impls*, *tiif*), the novel's own spellings (*dropt*, *expence*,
*curtsey*, *an union*, *quitted*), and Kubernetes terms (*cadvisor*,
*rootfs*, *nftables*).

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

Agreement errors with the verbs whose singular and plural are separate
words in the grammar (*was* and *were*, *is* and *are*, *has* and *have*,
*does* and *do*) were named by the grammar-error variant but never
reported: the correction was looked for among the word's inflections, and
*were* is not an inflection of *was*. With those pairs as candidate
corrections (each still accepted only if the grammar then analyses the
sentence), *The results was clear.*, *The new version are faster.* and
*She are a writer.* are reported with the right correction (2026-10-06). Two further changes
go with it. The grammar-error variant kept at most 20 readings and ranked
run-on fragments first, so a whole-sentence reading naming the error
(*Our users has asked for this feature*) could be crowded out; its
whole-sentence roots are now unpacked first, as the strict root is for
the grammar. That alone made two correct sentences of edited text look
wrong (*Well, much good may it do them!*, *to prevent a continued
legislative paralysis*), each with a "correction" the grammar accepts
only under another analysis (*much good may it does them* parses with
*may* as a noun). So a correction now counts only if a strict reading of
the corrected sentence keeps the lexical entry of every other word of the
reading that names the error. With all three:

| | Before | After |
|---|---|---|
| Real errors caught (377) | 162 | 162 |
| Flagged after the fix | 0 | 0 |
| Flags on the first development set (2,498 sentences) | 5 | 6 |
| Flags on the second development set (13,442 sentences) | 86 | 86 or fewer |

The new flag is *Its police includes a special unit*, where *police*
takes a plural verb in edited English. The second development set was
measured with the first two changes (87 flags, the one new being *much
good may it do them*); the third change only rejects corrections, and
rejects that one.

Three more kinds of error the grammar-error variant named were never
reported, for the same reason, a correction never offered
(2026-10-07):

- contracted auxiliaries (*They doesn't know*, *He don't know*): the
  error table describes these entries by their lexical type, not by their
  name, so they had no feedback and no correction; a word's error is now
  named by its type when the table has no entry for its name;
- regular pasts of irregular verbs (*buyed*, *runned*, *catched*,
  *goed*): the correction was the closest form by spelling (*buy*, which
  changes the tense); it is now the verb's irregular past from the
  grammar's irregular-forms table (*bought*, *ran*, *caught*, *went*);
- a past tense after *has* or *have* (*has went*, *have ate*, *have
  took*, *has wrote*): the grammar-error variant names it either on the
  auxiliary or as a wrong participle of the verb, two analyses that used
  to disagree; both are now the error of the verb, corrected to its
  irregular past participle (*has gone*, *have eaten*, *has written*);
- an inflected verb after a modal (*can goes*, *should went*): corrected
  to the verb's base form (*can go*, *should go*).
- a mass noun made plural (*informations*, *furnitures*, *softwares*):
  the error table says the noun "is always singular" but no correction
  was offered; the singular is, and where an auxiliary in the next few
  words agrees with the plural, the singular with that auxiliary in the
  singular too (*The informations are useful* to *information is*), each
  still accepted only if the grammar then analyses the sentence. The
  claim needs the plural to be missing from the word list: the grammar's
  lexicon has *lint*, *coercion*, *research* and *advice* as mass nouns
  only, and their plurals, in the list, are count uses in edited text
  (five false flags on the second development set before this
  condition, none after).

Some errors stay unclaimed on purpose: *He will makes it* also reads as
*His will makes it*, *He is go home* has a second analysis with another
error, and the grammar accepts *This is more better*, *I am interesting
in music* and *He must to go* as they stand.

On the real errors and the first development set nothing changes (162
caught; the same six flags); on the second development set the flags go
from 86 to 83, the three gone all false (*quitted* twice, correct in
Austen, and *we will ask you no questions*).

Four changes claim errors that were proved but not reported
(2026-10-10):

- When the best analyses of the grammar-error variant disagree on the
  error, each naming one, the sentence is still proved ungrammatical; it
  is reported with the corrections the grammar accepts, ordered by the
  ranker's probability of their analyses, those under 5% left out. An
  analysis whose corrections the grammar all rejects (*His will makes
  it*) does not stop the claim; one for which no correction can be tried
  does (*How he lived I know not*, read as a run-on that wants a comma).
  *He will makes it* is now reported, with *will make*.
- Two clauses run together with no punctuation between them count as an
  error of the analysis that joins them: the strict grammar joins clauses
  only with a semicolon or a dash, so such an analysis used to make a
  sentence look error-free to the grammar-error variant.
- The ERG's rules for verbs of saying in quotations (*He left, said
  Kim*; *Yes, said Kim*) also analyse agreement errors as quotations:
  *Policy objects describes the logic* as *Policy objects, "describes the
  logic"*, *the commands adds two contexts* with *adds* inverted. A
  reading that uses them with no punctuation next to the verb is no
  longer evidence that the sentence is grammatical, when no other strict
  reading exists (the parse is redone with up to 1,000 readings to make
  sure).
- An inflected verb after *to* (*needs to knows*) is corrected to its base
  form; the corrected sentence is searched for a reading that keeps the
  other words' entries among 100 readings instead of 20, and a word the
  erroneous analysis covers with a generic entry may have any entry.

A combined correction (*informations are* to *information is*) now marks
both words it replaces; it used to mark the first only, so applying the
suggestion gave *information is are*.

| | Before | After |
|---|---|---|
| Real errors caught (377) | 162 | 162 |
| Flagged after the fix | 0 | 0 |
| Flags on the first development set (2,498 sentences) | 6 | 6 |
| Flags on the second development set (13,442 sentences) | 83 | 83 |

The real errors these changes target are in longer sentences, which the
parser prunes; a pruned search proves nothing, so they stay unclaimed
(*Policy objects describes the abstract logic of a policy using Common
Expression Language (CEL)* is claimed only without its last six words).
*He is go home* also stays unclaimed: its second analysis wants an
article before *go*, and no correction adds an article yet.

Three more changes let such sentences be claimed when they are long
(2026-10-10): a pruned parse whose strict readings all quote without
quoting is redone without pruning, and the readings are dropped if that
search finishes and finds no other; a capital at the start of a run of
capitalized words (*Common Expression Language*) is licensed like the
rest of the run before the errors are counted, so it no longer pushes an
analysis over the limit of two; and an acronym the grammar does not know
(*CEL*) no longer blocks a claim, as an unknown word does. *Policy
objects describes the abstract logic of a policy using Common Expression
Language (CEL)* is now reported, with *describe*: 163 real errors caught
of 377, none flagged after the fix, and the same flags on both
development sets (6 and 83).

Corrections that add a missing article were tried and dropped
(2026-10-10). The grammar-error variant names a missing article
(`hdn_bnp_c_rbst`, "Add an article, like *the* or *a*") in *He is go
home* and *She bought car yesterday*, but it is also how it covers what
the grammar lacks in edited text: names (*Capitol Hill*, *Visual
Studio*), numbers and years (*line 7*, *2025*), code and links. Adding
*the* before them usually gives a sentence the grammar accepts, so
verifying the correction does not keep them out: the flags went from 6
to 22 on the first development set and from 83 to 142 on the second,
with no more real errors caught.

Of the 76 wrong verb forms and agreement errors among the real errors,
the strict grammar accepts 43 as written (many are word choices or UK
spellings, not errors), the search is cut short for 23, the analyses name
no single error for 8, and the correction fails for 2 (2026-10-10,
counted with the `why` example). Template shortcodes (`{{< glossary_tooltip
... >}}`) are now masked for the parser like other code, and a masked
span no longer blocks a claim as an unknown word does; corrections are
parsed with the same masking. *This tutorial show you how to install {{<
glossary_tooltip ... >}} drivers in your cluster* is now reported, with
*shows*; the real errors and both development sets are unchanged (163
caught; 6 and 83 flags).

The measurement tools now cache parses (see
[CONTRIBUTING.md](../CONTRIBUTING.md#measure)): with the cache filled,
the first development set takes 15 seconds instead of 38 minutes, the
second 78 seconds instead of about three hours, and the real errors 31
seconds.

A pruned parse that finds no analysis is retried without pruning, in
the time left; the retry was kept only if it found an analysis, so a
retry that finished with fragments only, which proves the sentence is
outside the grammar, was thrown away and the sentence counted as not
searched completely (2026-10-11). It is now kept. Of the 23 real errors
whose sentences were not searched completely, 2 now are; neither is
claimed, as the grammar-error variant names more than one error in them
or runs out of time. The flags on the first development set and the real
errors caught are unchanged (6; 163).

Of the 28 sentences of the coverage sample (first development set)
without a strict reading that are not cut at *Mr.*, 10 have readings that
use the ERG's paired comma, which it marks informal: the commas around an
interruption (*but, unable to contain herself, began*), an alternative
(*You, or a controller, must delete it*) or a shared object (*similar
to, but less extreme than, the climate*). Making that comma formal, as
`register.tdl` does for contractions, was tried and dropped (2026-10-11):
the same comma also makes *The dog, barked* and *B knows, we arrived*
strict, which edited English does not write.

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

Two more make common edited-prose constructions formal: comma-separated
adjectives before a noun (*a long, cold, dark night*), for which the ERG
has only its informal comma entry, and fronted prepositional phrases and
adverbs without a comma (*In 2019 we released the book.*), which the ERG
parses with informal rules (`grammar/emdysi/README.md`). The ranker now
also reads each extension rule or entry as the ERG one it stands for
(`EQUIVALENT` in `crates/emdysi-parse/src/rank.rs`). On the gold profiles
(2026-10-06) csli, esd and control are unchanged; on sh-spec one gold tree
and one parse are lost and one more gold tree is ranked first (359 vs 360
found, 521 vs 522 parsed, 314 vs 313 first).

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


## Coverage of edited text

How many sentences of edited prose get a strict (full, formal) reading:
300 sentences drawn at random (seed 7) from whole paragraphs of the first
development set (`corpora/edited`, `corpora/edited-by-sa`; 5 to 25 words,
paragraphs without code, links, quotes or Markdown markup), parsed with
`cargo run --release -p emdysi-parse --example parse`, which reports how
many of a sentence's readings are strict. Of the sentences without one,
most parse once shortened: the long ones ran out of search, not grammar.
About a dozen are fragments the sampling cut at *Mr.*

The parser stops a sentence when the structures its chart keeps pass a
node budget (25 million nodes, about 1 GB). It counted the structures of
edges it had released (frozen or pruned) as kept, so long sentences
stopped early; counting only those it keeps (2026-10-06):

| | Strict reading (of 300) | Out of budget | Total time | Slowest |
|---|---|---|---|---|
| Released structures counted | 254 | 5 | 400 s | 15 s |
| Only kept structures counted | 261 | 1 | 461 s | 34 s |

A 45-word sentence peaks at 1.07 GB (the grammar takes 365 MB of it).
The per-cell beam (`DEFAULT_CELL_BEAM`, from 20 tokens) helps more than
it costs: without it, more long sentences run out of budget (95 against
101 strict readings on the first 131 sentences). On the gold profiles the
change only adds analyses:

| Profile | Grammatical parsed | Gold tree found | Gold ranked first | Hit limits |
|---|---|---|---|---|
| csli | 938 of 960 | 908 of 921 | 885 | 0 |
| esd | 61 of 62 | 58 of 59 | 57 | 0 |
| control | 1,292 → 1,298 of 1,305 | 1,494 of 1,582 | 1,384 | 0 |
| sh-spec | 521 → 544 of 599 | 358 → 361 of 576 | 313 → 314 | 6 → 2 |

The search without pruning that follows a pruned one ran only when the
pruned search found no complete analysis. Pruning also loses strict
analyses of sentences it leaves informal ones (*If you must include
cross-references, consider putting them at the end of the text, like a
reference, rather than in the middle.*: no strict reading of 240 pruned,
111 strict of 646 without pruning, in 10 s). Searching again whenever
there is no strict analysis (2026-10-11, with the grammar extensions of
the same day; the counts include the fragments cut at *Mr.*):

| | No strict reading (of 300) | Total time |
|---|---|---|
| Retry without a complete analysis | 35 | 837 s |
| Retry without a strict analysis | 27 | 945 s |
| The same, judging strictness after emdysi's root and derivation conditions | 24 | 994 s |

| Then a beam twice and four times as wide before the search without one | 21 | 828 s |

On a long, ambiguous sentence the search without a beam runs out of its
budget where a wider beam finds the strict analysis, and finds it sooner,
so the widening is faster as well. On 300 sentences of the second
development set it leaves 46 without a strict reading, against 51 with
the retry alone (and the extensions of the day before the widening).

Of the 24 before the widening, 11 are the fragments cut at *Mr.*; the other 13 include
gapping (*and of his judgement the highest opinion*), lists of fragments
joined by semicolons, archaic negation (*knew not how*) and team names
with a plural verb (*South Africa are*).

### Grammar extensions of 2026-10-11

Exclamations without a verb, comparatives with a shared or extracted
object (*similar to, but less extreme than, the climate*), collective
nouns with a plural verb, ranges of amounts (*£3,000-£5,000*), *from* and
*to* with adjectives, a heavy object after an adjunct, and the search
retry above. On the gold profiles, against main the same day:

| Profile | Grammatical parsed | Gold tree found | Gold ranked first |
|---|---|---|---|
| csli | 938 of 960 (same) | 908 of 921 (same) | 885 (same) |
| control | 1,298 → 1,297 of 1,305 | 1,494 → 1,495 of 1,582 | 1,384 → 1,383 |
| sh-spec | 544 → 543 of 599 | 361 → 358 of 576 | 314 (same) |
| esd, mrs, ccs | same | same | same |

Ungrammatical items parsed are unchanged (csli 254 of 388, control 497 of
527). The first comparison found larger losses, all from the
extensions themselves: the heavy-object rule took verb phrases and
prepositional phrases as objects (*prayed to Athena to take care of
himself*), and the plural collective entries outranked the singular ones
and made bare noun phrases (*Chiang became a programmer and in Devito's
department* parsed as *Devito is departments*); both are fixed. What is
left comes from search limits on long sentences: the parse of a
grammatical item that main finished in time now takes 38 seconds with
four in parallel; above a heavy shift the formal and informal phrases do
not pack together, so the beam keeps fewer of the informal analyses the
gold trees use; and the retry without pruning, now run on sentences with
informal readings only, finds more readings, which can push the gold tree
out of the first 1,000.

### Checks after the extensions and the speed work

On the final state of the branch (2026-10-11):

| Measure | main | Here |
|---|---|---|
| False flags, first development set (2,498 sentences) | 6 | 6 |
| False flags, second development set (13,442 sentences) | 83 | 84 |
| Real errors caught (377) | 163 | 163 |

| Gold profile | Grammatical parsed | Gold tree found | Gold ranked first |
|---|---|---|---|
| csli | 938 of 960 (same) | 908 of 921 (same) | 885 (same) |
| control | 1,298 → 1,299 of 1,305 | 1,494 (same) of 1,582 | 1,384 → 1,382 |
| sh-spec | 544 → 560 of 599 | 361 → 357 of 576 | 314 → 313 |
| esd, mrs, ccs | same | same | same |

Two flags on the second development set were new. *PRs welcome!* now has
a strict reading (an exclamation without a verb) that reads the unknown
*PRs* generically, and the grammar-error variant read it as the plural of
a mass noun; the plural of an acronym is now a name there too, as an
acronym already was. The other stays: *My younger sister has left all her
friends--has eloped; has thrown herself into the power of--of Mr.
Wickham.* is outside the grammar (verb phrases joined by a dash and a
semicolon, without a conjunction), the search without pruning now
finishes and proves it, and the grammar-error variant's best analysis
makes *her friends--* the subject of *has eloped*.

## Parser speed

Changes to the parser that leave its results as they were, checked by
listing every reading of every sentence (root, score, derivation) with
`cargo run --release -p emdysi-parse --example dump` before and after
(2026-10-11; times with other measurements running on the same machine):

| Change | 60 sentences of the dev sample |
|---|---|
| Before | 183 s |
| Unpack readings lazily: an edge's instantiations are made as they are needed, in the order the eager unpacking gave them (best first over the grid of daughter instantiations, bounded by the best score the rule can give) | 127 s |
| 60 more quick-check paths, where the unifier most often failed past the ERG's 79 (learned on 30 sentences of the second development set) | 124 s |
| The cycle check after a copy only when an arc points back | 122 s |
| The unifier copies its sources' nodes and arcs into flat tables, so a handle is one lookup | 109 s |
| Quick-check vectors read in one walk over a trie of the paths | 103 s |

On the 300 sentences of the coverage sample, the time went from 824 s
to 552 s; 297 of 299 sentences got the same readings, and the other two
were cut off by the time limit before and finish now (*See paragraphs
(c) through (h) of Section 51 ...*: 61 s, now 23 s).

## Phrasal compounds

A run of three or more lower-case words joined by hyphens that the
lexicon lacks (*allow-by-default*, *pay-as-you-go*) gets a token of its
own beside the split tokens, and whole-chain words, generic or of the
lexicon (*wait-and-see*), are preferred to readings that take the
hyphens for dashes (2026-10-12):

| Measure | Before | After |
|---|---|---|
| Edited-text sample, sentences without a strict reading (300) | 21 | 19 |
| Second development set's sample (300) | 46 | 42 |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 84 | 84 (the same) |
| Real errors caught (377) | 163 | 163 |
| Gold profiles | | unchanged, all six |

### Checking speed: the grammar-error variant

Timing whole documents (12 documents of the second development set,
13,185 words, `en check` without the parse cache, four threads) showed
that the speed-ups of the parser alone did not shorten checking: most of
the time goes to hard sentences that search until their time limit, and
work saved inside a search goes into searching further. The grammar-error
variant took 45% of the parse time, most of it on sentences whose result
`grammar_errors` never reads. It is now parsed only where its result is
used (2026-10-12):

- without a strict analysis, only when a complete search found fragments
  only (errors are reported only then); `en rewrite` and `en parse`, which
  use its meaning, still ask for every such sentence (`Options::meaning`);
- with a strict analysis, only when an unknown word other than an acronym
  or masked code could carry the error (errors there now count only
  within such a word, not merely overlapping one).

| | Before | After |
|---|---|---|
| The 12 documents | 8 min 24 s | 6 min 30 s |
| Their diagnostics | | identical |
| False flags, first and second development sets | 6, 84 | the same flags |
| Real errors caught (377) | 163 | 163 |

### Coverage, measured again

The coverage samples above were cut into sentences by a quick splitter of
the measurement script, not by the checker: it broke sentences at *Mr.*
and *Mrs.* and let labels through (*Point of contact Ed Page*). Sampled
again with the checker's own segmentation (`en parse --no-parse`), 300
sentences of 5 to 25 words ending in `.`, `!` or `?` from paragraphs
without code, links, quotes or Markdown markup (seed 7), main has a
strict reading for 290 of 300 sentences of the first development set and
278 of 300 of the second. Of the 61 sentences the old samples left
without one, a search without pruning, with twelve times the time and
more than twice the memory, found a strict reading for one: the others
are outside the grammar (or not prose), not lost to the search.

Two extensions for what was left (2026-10-12): a subject followed by an
adjunct between commas (*The committee, however, decided to wait.*,
`grammar/emdysi/subjects.tdl`), and amounts with a magnitude suffix
(*£2bn*, *$5m*, `grammar/emdysi/tokenizer.rpp`).

| Measure | Before | After |
|---|---|---|
| First development set, sentences without a strict reading (300) | 10 | 9 |
| Second development set (300) | 22 | 22 |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 84 | 84 (the same) |
| Real errors caught (377) | 163 | 163 |
| Gold profiles | | unchanged but control: gold ranked first 1,382 → 1,380 |

On *control*, *I believed that Nixon, foolishly, was interested in ending
the war.* (two items) now has as its first reading the gold analysis with
the formal comma in place of the gold tree's informal one
(`comma_inf_pct`). The grammar cannot make a phrase formal while it
contains the informal comma (a phrase shares the GENRE of its
daughters), so the new rules take the formal comma.

### Gapping

A conjunct that leaves out the verb of the clause before it (*One came
from her books, and the other from her toilette.*) had no analysis: the
ERG has no gapped clause. `grammar/emdysi/gapping.tdl` makes one from a
subject and a prepositional or noun phrase remnant, with the elided verb
as an `ellipsis_rel` that the remnant modifies or takes as its second
argument. The parser builds it only right after a comma and a
coordinating conjunction, as that conjunction's clause, and never as a
sentence on its own (2026-10-12). Built anywhere, it filled the charts
of long sentences: *Frequently Asked Questions (FAQs) can be
overwhelming and leave your customers searching through a sea of
content.* lost its strict readings to the edge budget, and the
grammar-error variant ran out of time on a sentence whose error it then
missed.

| Measure | Before | After |
|---|---|---|
| First development set, sentences without a strict reading (300) | 9 | 9 |
| Second development set (300) | 22 | 20 |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 84 | 84 (the same) |
| Real errors caught (377) | 163 | 163 |
| Gold profiles | | unchanged but sh-spec: grammatical parsed 560 → 562 |

### Mentions and lexical gaps

A fresh sample of 948 sentences from both development sets (seed 11,
none from the samples above) had 54 without a strict reading. Two causes
recurred (2026-10-13):

- A mention (*@oli-obk*, *@rust-lang/lang*) had no reading at all: the
  ERG keeps an email address one token with a REPP mask, which emdysi's
  REPP does not apply, and has nothing for mentions, so it split off the
  `@` and split the name at dashes and slashes. `Erg::tokens` now joins
  the pieces that touch, and the tagger tags a mention as a proper name.
  The spelling rule had also reported the pieces (*naber* in
  *@b-naber*).
- *provision* with an object (*Autoscalers provision the Nodes*, *after
  swap is provisioned*): the ERG has it only with *with*.

Single sentences of the earlier samples added *oftener ... than*, *opt-in*
and *opt-out* as nouns, and *ill-behaved*.

| Measure | Before | After |
|---|---|---|
| First development set, sentences without a strict reading (300) | 9 | 9 |
| Second development set (300) | 20 | 18 |
| Fresh sample (948) | 54 | 45 (the 5 with mentions and 4 with *provision*; none lost) |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 84 | 79 (5 fewer, all mentions; none new) |
| Real errors caught (377) | 163 | 163 |
| Gold profiles | | unchanged, item for item |

### More lexical gaps

Nouns the ERG lacks or has only as verbs or mass nouns (*compile time*,
used 39 times in the development corpora, *a redirect*, *a recap of*, *the
decease of*, *the entail of*, *transport* as a count noun), *altogether*
before a verb and transitive *chunk* (2026-10-13). *recap*, *decease*,
*entail* and *chunk* were found by scanning the corpora for a word after a
determiner (or before an object) that the lexicon has only in another part
of speech, rather than sentence by sentence.

| Measure | Before | After |
|---|---|---|
| First development set, sentences without a strict reading (300) | 9 | 9 |
| Second development set (300) | 18 | 18 |
| Fresh sample (948) | 45 | 41 |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 79 | 79 (the same) |
| Real errors caught (377) | 163 | 163 |
| Gold profiles | | unchanged, item for item |

## AI tropes and vacuous text

The `ai-tells` and `substance` rules added on 2026-10-13 (decision M21 in
[plan.md](plan.md)), measured with `rule_rates` (`FLAGS=PATH` writes every
finding) on the first development set's edited documents (221 documents,
41,568 words) and the machine-written samples of `corpora/beemo`,
`corpora/cheat` and `corpora/hh-rlhf` (600 texts, 88,334 words); findings
per 10,000 words:

| Rule | Machine-written | Edited |
|---|---|---|
| `ai-tells.inflated` | 20.4 | 1.4 |
| `ai-tells.openers` | 12.7 | 1.7 |
| `ai-tells.rhetorical-question` | 1.8 | 0.2 |
| `ai-tells.stock-phrases` | 1.1 | 0.2 |
| `ai-tells.signposting` | 2.2 | 0.5 |
| `substance.filler` | 0.7 | 0.5 |
| `substance.platitude` | 0.1 | 0 |
| `substance.aphorism` | 0.1 | 0.2 |
| `ai-tells.its-about` | 0.1 | 0 |
| `ai-tells.setup`, `ai-tells.engagement`, `ai-tells.false-balance`, `ai-tells.one-line-paragraphs`, `substance.vacuous` | 0 | 0 |
| `ai-tells.dense` (documents) | 21 of 600 | 0 of 221 |

The rules for the newer tropes (setups, engagement questions, one-line
paragraphs, sentences of abstractions) find nothing in either set: the
machine-written samples come from 2023 models, which do not write that
way. They rest on the patterns documented for current models (Wikipedia,
*Signs of AI writing*) and on the examples each rule carries; measuring
them needs prose from current models (plan M21). The edited findings were
read one by one: the openers and inflated words are real uses (*Moreover*,
*crucial*), the aphorism is *Community happens everywhere!*, the
rhetorical question *Who are they?*; a rhetorical question in dialogue
(*“How so?”*) is no longer counted.

A restatement check (a sentence four fifths of whose content predicates an
earlier sentence already has) was measured and left out of the default
packs: it fires more often on edited text (4.8 per 10,000 words) than on
machine-written text (1.4), because edited documents restate on purpose:
plain-language guides show a passage before and after its rewrite, and
reference pages repeat a sentence in each tab. The check remains available
as `check = "restatement"` of the `semantics` kind.

### Machine-written prose from current models

The vendored machine-written samples come from 2023 models, so the rules
for newer tropes had nothing to measure against. `corpora/ai-current`
(2026-10-17) holds 24 documents written for this project in the voice of
current chat-model output: a blog post, a README, a launch announcement,
a social post, a support reply, an explainer, a newsletter, a listicle,
an onboarding guide, a talk summary, an answer, an essay, a cover letter,
a product page, a meeting summary, a press release, a travel guide, a job
posting, a sales email, a self-review, an abstract, a code review, an
outage apology and a recipe introduction (5,675 words). They were written
to be typical of their genre, not as parodies; but they were written,
not sampled, so they cannot by themselves show that a pattern is typical.
The evidence for each new rule is the other side: the edited corpora
(654 documents, 410,000 words), where each new phrase or pattern occurs
at most once.

Patterns the packs missed in these documents, and the new rules:

- hooks aimed at the reader (*Let's be honest*, *Sound familiar? You're
  not alone*, *Ready to get started?*, *Spoiler:*): `reader-hooks`; *Let's
  dive in* was left out, as the Rust book uses it;
- a run of fragments and a two-word payoff (*No more spreadsheets. No
  more manual matching. Just results.*): `just-fragment`;
- the contrast frame across two sentences without *about*: with *just*
  or *only* (*Rate limiting isn't just a nice-to-have. It's essential.*,
  `not-just-then-is`) or ending in a short punchline (*Setbacks aren't the
  end of your story. They're the plot twist.*, `not-then-punchline`); the
  edited corpora have two such pairs, both without either feature (*It
  isn't something to submit. It's a separate part of the instructions.*);
- list items that open with a bold label (*\*\*Logs:\*\* What
  happened.*): `bold-lead-ins`, a new structure check, once a document
  when at least three fifths of its list items do (in the edited corpora,
  36 of 2,348 list items open so; one document passes the threshold);
- an emoji at the start of a heading or list item (edited blog posts use
  emoji in running text, never at the start of a line), a line of
  hashtags, letter and email formulas (*I hope this email finds you
  well*; suggestions only, as the edited corpora have no letters), and
  about 40 more stock phrases (*the real magic happens*, *has you
  covered*, *will thank you*, *a must-visit*, *paves the way for*);
- a sentence opening with *Whether you're* (7 in the 5,675 words of the
  samples, once in the edited corpora: `whether-you`) and a listicle title
  (*7 Proven Strategies to ...*, `listicle-title`, headings only).

Documents each rule fires on (`en check --no-parse`, so without the rules
that need a parse: the rule of three, participial tails):

| Rule | Machine-written (24) | Edited (654) |
|---|---|---|
| `ai-tells.dense` | 14 | 0 |
| `ai-tells.stock-phrases` | 16 | 3 |
| `ai-tells.inflated` | 11 | 41 |
| `ai-tells.bold-lead-ins` | 10 | 1 |
| `ai-tells.vocabulary` | 9 | 27 |
| `ai-tells.em-dash` | 8 | 7 |
| `ai-tells.formulas` | 5 | 0 |
| `ai-tells.openers` | 4 | 34 |
| `ai-tells.not-x-but-y` | 3 | 92 |
| `ai-tells.reader-hooks`, `just-fragment`, `its-about`, `emoji-heading`, `chatbot`, `rhetorical-question` | 2 each | 1, 0, 0, 0, 0, 6 |
| `ai-tells.not-then-punchline`, `not-just-then-is`, `hashtags`, `emoji-list`, `setup` | 1 each | 0 |

`ai-tells.dense`, the verdict that a text reads as generated, fires on 14
of the 24 machine-written documents and none of the edited ones. The
documents it misses carry their tropes in the shape of the whole (a
social post of one-line paragraphs, an essay of aphorisms, an apology of
stock contrition) at densities below its threshold.

## Grammar and spelling fixes with a decision model: the upper bound

`core.grammar-fix` and the model's choice among spelling suggestions
(plan M20, `emdysi_check::assist`) need a model, which could not be
downloaded in the development environment. Their upper bound was measured
instead with a perfect model: `real_errors` with `ORACLE=1` answers every
question with the writer's own correction (2026-10-13).

| Setting | Real errors caught | Questions | Time |
|---|---|---|---|
| No model | 163 of 377 (43%); 166 (44%) with HTML paragraphs read as prose, 167 with typos of words the document uses (2026-10-15) | 0 | |
| Perfect model, corrections by inflection only, no screening | 164 of 377 | 93 | |
| Perfect model, with function-word substitution, deletion and insertion, no screening | 165 of 377 (44%) | 130 | 25 min |
| The same with screening of sentences the grammar accepts, on a quarter (`SHARD=0/4`) | 46 of 95 (48%; grammar errors 7 of 47) | 748 | 69 min |
| Wider candidates, rounds and ordering (2026-10-14), the same quarter | 57 of 95 (60%; grammar errors 18 of 47) | 732 | 20 min |
| The same on all pairs, in four quarters (`SHARD=0/4` to `3/4`) | 201 of 377 (53%; grammar errors 61 of 199, spelling 140 of 178) | 2,634 | |
| With real-word typos and the model asked which word is wrong (2026-10-14), all pairs | 253 of 377 (67%; grammar errors 107 of 199, spelling 146 of 178) | 4,616 | 33 min |
| With verification as the checker parses (masked, time by length, 30 s per sentence), the three lexical entries, more function words and the forms of every plain word (2026-10-14), all pairs | 286 of 377 (76%; grammar errors 137 of 199, spelling 149 of 178) | 4,528 | 42 min |
| With wider unpacking in the parser's later searches (2026-10-15), all pairs | 290 of 377 (77%; grammar errors 141 of 199, spelling 149 of 178) | 4,487 | |
| With corrections of sentences without final punctuation verified with a full stop (2026-10-15), all pairs | 294 of 377 (78%; grammar errors 145 of 199, spelling 149 of 178) | 4,602 | |
| With misspellings two edits away confirmed by the model (2026-10-15), all pairs | 299 of 377 (79%; grammar errors 145 of 199, spelling 154 of 178) | 4,672 | |
| With misspelled derivations confirmed by the model (2026-10-15), all pairs | 301 of 377 (80%; grammar errors 145 of 199, spelling 156 of 178) | 4,712 | |
| With paragraphs of raw HTML read as prose (2026-10-15), all pairs | 305 of 377 (81%; grammar errors 145 of 199, spelling 160 of 178) | 4,790 | |
| With a word one edit from a word the document uses taken for its typo (2026-10-15), all pairs | 306 of 377 (81%; grammar errors 145 of 199, spelling 161 of 178) | 4,812 | |
| With up to four candidates two edits away, and the word as written outside the parse check (2026-10-15), all pairs | 309 of 377 (82%; grammar errors 145 of 199, spelling 164 of 178) | 4,831 | |
| With candidates for sentences without any reading (2026-10-15), all pairs | 311 of 377 (82%; grammar errors 147 of 199, spelling 164 of 178) | 4,874 | |
| With the oracle matching whole sentences first (2026-10-15), all pairs | 311 of 377 (the same pairs) | 4,888 | |
| With *this* and *should* inserted, a capital at the start of a sentence and sentence adverbs (2026-10-15), all pairs | 314 of 377 (83%; grammar errors 150 of 199, spelling 164 of 178) | 4,905 | |
| With a pronoun after the last word and a word deleted with its comma (2026-10-15), all pairs | 315 of 377 (84%; grammar errors 151 of 199, spelling 164 of 178) | 4,924 | |
| With corrections of two words, related words and Hugo shortcodes (2026-10-15), all pairs | 326 of 377 (86%; grammar errors 159 of 199, spelling 167 of 178) | 4,902 | |
| With typos in title-cased names and 7-letter words two edits away put to the model (2026-10-15), all pairs | 328 of 377 (87%; grammar errors 159 of 199, spelling 169 of 178) | 4,942 | |
| With clause-by-clause verification, negations and three-letter typos (2026-10-15), all pairs | 337 of 377 (89%; grammar errors 165 of 199, spelling 172 of 178) | 4,998 | |

No flag on the corrected text in any setting. Without screening the model
can only help with sentences the grammar cannot analyse, and most real
errors leave a sentence grammatical (*for* for *to*, a missing *the*).

Where the writer's correction of a grammar error is lost, by stage
(`cargo run --release -p emdysi-check --example fix_bound`, no model
needed; 199 grammar errors; four minutes):

| Stage that loses the correction | Before (2026-10-13) | Wider candidates | Real-word typos | Real-word typos, the model names the word |
|---|---|---|---|---|
| Not parsed, or under 3 or over 40 words | 14 | 14 | 14 | 14 |
| Not among the candidate edits | 91 | 51 | 32 | 32 |
| Past the edits parsed (80, now 120) | 25 | 25 | 38 | 0 |
| The strict grammar rejects the corrected sentence | 17 | 31 | 32 | 52 |
| Past the versions shown to the model (9, now 27) | 27 | 25 | 29 | 1 |
| Reachable (with screening, for a sentence the grammar accepts) | 24 (11) | 52 (28) | 53 (28) | 99 (73) |
| Spans two sentences | 1 | 1 | 1 | 1 |

What changed: a word's base form was never a candidate (`inflections`
leaves out the word it is given, and it was given the lemma), so
*consumes* for *consume* could not be corrected; a noun's forms are now
nouns only (not *filed* for *file*); more function-word classes
(*which*/*that*, *or*/*and*, pronouns, possessives, modals) and
insertions (*you*, *is*, *in*); doubled words dropped first; for a
fragment reading, edits alternate with those nearest where its partial
analyses meet (alone, that order helped 18 errors and hurt 28; mixed, it
helps without the losses); the model sees up to 27 versions, nine at a
time and then the choices together. Parsing stops once 27 versions
parse, which makes the run three times faster. (One run over all pairs
outlasted the two-hour limit of the development environment's background
jobs; the four quarters run separately, from the parses it had cached, in
under an hour together.) The oracle also now
compares with the corrected sentences as the checker renders them: it
never matched a sentence with a Markdown link, which understated the
earlier bound.

Real-word typos (*Manger*/*Manager*, *Fore*/*For*, *though*/*thought*,
*serer*/*server*) are now candidates too: each word of four letters or
more may become a common word one edit away (a letter dropped, added,
changed, or two swapped). Alone they gain one error: almost all of them
are in sentences the grammar accepts, where dozens of other edits parse
as well, so the writer's correction falls past the edits parsed or the
versions shown. So when the model doubts a sentence it is next asked
which word is wrong (`LOCATE_QUESTION`, the words in groups of ten), and
the edits at that word are tried first, the others after them in the
earlier order. With a perfect model that answer is always right, so the
last column is the bound when the model locates errors well; how well a
2 GB model locates them is measured with the model. The 32 corrections
still not among the candidates change two or more words or a content
word (*iteration*/*iterator*); the 52 rejected corrections are coverage
gaps (M19), most of them hidden before behind the caps. A real model can
do no better than this bound.

### The corrections the grammar rejected

Of the 52 corrected sentences that the strict grammar seemed to reject
(the last `fix_bound` column above), 22 were rejected only by the
verification: it parsed the corrected sentence raw within 2 seconds,
while the checker parses the sentence itself with inline code masked
within 10 (2026-10-14). Seven of the 22 parsed within 2 seconds when run
alone, and 14 within 30. `assist::verify` now parses the masked text and
gives a sentence 2 seconds up to 15 words and 0.32 seconds more for each
word after, up to the checker's 10. Because the limits are wall-clock,
a run on four busy cores can lose a long sentence that a quiet one
keeps: `fix_bound` varies by a few sentences between runs.

The other 30, by kind:

| Kind | Sentences | Examples |
|---|---|---|
| Not a sentence | 8 | front matter; text cut at a code block (*... the CA certificate is retrieved via secure connection and then*); *You should see output similar to:* |
| Still ungrammatical after the writer's fix | 3 | *We can can call*, *we recommend to add*, *User namespaces is* |
| Lexical gaps | 3 | *in-built*, *stored in the clear*, *sends a delete* |
| A verb as the name of a verb or command | 1 | *Similar to the escalate verb, ...* |
| Long sentences whose clauses each parse | 15 | *The Kubernetes directory /etc/kubernetes is a constant in the application, since it is clearly the given path in a majority of cases, and the most intuitive location; ...* |

The three lexical gaps are now entries in
`grammar/emdysi/lexicon.tdl`: *in-built* as an adjective (the ERG has
only *built-in*), *in the clear* as a predicate and as a verb modifier
(in the forms of *in the know*), and *delete* as a count noun. A bare
verb naming itself (*the escalate verb*, *the get command*) is rare in
the edited corpora (once each for *send* and *hidden* with *operation*
and *verb*) and is left. The long sentences parse clause by clause and
the parser already widens its beam, then drops it, while time remains;
with 30 seconds none of them gets a strict reading, so they wait on the
search (M8) more than on the lexicon.

| Measure | Before | After |
|---|---|---|
| Grammar errors whose correction a perfect model reaches (`fix_bound`, 199) | 99 | 121 (verification), about the same with the entries (3 gained, 4 lost to timing) |
| Corrected sentences rejected | 52 | 29, then 30 (timing) |
| First development set, sentences without a strict reading (300) | 9 | 9 |
| Second development set (300) | 18 | 18 |
| Fresh 948-sentence sample | 41 | 41 |
| Real errors caught without a model (377) | 163 | 163 |
| False flags, first development set | 6 | 6 (the same) |
| False flags, second development set | 79 | 79 (the same) |
| Gold profiles | | identical |

### More candidates, and a budget for verification

The writer's corrections still not among the candidates (32) were mostly
function words outside the lists (*an*, *we*, *be*, *on*, *with*
inserted; *the* for *they* or *then*, too short for the real-word typos)
and inflections of words the best reading did not take as a noun or
verb (*provides* for *provide* in a fragment reading); these are now
candidates. The simulated locator in `fix_bound` and in the oracle also
named the wrong word when a correction adds letters to a word (*amount*
to *amounts*: the first difference falls just after the word), which put
five reachable corrections past the versions shown. With both,
`fix_bound` reaches 128 of the 199 grammar errors (121 before).

Time by length made verification slow: a candidate that does not parse
can use its whole limit (up to 10 seconds), and a doubted sentence has up
to 120 candidates, so a quarter of `real_errors` with the oracle took
over an hour instead of 8 minutes. Verification now has 30 seconds per
sentence in all; the edits at the word the model names come first, so
they are rarely the ones left out. A quarter takes 6 to 13 minutes.

### Unpacking in the searches for a full analysis

Of the corrected sentences the grammar rejected, the long ones parse
clause by clause, and with a 30-word sentence (*You can work around
this by using a cluster autoscaling tool that is aware of Pod topology
spread constraints and is also aware of the overall set of topology
domains.*) a complete search, without pruning and within its budgets,
found fragments only; removing almost any one word gave a strict reading
(2026-10-14). The chart had the strict analysis: unpacking keeps the 100
best instantiations of each edge (`unpack_beam`), and in a long,
ambiguous sentence those a strict root needs can rank below a hundred
others. With 300 the sentence gets a strict reading, in 6 seconds
instead of 20 without one.

A wider beam everywhere costs ranking: more competing readings are
unpacked, and some outrank the gold analysis. So the first search keeps
100, and the searches the parser makes for a sentence that has no strict
reading yet unpack three times as wide (`WIDE_UNPACK`, first with the
configured chart beam, then twice and four times as wide, then without
pruning); a sentence that gets a strict reading at once is parsed as
before. The sentence above now gets one in 9 seconds, within the
checker's 10. Four times as wide gave the same results with more
memory: unpacked instantiations are outside the parser's node budget,
and a run of `false_flags` on the second development set with four
threads ran out of memory.

| Measure | Before | 300 everywhere | Three times as wide in the later searches |
|---|---|---|---|
| Fresh 948-sentence sample, without a strict reading | 41 | 40 | 40 |
| First and second development sets (300 each) | 9, 18 | 9, 18 | 9, 18 |
| control: gold found, ranked first | 1,494, 1,380 | 1,480, 1,364 | 1,494, 1,380 |
| sh-spec: gold found, ranked first | 357, 313 | 358, 311 | 360, 313 |
| csli, esd, mrs, ccs | | the same | the same |
| control, time | 378 s | 485 s | 386 s |
| 948-sentence sample, one thread, no parse cache: peak memory, time | 1.8 GB, 734 s | | 2.1 GB, 771 s |
| Real errors caught without a model (377) | 163 | | 163 (the same) |
| False flags, first development set | 6 | | 6 (the same) |
| False flags, second development set | 79 | | 79 (the same) |

### Sentences without final punctuation

The strict grammar wants sentence-final punctuation, so the correction of
a list item or of a sentence cut short by a code block (*On each node,
restart the kubelet*) never verified, nor that of a sentence ending in
a colon (2026-10-15). Verification now adds a full stop, or puts one in
place of a final colon; such a sentence that parses with one counts as
grammatical, so it gets corrections only when the model doubts it (a
list item is not otherwise second-guessed). With a perfect model,
`fix_bound` reaches 139 of the 199 grammar errors (134 after the wider
unpacking, 128 before it), and `real_errors` catches 294 of 377.

### Misspellings two edits away

Of the 178 spelling errors, a perfect model still missed 29, most with no
finding at all: the spelling rule reports a word only when a listed word
is one edit away, and *neccesary*, *certicate*, *authorative*,
*admistrators* and *bencharmark* are two (2026-10-15). Reported without
a model (eight letters or more, not two known words written as one, and
exactly one common word that close), they caught six more real errors,
but added three false flags on the second development set, on correct
technical words (*distroless* for *distress*, *mebibytes* for
*megabytes*, *retpoline* for *retooling*), as well as three flags on
real typos there (*implemenent*, *borrowcking*, *modificators*). Length,
tier and the shape of the edits did not tell the two apart, so such a
finding needs the model (`Diagnostic::needs_model`): offered the word as
written with the suggestions, the model must prefer a suggestion.
Without a model nothing changes (163 real errors, the same 6 and 79
false flags); with a perfect one, spelling errors caught rise from 149
to 154 of 178.

### Misspelled derivations

Some misspellings read as a known word plus an affix: *assignement* as
*assignee* plus *-ment*, *securly* as *secure* plus *-ly*. The rule took
such a word for a coinage unless an inflection of the base was one edit
away, and *assignment* and *securely* are derivations (2026-10-15). A
listed word one edit away, formed on the same base, with the edit inside
the word, now marks it as a possible typo. Reported without a model,
that caught four more real errors, but flagged *nestings* (one edit from
*nestlings*) and *rationable* on the second development set; with the
edit allowed at the end of the word as well, it also flagged coinages
such as *flutterings*, *mortifications* and *finalish*. So, like a word
two edits away, such a finding needs the model's confirmation. Without
a model nothing changes; with a perfect one, spelling errors caught rise
from 154 to 156 of 178.

Of the 22 spelling errors a perfect model still missed, four were in raw
HTML blocks (*comunications*, *indiciates*, *canot*, *fileds*), which the
Markdown reader did not check; *witin* and *fileds* are words to the
grammar's lexicon; the rest are more than two edits away, too short
(*wil*, *tha*) or not typos (*perf* for *performance*, *Whoah!* for
*Cool!*).

### Paragraphs of raw HTML

The Markdown reader made every raw HTML block an opaque block, like code.
Generated reference pages write their prose as HTML paragraphs
(`<p><code>caCertPath</code> is the path to ...</p>`), so their
misspellings went unseen (2026-10-15). An HTML block made only of `<p>`
elements with inline markup (`a`, `b`, `code`, `em`, `i`, `span`,
`strong`, `br`) is now read as paragraphs: `<code>` is opaque, like
inline code, the common entities are decoded, and offsets point into the
source. Any other HTML stays opaque.

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 163 | 166 |
| Real errors caught with a perfect model | 301 (80%; spelling 156 of 178) | 305 (81%; spelling 160 of 178) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

The four misspellings are now caught with a perfect model, three of them
without one (*fileds*, a word to the grammar's lexicon, needs the model).
Without a model *canot* gets *canto* first, so its fix needs the model
too. The development sets have no HTML paragraphs, so their flags cannot
change. No flag on the corrected text.

### Typos of words the document uses

A suggestion must be a listed word of the common tiers, and
*cryptographic* is only in the rare list, so *crytographic* went
unflagged, though the same sentence writes *cryptographic* (2026-10-15).
A word one edit from exactly one word the document itself uses (and the
dictionary or grammar knows) is now taken for its typo, with that word as
the fix. A word the document uses more than once is still a term, not a
typo.

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 166 | 167 |
| Real errors caught with a perfect model | 305 (81%; spelling 160 of 178) | 306 (81%; spelling 161 of 178) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

No flag on the corrected text.

### Several words two edits away

A word two edits from a common word was a possible typo only when that
was the only common word so close, and *priotized* is two edits from
both *prioritized* and *privatized* (2026-10-15). Such a finding is for
the model to confirm anyway, offered the word as written as well, so it
now chooses among up to four (`TWO_EDIT_CANDIDATES`). And a choice the
model made could still be dropped: when one option parsed strictly and
the chosen one did not, the choice was refused, and the option with the
word as written counted, though an unknown word parses as a generic one.
*The new method here isn't strictly neccesary* parsed, *... strictly
necessary, but ...* did not within its time, and the model's right
answer was dropped. The word as written no longer counts there.

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 167 | 167 (the same pairs) |
| Real errors caught with a perfect model | 306 (81%; spelling 161 of 178) | 309 (82%; spelling 164 of 178) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

The new catches are *neccesary*, *priotized* and *partilar*. Without a
model nothing changes, since these findings need it; with one, more
words are put to it, and only the model's measurement can tell how often
it keeps a correct rare word.

Of the 14 spelling errors a perfect model still misses, *Mananger* and
*Adeliade* are capitalized, which the rule takes for names; *furutre* was
two edits away but had seven letters, under `TWO_EDIT_LENGTH` (see below);
*happenening*, *ephermal*, *refacting* and *authoratitively* are not
found by the two-edit path or are in sentences the oracle cannot match;
the rest are short (*wil*, *tha*, *Nex*), a word to the grammar's lexicon
(*witin*) or not typos (*perf*, *langs*, *Whoah!*).

### Sentences without any reading

A sentence the grammar gives no reading at all, not even a fragment
one, got no candidate corrections: they were built from the words of
its best reading (2026-10-15). *Binding objects link the policy to your
cluster and provides scoping.* is such a sentence, and its correction
(*provide*) parses. Such a sentence now gets the edits of its words
alone (the other forms of each plain word, function words, doubled
words); `fix_bound` finds 142 of 199 grammar errors reachable (139).

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 167 | 167 (the same pairs) |
| Real errors caught with a perfect model | 309 (82%; grammar errors 145 of 199) | 311 (82%; grammar errors 147 of 199) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

The new catches are *provides* and *labelled*. No flag on the corrected
text. *Policy objects describes ...* is fixed too when checked alone,
but not within the time of the full run.

### Words at the start of a sentence

Three grammar errors had no candidate (2026-10-15). *will generate
tokens that look like ...* lacks its subject (*This will ...*), *... must
remain on control plane nodes and not be shared* lacks *should*, and
*Additional, we are starting ...* wants its adverb. *this* and *should*
are now among the words inserted; a word inserted or substituted at the
start of a sentence is capitalized (it took the case of the word it
went before, so the insertion was *this will ...*); and a sentence's
first word before a comma also gets its form in *-ly* when that is a
listed word.

The perfect model took the first option inside the corrected text, and
*will generate tokens ...* is inside *This will generate tokens ...*: it
now takes an option that is a whole corrected sentence first. That alone
changes nothing (311, the same pairs).

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 167 | 167 (the candidates are only for the model) |
| Real errors caught with a perfect model | 311 (82%; grammar errors 147 of 199) | 314 (83%; grammar errors 150 of 199) |

No flag on the corrected text. *his means that ...* (for *This means*)
stays out of reach: the segmenter does not split the sentence off
the one before it (it starts in lower case), so the correction is two
sentences and never verifies as one.

### The end of a sentence

Insertions went before a word, so nothing could be added after the last
one: *... from the kubeconfig files and the files against* wants *it*
(2026-10-15). *it* and *them* are now tried after the last word, before any final
punctuation. And a
function word was deleted with the space after it, which left the comma
of one set off by commas (*as of this writing, this, it compiles*): it is
now deleted with its comma. With a perfect model, 315 of 377 (84%;
grammar errors 151 of 199, from 150; the new catch is *against it*). The
pair with *this,* is not reached: no correction of its 36-word sentence
parses within its time.
No flag on the corrected text; without a model nothing changes.

### Corrections of two words, related words and Hugo shortcodes

Corrections were of one word, or one word inserted or dropped
(2026-10-15). Now also:

- *is* or *are* becomes a modal and *be* (*The output should be similar
  to this*); two function words together are dropped (*the example of
  the flow* for *the example flow*); *to be* is inserted like the other
  inserted words;
- a relative pronoun is replaced together with another form of the verb
  after it (*which has status* for *that have status*);
- related words: another common word (up to tier 60) that shares the
  word's first letters (five, or all but one of a shorter word) and is
  at most three edits away (*iteration* for *iterator*, *extendable* for
  *extensible*, *media* for *medium*), tried before the insertions;
- Hugo shortcodes in Markdown: a glossary tooltip (`{{< glossary_tooltip
  text="API resources" ... >}}`) reads as its text, and any other
  shortcode is dropped, outside inline code, with the offsets into the
  source kept. The real-error pairs from the Kubernetes documentation
  have them; the development sets had them removed by `sample.py`.

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 167 | 167 (the same pairs) |
| Real errors caught with a perfect model | 315 (84%; grammar errors 151 of 199, spelling 164 of 178) | 326 (86%; grammar errors 159 of 199, spelling 167 of 178) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

The new catches: *should be*, *flow* (for *of the flow*), *that have*,
*extensible*, *iterator*, *medium*, *resides* (for *residents*), *shows*,
and three misspellings the spelling rule leaves alone, corrected as
related words: *authoritatively*, *happening*, *refactoring*. No flag on
the corrected text.

Two more spelling cases go to the model. A capitalized word was taken for
a name, but among title-cased common words (*Node Shutdown Mananger*) it
may be a typo: it is now checked like a lower-case word, and such a
finding needs the model. And a word two edits away is now a possible
typo from seven letters (`TWO_EDIT_LENGTH`, eight before): *furutre* for
*future*, offered with *furore* and *nurture*. Without a model nothing
changes (167; the same 6 and 79 false flags); with a perfect one, 328 of
377 (87%; spelling 169 of 178).

### Long sentences verified clause by clause

A correction was verified only if the whole corrected sentence had a
strict reading within its time, and a long sentence whose clauses each
parse often has none (search, M8): its corrections never reached the
model (2026-10-15). A correction of a sentence of 12 words or more that
fails so is now verified on the part around it, between punctuation
marks (`verify_locally`): it must turn that part from no strict reading
into one (*... and so, ultimately, hold different types*), the window
growing by one mark on each side up to three times, never to the whole
sentence. Also:

- negations are substituted (*does* and *doesn't*, *is* and *isn't*,
  *do*, *are*, *can*, *will*): *If the spec of the incoming Pod doesn't
  already contain ...*; *within* joins the prepositions;
- a three-letter word one edit from a word of the commonest tier (*wil*,
  *tha*, *Nex* at the start of a sentence) is a possible typo, shorter
  than the spelling rule's minimum, for the model to confirm.

| Measure | Before | After |
|---|---|---|
| Real errors caught without a model (377) | 167 | 167 (the same pairs) |
| Real errors caught with a perfect model | 328 (87%; grammar errors 159 of 199, spelling 169 of 178) | 337 (89%; grammar errors 165 of 199, spelling 172 of 178) |
| False flags, first and second development sets | 6, 79 | 6, 79 (the same flags) |

The new catches: *An*, *contribute*, *doesn't*, *hold*, *returns*,
*then*, *Next*, *that*, *will*; none lost. No flag on the corrected
text.

### Fewer false flags: names, technical words, rarer English words

The false flags without a model, looked at one by one (2026-10-16):
many were technical words the bundled lists lack (*async*, *libc*,
*rootfs*, *impls*), rare but correct English words (*corslet*, *gists*,
*curtsey*), and one name (*Matt*, taken for a British spelling of *mat*).
The changes:

- the consistency rule skips a sentence-initial word the document
  capitalizes elsewhere, or that another capitalized word follows (*Matt
  Mauer had an idea*), as the spelling rule already does;
- the plural and verb forms of the technical lists' words are accepted,
  and the singular of a listed plural (*impls* from *impl*, *finalizer*
  from *finalizers*, *autoscaled* from *autoscalers*); only for those
  lists: inferring forms of the rare English words let *diabled* through
  as *diable* plus *-d*;
- `core.spelling` ignores a short list of common technical words the
  lists lack (*async*, *libc*, *rootfs*, *nftables*, *subnet*,
  *implementor*, *iirc*) and *alia*, *curtsey*;
- the ESDB/SCOWL size-80 additions, generated from the same database and
  commit as the size-60 and size-70 lists (`data/scowl/en-80-extra.txt`,
  77,257 words; size 80 is the largest without additional copyright), are
  accepted like the size-70 words (*corslet*, *studier*, *gists*). Of the
  178 real spelling errors only *informations* is such a word, and the
  check for a mass noun made plural ignores this list, so it is still
  caught. A word the document uses only capitalized is no longer a word a
  typo is taken to mean (*aturon*, a handle, is no typo of *Turon* in
  *Aaron Turon*, a name that list made known).

| Measure | Before | After |
|---|---|---|
| False flags, first development set (2,498 sentences) | 6 | 4 |
| False flags, second development set (13,442 sentences) | 79 | 53 |
| False flags, held-out set (8,371 sentences) | 35 | 31 |
| Real errors caught without a model (377) | 167 | 167 (the same pairs) |
| Real errors caught with a perfect model | 337 | 337 (the same pairs) |

No new flag on any set. The held-out set, not used while developing the
checks, loses *gists*, *studier*, *corslet* and *autoscales*.

Of the 53 flags left on the second development set, 15 are errors in the
edited text after all (*is is*, *the the* four times, *and and*, *a
overflow* twice, and the misspellings *anyeone*, *consided*, *durnig*,
*reseearchers*, *righly*, *santisers*, *inspite*); 21 are a document
mixing spellings (*behaviour* where the text mostly has *behavior*), which
is what the consistency rule reports; 4 are period spellings in a
nineteenth-century novel (*expence*, *expences*, *an union* twice); 13 are
false: project jargon and handles in the Rust blog (*sret*, *tiif*,
*mgca*, *ambig*, *compiletest*, *redzone*, *hackend*, *funsigned*,
*epage*, *zulips*) and two readings of *has* (*The all-hands has
happened!*).

### Coverage: a conjunction with a comma, and a single verified correction

`culprits` on the fresh 948-sentence sample (2026-10-16): 40 sentences
without a strict reading, scattered, mostly archaic (*knew not how*,
*beyond competition her favourite child*) or not prose. Two recurring
gaps are fixed:

- a sentence-initial coordinating conjunction followed by a comma (*And,
  I have a deep passion for this problem.*, *Or, why did he thus come a
  day before he was expected?*): the grammar reads a conjunction with a
  comma as informal, so the parser is given the sentence without that
  comma (a space keeps the offsets);
- *mortifying*, an adjective the ERG lacks though it has *humiliating*
  and *embarrassing*.

And the consistency rule no longer counts *amongst*, *whilst* and
*amidst* as British spellings: they are other words, used in American
English too.

From the false flags of the second development set:

- a sentence that ends in a colon right after a subordinator (*CRDs are
  a good fit if:*) is a lead-in to the list or block that completes it,
  and is not parsed (no agreement error on *are*); a lead-in ending in
  a preposition is still parsed, since *You should see the output
  similar to:* is a real error;
- *all-hands* is a count noun (*The all-hands has happened!*);
- spelling skips a word the document also writes as a mention
  (*@epage*), the plural of an ignored word (*zulips*), and command-line
  options (*-funsigned-char*, *--dry-run*), which are now code-like
  tokens for the parser too.

Measured: real errors without a model unchanged (167 of 377, the same
pairs); false flags 4 → 4 on the first development set, 53 → 50 on the
second, 31 → 29 on the held-out set, each removed flag one of those
above and none added. With a perfect model, 333 of 377 (88%; spelling
172 of 178, unchanged) against 337 before: the grammar fixes that
differ (*the case*, *of futures*, *Label*, *change*, and three gained)
are ones whose verification runs into the 30-second budget, and the
binary from before these changes misses *Label* and *change* too when
they are rerun alone. The perfect-model figures move by a few pairs
between runs, and more on a cold parse cache (326 on the first run
after a lexicon change), since the budget is wall-clock time.

Measured and rejected: reporting, without a model, a sentence without a
strict reading for which exactly one candidate correction parses
(`single_fix`). Of the grammar errors' sentences without one, about a
fifth have a single verified correction, but apart from doubled words
(*the the*, *and and*, *to to*, which the repeated-word rule already
reports) it is seldom the right one: *afters the update*, *cluster
administered to*, *The API server i willing*. A single correction the
grammar accepts is no evidence of an error.


### Measured and rejected: work limits instead of time limits

The parser's limits are wall-clock time, so whether a candidate
correction verifies depends on the machine and its load: the
perfect-model figure moves by a few pairs between runs of one binary,
and by eleven on a cold parse cache. A limit on work instead (unifications
tried, in the chart and when unpacking, shared by the searches of one
parse) would make results the same everywhere, and was tried. It does
not keep the time limits' behaviour: the rate varies too much by
sentence (11 to 134 unifications a millisecond; 42 the median, 18 to 40
on sentences over 30 words), and neither unifications, chart nodes nor a
mix of the two predicts a search's time within 25% for half of them or a
factor of 2.7 for nine in ten. Calibrated at the median rate, short
sentences ran out of work before their former time and the perfect-model
run lost eight corrections; calibrated at the 75th percentile, long
sentences parsed for nearly twice their former time, and a single parse
reached 2.4 GB, so four measurement processes ran out of memory. Time
limits stay; measurements compare runs on a warm parse cache.

### Spelling without a model: misspellings the morphology or the model hid

Without a model, 35 of the 178 misspellings went unreported. Three causes,
found by tracing each:

- The grammar's morphology read them as word forms: *fileds* as *file*
  + passive + plural (a passive, `_odlr`, was not counted as an
  inflection), *specifing* as *-ing* on the noun *specif*, *refacting* as
  *re-* on the noun *fact*, *returing* as *re-* on the name *turing*,
  *witin* as the informal *wit* + *-in'*. A derived form now counts as a
  known word only when each rule fits its stem's word class (*un-* also
  takes adverbs: *unfrequently*), with one inflection at most, and not in
  an informal form. Two real words the overgeneration used to let
  through are now listed: *nightlies*, and *writeable*.
- The technical lists' inferred plurals took *valuess* for a plural of
  *values*: no plural of a plural now. And a word with one letter typed
  twice (*languagess*) is a typo, not a derivation; main reported
  *languagess* only by accident, through the junk form *languagesd*.
- Findings with a single candidate two edits away, or a misspelled
  derivation, waited for a model. Reported without one when the word is
  lower case, at least six letters, and the candidate is reached by
  dropping or adding letters only (*admistrators*, *neccesary*,
  *securly*, *assignement*). Reporting every single candidate also
  flagged *grafana* (*granada*), *mebibytes*, *syscalls*, *retpoline*,
  *conversible* and *massebot*: words only substitutions make a known
  word of are mostly names and terms. (That criterion was chosen after
  seeing the held-out set's flags.) *distroless* is listed.

A word glued to inline code (two `` `let`s ``) is no longer spell-checked:
the code's masking made it *xxxs*, which only the old overgeneration let
through. typos-dict, a curated misspelling table, cannot be used: it took
in codespell's and Wikipedia's share-alike lists (see `docs/prior-art.md`).

Spelling only (`en check --no-parse` on every document, against main):
18 more real errors flagged in the real-error corpus; on the false-flag
sets, three more flags, all real typos in the second development set
(*borrowcking*, *implemenent*, *rationable*), and no false ones.

## Where checking time goes

`timing` (`crates/emdysi-check/examples`) checks documents on one thread
with no parse cache and reports the time of each stage, the slowest
rules, and how parse time spreads over sentences. On 24 documents of the
first development set (1,388 sentences, 21,867 words):

| Stage | Before: ms a sentence | After the gate below |
|---|---|---|
| parse | 1,416 | 1,402 |
| re-parse (quoting, unpruned) | 24 | 24 |
| grammar-error variant | 610 | 248 |
| rules | 2 | 1 |
| rules' correction parses | 0 | 0 |
| all | 2,085 | 1,706 |

Rules cost nothing next to the parser; the spelling and grammar-error
rules are the slowest at about 1 s for all 1,388 sentences. The
grammar-error variant (the mal-rule grammar, run on sentences without a
strict reading, or with one built on a generic entry for an unknown word)
was the second cost. The gate: a sentence with a strict reading runs the
variant only if one of its generic words has a known stem (a misspelled
or wrongly inflected known word, which a mal-rule can name). Grammar
errors found without a model stay at 24 of 199, and all real errors at
183: the 362 seconds saved found nothing.

Parse time is concentrated in a few sentences:

| Parse time | Sentences | Share of parse time | With a strict reading | Search complete |
|---|---|---|---|---|
| 0–0.5 s | 744 | 7% | 542 | 670 |
| 0.5–1 s | 219 | 8% | 208 | 126 |
| 1–2 s | 162 | 12% | 156 | 67 |
| 2–5 s | 151 | 24% | 141 | 38 |
| 5–9.5 s | 51 | 18% | 35 | 9 |
| 9.5 s and over | 60 | 31% | 1 | 2 |

The 60 sentences that reach the time limit take 31% of parse time and
yield one strict reading between them. A lower limit would save most of
that, but the 5–9.5 s band still finds 35 strict readings in 51
sentences, and limits on work instead of time were measured and rejected
(above). The next step is to make those sentences cheaper (chart pruning
by the tagger's hypotheses, earlier fragment fallback), not to give up
on them sooner.

## The tagger on UD English EWT

`tagger_eval` (`crates/emdysi-parse/examples`) compares the rule-based
tagger's hypotheses with the gold Penn Treebank tags of the UD English
EWT development set (corpora/ud-ewt), counting words only (punctuation
has rules of its own in the grammar). The ERG uses the tags only for
words its lexicon lacks, to build generic entries, so the row that
matters is the second:

| Words | Count | First guess | Among guesses (before) | Among guesses | Guesses a word |
|---|---|---|---|---|---|
| all | 21,972 | 68.8% | 84.4% | 85.0% | 1.99 |
| unknown to the ERG | 1,440 | 66.7% | 76.2% | 83.4% | 2.25 |

Two fixes from the misses: *n't*, a token of its own in EWT, is an
adverb (89 misses), and a capitalized word ending in *-ian*, *-ese* or
*-ish* may be an adjective (*Palestinian*, *Japanese*; the tagger gave
only NNP and NN). The remaining misses among unknown words are mostly
e-mail addresses and URLs (gold ADD), informal spellings (*sooooo*,
*goin*) and lower-case place names.

## WordNet-confirmed lexical gaps

`culprits` finds, for a sentence without a strict reading, the words
whose replacement by a stand-in of some class gives one; `gaps.py`
writes entries for those words that Open English WordNet has in that
class and the ERG lacks. On 40 sentences of the pool of unparsed edited
sentences, before the stand-in run reached its time limit, it proposed
15 entries, most of them wrong: culprits often blames a function word,
and WordNet lists *in* (indium), *so* and *have* as nouns. With short
and function words left out, 8 remain, of which four are sound (*the
fidgets*, *megrim*, *plaice*, *retro* as a noun). Too few to justify a
generated lexicon file yet; the filter is in `gaps.py` for a larger run.

## Imported rule data: LanguageTool, slop and GitLab

The three new opt-in packs, checked without parsing on every document of
the three edited sets (654 documents: 221, 126 and 307) and on the 24
samples of current-model prose (`corpora/ai-current`):

| Pack | Rules | Machine-written: documents with findings | Findings | Edited: documents with findings | Findings |
|---|---|---|---|---|---|
| `languagetool` | 1,645 | 0 of 24 | 0 | 79 of 654 | 141 |
| `slop` | 98 | 23 of 24 | 94 | 171 of 654 | 512 |
| `gitlab` | 26 | 20 of 24 | 57 | 452 of 654 | 9,021 |

**languagetool.** Its findings on edited text are mostly style
suggestions (*all of the* for *all the*, *might perhaps*), and among them
real errors in the sources: *setup* used as a verb (*they've setup a
cloud instance*), *github* in prose, *up stairs*. On the real-error
corpus it finds none of the 199 grammar errors (missing function words
and wrong forms, which LanguageTool catches with its tagger, not with the
word-only rules converted here); its findings there are style
suggestions. An earlier run, before inline code was skipped, flagged *the
`Some` variant* (a Rust type, not an article error). LanguageTool's
wordiness and redundancy lists, measured in the same run, flagged 642 and
95 phrases on edited text, many with wrong replacements (*as shown in* →
*shows that*, *On the other hand* → *Although*) or flagging standard
phrasal verbs (*shut down*, *open up*); they are left out, as
LanguageTool itself ships them turned off.

**slop.** Fires on 96% of the machine-written samples and 26% of edited
documents. Most findings on both sides are its em-dash rule (41 of 94 on
machine-written text, 184 of 512 on edited text), which its own
description calls a house style; without it the pack still separates
the two (*ai-vocab*, *promo*, *genuinely*, *performative-honesty* on the
machine-written side; *stranded-auxiliary* and *load-bearing-adverbs* on
edited prose, where they are mostly ordinary English). Opt-in, for a
reader who wants the slop author's judgement on top of `ai-tells`.

**gitlab.** GitLab's own documentation rules, converted faithfully:
*inclusive-language* flags every *he*, *she* and *his* (4,574 findings,
nearly all in the fiction and blog documents), *future-tense* every
*will* (1,691). A house style for documentation written to GitLab's
guide, not a general check.

**Variant spellings.** The coherency pairs added to
`terms.variant-spelling` change nothing on the edited sets: no document
uses both spellings of a pair that differs in more than hyphens (those
were grouped already).

### Recall after the dictionaries

`real_errors` without a model, after codespell's misspelling list, the
UniMorph inflections and the CMUdict articles: 192 of 377 (51%), up from
183. Spelling 168 of 178 (from 159), grammar unchanged at 24 of 199, and
no corrected sentence flagged.

False flags with parsing on the first development set (`false_flags`,
the whole `core` pack, including the articles rule with its new lists and
CMUdict): 4 flags in 2,498 sentences, 1.6 per 1,000, the same as before
the dictionaries; none from `core.articles`.

### LanguageTool rules with parts of speech

The regex kind's tag conditions let LanguageTool rules that need
open-class parts of speech convert, with the tags taken from the parse
(`emdysi_check::tags`), and its transforms let suggestions change case,
rewrite a matched word or inflect it for a tag (UniMorph). 755 rules with
tag conditions convert; `lt_validate` parses each one's examples, and the
579 that pass are kept. The pack has 2,579 rules.

With parsing, on the first development set (`false_flags` with
`PACK=packs/languagetool.toml`, before the transforms): 17 findings in
2,498 sentences, 6.8 per 1,000, mostly style suggestions (*all of the*,
*cross reference*, *a small number of*) and some real errors in the
sources (*between 6 to 9*, *economical development*, *up to date* before
a noun); two came from rules with tag conditions.

On the real-error corpus (`real_errors` with
`PACKS=packs/languagetool.toml`, the 2,162-rule pack): grammar errors
caught stay at 24 of 199, and one corrected sentence is flagged. The
converted rules target other errors than the ones in this corpus
(missing function words, wrong forms), which need the grammar.

### A lower time limit: 5 seconds instead of 10

`timing` with `TIMEOUT=5` against the default 10 seconds, on the same
24 documents of the first development set (1,388 sentences, one thread,
no parse cache):

| Limit | ms a sentence | Sentences with a strict reading | Findings that changed |
|---|---|---|---|
| 10 s | 1,703 | 1,083 | |
| 5 s | 1,381 (−19%) | 1,056 (−27) | passive 208 → 201, agentless passive 127 → 120, rule of three 43 → 40, stacked negation 18 → 15, tense shift 14 → 12, noun stacks 34 → 32 |

Spelling and grammar-error findings do not change, but 27 sentences (2%)
lose their strict reading, and with it the findings of the rules that
read the parse. The default stays at 10 seconds; `--timeout` remains for
those who prefer speed.

The 81 sentences the 5-second run put in the "5 to 9.5 s" row are not
parses that overran the limit by seconds: parsed again on their own with
the same limit, the 116 sentences that took 5 seconds or more took at
most 5.4 seconds (the row's range is coarse). A parse's several searches
(narrow, wide and wider beams, then without pruning) and their grace
periods for unpacking now share one hard deadline, the limit plus a
fifth of it (at most two seconds), which makes that bound explicit.
