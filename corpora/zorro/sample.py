#!/usr/bin/env python3
"""Build sample.tsv from a Zorro checkout (sentences/babyberta/*.txt, where
odd-numbered lines are unacceptable and even-numbered lines acceptable):
the first N pairs of each paradigm, in the BLiMP sample's column layout.

Usage: python3 sample.py <Zorro checkout>/sentences/babyberta [N=30] > sample.tsv
"""
import os
import sys

src = sys.argv[1]
n = int(sys.argv[2]) if len(sys.argv) > 2 else 30
print("# uid\tfield\tphenomenon\tgood\tbad; first %d pairs per paradigm, see SOURCE.md" % n)
for name in sorted(os.listdir(src)):
    if not name.endswith(".txt"):
        continue
    lines = [l.strip() for l in open(os.path.join(src, name), encoding="utf-8") if l.strip()]
    uid = name[:-4]
    phenomenon = uid.split("-")[0]
    for bad, good in list(zip(lines[0::2], lines[1::2]))[:n]:
        print("\t".join([uid, "zorro", phenomenon, good, bad]))
