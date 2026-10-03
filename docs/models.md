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

Every command that takes a model accepts one of:

| Option | Backend | Model format | Probabilities |
|---|---|---|---|
| `--model FILE.gguf` (or `--lm FILE.gguf`) | llama.cpp in-process; needs a build with `--features llama` | GGUF | exact, every token |
| `--server URL` (or `--lm URL`, `--lm URL#MODEL`) | any server with the OpenAI-compatible API: llama.cpp's `llama-server`, `mlx_lm.server`, Ollama, LM Studio, vLLM | whatever the server runs: GGUF, MLX, ... | the server's top tokens (up to 20) |
| `--lm script:FILE` | a scripted stand-in ([`script.rs`](../crates/emdysi-lm/src/script.rs)) for tests and trials | | as scripted |

`--server-model NAME` sets the model name sent to the server when it serves
several.

Examples:

```sh
# GGUF in-process (cargo install --features llama)
en decide --model MODEL.gguf --statement "Paris is in France."

# GGUF served by llama.cpp
llama-server -m MODEL.gguf --port 8080
en check --pack decisions --server http://127.0.0.1:8080 notes.md

# MLX on Apple silicon
mlx_lm.server --model mlx-community/MODEL --port 8080
en rewrite --server http://127.0.0.1:8080 draft.md

# Ollama
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

### Calibration

A model's probabilities are often overconfident. `en decide --eval FILE.tsv`
asks the questions of a file with known answers and reports accuracy, log
loss, Brier score and calibration error, at the given temperature and at the
temperature that fits the file best:

```text
# question<TAB>context<TAB>option|option|...<TAB>index of the right option (from 0)
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
  unit = "sentence"          # or "block": a paragraph, heading or list item
  min_words = 6
  question = "Does this sentence hide who is responsible for something?"
  options = ["yes", "no"]
  flag = "yes"
  threshold = 0.85
  message = "Who did this? ({percent}% by the model)"
  ```

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
