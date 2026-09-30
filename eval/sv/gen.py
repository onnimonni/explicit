#!/usr/bin/env python3
"""Turn marked-up Swedish sources into clean Markdown plus labels.json and traps.json.

usage: uv run gen.py [marked-dir] [out-dir]      (defaults: marked -> seeded)

Markers (several per line allowed, never nested):
  ⟦category|wrong text|correction⟧    planted error; "wrong text" is emitted.
  ⟦category*|wrong text|correction⟧   same, but "wrong text" is a valid Swedish word on its own
                                      (real-word error: only grammar/context can catch it);
                                      label gets "real_word": true.
  ⟪kind|text⟫                         trap that must NOT be flagged; "text" is emitted.

Every non-blank line inside a fenced code block becomes a trap of kind "code_block"
automatically. Columns are 1-based character offsets on the emitted line; end_column is
exclusive. A "typo" whose only difference is one doubled/undoubled letter is relabelled
"double_consonant".
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
    "typo",              # letter swapped / missing / wrong, result is a non-word
    "sarskrivning",      # compound written apart ("sjuk vård", "användar gränssnitt")
    "compound_link",     # wrong linking element: -s- missing or extra ("arbetgrupp", "vårdscentral")
    "de_dem_gender",     # de/dem/dom, en/ett, den/det agreement
    "verb_form",         # wrong tense/supine/infinitive form ("har gådd", "att skriver", dropped -r)
    "double_consonant",  # single/double consonant ("komer", "instalation", "address")
    "capitalization",    # weekdays, months, languages, nationalities upper-cased mid-sentence; lower-case sentence start
    "de_dem", "en_ett", "adj_agreement", "present_tense", "supine", "att_infinitive",  # round-3 rule targets
}
TRAP_KINDS = {
    "compound", "name", "product", "code", "identifier", "url", "unit",
    "abbrev", "ordinal", "colloquial", "foreign", "acronym", "code_block", "number", "phrase", "grammar_ok",
    "fi_sv",  # Finland-Swedish vocabulary that is correct in sv-FI text
}


def is_doubling(wrong, fix):
    a, b = (wrong, fix) if len(wrong) < len(fix) else (fix, wrong)
    if len(b) - len(a) != 1:
        return False
    for i in range(len(b)):
        if b[:i] + b[i + 1:] == a:
            return (i > 0 and b[i - 1] == b[i]) or (i + 1 < len(b) and b[i + 1] == b[i])
    return False


labels, traps, files = [], [], 0
for root, dirs, names in os.walk(SRC):
    dirs.sort()
    for name in sorted(names):
        if not name.endswith(".md"):
            continue
        src_path = os.path.join(root, name)
        rel = os.path.relpath(src_path, SRC)
        out_lines, in_fence = [], False
        with open(src_path, encoding="utf-8") as fh:
            for lineno, raw in enumerate(fh.read().split("\n"), start=1):
                stripped = raw.lstrip()
                if stripped.startswith("```") or stripped.startswith("~~~"):
                    in_fence = not in_fence
                    if TOKEN.search(raw):
                        sys.exit(f"{rel}:{lineno}: marker on a fence line")
                    out_lines.append(raw)
                    continue
                if in_fence:
                    if TOKEN.search(raw):
                        sys.exit(f"{rel}:{lineno}: marker inside a code block")
                    if raw.strip():
                        lead = len(raw) - len(raw.lstrip())
                        traps.append({"file": rel, "line": lineno, "column": lead + 1,
                                      "end_column": len(raw.rstrip()) + 1, "text": raw.strip(),
                                      "kind": "code_block"})
                    out_lines.append(raw)
                    continue
                out, pos, col = [], 0, 0
                for m in TOKEN.finditer(raw):
                    plain = raw[pos:m.start()]
                    out.append(plain)
                    col += len(plain)
                    if m.group(1) is not None:
                        cat, wrong, fix = m.group(1), m.group(2), m.group(3)
                        real = cat.endswith("*")
                        cat = cat.rstrip("*")
                        if cat not in CATEGORIES:
                            sys.exit(f"{rel}:{lineno}: unknown category {cat!r}")
                        if not wrong or wrong == fix:
                            sys.exit(f"{rel}:{lineno}: bad span {wrong!r} -> {fix!r}")
                        if cat in ("typo", "double_consonant"):
                            d = is_doubling(wrong, fix)
                            if cat == "typo" and d:
                                cat = "double_consonant"
                            elif cat == "double_consonant" and not d:
                                sys.exit(f"{rel}:{lineno}: {wrong!r} -> {fix!r} is not a doubling change")
                        lab = {"file": rel, "line": lineno, "column": col + 1,
                               "end_column": col + 1 + len(wrong), "text": wrong,
                               "category": cat, "correction": fix}
                        if real:
                            lab["real_word"] = True
                        labels.append(lab)
                        out.append(wrong)
                        col += len(wrong)
                    else:
                        kind, text = m.group(4), m.group(5)
                        if kind not in TRAP_KINDS:
                            sys.exit(f"{rel}:{lineno}: unknown trap kind {kind!r}")
                        if not text:
                            sys.exit(f"{rel}:{lineno}: empty trap")
                        traps.append({"file": rel, "line": lineno, "column": col + 1,
                                      "end_column": col + 1 + len(text), "text": text, "kind": kind})
                        out.append(text)
                        col += len(text)
                    pos = m.end()
                tail = raw[pos:]
                if any(ch in tail for ch in "⟦⟧⟪⟫"):
                    sys.exit(f"{rel}:{lineno}: stray marker bracket in {tail!r}")
                out.append(tail)
                out_lines.append("".join(out))
        if in_fence:
            sys.exit(f"{rel}: unterminated code fence")
        dst = os.path.join(OUT, rel)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        with open(dst, "w", encoding="utf-8") as fh:
            fh.write("\n".join(out_lines))
        files += 1

with open(os.path.join(OUT, "labels.json"), "w", encoding="utf-8") as fh:
    json.dump(labels, fh, ensure_ascii=False, indent=1)
    fh.write("\n")
with open(os.path.join(OUT, "traps.json"), "w", encoding="utf-8") as fh:
    json.dump(traps, fh, ensure_ascii=False, indent=1)
    fh.write("\n")

print(f"{files} files -> {OUT}")
print(f"{len(labels)} labels ({sum(1 for l in labels if l.get('real_word'))} real-word):")
for cat, n in sorted(Counter(l["category"] for l in labels).items()):
    print(f"  {cat:17} {n}")
print(f"{len(traps)} traps:")
for kind, n in sorted(Counter(t['kind'] for t in traps).items()):
    print(f"  {kind:17} {n}")
front = sum(1 for n in os.listdir(OUT) if n.endswith(".md")
            and open(os.path.join(OUT, n), encoding="utf-8").read().startswith("---\n"))
print(f"front matter: {front}/{files} files")
