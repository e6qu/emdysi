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
