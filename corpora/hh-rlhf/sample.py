#!/usr/bin/env python3
"""Build sample.tsv from Anthropic HH-RLHF helpful-base/test.jsonl.gz (a Git
LFS object; fetch it with `git lfs pull` or from
media.githubusercontent.com).

For each of the first N records (in file order) whose final assistant turn
is non-empty: the text of the final "Assistant:" turn of the `chosen`
dialogue. The human turns and the `rejected` dialogue are dropped. The id
is the record's 1-based line number in test.jsonl.

Cells are escaped: backslash -> \\\\, tab -> \\t, newline -> \\n, CR -> \\r.

Usage: python3 sample.py <hh-rlhf checkout>/helpful-base/test.jsonl.gz [N=200] > sample.tsv
"""
import gzip
import json
import sys

MARK = "\n\nAssistant:"


def esc(s):
    return (s.replace("\\", "\\\\").replace("\t", "\\t")
            .replace("\r", "\\r").replace("\n", "\\n"))


path = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 200
print("# line\tassistant; final assistant turn of `chosen`, first %d "
      "non-empty records of helpful-base/test.jsonl; escaped \\\\ \\t \\n \\r; "
      "see SOURCE.md" % n)
k = 0
with gzip.open(path, "rt", encoding="utf-8") as f:
    for i, line in enumerate(f, 1):
        if not line.strip():
            continue
        chosen = json.loads(line)["chosen"]
        j = chosen.rfind(MARK)
        if j < 0:
            continue
        text = chosen[j + len(MARK):].strip()
        if not text:
            continue
        print("\t".join([str(i), esc(text)]))
        k += 1
        if k == n:
            break
