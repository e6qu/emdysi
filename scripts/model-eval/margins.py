#!/usr/bin/env python3
"""Ways of choosing among versions of a sentence, compared without the
model: from the scores `examples/versions` keeps (SCORES.tsv: the right
version, 0 when the original is right, then per version its
log-probability, tokens and the surprise of its words alone; the original
first).

    python3 scripts/model-eval/margins.py SCORES.tsv [MAX_CHANGED]

A version's score is its log-probability plus an adjustment; a
correction is made when the best other version beats the original by the
margin or more. For each way of scoring, the margin that corrects the
most real errors while changing at most MAX_CHANGED (default 1) of the
questions whose original is right, and what it gets. Then the choice as
`core.grammar-fix` makes it (probability 0.8 or more, above the
original's) from plain log-probabilities at several temperatures.
"""

import sys


def load(path):
    out = []
    for line in open(path):
        f = line.rstrip("\n").split("\t")
        if len(f) < 3:
            continue
        right = int(f[0])
        versions = [tuple(float(x) for x in v.split()) for v in f[1:]]
        out.append((right, versions))
    return out


def outcome(questions, score, margin):
    right = wrong = changed = 0
    for r, vs in questions:
        s = [score(lp, n, alone) for lp, n, alone in vs]
        best = max(range(1, len(s)), key=lambda i: s[i])
        if s[best] - s[0] < margin:
            continue
        if r == 0:
            changed += 1
        elif r == best:
            right += 1
        else:
            wrong += 1
    return right, wrong, changed


def softmax_outcome(questions, temperature, threshold):
    """As `core.grammar-fix` decides: the versions' plain log-probabilities
    at `temperature`, and the best correction reported when its
    probability is `threshold` or more and above the original's."""
    right = wrong = changed = 0
    for r, vs in questions:
        s = [lp / temperature for lp, _, _ in vs]
        top = max(s)
        p = [2.718281828459045 ** (x - top) for x in s]
        total = sum(p)
        p = [x / total for x in p]
        best = max(range(1, len(p)), key=lambda i: p[i])
        if p[best] < threshold or p[best] <= p[0]:
            continue
        if r == 0:
            changed += 1
        elif r == best:
            right += 1
        else:
            wrong += 1
    return right, wrong, changed


def main():
    questions = load(sys.argv[1])
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else 1
    errors = sum(1 for r, _ in questions if r > 0)
    clean = len(questions) - errors
    print(f"{len(questions)} questions: {errors} with a right correction, {clean} with the original right")
    print(f"at most {limit} originals changed\n")
    ways = {"plain": lambda lp, n, a: lp, "per token": lambda lp, n, a: lp / max(n, 1)}
    for c in (1, 2, 3, 4, 5):
        ways[f"plus {c} per token"] = lambda lp, n, a, c=c: lp + c * n
    for k in (0.25, 0.5, 0.75, 1.0):
        ways[f"plus {k} of words alone"] = lambda lp, n, a, k=k: lp + k * a
    print("| Score | Margin | Corrected right | Corrected wrong | Originals changed |")
    print("|---|---|---|---|---|")
    for name, score in ways.items():
        margins = [i / 20 for i in range(0, 61)] if name == "per token" else [i / 2 for i in range(0, 61)]
        best = None
        for m in margins:
            r, w, ch = outcome(questions, score, m)
            if ch <= limit and (best is None or r > best[1]):
                best = (m, r, w, ch)
        if best is None:
            print(f"| {name} | none | | | |")
        else:
            m, r, w, ch = best
            print(f"| {name} | {m:g} | {r} of {errors} | {w} | {ch} of {clean} |")
    print("\nAs core.grammar-fix decides, plain log-probabilities at a temperature, threshold 0.8:\n")
    print("| Temperature | Corrected right | Corrected wrong | Originals changed |")
    print("|---|---|---|---|")
    for t in (1, 2, 3, 4, 5, 6, 7, 8, 10):
        r, w, ch = softmax_outcome(questions, t, 0.8)
        print(f"| {t} | {r} of {errors} | {w} | {ch} of {clean} |")


if __name__ == "__main__":
    main()
