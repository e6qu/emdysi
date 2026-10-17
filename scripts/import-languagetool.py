#!/usr/bin/env python3
"""Generate packs/languagetool.toml from LanguageTool's English rules.

Converts the pattern rules of the vendored LanguageTool rule files
(data/languagetool/en/grammar.xml and style.xml) that need only words, not
LanguageTool's part-of-speech tagger, chunker or lemmatizer, into emdysi
`regex` rules: each token becomes a group (`t1`, `t2`, ...), the marked
tokens the group `m` that a finding covers, suggestions templates over the
groups. Antipatterns that need only words become the rule's `unless`.
LanguageTool's own `<regexp>` rules convert directly.

Every converted rule keeps LanguageTool's examples (incorrect ones as
`examples`, correct ones as `acceptable`), and a rule is kept only if the
converted pattern finds the marked span of every incorrect example and
nothing in the correct ones (checked here with the `regex` module, and
again by the test suite with emdysi's own engine).

    python3 scripts/import-languagetool.py           # write the pack
    python3 scripts/import-languagetool.py --check   # fail if out of date
    python3 scripts/import-languagetool.py --candidates FILE
        # rules with part-of-speech conditions, for lt_validate:
        # cargo run --release -p emdysi-check --example lt_validate FILE \\
        #     > scripts/languagetool-tagged.txt

Needs the `regex` module (pip install regex). The pack is registered in
VENDORED.toml; do not edit it by hand.
"""

import re
import sys
import xml.etree.ElementTree as ET
from pathlib import Path

import regex

ROOT = Path(__file__).resolve().parent.parent
SRC = ROOT / "data/languagetool/en"
OUT = ROOT / "packs/languagetool.toml"
UPSTREAM = (
    "https://github.com/languagetool-org/languagetool/blob/"
    "5b59afaf3a12489eb34f09b555cb31f1174926c7/languagetool-language-modules/"
    "en/src/main/resources/org/languagetool/rules/en/"
)
LICENSE = (
    "LanguageTool (LGPL-2.1-or-later, Copyright (C) 2001-2021 Daniel Naber, "
    "Marcin Miłkowski and the LanguageTool contributors; portions from After "
    "the Deadline, GPL-3.0-or-later). See data/languagetool/SOURCE.md."
)

# Categories left out: typography (quotes, dashes and spaces, which
# Markdown and code make unreliable), LanguageTool's April-fools rules, and
# the categories LanguageTool itself turns off by default.
SKIP_CATEGORIES = {"TYPOGRAPHY", "APRIL", "WIKIPEDIA", "CREATIVE_WRITING", "PLAIN_ENGLISH"}
# Rules left out after measuring them on edited text (docs/evaluation.md):
# `...` is mostly code or a quoted elision in technical writing.
SKIP_RULES = {"ELLIPSIS"}
WARNING_CATEGORIES = {
    "GRAMMAR",
    "CONFUSED_WORDS",
    "TYPOS",
    "NONSTANDARD_PHRASES",
    "MULTITOKEN_SPELLING",
    "COLLOCATIONS",
    "SEMANTICS",
    "CASING",
    "COMPOUNDING",
    "PROPER_NOUNS",
}
TOKEN_ATTRS = {
    "regexp",
    "case_sensitive",
    "spacebefore",
    "min",
    "max",
    "negate",
    "skip",
    "inflected",
    "postag",
    "postag_regexp",
}

