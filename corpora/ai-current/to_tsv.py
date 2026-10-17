#!/usr/bin/env python3
"""Write samples.tsv from the .md documents beside this script: one row per
document (id, genre, text), cells escaped as rule_rates reads them."""
import os

here = os.path.dirname(os.path.abspath(__file__))


def esc(t):
    return t.replace("\\", "\\\\").replace("\t", "\\t").replace("\r", "\\r").replace("\n", "\\n")


rows = ["# id\tgenre\ttext"]
for name in sorted(os.listdir(here)):
    if name.endswith(".md") and name != "README.md":
        with open(os.path.join(here, name), encoding="utf-8") as f:
            rows.append("\t".join([name[:-3], name.split("-")[0], esc(f.read())]))
with open(os.path.join(here, "samples.tsv"), "w", encoding="utf-8") as f:
    f.write("\n".join(rows) + "\n")
