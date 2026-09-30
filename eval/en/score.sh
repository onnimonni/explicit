#!/usr/bin/env bash
# Score an `explicit check --format json` run over seeded/ against labels.json.
#
# usage: score.sh <findings.json> <labels.json> [-t traps.json] [-v] [-j]
#   -t  traps file (default: traps.json next to labels.json, if present)
#   -v  list false positives and missed labels
#   -j  print the full result as JSON instead of text
#
# Only findings in the families `spelling` and `grammar/*` are scored. A finding matches a label
# when it is in the same file and its [line:column, end_line:end_column) span overlaps the label
# span on the label's line. Precision = scored findings that overlap some label / scored findings.
# Recall = labels overlapped by some scored finding / labels. F1 from those two. Findings of other
# families (md/*, slop/*, prose/*, links/*, ...) are counted separately and never affect the score.
set -euo pipefail

usage() { sed -n '2,12p' "$0" | sed 's/^# \{0,1\}//'; exit 2; }

findings=""; labels=""; traps=""; verbose=0; asjson=0
while [ $# -gt 0 ]; do
  case "$1" in
    -t) traps="$2"; shift 2 ;;
    -v) verbose=1; shift ;;
    -j) asjson=1; shift ;;
    -h|--help) usage ;;
    *) if [ -z "$findings" ]; then findings="$1"; elif [ -z "$labels" ]; then labels="$1"; else usage; fi; shift ;;
  esac
done
[ -n "$findings" ] && [ -n "$labels" ] || usage
[ -f "$findings" ] || { echo "no such file: $findings" >&2; exit 2; }
[ -f "$labels" ] || { echo "no such file: $labels" >&2; exit 2; }
if [ -z "$traps" ]; then
  candidate="$(dirname "$labels")/traps.json"
  [ -f "$candidate" ] && traps="$candidate"
fi
if [ -n "$traps" ]; then trapsarg=(--slurpfile traps "$traps"); else trapsarg=(--argjson traps '[[]]'); fi
command -v jq >/dev/null || { echo "jq is required" >&2; exit 2; }

jq -n --slurpfile findings "$findings" --slurpfile labels "$labels" "${trapsarg[@]}" \
   --argjson verbose "$verbose" --argjson asjson "$asjson" -r '
def fam: if .rule == "spelling" then "spelling"
         elif (.rule | startswith("grammar/")) then "grammar"
         else (.rule | split("/")[0]) end;
def samefile($p; $f): ($p == $f) or ($p | endswith("/" + $f));
def overlaps($f; $l):
  samefile($f.path; $l.file) and
  ( if $f.line == $f.end_line
    then ($f.line == $l.line and $f.column < $l.end_column and $f.end_column > $l.column)
    else ( ($f.line < $l.line and $l.line < $f.end_line)
         or ($f.line == $l.line and $f.column < $l.end_column)
         or ($f.end_line == $l.line and $f.end_column > $l.column) ) end );
def pct: if . == null then "n/a" else (. * 1000 | round / 10 | tostring) + "%" end;
def ratio($a; $b): if $b == 0 then null else ($a / $b) end;

