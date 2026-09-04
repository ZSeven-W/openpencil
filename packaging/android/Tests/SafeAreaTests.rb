# frozen_string_literal: true

require "rexml/document"

player_dir = File.expand_path("..", __dir__)
source_dir = File.join(player_dir, "app/src/main/kotlin/tech/zseven/openpencil")
activity = File.read(File.join(source_dir, "MainActivity.kt"))
edge_to_edge = File.read(File.join(source_dir, "EdgeToEdgeWindow.kt"))
surface = File.read(File.join(source_dir, "OpSurfaceView.kt"))
# The View's cohesive subsystems were split out of OpSurfaceView.kt; the
# contracts below assert against whichever file now owns the behavior.
surface_touch = File.read(File.join(source_dir, "OpSurfaceViewEditorTouch.kt"))
surface_ime = File.read(File.join(source_dir, "OpSurfaceViewImeCoordinator.kt"))
surface_bridge = File.read(File.join(source_dir, "OpSurfaceViewShellBridge.kt"))
surface_all = surface + surface_touch + surface_ime + surface_bridge
native = File.read(File.join(source_dir, "OpNative.kt"))
configuration_gate = File.read(File.join(source_dir, "ConfigurationViewportGate.kt"))
input_state = File.read(File.join(source_dir, "ViewportInputState.kt"))

manifest = REXML::Document.new(
  File.read(File.join(player_dir, "app/src/main/AndroidManifest.xml")),
)
activity_element = REXML::XPath.first(manifest, "/manifest/application/activity")
raise "Android activity missing" unless activity_element
raise "IME must not resize the whole editor" unless activity_element.attributes["android:windowSoftInputMode"] == "adjustNothing"

raise "activity must configure edge-to-edge before content" unless activity.index("configureEdgeToEdge(window)") < activity.index("setContentView(rootView)")
raise "insets must be observed on the full-window root" unless activity.include?("installEditorInsets(rootView, surfaceView)")
raise "SurfaceView must remain full-window" unless activity.scan("ViewGroup.LayoutParams.MATCH_PARENT").length >= 2
raise "legacy systemUiVisibility overrides modern edge-to-edge flags" if activity.include?("systemUiVisibility")
raise "density config changes must refresh the surviving surface" unless activity.include?("surfaceView.refreshDisplayMetrics()")
raise "config changes must request fresh insets" unless activity.include?("ViewCompat.requestApplyInsets(rootView)")

required_contract = [
  "WindowCompat.setDecorFitsSystemWindows(window, false)",
  "window.statusBarColor = Color.TRANSPARENT",
  "window.navigationBarColor = Color.TRANSPARENT",
  "updateSystemChromeAppearance(window, prefersLightIcons = false)",
  "LAYOUT_IN_DISPLAY_CUTOUT_MODE_SHORT_EDGES",
  "window.isNavigationBarContrastEnforced = false",
  "WindowInsetsCompat.Type.systemBars()",
  "WindowInsetsCompat.Type.displayCutout()",
  "surface.updateSafeAreaPx(safe.top, safe.right, safe.bottom, safe.left)",
  "insets.isVisible(imeType)",
  "root.resources.displayMetrics.density",
  "ViewCompat.requestApplyInsets(root)",
].freeze

required_contract.each do |contract|
  raise "safe-area contract missing: #{contract}" unless edge_to_edge.include?(contract)
end

