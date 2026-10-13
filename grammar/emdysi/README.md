# emdysi's extensions of the English Resource Grammar

TDL files loaded after the ERG (`grammar/erg`), in file-name order, by
`Erg::load_config` (`crates/emdysi-parse/src/lib.rs`). The ERG's own files
are not changed. Written for this project (MIT, see the repository's
`LICENSE`), in the ERG's conventions and on its types.

- `subjects.tdl`: a subject followed by an adjunct set off by commas
  (*The committee, however, decided to wait.*). The ERG's subject-head
  phrase takes a subject with a comma only through the informal comma
  (`comma_inf_pct`), so every such sentence was informal; these copies
  (`sb-hd_mc-pc_c`, `sb-hd_nmc-pc_c`) take the formal one, and the parser
  lets them build a clause only when the verb phrase begins with an
  adjunct ending in a comma (`Rule::paired_subject`).
- `gapping.tdl`: a conjunct that leaves out the verb of the clause
  before it (*One came from her books, and the other from her toilette.*,
  *Kim reads books, and Sandy magazines.*). The ERG has no gapped clause;
  these rules make one from a subject and a prepositional phrase
  (`cl_gap-pp_c`) or noun phrase (`cl_gap-np_c`) remnant, with the elided
  verb as an `ellipsis_rel`. The parser lets it be only the clause after a
  coordinating conjunction (`Rule::complement_of`), after a comma, and not
  a sentence on its own, so that *I saw Kim and Sandy in the park* keeps
  its noun coordination.
- `sluicing.tdl`: a sluice with a phrase (*without knowing in what
  direction*, *I do not know with whom*, *nobody knows which one*). The
  ERG has a sluice for a single wh-word only (`what_cp`, `how_cp`); these
  unary rules make the same elliptical question from a wh noun phrase
  (`cl_sluice-np_c`) or prepositional phrase (`cl_sluice-pp_c`). The
  parser lets the clause be only a verb's complement
  (`Rule::complement_of`) and not contain a relative clause, so that *I
  wonder which book he read* stays one embedded question.
- `tokens.tdl`: emdysi's token-mapping rules, applied before the ERG's.
  A phrasal compound the lexicon does not have (*allow-by-default*,
  *pay-as-you-go*: three or more lower-case words joined by hyphens) also
  gets a token of its own beside the split one (`Erg::phrasal_compounds`),
  which the generic noun entry analyses; `hyphenated_punct_suffix_clitic_tmr`
  attaches a following punctuation mark to it, as the ERG's
  `punct_suffix_clitic_tmr` does for a word without hyphens. A whole-chain
  word, generic or of the lexicon (*wait-and-see*), gets a bonus in the chart
  so that pruning and unpacking keep it, and readings that take its hyphens
  for dashes are dropped when a strict reading does without.
- `tokenizer.rpp`: rules for the tokenizer, applied after the ERG's
  (`grammar/erg/rpp/tokenizer.rpp`). A range of amounts with a currency
  sign on both ends (*£3,000-£5,000*, *$10-$20*) is split like the ERG
  splits *10-20*; the ERG left *10-$* as one token. An amount after a
  currency sign with a suffix for its magnitude (*£2bn*, *$5m*, *$300k*)
  is read as *£ 2 billion* and so on; *5m* alone may be metres.
- `adjectives.tdl`: comma-separated adjectives before a noun (*a
  selfish, hypocritical woman*, *a long, cold, dark night*): the ERG
  admits the comma only as its informal comma entry (`comma_inf_pct`);
  `comma_adj_pct` is the same comma for adjectives only (not for a
  conjunction, whose head the adjective type does not exclude: *Kim and,
  Sandy left*), not informal.
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
- `comparatives.tdl`: *more* and *less* comparatives whose *than* has its
  noun phrase elsewhere: shared with another conjunct (*similar to, but
  less extreme than, the climate*) or extracted (*Who is Kim less tall
  than?*). The ERG allows this for inflected comparatives (*bigger than*)
  but not after its degree words *more* and *less*. The copies
  `more_deg_gap` and `less_deg_gap` take a *than* phrase with one gap, and
  the rule `hd-cmp_u-than-sl_c` passes that gap up, as the ERG's
  head-complement phrase takes gaps from its head only. The parser allows
  no other rule over their phrase (`Rule::sealed_heads`).
