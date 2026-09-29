#!/usr/bin/env bash
# Regenerate src/rules/structure/linguist_languages.txt from GitHub Linguist's languages.yml.
#
# Output: one lowercase fence identifier per line, sorted, unique. Includes, per language:
#   - name lowercased (dropped if it has spaces: fence info splits on whitespace), and with whitespace -> '-' (Linguist's default alias)
#   - fs_name, all `aliases`
#   - all `extensions` without the leading dot
# Linguist's Language[] (used for fences) resolves names/aliases/fs_name; extensions are
# added as a permissive superset since GitHub also highlights e.g. ```cs.
set -euo pipefail

URL="https://raw.githubusercontent.com/github-linguist/linguist/main/lib/linguist/languages.yml"
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT="$ROOT/src/rules/structure/linguist_languages.txt"

curl -fsSL "$URL" | awk '
  function unq(s) { gsub(/^[ \t]*-?[ \t]*/, "", s); gsub(/^["\x27]|["\x27][ \t]*$/, "", s); return tolower(s) }
  /^---/ || /^#/ { next }
  /^[^ ].*:$/ {
    name = substr($0, 1, length($0) - 1); gsub(/^["\x27]|["\x27]$/, "", name)
    n = tolower(name); print n; gsub(/[ \t]+/, "-", n); print n
    field = ""; next
  }
  /^  [a-z_]+:/ {
    field = $1; sub(/:$/, "", field)
    if (field == "fs_name") { v = $0; sub(/^  fs_name:[ \t]*/, "", v); print unq(v) }
    next
  }
  /^  - / {
    v = unq($0)
    if (field == "aliases") print v
    else if (field == "extensions") { sub(/^\./, "", v); print v }
  }
' | grep -v ' ' | LC_ALL=C sort -u > "$OUT"

echo "wrote $(wc -l < "$OUT" | tr -d " ") ids to $OUT"
