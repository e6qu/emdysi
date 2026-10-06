#!/usr/bin/env python3
"""Check the provenance registry (VENDORED.toml) and content manifest
(VENDORED.sha256).

- Every entry has all required fields, its license files and source doc
  exist, and its paths exist. Entries with `ai_generated = true`
  (machine-generated text) must also name their `generator`.
- Every file under grammar/, corpora/ and data/ is covered by exactly one
  entry (files compiled into crates are listed explicitly).
- Every covered file is in the manifest with a matching SHA-256, and the
  manifest lists nothing else.
- docs/vendored.md and THIRD_PARTY_NOTICES.md, which are generated from
  the registry, are up to date.

Usage: python3 scripts/check-vendored.py [--update]

--update rewrites VENDORED.sha256, docs/vendored.md and
THIRD_PARTY_NOTICES.md.
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


LICENSE_NAMES = {
    "MIT": "MIT License",
    "Apache-2.0": "Apache License 2.0",
    "CC-BY-4.0": "Creative Commons Attribution 4.0",
    "CC-BY-SA-3.0": "Creative Commons Attribution-ShareAlike 3.0",
    "CC0-1.0": "CC0 1.0 (public domain dedication)",
    "LicenseRef-PublicDomain": "Public domain",
    "LicenseRef-SCOWL": "SCOWL/ESDB permissive notice",
}


def third_party(item):
    return not item["upstream"].startswith("this repository")


def cell(text):
    return str(text).replace("|", "\\|").replace("\n", " ")


def links(paths):
    return ", ".join(f"[`{p}`](../{p})" for p in paths)


def vendored_md(reg):
    items = reg["item"]
    out = [
        "# Vendored resources and their licenses",
        "",
        "<!-- Generated from VENDORED.toml by scripts/check-vendored.py",
        "     --update; do not edit by hand. -->",
        "",
        "Everything in this repository that is not emdysi's own Rust code is",
        "listed here: the grammar, word lists, rule data, model weights and",
        "the corpora used to test and evaluate the tool. The registry",
        "[`VENDORED.toml`](../VENDORED.toml) is the source of this page;",
        "[`VENDORED.sha256`](../VENDORED.sha256) pins the content of every",
        "file, and `python3 scripts/check-vendored.py` (run in CI) verifies",
        "both, that every file under `grammar/`, `corpora/` and `data/` belongs",
        "to exactly one entry, and that this page is current.",
        "",
        "Policy ([`docs/dependencies.md`](dependencies.md)): only licenses",
        "compatible with emdysi's MIT license are allowed (MIT, Apache-2.0,",
        "BSD, ISC, CC0, CC BY, public domain, the SCOWL notice). Share-alike",
        "material (CC BY-SA) is allowed only as test data in its own",
        "directory and never shapes what the tool ships. Non-commercial,",
        "no-derivatives, LDC and unclear licenses are not allowed. Material",
        "marked *shipped* is compiled into or loaded by `en`; material",
        "marked *test* is used only by tests and evaluations. Rust crate",
        "dependencies are covered separately, by `cargo deny` and",
        "[`docs/dependencies.md`](dependencies.md).",
        "",
        "## Summary",
        "",
        "| Resource | License | Use | Paths |",
        "|---|---|---|---|",
    ]
    for it in items:
        lic = it["license"]
        ai = " (machine-generated text)" if it.get("ai_generated") else ""
        out.append(
            f"| [{cell(it['name'])}](#{it['id']}){ai} | {lic} | {it['use']} | {links(it['paths'])} |"
        )
    counts = {}
    for it in items:
        counts[it["license"]] = counts.get(it["license"], 0) + 1
    out += [
        "",
        "Licenses: "
        + ", ".join(f"{LICENSE_NAMES.get(k, k)} ({v})" for k, v in sorted(counts.items(), key=lambda kv: -kv[1]))
        + ".",
        "",
        "## Details",
    ]
    fields = [
        ("upstream", "Upstream"),
        ("version", "Version"),
        ("retrieved", "Retrieved"),
        ("license", "License"),
        ("license_files", "License text"),
        ("copyright", "Copyright"),
        ("provenance", "Provenance"),
        ("generator", "Generated by"),
        ("modifications", "Modifications"),
        ("use", "Use"),
        ("source_doc", "Details"),
    ]
    for it in items:
        out += ["", f'<a id="{it["id"]}"></a>', "", f"### {it['name']}", ""]
        out.append(f"- **Paths:** {links(it['paths'])}")
        for k, label in fields:
            if k not in it:
                continue
            v = it[k]
            if k == "license_files":
                v = links(v)
            elif k == "source_doc":
                v = f"[`{v}`](../{v})"
            elif k == "license":
                v = f"{v} ({LICENSE_NAMES.get(v, v)})"
            elif k == "upstream" and str(v).startswith("http"):
                v = f"<{v.split(' ')[0]}>" + (" " + v.split(" ", 1)[1] if " " in v else "")
            out.append(f"- **{label}:** {v}")
    return "\n".join(out) + "\n"


def notices_md(reg):
    out = [
        "# Third-party notices",
        "",
        "<!-- Generated from VENDORED.toml by scripts/check-vendored.py",
        "     --update; do not edit by hand. -->",
        "",
        "emdysi is MIT-licensed ([`LICENSE`](LICENSE)). It includes the",
        "following third-party material, each under its own license, whose",
        "full text is in the file named. Details, versions and changes are in",
        "[`docs/vendored.md`](docs/vendored.md).",
    ]
    for it in reg["item"]:
        if not third_party(it):
            continue
        lic = it["license"]
        files = ", ".join(f"[`{p}`]({p})" for p in it["license_files"])
        out += [
            "",
            f"## {it['name']}",
            "",
            f"- Location: " + ", ".join(f"`{p}`" for p in it["paths"]),
            f"- Copyright: {it['copyright']}",
            f"- License: {LICENSE_NAMES.get(lic, lic)} (`{lic}`); full text in {files}",
            f"- Used for: {'the tool (shipped)' if it['use'] == 'shipped' else 'tests and evaluation only'}",
        ]
    return "\n".join(out) + "\n"


GENERATED = [("docs/vendored.md", vendored_md), ("THIRD_PARTY_NOTICES.md", notices_md)]


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
        ai = item.get("ai_generated", False)
        if not isinstance(ai, bool):
            errors.append(f"{iid}: `ai_generated` must be true or false")
        gen = item.get("generator")
        if gen is not None and (not isinstance(gen, str) or not gen.strip()):
            errors.append(f"{iid}: `generator` must be a non-empty string")
        if ai is True and gen is None:
            errors.append(f"{iid}: `ai_generated = true` requires `generator`")
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
    for rel, render in GENERATED:
        text = render(reg)
        path = os.path.join(ROOT, rel)
        if update:
            with open(path, "w") as f:
                f.write(text)
            print(f"wrote {rel}")
        elif not os.path.exists(path) or open(path).read() != text:
            errors.append(f"{rel}: out of date; run with --update")
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
