#!/usr/bin/env python3
"""Plant one marker per line of a plan into marked sources (first plain occurrence of the word).

usage: uv run plant.py <marked-dir> <plan.tsv>
plan lines: file<TAB>category<TAB>correct word<TAB>wrong form
Only matches whole words outside existing markers and outside fenced code; prints what it did.
"""
import os
import re
import sys

src, plan = sys.argv[1], sys.argv[2]
MARK = re.compile(r"⟦[^⟧]*⟧|⟪[^⟫]*⟫|`[^`]*`")

for raw in open(plan, encoding="utf-8"):
    raw = raw.rstrip("\n")
    if not raw or raw.startswith("#"):
        continue
    fname, cat, word, wrong = raw.split("\t")
    path = os.path.join(src, fname)
    text = open(path, encoding="utf-8").read()
    pat = re.compile(r"(?<![\w|])" + re.escape(word) + r"(?![\w|])")
    done = False
    out, in_fence = [], False
    for line in text.split("\n"):
        if line.lstrip().startswith("```"):
            in_fence = not in_fence
        if done or in_fence:
            out.append(line)
            continue
        # skip matches inside existing markers / code spans
        pieces, pos = [], 0
        for m in MARK.finditer(line):
            seg = line[pos:m.start()]
            if not done:
                seg, n = pat.subn(f"⟦{cat}|{wrong}|{word}⟧", seg, count=1)
                done = done or n > 0
            pieces.append(seg)
            pieces.append(m.group(0))
            pos = m.end()
        seg = line[pos:]
        if not done:
            seg, n = pat.subn(f"⟦{cat}|{wrong}|{word}⟧", seg, count=1)
            done = done or n > 0
        pieces.append(seg)
        out.append("".join(pieces))
    if done:
        open(path, "w", encoding="utf-8").write("\n".join(out))
    print(f"{'ok  ' if done else 'MISS'} {fname}: {word} -> {wrong}")
