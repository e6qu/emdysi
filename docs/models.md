# Local models

emdysi's grammar, rules and spelling need no model. A local language
model is optional and only ever helps where they cannot decide (decisions
[D10](decisions.md) and [D12](decisions.md)):

- **Decisions** (`en decide`, the `decisions` pack, `--decide-readings`): a
  question with fixed answers gets a probability for each answer, in the
  manner of decision-only models such as Jev. No text is generated.
- **Rewriting** (`en rewrite`): the model proposes rewrites of sentences the
  rules cannot fix; the grammar and the checks keep only those that parse,
  keep the meaning and introduce no new problem.

Weights are never bundled or downloaded by emdysi: you point it at a file
or a server.

## Backends

Every command that takes a model accepts one of the options below; without
one, the `EMDYSI_LM` environment variable names the model the same way as
`--lm` (`EMDYSI_LM=gguf:/models/gemma-4-e2b-it-Q4_K_M.gguf`,
`EMDYSI_LM=http://127.0.0.1:8080`), so it is used by every command:

| Option | Backend | Model format | Probabilities |
|---|---|---|---|
| `--model FILE.gguf` (or `--lm FILE.gguf`) | llama.cpp in-process; needs a build with `--features llama` | GGUF | exact, every token |
| `--server URL` (or `--lm URL`, `--lm URL#MODEL`) | any server with the OpenAI-compatible API: llama.cpp's `llama-server`, `mlx_lm.server`, Ollama, LM Studio, vLLM | whatever the server runs: GGUF, MLX, ... | the server's top tokens (up to 20) |
| `--lm script:FILE` | a scripted stand-in ([`script.rs`](../crates/emdysi-lm/src/script.rs)) for tests and trials | | as scripted |

An in-process model uses every core; `EMDYSI_LM_THREADS=N` caps its
threads, for several models at once (the measurement examples open one
per `THREADS`).

`--server-model NAME` sets the model name sent to the server when it serves
several.

Examples. A GGUF file in-process, with a build that has the `llama`
feature (`cargo install --path crates/emdysi --features llama`):

```sh
en decide --model MODEL.gguf --statement "Paris is in France."
```

A GGUF file served by llama.cpp:

```sh
llama-server -m MODEL.gguf --port 8080
en check --pack decisions --server http://127.0.0.1:8080 notes.md
```

An MLX model on Apple silicon:

```sh
mlx_lm.server --model mlx-community/MODEL --port 8080
en rewrite --server http://127.0.0.1:8080 draft.md
```

A model served by Ollama:

```sh
en decide --server http://127.0.0.1:11434 --server-model MODEL \
   --question "Which is clearer?" --option "Utilize the tool." --option "Use the tool."
```