raise "root insets must not be consumed" if edge_to_edge.include?("WindowInsetsCompat.CONSUMED")
raise "edge-to-edge must not hide system bars" if edge_to_edge.match?(/hide\s*\(.*systemBars/i)
raise "system icon preference must invert Android light-bar naming" unless edge_to_edge.include?("val useDarkIcons = !prefersLightIcons")
raise "status/nav icon contrast must update together" unless edge_to_edge.include?("isAppearanceLightStatusBars = useDarkIcons") && edge_to_edge.include?("isAppearanceLightNavigationBars = useDarkIcons")
raise "startup backdrop must follow icon contrast" unless edge_to_edge.include?("Color.rgb(245, 245, 247)")

raise "surfaceCreated must refresh density" unless surface.match?(/surfaceCreated.*?refreshDensityFromResources\(\)/m)
raise "surfaceChanged must refresh density before resize" unless surface.match?(/surfaceChanged.*?refreshDensityFromResources\(\).*?nativeResize/m)
raise "touch conversion must use committed native density" unless surface.include?("viewportInputState.committedDensity")
raise "IME visibility must come from platform insets" unless surface_ime.include?("Actual platform visibility, updated only from WindowInsets")
raise "show request must not latch visibility" if surface_all.match?(/imeVisible\s*=\s*imm\.showSoftInput/)
raise "hidden focused IME must remain retryable" unless surface_ime.include?("focused && !imeVisible && imeShowNeeded")
raise "IME request gate must expire" unless surface_ime.include?("postDelayed(clearImeShowRequest, 400L)")
raise "rotation must reset the IME retry state" unless surface.include?("retryAfterConfiguration()") && surface_ime.include?("fun retryAfterConfiguration")
raise "IME retry must be bounded" unless surface_ime.include?("imeShowAttempts < 2")
raise "a rejected IME request must still schedule its retry" unless surface_ime.match?(/showSoftInput.*?removeCallbacks\(clearImeShowRequest\).*?postDelayed\(clearImeShowRequest, 400L\)/m)
raise "landscape IME must not replace the editor with extract UI" unless surface.include?("EditorInfo.IME_FLAG_NO_EXTRACT_UI")
raise "Kotlin JNI bridge must expose theme contrast" unless native.include?("external fun nativePrefersLightSystemIcons(engine: Long): Boolean")
raise "successful frames must poll theme contrast" unless surface.match?(/nativeFrame.*?0 -> \{.*?syncSystemChromeAppearance\(\)/m)
raise "system chrome updates must be value-deduplicated" unless surface_bridge.include?("if (prefersLightSystemIcons == next) return")
raise "configuration changes must replay cached contrast" unless activity.include?("surfaceView.replaySystemChromeAppearance()")
raise "Kotlin JNI bridge must expose atomic viewport geometry" unless native.include?("external fun nativeResizeWithSafeArea(")
raise "insets must not be sent independently" if surface.include?("nativeSetSafeArea(")
raise "legacy independent resize must not remain" if surface_all.include?("OpNative.nativeResize(")
raise "surface and inset callbacks must coalesce viewport geometry" unless surface.scan("scheduleViewportUpdate()").length >= 4
raise "viewport tuple must be pre-draw coalesced" unless surface.match?(/viewportPreDrawListener.*?applyViewportTuple\(\).*?addOnPreDrawListener\(viewportPreDrawListener\)/m)
raise "surfaceChanged dimensions must be authoritative" unless surface.match?(/surfaceChanged.*?surfaceWidthPx = wPx.*?surfaceHeightPx = hPx/m)
raise "handled extent changes must rebind the EGL window surface" unless surface.match?(/surfaceChanged.*?extentChanged.*?if \(attachedOnce\) \{.*?cancelStreamsBeforeSuspend\(\).*?OpNative\.nativeSuspend\(engine\).*?attachOrResume\(holder\.surface\)/m)
raise "configuration must wait for inset redispatch" unless surface.include?("configurationViewportGate.onInsetsDispatched()")
raise "configuration gate must be evaluated after traversal" unless surface.match?(/viewportPreDrawListener.*?configurationViewportGate\.evaluatePreDraw/m)
raise "animation phase must only schedule another pre-draw" unless surface.include?("postOnAnimation { scheduleViewportUpdate(requestedEpoch) }")
raise "density fallback decision must not run in an animation callback" if surface.include?("postOnAnimation(configurationInsetsFallback)")
raise "density-only changes need two stable pre-draw samples" unless configuration_gate.include?("stablePreDrawSamples < 2")
raise "rotation bounds must continue waiting for insets" unless configuration_gate.match?(/!unchangedAfterTraversal.*?WAIT_FOR_INSETS/m)
raise "density fallback must require matching View and Surface bounds" unless configuration_gate.include?("surfaceWidthPx == viewWidthPx && surfaceHeightPx == viewHeightPx")
raise "safe area must remain physical until current density is known" unless surface.include?("private var safeAreaPx = intArrayOf")
raise "atomic viewport must use matching laid-out/surface bounds" unless surface.include?("if (surfaceWidthPx != width || surfaceHeightPx != height) return")
raise "atomic viewport must convert physical insets with current density" unless surface.match?(/nativeResizeWithSafeArea\(.*?surfaceWidthPx \/ viewportDensity.*?surfaceHeightPx \/ viewportDensity.*?safeAreaPx\[0\] \/ viewportDensity.*?safeAreaPx\[3\] \/ viewportDensity/m)
raise "resource density must remain pending until atomic commit" unless input_state.include?("pendingDensity") && input_state.include?("committedDensity")
raise "input density must publish only after successful atomic viewport" unless surface.match?(/val status = OpNative\.nativeResizeWithSafeArea\(.*?commitIfSuccessful\(status\)/m)
raise "input must be blocked while viewport geometry is split" unless input_state.include?("if (inputBlocked) return false")
raise "interrupted physical stream tail must stay suppressed after commit" unless surface.include?("viewportInputState.acceptsTouch(event.actionMasked, event.pointerCount)") && input_state.include?("suppressPhysicalStream")
raise "geometry transition must cancel active editor and viewer streams" unless surface.match?(/markViewportInputPending.*?nativeEditorCancelGestureAt\(engine, uptimeClockMs\(\).*?nativePointer\(engine, 0, PHASE_CANCEL/m)
raise "Kotlin JNI bridge must expose transform ownership" unless native.include?("external fun nativeEditorBeginTransform(engine: Long, x: Float, y: Float): Int")
raise "second-finger Down must latch transform after cancelling the press" unless surface_touch.match?(/ACTION_POINTER_DOWN.*?nativeEditorCancelGestureAt\(\s*view\.editorEngine\(\),\s*uptimeClockMs\(\).*?lastMidX = midX.*?lastMidY = midY.*?nativeEditorBeginTransform/m)
raise "pointer Up must end transform ownership" unless surface_touch.match?(/ACTION_POINTER_UP.*?twoFingerActive.*?nativeEditorCancelGestureAt\(\s*view\.editorEngine\(\),\s*uptimeClockMs\(\).*/m)
raise "remaining pointer Up must not release a cancelled press" unless surface_touch.match?(/editorReleaseSuppressed = true.*?!longPressFired && !editorReleaseSuppressed.*?nativeEditorReleaseAt\(view\.editorEngine\(\), x, y, event\.eventTime\)/m)


# Factual pointer clock: every editor press/move/release carries the event's
# own MotionEvent.eventTime, and every synthetic Cancel (geometry transition,
# two-finger takeover, long-press paste, ACTION_CANCEL) carries the SAME
# monotonic boot-uptime domain via SystemClock.uptimeMillis — the engine must
# never see an epoch clock on the editor preview pointer path.
raise "editor press must carry MotionEvent.eventTime" unless surface_touch.match?(
  /nativeEditorPressAt\(\s*view\.editorEngine\(\),\s*event\.x \/ inputDensity,\s*event\.y \/ inputDensity,\s*event\.eventTime,/m
)
raise "editor move must carry MotionEvent.eventTime" unless surface_touch.match?(
  /nativeEditorMoveAt\(view\.editorEngine\(\), x, y, event\.eventTime\)/m
)
raise "editor release must carry MotionEvent.eventTime" unless surface_touch.match?(
  /nativeEditorReleaseAt\(view\.editorEngine\(\), x, y, event\.eventTime\)/m
)
raise "ACTION_CANCEL must carry MotionEvent.eventTime" unless surface_touch.match?(
  /ACTION_CANCEL.*?nativeEditorCancelGestureAt\(view\.editorEngine\(\), event\.eventTime\).*?resetTracking/m
)
raise "synthetic cancels must use SystemClock.uptimeMillis" unless surface_touch.include?(
  "import android.os.SystemClock"
) && surface_touch.match?(/internal fun uptimeClockMs\(\): Long = SystemClock\.uptimeMillis\(\)/m)
raise "the JNI bridge must declare the timestamped editor entry points" unless
  native.include?("external fun nativeEditorPressAt(engine: Long, x: Float, y: Float, tMs: Long): Int") &&
  native.include?("external fun nativeEditorMoveAt(engine: Long, x: Float, y: Float, tMs: Long): Int") &&
  native.include?("external fun nativeEditorReleaseAt(engine: Long, x: Float, y: Float, tMs: Long): Int") &&
  native.include?("external fun nativeEditorCancelGestureAt(engine: Long, tMs: Long): Int")

puts "Android edge-to-edge safe-area contract validates"
