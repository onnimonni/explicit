#!/usr/bin/env python3
"""Thin out over-represented label categories in marked sources, in place.

usage: uv run trim.py <marked-dir> category=keep [category=keep ...]

For each named category, every marker of that category is numbered in file order (files sorted
by name) and only `keep` of them are kept, spread evenly; the rest are replaced by their
correction as plain text. Real-word (`category*`) markers count with their base category.
Run gen.py afterwards.
"""
import os
import re
import sys

TOKEN = re.compile(r"⟦([^|⟧]+)\|([^|⟧]*)\|([^⟧]*)⟧")


def is_doubling(wrong, fix):
    a, b = (wrong, fix) if len(wrong) < len(fix) else (fix, wrong)
    if len(b) - len(a) != 1:
        return False
    for i in range(len(b)):
        if b[:i] + b[i + 1:] == a:
            return (i > 0 and b[i - 1] == b[i]) or (i + 1 < len(b) and b[i + 1] == b[i])
    return False


def category(m):
    """Effective category, with the same typo -> double_letter reclassification as gen.py."""
    c = m.group(1).rstrip("*")
    if c == "typo" and is_doubling(m.group(2), m.group(3)):
        return "double_letter"
    return c


src = sys.argv[1]
targets = {}
for arg in sys.argv[2:]:
    cat, keep = arg.split("=")
    targets[cat] = int(keep)

paths = []
for root, dirs, names in os.walk(src):
    dirs.sort()
    paths += [os.path.join(root, n) for n in sorted(names) if n.endswith(".md")]

# First pass: count per category.
total = {c: 0 for c in targets}
for p in paths:
    with open(p, encoding="utf-8") as fh:
        for m in TOKEN.finditer(fh.read()):
            c = category(m)
            if c in total:
                total[c] += 1

# Which ordinal indices to keep: evenly spread.
keep_idx = {}
for c, n in total.items():
    k = min(targets[c], n)
    keep_idx[c] = set(int(i * n / k) for i in range(k)) if k else set()

seen = {c: 0 for c in targets}
removed = {c: 0 for c in targets}
for p in paths:
    with open(p, encoding="utf-8") as fh:
        text = fh.read()

    def repl(m):
        c = category(m)
        if c not in targets:
            return m.group(0)
        i = seen[c]
        seen[c] += 1
        if i in keep_idx[c]:
            return m.group(0)
        removed[c] += 1
        return m.group(3)

    new = TOKEN.sub(repl, text)
    if new != text:
        with open(p, "w", encoding="utf-8") as fh:
            fh.write(new)

for c in targets:
    print(f"{c}: {total[c]} -> {total[c] - removed[c]}")
