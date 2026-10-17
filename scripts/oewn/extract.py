#!/usr/bin/env python3
"""Extract a compact word list from Open English WordNet's YAML source.

Usage: extract.py OEWN_REPO/src/yaml > data/oewn/words.tsv

One line per single-word, lower-case lemma: the lemma, its parts of speech
(n noun, v verb, a adjective, r adverb), the verb frames of its senses
(frames.yaml names them: vtai "Somebody ----s something", via "Somebody
----s", ...) and its irregular forms. Multiword lemmas and names are left
out: the grammar's lexicon has its own multiword entries, and capitalized
words are read as names.
"""
import glob
import os
import sys

import yaml

POS = {"n": "n", "v": "v", "a": "a", "s": "a", "r": "r"}


def main(src):
    words = {}
    for path in sorted(glob.glob(os.path.join(src, "entries-*.yaml"))):
        with open(path, encoding="utf-8") as f:
            entries = yaml.safe_load(f)
        for lemma, by_pos in entries.items():
            lemma = str(lemma)
            if not lemma.isalpha() or not lemma.islower():
                continue
            w = words.setdefault(lemma, {"pos": set(), "frames": set(), "forms": set()})
            for pos, entry in by_pos.items():
                if pos not in POS:
                    continue
                w["pos"].add(POS[pos])
                for form in entry.get("form", []) or []:
                    w["forms"].add(str(form))
                for sense in entry.get("sense", []) or []:
                    for frame in sense.get("subcat", []) or []:
                        w["frames"].add(str(frame))
    print("# lemma\tparts of speech\tverb frames\tirregular forms")
    for lemma in sorted(words):
        w = words[lemma]
        print(
            "\t".join(
                [
                    lemma,
                    ",".join(p for p in "nvar" if p in w["pos"]),
                    ",".join(sorted(w["frames"])),
                    ",".join(sorted(w["forms"])),
                ]
            )
        )


if __name__ == "__main__":
    main(sys.argv[1])
