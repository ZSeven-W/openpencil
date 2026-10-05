#!/usr/bin/env bash
# Copy the shared design-font notices beside the distributed desktop binary.
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: stage-font-notices.sh DESTINATION" >&2
  exit 1
fi
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
mkdir -p "$1"
for name in NOTICE.txt OFL-Inter.txt OFL-NotoSansSC.txt OFL-PlusJakartaSans.txt; do
  cp "$repo_root/packaging/shared/fonts/$name" "$1/$name"
  chmod 644 "$1/$name"
done