A small instruction-tuned model is enough for decisions: Gemma 4 E2B or
E4B, or similar models of 1 to 4 billion parameters, quantized to 4 bits.
Larger models decide better and more slowly; measure before choosing (see
[Calibration](#calibration)).

## Decisions

```sh
en decide [MODEL] --question Q --option X --option Y ... [--context TEXT]
en decide [MODEL] --statement S [--context TEXT]
en decide [MODEL] --question Q --scale 1..5 [--context TEXT]
```

print the probability of each answer. Up to ten options; a statement gets
the probability that it is true; a scale gets a distribution over its
points.

How it works ([`decide.rs`](../crates/emdysi-lm/src/decide.rs)): the options
are labelled A, B, C, ...; the model gets the system message *You make
decisions. Reply with only the letter of your choice.* and a user message
with the context, the question and the labelled options; the probabilities
of the letters as the first token of its reply, renormalised over the
options, are the answer. This needs only the probabilities of generated
tokens, which every backend above provides (llama.cpp's server and MLX's do
not return the probabilities of prompt tokens, so scoring the options as
text would not be portable). A server returns only its top tokens (up to 20; `mlx_lm.server`
allows at most 11, and emdysi then asks for 10); an option outside them
gets the probability of the least likely token returned, an upper bound.

Models prefer some positions (often A). By default each question is asked
twice, with the options in the given and the reverse order, and the two
answers are averaged; `--no-debias` asks once.

Small models prefer some positions far more than that cancels: Gemma 4
E2B answers A to almost any question, and the two orders average out near
one half (docs/evaluation.md). Where the options are versions of one
sentence, a model that scores text compares them directly instead: the
probability of each version after the context (`--likelihood`; E2B 92%
right on the real-error questions, against 75% with letters). Spelling
and grammar corrections are chosen this way whenever the model allows it
(a GGUF file in-process; a server only if it returns the log-probabilities
of an echoed prompt), and with letters otherwise. With such a model,
whether a sentence the grammar accepts is worth correcting, and which of
its words is wrong, is not asked either but read from each word's
*surprise*: the negative log-probability of its tokens where it stands,
less that of the same word in a list of the sentence's words (a rare word
is surprising anywhere; a doubled *the*, or the word after a gap, only
where it stands).

### Calibration

A model's probabilities are often overconfident. `en decide --eval FILE.tsv`
asks the questions of a file with known answers and reports accuracy, log
loss, Brier score and calibration error, at the given temperature and at the
temperature that fits the file best. Each line has four tab-separated
fields: the question, its context (which may be empty), the options
separated by `|`, and the index of the right option, counting from 0:

```text
Which spelling is American English?		colour|color	1
```

Pass the fitted temperature with `--temperature T` to every command that
asks the model. Thresholds in the `decisions` pack (0.85 to 0.9) assume a
calibrated model.

### Where emdysi asks

- **The `decisions` pack** (opt-in: `--pack decisions` with a model): rules
  of kind `decide` ask a question about each sentence or block and report it
  when the flagged answer is likely enough: sentences with nothing a reader
  could check, sentences that could be cut without losing information,
  headings that name a topic without saying anything, unexplained jargon.
  Without a model these rules do nothing. Pack authors can write their own:

  ```toml
  [[rule]]
  id = "mypack.passive-blame"
  kind = "decide"
  unit = "sentence"
  min_words = 6
  question = "Does this sentence hide who is responsible for something?"
  options = ["yes", "no"]
  flag = "yes"
  threshold = 0.85
  message = "Who did this? ({percent}% by the model)"
  ```

  `unit` is `sentence` or `block` (a paragraph, heading or list item).

- **Parse disambiguation** (`--decide-readings` with `check`, `fix` or
  `parse`): when the two best readings of a sentence are close, the model
  sees the sentence bracketed both ways (*The [relationship between sleep]
  and memory* against *The relationship between [sleep and memory]*) and the
  second reading becomes the best one if the model prefers it with
  probability 0.6 or more. The model also sees the sentences before and
  after, so the context can settle what the sentence alone cannot. Every
  rule that reads the analysis benefits. Before any model is asked, emdysi
  already prefers, among readings that score about the same, the one that
  groups words as the document does elsewhere (a heading "Sleep and
  Memory" supports *between [sleep and memory]*); `--no-context-readings`
  turns that off.

- **Spelling and grammar fixes** (`check` and `fix`, whenever a model is
  given or `EMDYSI_LM` names one; see
  [`assist.rs`](../crates/emdysi-check/src/assist.rs)): the grammar proposes
  and verifies, the model chooses in context.
  - A sentence with no full analysis is corrected by dropping a doubled
    word (*the the*), changing one word to another listed form of itself
    (*consumes*/*consume*), changing a function word to another of its
    class (*are*/*is*, *for*/*to*, *which*/*that*, *or*/*and*), dropping
    one or inserting one (*the*, *of*, *to*, *you*, ...), or a real-word
    typo for a common word one edit away (*Manger*/*Manager*); the edits
    at the words the model finds most surprising (up to three), or, with a
    model that cannot score text, at the word it names when asked which
    word is wrong, are tried first, then those nearest where the grammar's
    partial analyses meet. A sentence the grammar accepts is corrected the
    same way only when one of its words is 3 nats or more surprising
    (`screen_surprise` of `core.grammar-fix`), or, without scores, when
    the model, asked first (*Is this sentence correct English, as
    written?*), doubts it with probability 0.5 or more (`screen`), since
    many errors leave a sentence grammatical; in such a sentence no word
    is dropped unless its neighbour repeats it (a model prefers a version
    without a word for being shorter). Up to 120 edits are parsed,
    and up to 27 that the strict grammar accepts are shown to the model
    with the original, compared by likelihood or asked (*Which version of
    the sentence is correct English, as the writer meant it?*), nine at a
    time and then the choices together; a model that scores tokens first
    estimates from one pass over the original which five are most
    promising (the likelihood of each version's new word where the old
    one stands), and only those are compared in full. `core.grammar-fix`
    reports one, with the fix, when the model prefers it with probability
    0.8 or more. *The quick brown fox jump over the
    crazy wall.* is the case it is for: the grammar reads it as a noun
    phrase (a jump), so it cannot claim an error alone; the model alone
    could prefer a sentence the grammar rejects.
  - When several known words are one edit from a misspelling (*chck*:
    *check*, *chick*, *chuck*, ...), the model chooses one in the
    sentence; it becomes the fix at probability 0.8 or more
    (`model_threshold` of `core.spelling`), if the sentence with it parses
    as well as with any other.

- **Rewriting**: when the server cannot score text, the model chooses among
  the candidates that passed every check.

## A tiny model for testing

[`scripts/tiny-model/build.py`](../scripts/tiny-model/build.py) trains a
model of about five million parameters on CPU in about an hour, from the
vendored corpora, and writes it as MLX (Hugging Face layout) and GGUF from
the same weights. It is useless as a language model, but it exercises every
backend: it learns the decision prompt on two toy tasks (American spelling,
from `data/scowl`, and grammaticality, from `corpora/blimp`), so its answers
are better than chance. Nothing it writes is committed.

```sh
pip install "jax[cpu]" optax safetensors tokenizers gguf numpy "mlx[cpu]" mlx-lm
python scripts/tiny-model/build.py --out ~/.cache/emdysi/tiny-model
mlx_lm.server --model ~/.cache/emdysi/tiny-model/mlx --port 8091 &
llama-server -m ~/.cache/emdysi/tiny-model/tiny.gguf --port 8092 &
EMDYSI_TEST_GGUF=~/.cache/emdysi/tiny-model/tiny.gguf \
EMDYSI_TEST_SERVERS=http://127.0.0.1:8091,http://127.0.0.1:8092 \
EMDYSI_TEST_SAME_WEIGHTS=1 \
  cargo test -p emdysi-lm --features llama --test backends
```

The test checks that every backend answers and that backends running the
same weights agree.

Measured on 2026-10-03 (5.2M parameters, 2,000 steps, about 35 minutes on
four CPU cores), with `en decide --eval` on the 304 held-out toy questions
the script writes to `eval.tsv`:

| Backend | Accuracy | Log loss | Calibration error | Fitted temperature |
|---|---|---|---|---|
| GGUF in-process (llama.cpp) | 0.635 | 0.632 (0.622 fitted) | 0.055 (0.033 fitted) | 0.52 |
| GGUF, `llama-server` | 0.635 | 0.632 (0.622 fitted) | 0.055 (0.029 fitted) | 0.52 |
| MLX, `mlx_lm.server` | 0.635 | 0.632 (0.622 fitted) | 0.055 (0.029 fitted) | 0.52 |
| `llama-server`, `--no-debias` | 0.602 | 0.695 | 0.097 | 2.15 |

The three backends agree (the two servers to within 0.001 in every
log-probability), and asking in both option orders is worth three points of
accuracy even for this model. A real instruction-tuned model should do far
better; these numbers only show that the machinery works.
