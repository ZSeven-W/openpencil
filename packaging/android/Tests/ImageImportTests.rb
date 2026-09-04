# frozen_string_literal: true

player_dir = File.expand_path("..", __dir__)
source_dir = File.join(player_dir, "app/src/main/kotlin/tech/zseven/openpencil")
activity = File.read(File.join(source_dir, "MainActivity.kt"))
surface = File.read(File.join(source_dir, "OpSurfaceView.kt"))
# pollShellAction / importImageOrSvg moved into the shell bridge split.
surface_bridge = File.read(File.join(source_dir, "OpSurfaceViewShellBridge.kt"))
native = File.read(File.join(source_dir, "OpNative.kt"))

raise "Android image-import shell action missing" unless native.include?("SHELL_ACTION_IMPORT_IMAGE_OR_SVG = 12")
raise "Android image-import JNI declaration missing" unless native.include?("external fun nativeEditorImportImageOrSvg")
raise "Activity must own the image picker" unless activity.include?("ActivityResultContracts.OpenDocument()")
raise "surface must defer picker presentation to the Activity" unless surface.include?("setImportImageOrSvgHandler")
raise "Activity must register the image picker handler" unless activity.include?("setImportImageOrSvgHandler(::launchImageOrSvgPicker)")

action = surface_bridge[/fun pollShellAction\(\).*?(?=\n    \/\*\*)/m]
raise "shell-action drain missing" unless action
branch = action.index("SHELL_ACTION_IMPORT_IMAGE_OR_SVG")
post = action.index("post {", branch || 0)
invoke = action.index("importImageOrSvgHandler?.invoke()", branch || 0)
unless branch && post && invoke && branch < post && post < invoke
  raise "image picker must be presented after the editor JNI stack unwinds"
end

picker = activity[/private fun launchImageOrSvgPicker\(\).*?(?=\n    \/\*\*)/m]
raise "image picker launcher missing" unless picker
%w[image/png image/jpeg image/gif image/webp image/svg+xml].each do |mime|
  raise "image picker MIME missing: #{mime}" unless picker.include?(mime)
end
raise "image picker must suppress concurrent launches" unless picker.include?("imageImportInProgress")
raise "picker cancellation must retire ownership" unless activity.match?(/imageImportLauncher.*?uri == null.*?imageImportInProgress = false/m)

read = activity[/private fun readAndImportImageOrSvg\(.*?(?=\n    private fun importImageOrSvg)/m]
raise "bounded image reader missing" unless read
raise "known-size image must be bounded" unless read.include?("metadata.size > MAX_DOCUMENT_BYTES")
raise "unknown-size image must use the bounded reader" unless read.include?("readDocumentBytes(uri, metadata.size)")
raise "image bytes must be read off the main thread" unless read.include?("OpenPencilImageImporter")

return_bytes = surface_bridge[/fun importImageOrSvg\(.*?(?=\n    \/\*\*)/m]
raise "image ABI return missing" unless return_bytes&.include?("nativeEditorImportImageOrSvg")
raise "image ABI return must repaint collaboration rejection" unless return_bytes.include?("requestFrame()")
raise "collaboration rejection must rely on the engine notice" unless activity.include?("status != OpNative.STATUS_BUSY")
raise "SVG MIME must restore a missing SVG suffix" unless activity.include?("mime == \"image/svg+xml\"") && activity.include?('"$candidate.svg"')
# Handler release lives in the bridge; destroy() must route through it.
raise "teardown must drop the picker handler" unless surface.match?(/fun destroy\(\).*?shellBridge\.releaseHandlers\(\)/m) &&
  surface_bridge.match?(/fun releaseHandlers\(\).*?importImageOrSvgHandler = null/m)

puts "Android image import contract validates"
