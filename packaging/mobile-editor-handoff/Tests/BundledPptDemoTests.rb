# frozen_string_literal: true

require "json"

repo_dir = File.expand_path("../../..", __dir__)
canonical_path = File.join(
  repo_dir,
  "crates/op-editor-core/assets/scene_templates/slide-deck.op"
)
ios_demo_path = File.join(repo_dir, "packaging/ios-player/Resources/ppt-demo.op")
android_demo_path = File.join(
  repo_dir,
  "packaging/android-player/app/src/main/assets/ppt-demo.op"
)

ios_bytes = File.binread(ios_demo_path)
android_bytes = File.binread(android_demo_path)
raise "iOS and Android PPT demos must be byte-identical" unless ios_bytes == android_bytes

canonical = JSON.parse(File.read(canonical_path))
demo = JSON.parse(ios_bytes)
editor_meta = demo.delete("editorMeta")

raise "PPT demo content drifted from the canonical slide deck" unless demo == canonical

expected_meta = {
  "activePageIndex" => 0,
  "scenario" => "slides",
  "pinnedStyleGuide" => "corporate-blue-light"
}
raise "PPT demo editorMeta is incorrect: #{editor_meta.inspect}" unless editor_meta == expected_meta

slides = demo.fetch("children")
raise "PPT demo must contain six slides" unless slides.length == 6
unless slides.all? { |slide| slide["type"] == "frame" && slide["width"] == 1920 && slide["height"] == 1080 }
  raise "every PPT demo slide must be a 1920x1080 frame"
end

ios_source = File.read(File.join(repo_dir, "packaging/ios-player/Sources/OpEngineHost.swift"))
unless ios_source.include?('return "ppt-demo"') &&
       ios_source.include?('Bundle.main.url(forResource: docName, withExtension: "op")')
  raise "iOS must load the bundled ppt-demo.op by default while preserving -doc overrides"
end

android_source = File.read(
  File.join(repo_dir, "packaging/android-player/app/src/main/kotlin/tech/zseven/openpencil/MainActivity.kt")
)
unless android_source.include?('intent.getStringExtra("doc") ?: "ppt-demo"') &&
       android_source.include?('readAsset("$docName.op")')
  raise "Android must load the bundled ppt-demo.op by default while preserving doc overrides"
end

puts "bundled PPT demo contract validates"