- `conjunctions.tdl`: a coordinating conjunction followed by an adjunct
  between commas (*Kim stayed, but, in the end, Sandy left.*, *Bennet did
  not reply, but, unable to wait, began scolding.*): the ERG attaches the
  adjunct to the conjunction (`hd-aj_cmod_c`, `hd-aj_cmod-s_c`), but its
  marker rules take a marker ending in a comma only as the informal comma.
  `mrk-nh_cl-pc_c` and `mrk-nh_evnt-pc_c` are the clause and verb-phrase
  marker rules for a conjunction with its adjunct ending in a formal
  comma. The comma after the conjunction must open the bracket: the
  parser keeps a reading with them only if the conjunction carries a
  comma (`Erg::licensed`, as the ERG's phrases do not keep their
  daughters), so *Kim stayed and in the end, left* gets nothing new. *Kim
  and, Sandy left* stays without a strict analysis.
- `coordination.tdl`: two noun phrases coordinated between a pair of
  commas (*You, or a controller, must delete the object.*, *Kim, and
  Sandy, left.*): the ERG ties a comma on the left conjunct to the
  conjunction so that only its informal comma can mark the left one of
  two conjuncts (to rule out *Abrams, and Browne arrived*), which left
  the bracketed coordination only an informal analysis.
  `np-np_crd-pc-t_c` takes formal commas on both conjuncts, and the
  coordinated phrase ends without punctuation; its semantics is that of
  `np-np_crd-t_c`. *Abrams, and Browne arrived*, *The dog, barked* and *I
  saw Kim, and Sandy.* stay without a strict analysis.
- `dates.tdl`: US-style dates with a comma before the year (*on October
  31, 1832*): the ERG takes that comma only as its informal comma entry,
  so the date analysis was informal (the sentence kept strict readings
  only with other, wrong structures); `comma_dom_pct` is the same comma
  for a day of the month only, not informal.
- `discourse.tdl`: a sentence that starts with a coordinating conjunction
  (*And his sisters are charming women.*, *But I left.*), which the ERG
  analyses as a fragment: the conjunction-marked clause becomes a main
  clause when it spans the whole input (a spanning-only rule, declared in
  `settings.cfg`, which the loader reads in the syntax of ACE's
  configuration files). The conjunction's relation keeps its left
  argument unfilled.
- `roots.tdl`: exclamations without a verb (*What a contrast!*,
  *Nonsense!*): the ERG's strict root takes clauses only, so these were
  formal fragments. `root_strict_excl` is its fragment root for edited
  prose, declared a strict root in `settings.cfg`; the parser keeps a
  reading under it as strict only if it ends in an exclamation mark
  (`Erg::licensed_root`), as the root cannot tell *!* from *.*, and as a
  formal fragment otherwise (*The cat.*, *Because it failed.*).
