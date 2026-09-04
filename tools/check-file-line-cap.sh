#!/usr/bin/env bash
# CLAUDE.md's 800-line-per-file convention, enforced.
#
# The cap had drifted to eight violations by 2026-08-27 (paint.rs 911,
# tests.rs 894, lib.rs 868, …) precisely because nothing checked it — the
# doc claimed "zero violations" while the tree said otherwise. Splitting is
# a spine plus sibling files with re-exports keeping import paths stable;
# see CLAUDE.md's "Code Style" section.
set -euo pipefail

cap=800
root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$root"

violations="$(find crates -name '*.rs' -not -path '*/target/*' -exec wc -l {} + \
  | awk -v cap="$cap" '$2 != "total" && $1 > cap { print $1 " " $2 }' | sort -rn)"

if [[ -n "$violations" ]]; then
  echo "::error::files exceed the ${cap}-line cap (CLAUDE.md Code Style):"
  echo "$violations" | sed 's/^/  /'
  exit 1
fi
echo "file line cap OK (no crates/**/*.rs over ${cap} lines)"
