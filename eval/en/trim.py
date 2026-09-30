#!/usr/bin/env python3
"""Unplant a deterministic random subset of error markers in marked/ to hit target counts.

Unplanted confusable-word markers become ⟪correct|...⟫ traps (the corrected word must not be
flagged); other categories become plain corrected text. Run once; it rewrites marked/ in place.
"""
import os
import random
import re

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, "marked")
ERR = re.compile(r"⟦([^|⟧]+)\|([^|⟧]*)\|([^⟧]*)⟧")

TARGET = {
    "spelling": 62, "a_an": 26, "its_its": 21, "your_youre": 16, "their_there": 16,
    "then_than": 16, "repeated_word": 21, "agreement": 21, "homophone": 26,
    "missing_extra_word": 16, "capitalization": 21, "british": 16, "fragment": 11,
    "punctuation": 16,
}
AS_TRAP = {"a_an", "its_its", "your_youre", "their_there", "then_than", "homophone"}

files = {}
occ = {}  # category -> list of (file, ordinal)
for root, _, names in os.walk(SRC):
    for name in sorted(names):
        path = os.path.join(root, name)
        text = open(path, encoding="utf-8").read()
        files[path] = text
        for i, m in enumerate(ERR.finditer(text)):
            occ.setdefault(m.group(1), []).append((path, i))

rng = random.Random(20260929)
drop = set()
for cat, items in sorted(occ.items()):
    keep = TARGET[cat]
    if len(items) > keep:
        items = items[:]
        rng.shuffle(items)
        drop.update(items[keep:])

for path, text in files.items():
    counter = {"i": -1}

    def sub(m):
        counter["i"] += 1
        if (path, counter["i"]) not in drop:
            return m.group(0)
        cat, fix = m.group(1), m.group(3)
        if cat in AS_TRAP and "|" not in fix:
            return f"⟪correct|{fix}⟫"
        return fix

    new = ERR.sub(sub, text)
    if new != text:
        open(path, "w", encoding="utf-8").write(new)

print(f"unplanted {len(drop)} markers")
