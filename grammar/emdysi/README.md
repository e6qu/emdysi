# emdysi's extensions of the English Resource Grammar

TDL files loaded after the ERG (`grammar/erg`), in file-name order, by
`Erg::load_config` (`crates/emdysi-parse/src/lib.rs`). The ERG's own files
are not changed. Written for this project (MIT, see the repository's
`LICENSE`), in the ERG's conventions and on its types.

- `cmpcorr.tdl`: comparative correlatives, which the ERG does not
  analyse: *The more you read, the more you know.*, *The bigger they are,
  the harder they fall.*, *The more money you make, the more you spend.*,
  *The sooner we start, the better.*, *The more, the merrier.* Each half
  is a clause with a fronted comparative (a filler-head phrase) or a bare
  comparative, marked by "the" as a paired marker (like "either"); a
  binary rule joins the halves into a sentence whose semantics is a
  `comparative_correlative_rel` with the first half (the condition) as
  ARG1 and the second as ARG2.
- `discourse.tdl`: a sentence that starts with a coordinating conjunction
  (*And his sisters are charming women.*, *But I left.*), which the ERG
  analyses as a fragment: the conjunction-marked clause becomes a main
  clause when it spans the whole input (a spanning-only rule, declared in
  `settings.cfg`, which the loader reads in the syntax of ACE's
  configuration files). The conjunction's relation keeps its left
  argument unfilled.
- `lexicon.tdl`: adjectives used as plural nouns for people with "the"
  (*the brave*, *the free*, *the meek*), in the form of the ERG's 94 such
  entries (type `n_-_c-pl-def_le`, e.g. *the poor*).

Tests: `crates/emdysi-check/tests/stress.rs` (`corpora/stress`). For
grammar work, `EMDYSI_CHART=1` (with `EMDYSI_CHART_PATH="SYNSEM LOCAL
CONJ"`) prints the chart, `EMDYSI_TRACE_RULE=<rule>` traces a rule's
attempts and root checks, and
`cargo run --release -p emdysi-parse --example grammar_check -- <type|rule|word>...`
reports types whose constraints are inconsistent.
