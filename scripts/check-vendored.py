#!/usr/bin/env python3
"""Check the provenance registry (VENDORED.toml) and content manifest
(VENDORED.sha256).

- Every entry has all required fields, its license files and source doc
  exist, and its paths exist.
- Every file under grammar/, corpora/ and data/ is covered by exactly one
  entry (files compiled into crates are listed explicitly).
- Every covered file is in the manifest with a matching SHA-256, and the
  manifest lists nothing else.

Usage: python3 scripts/check-vendored.py [--update]
"""
import hashlib
import os
import sys
import tomllib

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
GUARDED = ["grammar", "corpora", "data"]
REQUIRED = [
    "id", "paths", "name", "upstream", "version", "retrieved", "license",
    "license_files", "copyright", "provenance", "modifications", "use",
    "source_doc",
]
ALLOWED_LICENSES = {
    "MIT", "Apache-2.0", "BSD-2-Clause", "BSD-3-Clause", "ISC", "CC0-1.0",
    "CC-BY-3.0", "CC-BY-4.0", "Unlicense", "LicenseRef-SCOWL",
    "LicenseRef-PublicDomain",
}
# Allowed only in their own directory, never used to train shipped data.
ISOLATED_LICENSES = {"CC-BY-SA-3.0", "CC-BY-SA-4.0"}


def files_under(rel):
    full = os.path.join(ROOT, rel)
    if os.path.isfile(full):
        return [rel]
    out = []
    for d, _, fs in os.walk(full):
        for f in fs:
            out.append(os.path.relpath(os.path.join(d, f), ROOT))
    return out


def sha256(rel):
    h = hashlib.sha256()
    with open(os.path.join(ROOT, rel), "rb") as f:
        for chunk in iter(lambda: f.read(1 << 20), b""):
            h.update(chunk)
    return h.hexdigest()


def main():
    update = "--update" in sys.argv[1:]
    errors = []
    with open(os.path.join(ROOT, "VENDORED.toml"), "rb") as f:
        reg = tomllib.load(f)
    owner = {}
    ids = set()
    for item in reg.get("item", []):
        iid = item.get("id", "?")
        for k in REQUIRED:
            v = item.get(k)
            if v is None or v == "" or v == []:
                errors.append(f"{iid}: missing field `{k}`")
        if iid in ids:
            errors.append(f"{iid}: duplicate id")
        ids.add(iid)
        lic = item.get("license", "")
        if lic not in ALLOWED_LICENSES | ISOLATED_LICENSES:
            errors.append(f"{iid}: license {lic!r} is not on the allowlist")
        if lic in ISOLATED_LICENSES and item.get("use") != "test":
            errors.append(f"{iid}: {lic} material may only be used for tests")
        if item.get("use") not in ("shipped", "test"):
            errors.append(f"{iid}: `use` must be shipped or test")
        for p in item.get("license_files", []) + [item.get("source_doc", "")]:
            if p and not os.path.exists(os.path.join(ROOT, p)):
                errors.append(f"{iid}: {p} does not exist")
        for p in item.get("paths", []):
            if not os.path.exists(os.path.join(ROOT, p)):
                errors.append(f"{iid}: path {p} does not exist")
                continue
            for f in files_under(p):
                if f in owner:
                    errors.append(f"{f}: covered by both {owner[f]} and {iid}")
                owner[f] = iid
    for g in GUARDED:
        for f in files_under(g):
            if f not in owner:
                errors.append(f"{f}: not covered by any VENDORED.toml entry")
    manifest_path = os.path.join(ROOT, "VENDORED.sha256")
    want = {f: sha256(f) for f in sorted(owner)}
    if update:
        with open(manifest_path, "w") as m:
            for f, h in want.items():
                m.write(f"{h}  {f}\n")
        print(f"wrote {len(want)} entries to VENDORED.sha256")
    else:
        have = {}
        if os.path.exists(manifest_path):
            for line in open(manifest_path):
                h, _, f = line.rstrip("\n").partition("  ")
                have[f] = h
        for f, h in want.items():
            if f not in have:
                errors.append(f"{f}: not in VENDORED.sha256")
            elif have[f] != h:
                errors.append(f"{f}: content differs from VENDORED.sha256")
        for f in have:
            if f not in want:
                errors.append(f"{f}: in VENDORED.sha256 but not covered or missing")
    if errors:
        for e in errors:
            print(e, file=sys.stderr)
        print(f"{len(errors)} problem(s); see VENDORED.toml", file=sys.stderr)
        return 1
    if not update:
        print(f"ok: {len(reg['item'])} entries, {len(want)} files")
    return 0


if __name__ == "__main__":
    sys.exit(main())
