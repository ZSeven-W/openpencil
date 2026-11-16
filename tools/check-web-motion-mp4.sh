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

video=$(ffprobe -v error -count_frames -select_streams v:0 \
  -show_entries stream=codec_name,width,height,nb_read_frames \
  -of csv=p=0 "$work/smoke.mp4")
test "$video" = "h264,66,66,3"