($labels[0]) as $L
| ($traps[0] // []) as $T
| (($findings[0] // []) | if type == "array" then . else [] end
   | map(. + { path: (.path | tostring),
               end_line: (.end_line // .line),
               end_column: (.end_column // (.column + ((.text // " ") | length) | if . == 0 then 1 else . end)),
               fam: fam })) as $F
| [ $F[] | select(.fam == "spelling" or .fam == "grammar") ] as $tgt
| [ $F[] | select(.fam != "spelling" and .fam != "grammar") ] as $oth
| [ $L[] as $l | $l + { hits: [ $tgt[] | select(overlaps(.; $l)) | .rule ] | unique } ] as $LL
| [ $tgt[] as $f | $f + { tp: ([ $L[] | select(overlaps($f; .)) ] | length > 0),
                          trap: ([ $T[] | select(overlaps($f; .)) | .kind ] | unique) } ] as $TT
| ($LL | map(select(.hits | length > 0)) | length) as $hit
| ($TT | map(select(.tp)) | length) as $tp
| ($TT | map(select(.tp | not))) as $fps
| ($tgt | length) as $ntgt
| ($L | length) as $nl
| ratio($tp; $ntgt) as $prec
| ratio($hit; $nl) as $rec
| (if $prec == null or $rec == null or ($prec + $rec) == 0 then null else 2 * $prec * $rec / ($prec + $rec) end) as $f1
| {
    labels: $nl, labels_hit: $hit,
    scored_findings: $ntgt, true_positives: $tp, false_positives: ($fps | length),
    precision: $prec, recall: $rec, f1: $f1,
    recall_by_category: ( $LL | group_by(.category) | map({ key: .[0].category,
        value: { total: length, hit: (map(select(.hits | length > 0)) | length),
                 recall: ratio((map(select(.hits | length > 0)) | length); length) } }) | from_entries ),
    scored_by_family: ( $tgt | group_by(.fam) | map({ key: .[0].fam, value: length }) | from_entries ),
    fp_by_rule: ( $fps | group_by(.rule) | map({ key: .[0].rule, value: length }) | sort_by(-.value) | from_entries ),
    fp_on_traps: ( [ $fps[] | .trap[] ] | group_by(.) | map({ key: .[0], value: length }) | sort_by(-.value) | from_entries ),
    fp_not_on_trap: ( [ $fps[] | select(.trap | length == 0) ] | length ),
    other_families: ( $oth | group_by(.fam) | map({ key: .[0].fam,
        value: { findings: length,
                 overlapping_labels: ( map(. as $f | select([ $L[] | select(overlaps($f; .))] | length > 0)) | length ) } }) | from_entries ),
    false_positive_list: ( $fps | map({ where: "\(.path):\(.line):\(.column)", rule, text, trap }) ),
    missed_labels: ( $LL | map(select(.hits | length == 0) | { where: "\(.file):\(.line):\(.column)", category, text, correction }) )
  }
| if $asjson == 1 then . else
  "labels: \(.labels)   scored findings (spelling + grammar/*): \(.scored_findings)   other families: \($oth | length)",
  "precision: \(.precision | pct)  (\(.true_positives) TP / \(.false_positives) FP)",
  "recall:    \(.recall | pct)  (\(.labels_hit) of \(.labels) labels hit)",
  "F1:        \(.f1 | pct)",
  "",
  "recall by category:",
  ( .recall_by_category | to_entries | sort_by(.key)[] | "  \(.key | . + " " * (20 - length)) \(.value.hit | tostring | " " * (3 - length) + .)/\(.value.total | tostring | " " * (3 - length) + .)  \(.value.recall | pct)" ),
  "",
  "scored findings by family: \(.scored_by_family | to_entries | map("\(.key)=\(.value)") | join("  "))",
  "false positives by rule:   \(.fp_by_rule | to_entries | map("\(.key)=\(.value)") | join("  ") | if . == "" then "-" else . end)",
  "false positives on traps:  \(.fp_on_traps | to_entries | map("\(.key)=\(.value)") | join("  ") | if . == "" then "-" else . end)   (not on any trap: \(.fp_not_on_trap))",
  "",
  "other families (not scored): \(.other_families | to_entries | map("\(.key)=\(.value.findings) (\(.value.overlapping_labels) on labels)") | join("  ") | if . == "" then "-" else . end)",
  ( if $verbose == 1 then
      "", "FALSE POSITIVES:", ( .false_positive_list[] | "  \(.where)  \(.rule)  \(.text | tojson)  \(if (.trap | length) > 0 then "trap:" + (.trap | join(",")) else "" end)" ),
      "", "MISSED LABELS:", ( .missed_labels[] | "  \(.where)  \(.category)  \(.text | tojson) -> \(.correction | tojson)" )
    else empty end )
  end
'
