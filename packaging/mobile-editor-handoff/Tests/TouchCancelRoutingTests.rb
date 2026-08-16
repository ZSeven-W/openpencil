# frozen_string_literal: true

# Source-contract guard for the platform gesture bridge. UIKit and Android
# cancellation must stay distinct from release, and a second finger must
# cancel the single-pointer editor capture before pan/pinch takes ownership.

ios_path, android_path = ARGV
raise "expected iOS and Android source paths" unless ios_path && android_path

ios = File.read(ios_path)
android = File.read(android_path)

ios_cancel = ios[/override func touchesCancelled.*?\n    }\n\n    private func finish/m]
raise "iOS touchesCancelled method not found" unless ios_cancel
raise "iOS cancellation must call editorCancelGesture" unless ios_cancel.include?("host.editorCancelGesture()")
raise "iOS cancellation must clear platform tracking" unless ios_cancel.include?("resetEditorTouchTracking()")
raise "iOS cancellation must not route through editorTouchEnded" if ios_cancel.include?("editorTouchEnded")

ios_second_finger = ios[/else if touchIDs\.count == 2 \{.*?\n        }/m]
raise "iOS second-finger branch not found" unless ios_second_finger
raise "iOS second-finger takeover must cancel the first capture" unless ios_second_finger.include?("host.editorCancelGesture()")

ios_reset = ios[/private func resetEditorTouchTracking\(\).*?\n    }/m]
raise "iOS touch reset helper not found" unless ios_reset
%w[touchIDs storedTouches pinchTouches].each do |field|
  raise "iOS touch reset must clear #{field}" unless ios_reset.include?("#{field}.removeAll()")
end

unless ios.include?("private static let editorLongPressSlop: CGFloat = 8")
  raise "iOS long-press slop must match the 8-point canvas pan slop"
end
ios_move = ios[/private func editorTouchMoved\(.*?\n    }\n\n    private func editorTouchEnded/m]
raise "iOS editorTouchMoved method not found" unless ios_move
unless ios_move.include?("let slop = Self.editorLongPressSlop")
  raise "iOS editorTouchMoved must use editorLongPressSlop"
end
unless ios_move.include?("let deltaX = point.x - downPoint.x") &&
       ios_move.include?("let deltaY = point.y - downPoint.y") &&
       ios_move.include?("deltaX * deltaX + deltaY * deltaY > slop * slop")
  raise "iOS long-press movement must use Euclidean distance"
end

raise "Android ACTION_UP and ACTION_CANCEL must remain separate" if android.include?("MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL")

android_editor = android[/private fun editorTouch\(.*?\n    }\n\n    private fun resetEditorTouchTracking/m]
raise "Android editorTouch method not found" unless android_editor

android_second_finger = android_editor[/MotionEvent\.ACTION_POINTER_DOWN -> \{.*?\n            }/m]
raise "Android second-pointer branch not found" unless android_second_finger
unless android_second_finger.include?("OpNative.nativeEditorCancelGesture(engine)")
  raise "Android second-pointer takeover must cancel the first capture"
end
unless android_second_finger.include?("editorReleaseSuppressed = true")
  raise "Android transform takeover must suppress the remaining pointer release"
end

android_cancel = android_editor[/MotionEvent\.ACTION_CANCEL -> \{.*?\n            }/m]
raise "Android ACTION_CANCEL branch not found" unless android_cancel
unless android_cancel.include?("OpNative.nativeEditorCancelGesture(engine)")
  raise "Android ACTION_CANCEL must call nativeEditorCancelGesture"
end
raise "Android ACTION_CANCEL must clear platform tracking" unless android_cancel.include?("resetEditorTouchTracking()")
raise "Android ACTION_CANCEL must not call nativeEditorRelease" if android_cancel.include?("nativeEditorRelease")

android_reset = android[/private fun resetEditorTouchTracking\(\).*?\n    }/m]
raise "Android touch reset helper not found" unless android_reset
%w[primaryPointerId longPressArmed longPressFired twoFingerActive editorReleaseSuppressed].each do |field|
  raise "Android touch reset must reset #{field}" unless android_reset.include?(field)
end


unless android.include?("private const val LONG_PRESS_SLOP = 8f")
  raise "Android long-press slop must match the 8-dp canvas pan slop"
end
unless android_editor.include?("val inputDensity = viewportInputState.committedDensity") &&
       android_editor.include?("downX = event.x / inputDensity") &&
       android_editor.include?("downY = event.y / inputDensity")
  raise "Android long-press origin must be stored in logical dp"
end
unless android_editor.include?("val deltaX = x - downX") &&
       android_editor.include?("val deltaY = y - downY") &&
       android_editor.include?("deltaX * deltaX + deltaY * deltaY >") &&
       android_editor.include?("LONG_PRESS_SLOP * LONG_PRESS_SLOP")
  raise "Android long-press movement must use Euclidean logical-dp distance"
end
if android_editor.include?("LONG_PRESS_SLOP / density")
  raise "Android must not scale the logical long-press slop twice"
end
