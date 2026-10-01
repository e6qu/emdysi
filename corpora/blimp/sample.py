#!/usr/bin/env python3
"""Build sample.tsv from a BLiMP checkout: the first N minimal pairs (by
pairID) of each of the 67 paradigms.

Usage: python3 sample.py <blimp checkout>/data [N=30] > sample.tsv
"""
import json
import os
import sys

data = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 30
print("# uid\tfield\tphenomenon\tgood\tbad; first %d pairs per paradigm, see SOURCE.md" % n)
for name in sorted(os.listdir(data)):
    if not name.endswith(".jsonl"):
        continue
    rows = [json.loads(l) for l in open(os.path.join(data, name), encoding="utf-8") if l.strip()]
    rows.sort(key=lambda r: int(r["pairID"]))
    for r in rows[:n]:
        cells = [r["UID"], r["field"], r["linguistics_term"], r["sentence_good"], r["sentence_bad"]]
        print("\t".join(c.replace("\t", " ").strip() for c in cells))
