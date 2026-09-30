#!/usr/bin/env python3
"""Turn marked-up sources in marked/ into seeded/ files plus labels.json and traps.json.

Markers (may appear several times per line, never nested):
  ⟦category|wrong text|correction⟧   planted error; "wrong text" is emitted
  ⟪kind|text⟫                        trap that must NOT be flagged; "text" is emitted
Columns are 1-based character offsets on the emitted line; end_column is exclusive.
"""
import json
import os
import re
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(HERE, sys.argv[1] if len(sys.argv) > 1 else "marked")
OUT = os.path.join(HERE, sys.argv[2] if len(sys.argv) > 2 else "seeded")

TOKEN = re.compile(r"⟦([^|⟧]+)\|([^|⟧]*)\|([^⟧]*)⟧|⟪([^|⟫]+)\|([^⟫]*)⟫")

CATEGORIES = {
    "spelling", "spelling_1edit", "a_an", "its_its", "your_youre", "their_there", "then_than",
    "repeated_word", "agreement", "homophone", "missing_extra_word",
    "capitalization", "british", "fragment", "punctuation",
}

labels, traps, files = [], [], 0
for root, dirs, names in os.walk(SRC):
    dirs.sort()
    for name in sorted(names):
        src_path = os.path.join(root, name)
        rel = os.path.relpath(src_path, SRC)
        out_lines = []
        with open(src_path, encoding="utf-8") as fh:
            for lineno, raw in enumerate(fh.read().split("\n"), start=1):
                out, pos = [], 0
                col = 0  # chars emitted so far on this line
                for m in TOKEN.finditer(raw):
                    plain = raw[pos:m.start()]
                    out.append(plain)
                    col += len(plain)
                    if m.group(1) is not None:
                        cat, wrong, fix = m.group(1), m.group(2), m.group(3)
                        if cat not in CATEGORIES:
                            sys.exit(f"{rel}:{lineno}: unknown category {cat!r}")
                        if not wrong or wrong == fix:
                            sys.exit(f"{rel}:{lineno}: bad span {wrong!r} -> {fix!r}")
                        labels.append({
                            "file": rel, "line": lineno, "column": col + 1,
                            "end_column": col + 1 + len(wrong), "text": wrong,
                            "category": cat, "correction": fix,
                        })
                        out.append(wrong)
                        col += len(wrong)
                    else:
                        kind, text = m.group(4), m.group(5)
                        traps.append({
                            "file": rel, "line": lineno, "column": col + 1,
                            "end_column": col + 1 + len(text), "text": text, "kind": kind,
                        })
                        out.append(text)
                        col += len(text)
                    pos = m.end()
                tail = raw[pos:]
                if "⟦" in tail or "⟪" in tail or "⟧" in tail or "⟫" in tail:
                    sys.exit(f"{rel}:{lineno}: stray marker bracket: {tail!r}")
                out.append(tail)
                out_lines.append("".join(out))
        dst = os.path.join(OUT, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        with open(dst, "w", encoding="utf-8") as fh:
            fh.write("\n".join(out_lines))
        files += 1

# Sanity: every label span must literally be at its column in the emitted file.
for l in labels + traps:
    with open(os.path.join(OUT, l["file"]), encoding="utf-8") as fh:
        line = fh.read().split("\n")[l["line"] - 1]
    got = line[l["column"] - 1:l["end_column"] - 1]
    assert got == l["text"], (l, got)

with open(os.path.join(OUT, "labels.json"), "w", encoding="utf-8") as fh:
    json.dump(labels, fh, indent=1, ensure_ascii=False)
with open(os.path.join(OUT, "traps.json"), "w", encoding="utf-8") as fh:
    json.dump(traps, fh, indent=1, ensure_ascii=False)

print(f"files: {files}  labels: {len(labels)}  traps: {len(traps)}")
for cat, n in sorted(Counter(l["category"] for l in labels).items()):
    print(f"  {cat:20s} {n}")
print("trap kinds:")
for kind, n in sorted(Counter(t["kind"] for t in traps).items()):
    print(f"  {kind:20s} {n}")
