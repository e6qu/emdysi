# emdysi's extensions of the English Resource Grammar

TDL files loaded after the ERG (`grammar/erg`), in file-name order, by
`Erg::load_config` (`crates/emdysi-parse/src/lib.rs`). The ERG's own files
are not changed. Written for this project (MIT, see the repository's
`LICENSE`), in the ERG's conventions and on its types.

- `adjectives.tdl`: comma-separated adjectives before a noun (*a
  selfish, hypocritical woman*, *a long, cold, dark night*): the ERG
  admits the comma only as its informal comma entry (`comma_inf_pct`);
  `comma_adj_pct` is the same comma for adjectives only, not informal.
- `adjuncts.tdl`: fronted prepositional phrases and adverbs without a
  comma (*In 2019 we released the book.*, *Yesterday we left.*): the ERG's
  no-comma filler-head rules (`flr-hd_nwh-nc_c`, `-nmc_c`) are informal;
  `flr-hd_nwh-nc-adj_c` and `-adj-nmc_c` are their formal counterparts for
  adjuncts only. Fronted adjectives and noun phrases without a comma stay
  informal.
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
- `register.tdl`: contracted auxiliaries (*we'll*, *it's*, *they're*)
  count as formal: the ERG marks them informal to steer its generator,
  which leaves every sentence with a contraction without a strict
  analysis; edited prose uses them throughout.
- `lexicon.tdl`: adjectives used as plural nouns for people with "the"
  (*the brave*, *the free*, *the meek*), in the form of the ERG's 94 such
  entries (type `n_-_c-pl-def_le`, e.g. *the poor*); adjectives used as
  abstract mass nouns with "the" (*the realm of the unimaginable*, *the
  sublime*), in the form of the ERG's 23 (`n_-_m-def_le`, e.g. *the
  impossible*); *onboard* (preposition, adjective and verb); hyphenated
  *out-of-X* adjectives (*out-of-memory kills*, *out-of-band data*). A
  native entry replaces the ERG's generic entries for an unknown word, so
  a word the ERG lacks gets all its common uses (*unusual*, *unexpected*
  also as adjectives, in the types of *usual* and *probable*).

The ranking model was trained on the ERG's own rule and entry names; the
ranker and the evaluation read an extension's rule or entry as the ERG one
it stands for (`EQUIVALENT` in `crates/emdysi-parse/src/rank.rs`). On the
gold profiles (`docs/evaluation.md`) the two newest files cost one gold
tree and one parse on sh-spec (359 vs 360 gold trees found, 521 vs 522
parsed) and gain one top-ranked tree (314 vs 313); csli, esd and control
are unchanged.

Tests: `crates/emdysi-check/tests/stress.rs` (`corpora/stress`). For
grammar work, `EMDYSI_CHART=1` (with `EMDYSI_CHART_PATH="SYNSEM LOCAL
CONJ"`) prints the chart, `EMDYSI_TRACE_RULE=<rule>` traces a rule's
attempts and root checks, and
`cargo run --release -p emdysi-parse --example grammar_check -- <type|rule|word>...`
reports types whose constraints are inconsistent.
