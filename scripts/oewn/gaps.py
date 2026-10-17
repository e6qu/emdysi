#!/usr/bin/env python3
"""Lexical entries for the grammar's gaps that Open English WordNet confirms.

Usage: gaps.py CULPRITS_OUTPUT... > entries.tdl

Reads the output of `culprits` (crates/emdysi-parse/examples/culprits.rs):
one line per sentence without a strict reading, the words whose
replacement gives one, each with the class of its stand-in
(`provision:verb`), then a tab and the sentence. For each such word whose
lemma the ERG's lexicon (grammar/erg/lexicon.tdl and grammar/emdysi/*.tdl)
lacks in that class, and which data/oewn/words.tsv has in it, writes an
entry in the ERG's plainest type for the class, with the sentences that
needed it as comments. Writes what it skips, and why, to standard error.

Classes and types: noun, n_-_mc_le (count or mass, as the stand-in
*water* is mass and *things* count); verb, v_np_le (transitive: WordNet
gives a frame with an object, vtai/vtaa/vtii/vtia...); verb-intr, v_-_le
(a frame without one, via/vii); adjective, aj_-_i_le; adverb,
av_-_i-vp_le. Names and numbers are left alone.
"""
import collections
import glob
import os
import re
import sys

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")

TYPES = {
    "noun": ("n_-_mc_le", "n"),
    "verb": ("v_np_le", "v"),
    "verb-intr": ("v_-_le", "v"),
    "adjective": ("aj_-_i_le", "a"),
    "adverb": ("av_-_i-vp_le", "a"),
}


def erg_lexicon():
    """Single-word lemma -> set of the classes of its entries."""
    classes = collections.defaultdict(set)
    files = [os.path.join(ROOT, "grammar/erg/lexicon.tdl")] + sorted(
        glob.glob(os.path.join(ROOT, "grammar/emdysi/*.tdl"))
    )
    entry = re.compile(r'^\S+ := (\S+) &\s*\[\s*ORTH < "([^"]+)" >', re.M)
    for f in files:
        with open(f, encoding="utf-8") as fh:
            for m in entry.finditer(fh.read()):
                t, orth = m.group(1), m.group(2).lower()
                if t.startswith("v_np") or t.startswith("v_pp*-cp") or t.startswith("v_np-"):
                    classes[orth].add("verb")
                if t.startswith("v_-_") or t.startswith("v_pp_"):
                    classes[orth].add("verb-intr")
                if t.startswith("v_"):
                    classes[orth].add("any-verb")
                if t.startswith("n_"):
                    classes[orth].add("noun")
                if t.startswith("aj_"):
                    classes[orth].add("adjective")
                if t.startswith("av_"):
                    classes[orth].add("adverb")
    return classes


def oewn():
    """Lemma -> (parts of speech, verb frames); form -> lemma."""
    words, forms = {}, {}
    with open(os.path.join(ROOT, "data/oewn/words.tsv"), encoding="utf-8") as fh:
        for line in fh:
            if line.startswith("#"):
                continue
            lemma, pos, frames, irregular = line.rstrip("\n").split("\t")
            words[lemma] = (set(pos.split(",")), set(filter(None, frames.split(","))))
            for f in filter(None, irregular.split(",")):
                forms.setdefault(f, lemma)
    return words, forms


def lemmas(word, words, forms):
    """Candidate lemmas of a word form, most likely first."""
    w = word.lower()
    out = [w]
    if w in forms:
        out.append(forms[w])
    for suffix, repl in [
        ("ies", "y"), ("es", ""), ("s", ""), ("ied", "y"), ("ed", ""), ("ed", "e"),
        ("ing", ""), ("ing", "e"), ("er", ""), ("est", ""), ("ly", ""),
    ]:
        if w.endswith(suffix) and len(w) > len(suffix) + 2:
            stem = w[: -len(suffix)] + repl
            out.append(stem)
            # Doubled final consonant: "stopped", "running".
            if repl == "" and len(stem) > 3 and stem[-1] == stem[-2]:
                out.append(stem[:-1])
    return [x for x in dict.fromkeys(out) if x in words]


def confirmed(cls, pos, frames):
    if cls == "noun":
        return "n" in pos
    if cls == "verb":
        return "v" in pos and any(f.startswith("vt") or f == "ditransitive" for f in frames)
    if cls == "verb-intr":
        return "v" in pos and any(f.startswith("vi") for f in frames)
    if cls == "adjective":
        return "a" in pos
    if cls == "adverb":
        return "r" in pos
    return False


def onset(lemma):
    """PHON.ONSET for |a|/|an|: voc for a vowel sound, con otherwise."""
    if re.match(r"(?:uni|use|usu|uti|ute|ura|ure|uro|eu|one|once|ewe)", lemma):
        return "con"
    if re.match(r"(?:hour|honest|honor|honour|heir)", lemma):
        return "voc"
    return "voc" if lemma[0] in "aeiou" else "con"


def main(paths):
    erg = erg_lexicon()
    words, forms = oewn()
    needed = collections.defaultdict(list)
    for path in paths:
        with open(path, encoding="utf-8") as fh:
            for line in fh:
                if "\t" not in line:
                    continue
                found, sentence = line.rstrip("\n").split("\t", 1)
                for item in filter(None, (x.strip() for x in found.split(","))):
                    if ":" not in item:
                        continue
                    word, cls = item.rsplit(":", 1)
                    if cls in TYPES:
                        needed[(word, cls)].append(sentence)
    entries = {}
    for (word, cls), sentences in sorted(needed.items()):
        cands = lemmas(word, words, forms)
        lemma = next((l for l in cands if confirmed(cls, *words[l])), None)
        if lemma is None:
            print(f"skip {word}:{cls}: not in WordNet as a {cls}", file=sys.stderr)
            continue
        if cls in erg.get(lemma, set()):
            print(f"skip {word}:{cls}: the ERG has {lemma} as a {cls}", file=sys.stderr)
            continue
        key = (lemma, cls)
        entries.setdefault(key, []).extend(sentences)
    print(";;; Generated by scripts/oewn/gaps.py: entries for words the ERG lacks in")
    print(";;; a class that sentences of the development corpora need (found by")
    print(";;; culprits) and Open English WordNet confirms (data/oewn/words.tsv).")
    print()
    print(":begin :instance :status lex-entry.")
    for (lemma, cls), sentences in sorted(entries.items()):
        t, tag = TYPES[cls]
        name = f"{lemma}_{tag}_wn" + ("_intr" if cls == "verb-intr" else "")
        print()
        for s in sorted(set(sentences))[:2]:
            print(f";; {s}")
        print(f"{name} := {t} &")
        print(f'  [ ORTH < "{lemma}" >,')
        print(f'    SYNSEM [ LKEYS.KEYREL.PRED "_{lemma}_{tag}_1_rel",')
        print(f"             PHON.ONSET {onset(lemma)} ] ].")
    print()
    print(":end :instance.")
    print(f"{len(entries)} entries", file=sys.stderr)


if __name__ == "__main__":
    main(sys.argv[1:])