# The closed-class part-of-speech tags of LanguageTool's tagger (the Penn
# Treebank's), as word lists: a token that asks for one of these needs no
# tagger. Open classes (nouns, verbs, adjectives, adverbs) are left out.
NUMBER = (
    r"\d[\d,.]*|zero|one|two|three|four|five|six|seven|eight|nine|ten|eleven|twelve|"
    r"thirteen|fourteen|fifteen|sixteen|seventeen|eighteen|nineteen|twenty|thirty|"
    r"forty|fifty|sixty|seventy|eighty|ninety|hundred|thousand|million|billion|trillion"
)
CLOSED = {
    "DT": "a an the this that these those some any no every each either neither another all both",
    "PDT": "all both half such quite rather",
    "PRP": "i me you he him she her it we us they them myself yourself himself herself itself "
    "ourselves yourselves themselves",
    "PRP$": "my your his her its our their",
    "MD": "can could may might must shall should will would ought 'll 'd",
    "CC": "and or but nor yet plus",
    "TO": "to",
    "WDT": "which that what whatever whichever",
    "WP": "who whom what whoever whomever whatever",
    "WP$": "whose",
    "WRB": "how where when why whenever wherever however",
    "EX": "there",
    "IN": "about above across after against along amid among around as at because before behind "
    "below beneath beside besides between beyond by despite down during except for from if in "
    "inside into like near of off on onto out outside over per since than that though through "
    "throughout till toward towards under underneath unless unlike until up upon via whereas "
    "whether while with within without",
}
OPEN = (
    "NN NNS NNP NNPS NN:U NN:UN JJ JJR JJS RB RBR RBS RP VB VBD VBG VBN VBP VBZ UH FW LS SYM "
    "POS PCT CD SENT_END UNKNOWN"
).split()


def closed_words(tag, is_regexp):
    """A pattern for the words of the closed-class tags `tag` names."""
    if tag == "CD" and not is_regexp:
        return NUMBER
    tags = [t for t in CLOSED if (regex.fullmatch(tag, t) if is_regexp else t == tag)]
    if not tags or any(regex.fullmatch(tag, t) if is_regexp else t == tag for t in OPEN):
        raise Skip("open-class tag")
    words = sorted({w for t in tags for w in CLOSED[t].split()}, key=lambda w: (-len(w), w))
    return "|".join(literal(w) for w in words)
EXCEPTION_ATTRS = {"regexp", "case_sensitive", "scope", "inflected", "postag", "postag_regexp"}

# Inflected forms by lemma, for LanguageTool's `inflected="yes"` tokens
# (which match any form of a lemma), from UniMorph (data/unimorph/eng).
# UniMorph lacks *be*.
FORMS = {"be": {"be", "am", "is", "are", "was", "were", "been", "being", "'s", "'re", "'m"}}
for line in (ROOT / "data/unimorph/eng").read_text().splitlines():
    lemma, form, _ = line.split("\t")
    if " " not in lemma and " " not in form:
        FORMS.setdefault(lemma.lower(), {lemma.lower()}).add(form.lower())


def inflections(text, is_regexp):
    """A pattern for every form of the lemmas `text` names (a word or a
    pattern over lemmas)."""
    if is_regexp:
        r = regex.compile(f"(?i:{text})")
        lemmas = [lem for lem in FORMS if r.fullmatch(lem)]
        if not lemmas or len(lemmas) > 40:
            raise Skip("inflected pattern too broad")
    else:
        lemmas = [text.lower()]
    forms = set()
    for lem in lemmas:
        forms |= FORMS.get(lem, {lem})
    return "|".join(literal(f) for f in sorted(forms, key=lambda f: (-len(f), f)))


class Skip(Exception):
    """A construct the converter does not handle; the rule is left out."""


def word_dots(pattern):
    """A token pattern for use in a pattern over text: `.` matches a word
    character (LanguageTool matches a pattern against one token, which has
    no spaces and no punctuation but hyphens), and anchors go."""
    out, i, in_class = [], 0, False
    while i < len(pattern):
        c = pattern[i]
        if c == "\\":
            if i + 1 < len(pattern) and pattern[i + 1].isdigit():
                raise Skip("backreference")
            out.append(pattern[i : i + 2])
            i += 2
            continue
        if in_class:
            in_class = c != "]"
        elif c == "[":
            in_class = True
            # `[]...]` and `[^]...]` start with a literal `]`.
            j = i + 1 + (pattern[i + 1 : i + 2] == "^")
            if pattern[j : j + 1] == "]":
                out.append(pattern[i : j + 1])
                i = j + 1
                continue
        elif c == ".":
            c = r"[\w-]"
        elif c == "'":
            c = "['’]"
        elif c in "^$":
            # LanguageTool anchors every token pattern; so does the caller.
            c = ""
        out.append(c)
        i += 1
    return "".join(out)


