# frozen_string_literal: true

# Source-contract guard for the platform pinch bridges. Both shells must map
# the fingers' distance ratio to the editor's exponential wheel-delta ABI.

ios_path, android_path = ARGV
raise "expected iOS and Android source paths" unless ios_path && android_path

ios = File.read(ios_path)
android = File.read(android_path)

ios_move = ios[/private func editorTouchMoved\(.*?\n    }\n\n    private func editorTouchEnded/m]
raise "iOS editorTouchMoved method not found" unless ios_move
unless ios_move.include?("PinchZoomDelta.wheelDelta(") &&
       ios_move.include?("previousDistance: previousDistance") &&
       ios_move.include?("currentDistance: lastPinchDistance")
  raise "iOS pinch must use the ratio-to-wheel helper"
end
if ios_move.include?("lastPinchDistance - previousDistance")
  raise "iOS pinch must not pass a point-distance delta to the wheel ABI"
end

android_editor = android[/private fun editorTouch\(.*?\n    }\n\n    private fun resetEditorTouchTracking/m]
raise "Android editorTouch method not found" unless android_editor
unless android_editor.include?("PinchZoomDelta.wheelDelta(") &&
       android_editor.include?("previousDistance = lastPinchDist") &&
       android_editor.include?("currentDistance = dist")
  raise "Android pinch must use the ratio-to-wheel helper"
end
if android_editor.include?("(dist - lastPinchDist) / inputDensity")
  raise "Android pinch must not pass a pixel-distance delta to the wheel ABI"
end

puts "Mobile pinch zoom routing contract validates"