- `register.tdl`: contracted auxiliaries (*we'll*, *it's*, *they're*)
  count as formal: the ERG marks them informal to steer its generator,
  which leaves every sentence with a contraction without a strict
  analysis; edited prose uses them throughout.
- `heavy.tdl`: a long object after an adjunct (*express through their
  disapproval all their hatred of modern parenting*). The ERG marks every
  verb-adjunct-object order informal (`hd-aj_vmod_c`, `hd-aj_vmod-s_c`);
  `hd-aj_vmod-hv_c` and `hd-aj_vmod-s-hv_c` are formal copies, which the
  parser lets take only an object longer than the adjunct and at least
  five tokens long (`Rule::heavy_object_parents`). *We saw yesterday the
  cats* stays informal. An adjunct set off by a pair of commas is formal
  whatever follows it (*He could, in her presence, be rude.*):
  `hd-aj_vmod-pc_c` and `hd-aj_vmod-s-pc_c`.
- `lexicon.tdl`: adjectives used as plural nouns for people with "the"
  (*the brave*, *the free*, *the meek*), in the form of the ERG's 94 such
  entries (type `n_-_c-pl-def_le`, e.g. *the poor*); adjectives used as
  abstract mass nouns with "the" (*the realm of the unimaginable*, *the
  sublime*), in the form of the ERG's 23 (`n_-_m-def_le`, e.g. *the
  impossible*); *onboard* (preposition, adjective and verb); hyphenated
  *out-of-X* adjectives (*out-of-memory kills*, *out-of-band data*);
  collective nouns with a plural verb (*The team are ready*), which the
  ERG allows as British for *class*, *committee* and *government* only
  (`n_-_c-pl-nomcase_le`): 46 more in the same form, without the dialect
  for *majority*, *minority* and *staff*, and with an explicit
  determiner (not a bare *team*). A reading with one is kept only when no
  strict reading does without it, as it adds nothing where the verb does
  not agree. Team names (*South Africa
  are*) are not covered: the lexicon cannot tell them from people's
  names (*Kim are* is an error); *from* and *to* with adjectives
  (*range from yellowish to bluish*, *went from bad to worse*): the ERG's
  *from ... to* takes noun phrases only. `from_to_adj` is a copy for an
  adjective phrase whose *to* phrase is a semantically empty *to*
  (`to_prtcl_adj`, keyed apart so that *belong to* cannot take it); it
  contributes the *to* relation itself, so *range from yellow to blue*
  means the same with its adjectives as with its nouns; the older units
  of time *twelvemonth* and *sennight*, in the form of *fortnight*;
  transitive *meditate*, to plan (*Miss Darcy's meditated elopement*), which
  the ERG has only intransitive and with *on*; *oftener* with a *than*
  complement (*oftener from home than anything else could do*), in the
  form of the ERG's *sooner*;
  transitive *provision* (*Autoscalers provision the Nodes*), which the
  ERG has only with *with*, in the form of its *supply*;
  nouns it lacks or has only as verbs or mass nouns: *compile time*
  (*at compile time*), *a redirect*, *a recap of*, *the decease of*, *the
  entail of*, *the first transports of rage*; *altogether* before a verb
  (*he altogether failed*), which the ERG has only after one;
  unknown plural names with a determiner (*the Bennets were*, *the
  Gardiners*): a generic entry for words the tagger marks NNPS, in the
  ERG's type for plural names (`n_-_pn-pl-def_le`, *The Giants play
  today*), exempt from the ERG's lexical filter that would remove it
  beside the singular generic name. A
  native entry replaces the ERG's generic entries for an unknown word, so
  a word the ERG lacks gets all its common uses (*inexplicable*,
  *improbable* also as adjectives; *unusual*, *unexpected* need none, the
  ERG derives them with its "un-" rule); mass uses
  of nouns the ERG has only as count nouns (*too much ceremony*), one by
  one, since a bare count noun is a grammar error the checker reports;
  hyphenated compound adjectives (*in-person*, *in-memory*, *at-risk*,
  *compile-time*, *opt-in*, *ill-behaved*) in the form of the ERG's *in-house*, and
  *opt-in* and *opt-out* as nouns in the form of its *roll-out*;
  technical vocabulary (*a borrow*, *swap* as a mass noun, *metrics* as a
  noun modifier like *statistics*, intransitive *evaluate*,
  *out-of-the-box*). *true* and *false* as nouns (*evaluates to true*)
  were tried and left out: they made *That is true.* ambiguous.

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