def literal(text):
    """A pattern for literal text; an apostrophe matches either kind."""
    return "".join("\\" + c if c in ".^$*+?{}[]\\|()#&-~" else c for c in text).replace("'", "['’]")


# Any one token: a word (with its hyphens) or a punctuation mark.
WORD = r"(?:(?<!\w)\w[\w-]*(?!\w)|[^\w\s])"


def text_of(el):
    return (el.text or "").strip()


def is_punct(s):
    return s != "" and not any(c.isalnum() for c in s)


# The tags emdysi derives from the parse (crates/emdysi-check/src/tags.rs).
PARSE_TAGS = (
    "CD EX WP PRP NNP NNPS NN NNS MD VBP VBZ VBD VBN VBG VB JJR JJS JJ RBR RBS WRB RB "
    "TO IN PRP$ WDT DT CC"
).split()


def tag_pattern(el):
    """A part-of-speech condition for the parse's tags, from a token's or
    exception's `postag`; Skip if no tag the parse gives can match."""
    tag = el.get("postag")
    pat = tag if el.get("postag_regexp") == "yes" else regex.escape(tag)
    try:
        r = regex.compile(pat)
    except regex.error:
        raise Skip("bad tag pattern")
    if not any(r.fullmatch(t) for t in PARSE_TAGS):
        raise Skip("tag the parse does not give")
    return pat


def token_regex(tok, conds=None):
    """A token's pattern, with its exceptions as negative lookaheads: the
    token's own, then (`scope="next"`) the next token's. With `conds`, an
    open-class part of speech becomes a condition on the token's group
    (pattern, negated), checked against the parse; without, Skip."""
    extra = set(tok.attrib) - TOKEN_ATTRS
    if extra:
        raise Skip(f"token attributes {extra}")
    if tok.get("min", "1") not in ("0", "1") or tok.get("max", "1") != "1":
        raise Skip("repeated token")
    text = text_of(tok)
    tag = tok.get("postag")
    if tag == "SENT_START" and not text and not list(tok):
        return "^", True
    if tok.get("inflected") == "yes" and text:
        body = inflections(text, tok.get("regexp") == "yes")
    elif tok.get("regexp") == "yes":
        body = word_dots(text) if text else WORD
    else:
        body = literal(text) if text else WORD
    if tag is not None:
        try:
            words = closed_words(tag, tok.get("postag_regexp") == "yes")
            body = f"(?:{words})" if not text else rf"(?=(?:{words})(?![\w'’]))(?:{body})"
        except Skip:
            if conds is None or tok.get("negate") == "yes":
                raise
            conds.append((tag_pattern(tok), False))
    own, after = [], []
    for ch in tok:
        if ch.tag != "exception":
            raise Skip(f"<{ch.tag}> in token")
        scope = ch.get("scope", "current")
        if set(ch.attrib) - EXCEPTION_ATTRS or scope not in ("current", "next"):
            raise Skip("exception attributes")
        e = text_of(ch)
        if ch.get("postag") is not None and not e:
            try:
                e = closed_words(ch.get("postag"), ch.get("postag_regexp") == "yes")
            except Skip:
                if conds is None or scope != "current" or ch.get("negate_pos"):
                    raise
                conds.append((tag_pattern(ch), True))
                continue
        elif ch.get("inflected") == "yes":
            e = inflections(e, ch.get("regexp") == "yes")
        else:
            e = word_dots(e) if ch.get("regexp") == "yes" else literal(e)
        if ch.get("case_sensitive") == "yes":
            e = f"(?-i:{e})"
        (own if scope == "current" else after).append(e)
    if after and tok.get("skip"):
        raise Skip("exception over skipped tokens")
    if tok.get("case_sensitive") == "yes":
        body = f"(?-i:{body})"
    punct = is_punct(regex.sub(r"\\.|\[\^?", "", text))
    # A token is a whole word: it starts and ends where a word does (a
    # hyphen joins words into one token).
    # A token may start with an apostrophe (*'s*), and a word may end in
    # *n't*, a token of its own.
    start, end = ("", "") if punct else (r"(?:(?=\w)(?<!\w)(?<!\w-)|(?!\w))", r"(?!(?!n['’]t)\w|-\w)")
    neg = "".join(f"(?!(?:{e}){end})" for e in own)
    if tok.get("negate") == "yes":
        # A negated token also matches the start of the sentence.
        out = rf"(?:^|(?!(?:{body}){end}){neg}{WORD})"
    else:
        out = f"{start}{neg}(?:{body}){end}"
    out += "".join(rf"(?!\s*(?:{e})(?!\w))" for e in after)
    return out, punct


