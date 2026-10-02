#!/usr/bin/env python3
"""Build sample.tsv from a CHEAT checkout: the first N records (in file
order) of data/ieee-chatgpt-generation.jsonl, abstracts written by ChatGPT
from an IEEE paper's title and keywords.

Columns kept: id, abstract (the generated text). Dropped: title and
keyword, which are taken from the IEEE papers. The init, polish and fusion
files (IEEE abstracts or derivatives of them) are not used.

Cells are escaped: backslash -> \\\\, tab -> \\t, newline -> \\n, CR -> \\r.

Usage: python3 sample.py <CHEAT checkout>/data/ieee-chatgpt-generation.jsonl [N=200] > sample.tsv
"""
import json
import sys


def esc(s):
    return (s.replace("\\", "\\\\").replace("\t", "\\t")
            .replace("\r", "\\r").replace("\n", "\\n"))


path = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 200
print("# id\tabstract; first %d records of ieee-chatgpt-generation.jsonl; "
      "escaped \\\\ \\t \\n \\r; see SOURCE.md" % n)
k = 0
for line in open(path, encoding="utf-8"):
    if not line.strip():
        continue
    r = json.loads(line)
    print("\t".join(esc(c) for c in [str(r["id"]), r["abstract"].strip()]))
    k += 1
    if k == n:
        break
