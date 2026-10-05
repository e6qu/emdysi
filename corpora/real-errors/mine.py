#!/usr/bin/env python3
"""Mine real errors from the commit history of a documentation repository.

Usage: python3 corpora/real-errors/mine.py REPO PREFIX OUT.tsv [MAX_COMMITS]

REPO is a full clone (a blob-less partial clone is enough), PREFIX the
directory of the Markdown text (e.g. `src/`). The most recent MAX_COMMITS
non-merge commits whose message mentions a typo, spelling, grammar or a
misspelling are read. For each changed Markdown file, the paragraphs (code
blocks, tables, headings, HTML and link definitions left out) before and
after the commit are aligned; a paragraph pair is kept when it differs in
one place by at most two words, outside inline code and links, and the
change is one of:

- spelling: one word not in the word lists (data/scowl, sizes 60 and 70)
  replaced by one that is;
- grammar-function-word: only function words added, removed or replaced;
- grammar-inflection: one listed word replaced by a similar listed word
  (same first three letters), e.g. "holds" -> "hold".

Columns: commit, kind, before, after, paragraph before, paragraph after.
"""
import os, subprocess, sys, re, difflib
repo, prefix, out = sys.argv[1], sys.argv[2], sys.argv[3]
ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..")
words60 = set(l.split('\t')[0].lower() for l in open(os.path.join(ROOT, 'data/scowl/en-60.tsv')) if not l.startswith('#'))
words60 |= set(l.strip().lower() for l in open(os.path.join(ROOT, 'data/scowl/en-70-extra.txt')) if not l.startswith('#'))
FUNC = set("a an the of to in on at by for with from and or but as is are was were be been being am has have had do does did will would can could should may might must that which who whom whose this these those it its they them their he him his she her we us our you your i me my not no".split())
def git(*a):
    return subprocess.run(["git","-C",repo,*a],capture_output=True,text=True).stdout
commits = git("log","--no-merges","-i","--grep=typo","--grep=spelling","--grep=grammar","--grep=misspell","--format=%H","--",prefix).split()[:int(sys.argv[4]) if len(sys.argv)>4 else None]
def paras(text):
    out=[]; cur=[]; fence=False
    for line in text.splitlines():
        if line.strip().startswith("```"):
            fence = not fence; 
            if cur: out.append(" ".join(cur)); cur=[]
            continue
        if fence: continue
        if not line.strip():
            if cur: out.append(" ".join(cur)); cur=[]
            continue
        if re.match(r"^(\||<|\[[^\]]+\]:|#|\{\{|    )", line):
            if cur: out.append(" ".join(cur)); cur=[]
            continue
        cur.append(line.strip())
    if cur: out.append(" ".join(cur))
    return out
def clean(w): return re.sub(r"^[^\w]+|[^\w]+$","",w).lower()
rows=[]; seen=set()
for c in commits:
    files=[f for f in git("show","--format=","--name-only",c).split() if f.startswith(prefix) and f.endswith(".md")]
    for f in files:
        old=git("show",f"{c}^:{f}"); new=git("show",f"{c}:{f}")
        if not old or not new: continue
        po,pn=paras(old),paras(new)
        sm=difflib.SequenceMatcher(a=po,b=pn,autojunk=False)
        for t,i1,i2,j1,j2 in sm.get_opcodes():
            if t!="replace" or i2-i1!=j2-j1: continue
            for o,n in zip(po[i1:i2],pn[j1:j2]):
                ow,nw=o.split(),n.split()
                ws=difflib.SequenceMatcher(a=ow,b=nw,autojunk=False)
                ch=[x for x in ws.get_opcodes() if x[0]!="equal"]
                k=sum(max(x[2]-x[1],x[4]-x[3]) for x in ch)
                if k==0 or k>2 or len(ch)!=1: continue
                tag,a1,a2,b1,b2=ch[0]
                oldseg=" ".join(ow[a1:a2]); newseg=" ".join(nw[b1:b2])
                if "`" in oldseg+newseg or "](" in oldseg+newseg: continue
                ot=[clean(w) for w in ow[a1:a2]]; nt=[clean(w) for w in nw[b1:b2]]
                if any(not w for w in ot+nt) and tag=="replace": continue
                if ot==nt: continue  # punctuation only
                # Classify.
                if tag=="replace" and len(ot)==1 and len(nt)==1 and ot[0].isalpha() and ot[0] not in words60 and nt[0] in words60:
                    kind="spelling"
                elif all(w in FUNC for w in ot+nt) and (ot or nt):
                    kind="grammar-function-word"
                elif tag=="replace" and len(ot)==1 and len(nt)==1 and ot[0][:3]==nt[0][:3] and ot[0] in words60 and nt[0] in words60 and difflib.SequenceMatcher(a=ot[0],b=nt[0]).ratio()>0.7:
                    kind="grammar-inflection"
                else:
                    continue
                if (o,n) in seen: continue
                seen.add((o,n))
                rows.append((c[:12],kind,oldseg,newseg,o,n))
with open(out,"w") as fh:
    fh.write("# commit\tkind\tbefore\tafter\tparagraph before\tparagraph after\n")
    for r in rows: fh.write("\t".join(x.replace("\t"," ") for x in r)+"\n")
from collections import Counter
print(len(commits),"commits",len(rows),"pairs",Counter(r[1] for r in rows))
