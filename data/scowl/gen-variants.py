#!/usr/bin/env python3
"""Generate the US/GB spelling-variant table from an ESDB (SCOWL) database.

Usage: gen-variants.py scowl.db en-60.tsv > variants.tsv

A word is marked `us` when ESDB lists it as a standard American spelling
and as a standard British one in none of its uses; `gb` the other way
round. Standard means variant level 4 ("common") or below: a common
variant is still standard usage (British "judgment"). Among British spellings, `ise` and `ize`
mark words standard in only one of the -ise (B) and -ize (Z) lists. Each
marked word is paired with the corresponding spelling of the other kind
from the same ESDB group. Only words in the bundled size-60 list are kept.
"""
import sqlite3
import sys
from collections import defaultdict

db, wl = sys.argv[1], sys.argv[2]
known = set()
for line in open(wl, encoding="utf-8"):
    if not line.startswith("#"):
        known.add(line.split("\t")[0])

c = sqlite3.connect(db)
STD = 4  # variant levels 0-4: preferred, included, equal, disagreement, common
info = defaultdict(dict)  # lemma_id -> spelling -> level
for lemma, sp, lvl in c.execute("select lemma_id, spelling, variant_level from lemma_variant_info"):
    info[lemma][sp] = lvl
derived = defaultdict(dict)
for wid, sp, lvl in c.execute("select word_id, spelling, variant_level from derived_variant_info"):
    derived[wid][sp] = lvl

def ok(levels, sp):
    # No entry for a spelling means the word is not region-specific.
    if not levels:
        return True
    return levels.get(sp, 9) <= STD

flags = defaultdict(lambda: {"A": False, "B": False, "Z": False})
slots = defaultdict(list)  # (group, pos) -> [(word, A, B, Z)]
for wid, gid, lemma, pos, word in c.execute("select word_id, group_id, lemma_id, pos, word from words"):
    if word not in known or not word.isalpha() or not word.islower():
        continue
    levels = derived.get(wid) or info.get(lemma, {})
    a, b, z = ok(levels, "A"), ok(levels, "B"), ok(levels, "Z")
    f = flags[word]
    f["A"] |= a
    f["B"] |= b
    f["Z"] |= z
    slots[(gid, pos)].append((word, a, b, z))

def marks(w):
    f = flags[w]
    gb = f["B"] or f["Z"]
    out = set()
    if f["A"] and not gb:
        out.add("us")
    if gb and not f["A"]:
        out.add("gb")
    if f["B"] and not f["Z"]:
        out.add("ise")
    if f["Z"] and not f["B"]:
        out.add("ize")
    return out

def fits(m, x, a, b, z):
    """Whether `x` (standard in American, -ise, -ize spelling per a, b, z
    in this group) is a counterpart of a word marked `m`."""
    if m == "us":
        return b or z
    if m == "gb":
        return a
    if m == "ise":
        return z and not b
    return b and not z

table = {}
for (gid, pos), words in sorted(slots.items()):
    for w, *_ in words:
        for m in sorted(marks(w)):
            if (w, m) in table:
                continue
            # A British spelling pairs with a standard American one (which
            # may also be standard in Oxford spelling, e.g. organise and
            # organize), and the other way round.
            other = [x for x, a, b, z in words if x != w and fits(m, x, a, b, z)]
            if other:
                table[(w, m)] = other[0]

print("# word<TAB>kind (us, gb, ise, ize)<TAB>counterpart; generated from ESDB, see SOURCE.md")
for (w, m), o in sorted(table.items()):
    print(f"{w}\t{m}\t{o}")
