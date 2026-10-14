#!/usr/bin/env python3
"""Questions with known answers for calibrating and measuring the decision
model (docs/models.md, docs/plan.md M20), from the real-error corpus.

Each error in corpora/real-errors/*.tsv becomes one question in the format
of `en decide --eval`: the sentence as written and as the writer corrected
it, in random order, with the corrected one as the answer. The paragraph's
other sentences are not given, so the model decides from the sentence, as
it does in `core.grammar-fix` and `core.spelling`.

    python3 scripts/model-eval/questions.py > questions.tsv
    en decide --lm SPEC --eval questions.tsv

With an argument 0 or 1, only every other question (the first or the
second half): fit the temperature on one, measure on the other.

Writes TSV to standard output: question, context (empty), options
separated by '|', index of the right option.
"""

import glob
import os
import random
import re
import sys

QUESTION = "Which version of the sentence is correct English, as the writer meant it?"
ROOT = os.path.join(os.path.dirname(__file__), "..", "..")


def unescape(s):
    table = {"n": "\n", "t": "\t", "r": "\r"}
    return re.sub(r"\\(.)", lambda m: table.get(m.group(1), m.group(1)), s)


def sentences(text):
    return [s.strip() for s in re.split(r"(?<=[.!?])\s+|\n+", text) if s.strip()]


def changed(before, after):
    """The one sentence that differs between two versions of a paragraph."""
    a, b = sentences(before), sentences(after)
    if len(a) != len(b):
        return None
    diff = [(x, y) for x, y in zip(a, b) if x != y]
    return diff[0] if len(diff) == 1 else None


def main():
    half = int(sys.argv[1]) if len(sys.argv) > 1 else None
    rng = random.Random(7)
    n = 0
    k = 0
    for path in sorted(glob.glob(os.path.join(ROOT, "corpora/real-errors/*.tsv"))):
        for line in open(path, encoding="utf-8"):
            if line.startswith("#"):
                continue
            cells = line.rstrip("\n").split("\t")
            if len(cells) < 6:
                continue
            pair = changed(unescape(cells[4]), unescape(cells[5]))
            if not pair or any("|" in s or "\t" in s for s in pair):
                continue
            wrong, right = pair
            options = [wrong, right]
            rng.shuffle(options)
            k += 1
            if half is not None and k % 2 != half:
                continue
            print(f"{QUESTION}\t\t{'|'.join(options)}\t{options.index(right)}")
            n += 1
    print(f"{n} questions", file=sys.stderr)


if __name__ == "__main__":
    main()
