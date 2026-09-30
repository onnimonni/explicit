#!/usr/bin/env python3
"""Build a fake `explicit check --format json` output from labels.json/traps.json to validate score.sh.

usage: uv run fake_findings.py <set-dir> [path-prefix]   -> writes <set-dir>/fake-findings.json

Hits every third non-real-word label and every fifth real-word label, adds false positives on a
few traps (names, products, code blocks, foreign passages) and one md/* finding that overlaps a
label (must not count). Rule names mimic what a Finnish engine could emit.
"""
import json
import os
import sys

d = sys.argv[1]
prefix = sys.argv[2] if len(sys.argv) > 2 else os.path.basename(os.path.normpath(d)) + "/"
labels = json.load(open(os.path.join(d, "labels.json"), encoding="utf-8"))
traps = json.load(open(os.path.join(d, "traps.json"), encoding="utf-8"))

def finding(rule, x, text=None):
    return {
        "path": prefix + x["file"], "rule": rule, "severity": "warning",
        "line": x["line"], "column": x["column"], "end_line": x["line"], "end_column": x["end_column"],
        "text": text or x["text"], "message": "fake",
    }

out = []
nr = [l for l in labels if not l.get("real_word")]
rw = [l for l in labels if l.get("real_word")]
for i, l in enumerate(nr):
    if i % 3 == 0:
        rule = "grammar/fi-compound" if l["category"].startswith("compound") else "spelling"
        out.append(finding(rule, l))
for i, l in enumerate(rw):
    if i % 5 == 0:
        out.append(finding("grammar/fi-agreement", l))
# partial overlap: a finding covering only the first character of a label still counts
if len(nr) > 1:
    l = nr[1]
    f = finding("spelling", l)
    f["end_column"] = l["column"] + 1
    f["text"] = l["text"][:1]
    out.append(f)
# false positives on traps
seen = {}
for t in traps:
    k = t["kind"]
    if k in ("name", "product", "code_block", "foreign", "compound") and seen.get(k, 0) < 3:
        seen[k] = seen.get(k, 0) + 1
        out.append(finding("spelling", t))
# a false positive on nothing in particular
out.append({"path": prefix + labels[0]["file"], "rule": "spelling", "severity": "warning",
            "line": 1, "column": 3, "end_line": 1, "end_column": 8, "text": "xxxxx", "message": "fake"})
# an unscored family overlapping a label
out.append(finding("md/line-length", labels[0]))
json.dump(out, open(os.path.join(d, "fake-findings.json"), "w", encoding="utf-8"), ensure_ascii=False, indent=1)
print(f"{len(out)} fake findings -> {os.path.join(d, 'fake-findings.json')}")
