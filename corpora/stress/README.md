# Grammatical stress sentences

`stress.tsv`: sentences that are grammatical English but hard for a parser
or a checker. Each line has a kind, the sentence and a note.

- `repetition`: the same word many times in a row or in different roles:
  *Buffalo buffalo Buffalo buffalo buffalo buffalo Buffalo buffalo*
  (William J. Rapaport), *Police police police ...*, *James, while John had
  had "had" ...*, *That that is is that that is not is not*, *Will Will
  will Will Will's will?*, *Can can can can can can*.
- `garden-path`: sentences whose first reading leads the reader astray
  (*The horse raced past the barn fell*, Thomas Bever; *The old man the
  boat*; *Fat people eat accumulates*).
- `ambiguity`: sentences with more than one meaning (*Time flies like an
  arrow; fruit flies like a banana*; *I saw the man with the telescope*;
  *Visiting relatives can be boring*) and a sentence that is grammatical
  but meaningless (*Colorless green ideas sleep furiously*, Noam Chomsky).
- `embedding`: center embedding (*The rat the cat the dog chased killed ate
  the malt*) and right branching.
- `other`: pangrams, inversion, comparative correlatives, the first
  sentence of *Pride and Prejudice* (public domain), and the comparative
  illusion *More people have been to Russia than I have*, which readers
  accept although it has no coherent meaning.

These are short, well-known example sentences from the linguistics
literature and folklore, quoted for testing; the list and notes were
written for this project. `crates/emdysi-check/tests/stress.rs` checks
that the `core` pack claims no error in any of them (except one
deliberately informal sentence, see its note) and how many get a full
analysis.