def token_words(tok):
    """Words one of which any match of the token contains (lowercase), or
    `None` if there is no such short list."""
    if tok.get("min") == "0" or tok.get("negate") == "yes":
        return None
    text = text_of(tok)
    is_re = tok.get("regexp") == "yes"
    if not text:
        tag = tok.get("postag")
        if tag and tag != "SENT_START":
            try:
                text, is_re = closed_words(tag, tok.get("postag_regexp") == "yes"), True
            except Skip:
                return None
            if text == NUMBER:
                return None
        else:
            return None
    if tok.get("inflected") == "yes":
        return set(inflections(text, is_re).replace("['’]", "'").replace("\\", "").split("|"))
    if not is_re:
        return {text.lower()}
    if regex.fullmatch(r"[\w' -]+(?:\|[\w' -]+)*", text):
        return {w.lower() for w in text.split("|")}
    return None


def needs_of(tokens):
    """The word list some token of the pattern needs that sentences
    contain least often (short words are the common ones)."""
    best, cost = None, None
    for tok, _ in tokens:
        w = token_words(tok)
        if not w or not all(w):
            continue
        c = sum(50 / len(x) ** 2 for x in w)
        if best is None or c < cost:
            best, cost = w, c
    if best is None:
        return []
    return sorted(best | {x.replace("'", "’") for x in best})


def flatten(pattern):
    """The pattern's tokens in order, each with whether it is marked."""
    out = []
    marked = False
    for el in pattern:
        if el.tag == "token":
            out.append((el, False))
        elif el.tag == "marker":
            marked = True
            for t in el:
                if t.tag != "token":
                    raise Skip(f"<{t.tag}> in marker")
                out.append((t, True))
        elif el.tag != "example":
            raise Skip(f"<{el.tag}> in pattern")
    if not marked:
        out = [(t, True) for t, _ in out]
    return out


def sequence(tokens, groups=True, conds=None):
    """A regex for a token sequence; marked tokens form the group `m`.
    Every word token carries its own word boundaries, so the tokens may be
    separated by any space or none (LanguageTool splits *I've* into *I* and
    *'ve*, and punctuation from words)."""
    parts = []
    in_m = False
    for i, (tok, marked) in enumerate(tokens, 1):
        local = [] if groups and conds is not None else None
        body, _ = token_regex(tok, local)
        for pat, negate in local or []:
            conds.append((f"t{i}={pat}", negate))
        sep = "" if i == 1 or tok.get("spacebefore") == "no" else r"\s*"
        if tok.get("spacebefore") == "yes" and i > 1:
            sep = r"\s+"
        piece = f"(?P<t{i}>{body})" if groups else f"(?:{body})"
        if marked and not in_m:
            parts.append(sep + ("(?P<m>" if groups else "(?:"))
            sep = ""
            in_m = True
        elif not marked and in_m:
            parts.append(")")
            in_m = False
        piece = sep + piece
        if tok.get("min") == "0":
            piece = f"(?:{piece})?"
        parts.append(piece)
        skip = int(tok.get("skip", "0"))
        if skip and i < len(tokens):
            n = "" if skip < 0 else str(skip)
            parts.append(rf"(?:\s*{WORD}){{0,{n}}}?" if n else rf"(?:\s*{WORD})*?")
    if in_m:
        parts.append(")")
    return "".join(parts)


