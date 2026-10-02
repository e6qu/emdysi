#!/usr/bin/env python3
"""Build sample.tsv from a Beemo checkout (dataset.parquet; needs pyarrow).

Row filter, applied in upstream order (ascending `id`):
  - `model` is one of the Apache-2.0/MIT-licensed generators below;
  - `category` is "Generation" or "Open QA" (prompts without an input
    text, so the output does not restate No Robots text, CC BY-NC 4.0);
  - first N such rows.
Columns kept: id, model, category, model_output, human_edits (the
expert-edited output). Dropped: prompt_id, prompt and human_output (No
Robots, CC BY-NC 4.0) and the Llama-3.1/GPT-4o edits (other terms).

Cells are escaped: backslash -> \\\\, tab -> \\t, newline -> \\n, CR -> \\r.

Usage: python3 sample.py <beemo checkout>/dataset.parquet [N=200] > sample.tsv
"""
import sys

import pyarrow.parquet as pq

MODELS = {
    "HuggingFaceH4/zephyr-7b-beta",  # MIT
    "mistralai/Mistral-7B-Instruct-v0.1",  # Apache-2.0
    "mistralai/Mixtral-8x7B-Instruct-v0.1",  # Apache-2.0
}
CATEGORIES = {"Generation", "Open QA"}


def esc(s):
    return (s.replace("\\", "\\\\").replace("\t", "\\t")
            .replace("\r", "\\r").replace("\n", "\\n"))


path = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 200
rows = pq.read_table(path).to_pylist()
rows.sort(key=lambda r: int(r["id"]))
print("# id\tmodel\tcategory\tmodel_output\texpert_edited; first %d rows of "
      "zephyr-7b-beta/Mistral-7B/Mixtral-8x7B, Generation and Open QA only; "
      "escaped \\\\ \\t \\n \\r; see SOURCE.md" % n)
k = 0
for r in rows:
    if r["model"] not in MODELS or r["category"] not in CATEGORIES:
        continue
    cells = [str(r["id"]), r["model"], r["category"], r["model_output"], r["human_edits"]]
    print("\t".join(esc(c) for c in cells))
    k += 1
    if k == n:
        break
