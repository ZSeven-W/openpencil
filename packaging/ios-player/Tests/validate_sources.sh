#!/usr/bin/env bash
set -euo pipefail

player_dir="$(cd "$(dirname "$0")/.." && pwd)"
repo_dir="$(cd "$player_dir/../.." && pwd)"
header_dir="$repo_dir/crates/op-engine-ffi/include"
fixture="$repo_dir/crates/op-editor-core/assets/scene_templates/daily-sign-card.op"

required=(
  "$player_dir/OpenPencilPlayer-Bridging-Header.h"
  "$player_dir/project.yml"
  "$player_dir/README.md"
  "$player_dir/Resources/sample.op"
  "$player_dir/Sources/OpPlayerApp.swift"
  "$player_dir/Sources/OpPlayerView.swift"
  "$player_dir/Sources/OpEngineHost.swift"
)

for path in "${required[@]}"; do
  if [[ ! -f "$path" ]]; then
    echo "missing required iOS Player source: $path" >&2
    exit 1
  fi
done

cmp "$fixture" "$player_dir/Resources/sample.op"

while IFS= read -r source; do
  lines="$(wc -l < "$source" | tr -d ' ')"
  if (( lines > 800 )); then
    echo "$source has $lines lines; new files are capped at 800" >&2
    exit 1
  fi
done < <(find "$player_dir/Sources" -name '*.swift' -type f -print)

ruby - "$player_dir/project.yml" <<'RUBY'
require "yaml"
project = YAML.safe_load(File.read(ARGV.fetch(0)), aliases: true)
target = project.fetch("targets").fetch("OpenPencilPlayer")
raise "OpenPencilPlayer must be an iOS application" unless target["platform"] == "iOS" && target["type"] == "application"
settings = target.fetch("settings").fetch("base")
raise "deployment target must be iOS 15+" unless settings.fetch("IPHONEOS_DEPLOYMENT_TARGET").to_f >= 15.0
raise "bridging header setting missing" unless settings.key?("SWIFT_OBJC_BRIDGING_HEADER")
raise "op_engine.h search path missing" unless settings.fetch("HEADER_SEARCH_PATHS").to_s.include?("op-engine-ffi/include")
raise "device staticlib search path missing" unless settings.fetch("LIBRARY_SEARCH_PATHS[sdk=iphoneos*]").include?("aarch64-apple-ios/release")
raise "simulator staticlib search path missing" unless settings.fetch("LIBRARY_SEARCH_PATHS[sdk=iphonesimulator*]").include?("aarch64-apple-ios-sim/release")
frameworks = target.fetch("dependencies").map { |entry| entry["sdk"] }.compact
%w[Metal.framework QuartzCore.framework UIKit.framework].each do |framework|
  raise "#{framework} dependency missing" unless frameworks.include?(framework)
end
RUBY

grep -Fq -- "-sdk iphonesimulator26.4" "$player_dir/README.md"
grep -Fq -- "-destination 'platform=iOS Simulator,id=<sim-id>'" "$player_dir/README.md"
grep -Fq -- "aarch64-apple-ios-sim/release/libop_engine_ffi.a -lc++" "$player_dir/README.md"
grep -Fq -- "aarch64-apple-ios/release/libop_engine_ffi.a -lc++" "$player_dir/README.md"
grep -Fq -- "-framework Metal" "$player_dir/README.md"
grep -Fq -- "UIDocumentPickerViewController" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "BoundedDocumentReader.read" "$player_dir/Sources/OpPlayerView.swift"
grep -Fq -- "maximumBytes = 32 * 1024 * 1024" "$player_dir/Sources/BoundedDocumentReader.swift"
grep -Fq -- "op_editor_take_shell_action" "$player_dir/Sources/OpEngineHost.swift"
grep -Fq -- "op_editor_open_document" "$player_dir/Sources/OpEngineHost.swift"

sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
target="arm64-apple-ios15.0-simulator"
module_cache="${TMPDIR:-/tmp}/op-ios-player-module-cache"
mkdir -p "$module_cache"
export CLANG_MODULE_CACHE_PATH="$module_cache"

xcrun clang \
  -target "$target" \
  -isysroot "$sdk" \
  -fsyntax-only \
  -x objective-c \
  -I "$header_dir" \
  "$player_dir/OpenPencilPlayer-Bridging-Header.h"

xcrun swiftc \
  -typecheck \
  -warnings-as-errors \
  -parse-as-library \
  -target "$target" \
  -sdk "$sdk" \
  -import-objc-header "$player_dir/OpenPencilPlayer-Bridging-Header.h" \
  -module-cache-path "$module_cache" \
  -Xcc -I \
  -Xcc "$header_dir" \
  "$player_dir"/Sources/*.swift

reader_test_dir="$(mktemp -d "${TMPDIR:-/tmp}/op-bounded-document-reader-tests.XXXXXX")"
trap 'rm -rf "$reader_test_dir"' EXIT
reader_test="$reader_test_dir/runner"
xcrun swiftc \
  -warnings-as-errors \
  -parse-as-library \
  "$player_dir/Sources/BoundedDocumentReader.swift" \
  "$player_dir/Tests/BoundedDocumentReaderTests.swift" \
  -o "$reader_test"
"$reader_test"

echo "iOS Player sources and ABI imports validate"