def template(el, optional, regexp_rule=False):
    """A suggestion or message as an expansion template; `None` when it
    has nothing but text."""
    out = [(el.text or "").replace("$", "$$")]
    for ch in el:
        if ch.tag != "match" or set(ch.attrib) - {"no"}:
            raise Skip(f"<{ch.tag} {dict(ch.attrib)}> in suggestion")
        n = int(ch.get("no"))
        if n in optional:
            raise Skip("suggestion uses an optional token")
        out.append(f"${{{n}}}" if regexp_rule else f"${{t{n}}}")
        out.append((ch.tail or "").replace("$", "$$"))
    s = "".join(out)
    if regexp_rule:
        s = re.sub(r"\\(\d)", r"${\1}", s)
    else:
        s = re.sub(r"\\(\d)", lambda m: f"${{t{m.group(1)}}}", s)
    return " ".join(s.split())


def message(el, optional, regexp_rule=False):
    """The message as a template with each suggestion quoted, and the
    suggestions."""
    esc = lambda t: (t or "").replace("$", "$$")
    parts = [esc(el.text)]
    suggestions = []
    for ch in el:
        if ch.tag == "suggestion":
            t = template(ch, optional, regexp_rule)
            suggestions.append(t)
            parts.append(f"'{t}'")
        elif ch.tag == "match" and set(ch.attrib) == {"no"}:
            n = int(ch.get("no"))
            parts.append(f"${{{n}}}" if regexp_rule else f"${{t{n}}}")
        else:
            raise Skip(f"<{ch.tag}> in message")
        parts.append(esc(ch.tail))
    text = "".join(parts)
    ref = (lambda m: f"${{{m.group(1)}}}") if regexp_rule else (lambda m: f"${{t{m.group(1)}}}")
    text = " ".join(re.sub(r"\\(\d)", ref, text).split())
    if "${" not in text:
        text = text.replace("$$", "$")
    return text, suggestions


def example(el):
    """An example's text and the span of its marker."""
    parts, span = [el.text or ""], None
    for ch in el:
        if ch.tag != "marker":
            raise Skip("example markup")
        start = len("".join(parts))
        parts.append(ch.text or "")
        span = (start, len("".join(parts)))
        parts.append(ch.tail or "")
    return "".join(parts), span


def convert(rule, group, category, file):
    """One LanguageTool rule as an emdysi rule (a dict), or raise Skip."""
    if (group if group is not None else rule).get("id") in SKIP_RULES:
        raise Skip("left out")
    for el in (rule, group):
        # Picky rules run only in LanguageTool's picky mode.
        if el is not None and "picky" in el.get("tags", "").split():
            raise Skip("picky")
        if el is not None and el.get("default") in ("off", "temp_off"):
            raise Skip("off by default")
    if rule.find("filter") is not None or rule.find("disambig") is not None:
        raise Skip("filter")
    pattern = rule.find("pattern")
    rx = rule.find("regexp")
    case = pattern is not None and pattern.get("case_sensitive") == "yes"
    if pattern is not None:
        if set(pattern.attrib) - {"case_sensitive"}:
            raise Skip("pattern attributes")
        tokens = flatten(pattern)
        optional = {i for i, (t, _) in enumerate(tokens, 1) if t.get("min") == "0"}
        conds = []
        body = sequence(tokens, conds=conds)
        needs = needs_of(tokens)
        regexp_rule = False
    elif rx is not None:
        if set(rx.attrib) - {"case_sensitive"}:
            raise Skip("regexp attributes")
        case = rx.get("case_sensitive") == "yes"
        body = text_of(rx)
        optional = set()
        needs = []
        conds = []
        regexp_rule = True
    else:
        raise Skip("no pattern")
    pat = ("" if case else "(?i)") + body
    unless = []
    for anti in rule.findall("antipattern") + (group.findall("antipattern") if group is not None else []):
        if set(anti.attrib) - {"case_sensitive"}:
            raise Skip("antipattern attributes")
        u = sequence([(t, True) for t, _ in flatten(anti)], groups=False)
        unless.append(f"(?-i:{u})" if anti.get("case_sensitive") == "yes" else u)
    msg_el = rule.find("message")
    if msg_el is None and group is not None:
        msg_el = group.find("message")
    if msg_el is None:
        raise Skip("no message")
    msg, suggestions = message(msg_el, optional, regexp_rule)
    for s in rule.findall("suggestion"):
        suggestions.append(template(s, optional, regexp_rule))
    examples, acceptable = [], []
    for ex in rule.findall("example"):
        text, span = example(ex)
        if ex.get("correction") is not None or ex.get("type") == "incorrect":
            if span is None:
                raise Skip("incorrect example without a marker")
            examples.append((text, span))
        else:
            acceptable.append(text)
    if not examples:
        raise Skip("no incorrect example")
    rid = (group.get("id") if group is not None else rule.get("id")) or ""
    url = rule.findtext("url") or (group.findtext("url") if group is not None else None)
    return {
        "lt_id": rid,
        "name": (group.get("name") if group is not None else rule.get("name")) or rid,
        "category": category,
        "severity": "warning" if category in WARNING_CATEGORIES else "suggestion",
        "message": msg,
        "pattern": pat,
        "needs": needs,
        "tags": [c for c, neg in conds if not neg],
        "not_tags": [c for c, neg in conds if neg],
        "unless": ("" if case else "(?i)") + "|".join(f"(?:{u})" for u in unless) if unless else None,
        "suggest": suggestions,
        "examples": examples,
        "acceptable": acceptable,
        "source": (url + "; " if url else "") + UPSTREAM + file + f" ({rid})",
    }


