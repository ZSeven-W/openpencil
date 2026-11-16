#!/bin/sh
# Exercise the packaged daemon and the encoder as the unprivileged runtime user.
set -eu

daemon="$1"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cat > "$work/smoke.op" <<'DOC'
{"version":"1.1","children":[{"type":"rectangle","id":"motion","width":65,"height":65,"fill":[{"type":"solid","color":"#1765cc"}],"animations":[{"trigger":"mount","durationMs":80,"easing":"linear","keyframes":[{"offset":0,"opacity":0},{"offset":1,"opacity":1}]}]}]}
DOC

printf '%s\n' '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"export_motion","arguments":{"rootId":"motion","format":"mp4","fps":25}}}' \
  | "$daemon" --mcp "$work/smoke.op" > "$work/reply.json"

# The stdio MCP wire puts the export result's JSON in a text block.
payload=$(sed -n 's/.*\\"bytes_base64\\":\\"\([A-Za-z0-9+/=]*\)\\".*/\1/p' "$work/reply.json")
if [ -z "$payload" ]; then
  cat "$work/reply.json" >&2
  exit 1
fi
printf '%s' "$payload" | base64 --decode > "$work/smoke.mp4"

# Keep the runtime smoke test independent of external media tools. The
# renderer tests read samples and timestamps with the Rust MP4 parser.
test "$(od -An -j 4 -N 4 -t x1 "$work/smoke.mp4" | tr -d '[:space:]')" = "66747970"
LC_ALL=C grep -aq 'avc1' "$work/smoke.mp4"
LC_ALL=C grep -aq 'moov' "$work/smoke.mp4"
LC_ALL=C grep -aq 'mdat' "$work/smoke.mp4"