def expand(template, m):
    return m.expand(re.sub(r"\$\{(\w+)\}", r"\\g<\1>", template).replace("$$", "$"))


def validate(r):
    """Whether the converted rule finds every incorrect example's marked
    span and nothing in the correct examples."""
    try:
        pat = regex.compile(r["pattern"], regex.V0)
        unless = regex.compile(r["unless"]) if r["unless"] else None
    except regex.error as e:
        raise Skip(f"pattern: {e}")
    if "(?P<m>" not in r["pattern"] and "(?P<t" not in r["pattern"]:
        pass

    def spans(text):
        if r["needs"] and not any(w in text.lower() for w in r["needs"]):
            return []
        if unless and unless.search(text):
            return []
        out = []
        for m in pat.finditer(text):
            g = "m" if "m" in pat.groupindex else 0
            out.append(m.span(g))
        return out

    for text, span in r["examples"]:
        if span not in spans(text):
            raise Skip("misses its example")
    # A rule with part-of-speech conditions may match a correct example
    # here and still pass: the conditions are checked with the parse
    # (crates/emdysi-check/examples/lt_validate.rs).
    for text in r["acceptable"]:
        if spans(text) and not (r["tags"] or r["not_tags"]):
            raise Skip("flags a correct example")
    # The suggestions must expand (a group they name must exist).
    for text, _ in r["examples"]:
        m = pat.search(text)
        try:
            for s in r["suggest"]:
                expand(s, m)
        except (IndexError, regex.error) as e:
            raise Skip(f"suggestion: {e}")


def rules_of(file):
    tree = ET.parse(SRC / file)
    for cat in tree.getroot().iter("category"):
        cid = cat.get("id")
        if cid in SKIP_CATEGORIES or cat.get("default") == "off":
            continue
        for el in cat:
            if el.tag == "rule":
                yield el, None, cid
            elif el.tag == "rulegroup":
                for r in el.findall("rule"):
                    yield r, el, cid


def toml_str(s):
    if "'" not in s and "\n" not in s and all(ord(c) >= 32 for c in s):
        return f"'{s}'"
    out = s.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
    return '"' + "".join(c if ord(c) >= 32 else f"\\u{ord(c):04x}" for c in out) + '"'


def slug(s):
    return re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")


# Rules with part-of-speech conditions that pass their own examples with
# emdysi's parse (written by crates/emdysi-check/examples/lt_validate.rs
# from a `--candidates` pack).
VALIDATED = ROOT / "scripts/languagetool-tagged.txt"


def build(candidates=False):
    """The pack's text, the rules in it, and the reasons rules were left
    out. With `candidates`, the pack holds every rule with part-of-speech
    conditions instead, for lt_validate."""
    kept, skipped = [], {}
    ids = {}
    validated = {
        line.strip()
        for line in (VALIDATED.read_text().splitlines() if VALIDATED.exists() else [])
        if line.strip() and not line.startswith("#")
    }
    for file in ("grammar.xml", "style.xml"):
        for rule, group, cat in rules_of(file):
            try:
                r = convert(rule, group, cat, file)
                validate(r)
            except Skip as e:
                reason = str(e).split(":")[0]
                skipped[reason] = skipped.get(reason, 0) + 1
                continue
            base = "languagetool." + slug(r["lt_id"])
            ids[base] = ids.get(base, 0) + 1
            r["id"] = base if ids[base] == 1 else f"{base}-{ids[base]}"
            tagged = bool(r["tags"] or r["not_tags"])
            if candidates and not tagged:
                continue
            if tagged and not candidates and r["id"] not in validated:
                skipped["fails its examples with the parse"] = (
                    skipped.get("fails its examples with the parse", 0) + 1
                )
                continue
            kept.append(r)
    out = [
        "# Generated by scripts/import-languagetool.py from data/languagetool/en/",
        "# (LanguageTool, LGPL-2.1-or-later; portions GPL-3.0-or-later). Do not edit.",
        "",
        "[pack]",
        'name = "languagetool"',
        'description = "LanguageTool\'s English rules: those that need only words, and those whose parts of speech the parse gives (opt-in)."',
        "",
    ]
    for r in kept:
        out += [
            "[[rule]]",
            f'id = "{r["id"]}"',
            'kind = "regex"',
            f'severity = "{r["severity"]}"',
            f"message = {toml_str(r['message'])}",
            f"source = {toml_str(r['source'])}",
            f"license = {toml_str(LICENSE)}",
            f"pattern = {toml_str(r['pattern'])}",
        ]
        if r["needs"]:
            out.append("needs = [" + ", ".join(toml_str(w) for w in r["needs"]) + "]")
        if r["tags"]:
            out.append("tags = [" + ", ".join(toml_str(c) for c in r["tags"]) + "]")
        if r["not_tags"]:
            out.append("not_tags = [" + ", ".join(toml_str(c) for c in r["not_tags"]) + "]")
        if r["unless"]:
            out.append(f"unless = {toml_str(r['unless'])}")
        if "${" in r["message"]:
            out.append("expand = true")
        if len(r["suggest"]) == 1:
            out.append(f"replace = {toml_str(r['suggest'][0])}")
        if r["suggest"]:
            out.append("suggest = [" + ", ".join(toml_str(s) for s in r["suggest"]) + "]")
        out.append("examples = [" + ", ".join(toml_str(t + "\n") for t, _ in r["examples"]) + "]")
        if r["acceptable"]:
            out.append("acceptable = [" + ", ".join(toml_str(t + "\n") for t in r["acceptable"]) + "]")
        out.append("")
    # LanguageTool's wordiness and redundancy lists (wordiness.txt,
    # redundancies.txt) are left out, as LanguageTool turns them off by
    # default: on edited text they flagged 642 and 95 phrases, many with
    # wrong or placeholder replacements (|as shown in| -> |shows that|,
    # [OMIT]) or standard phrasal verbs (|shut down|, |open up|).
    return "\n".join(out), kept, skipped


def main():
    if "--candidates" in sys.argv:
        path = Path(sys.argv[sys.argv.index("--candidates") + 1])
        text, kept, _ = build(candidates=True)
        path.write_text(text)
        print(f"{len(kept)} candidates with part-of-speech conditions", file=sys.stderr)
        return
    text, kept, skipped = build()
    if "--check" in sys.argv:
        if OUT.read_text() != text:
            sys.exit(f"{OUT.relative_to(ROOT)} is out of date: run scripts/import-languagetool.py")
        return
    OUT.write_text(text)
    print(f"{len(kept)} rules kept", file=sys.stderr)
    for k, v in sorted(skipped.items(), key=lambda kv: -kv[1])[:25]:
        print(f"  skipped {v:5}  {k}", file=sys.stderr)


if __name__ == "__main__":
    main()
