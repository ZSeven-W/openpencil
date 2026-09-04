# frozen_string_literal: true

require "rexml/document"

player_dir = File.expand_path("..", __dir__)
source_dir = File.join(player_dir, "app/src/main/kotlin/tech/zseven/openpencil")
manifest = REXML::Document.new(
  File.read(File.join(player_dir, "app/src/main/AndroidManifest.xml")),
)
activity = File.read(File.join(source_dir, "MainActivity.kt"))
surface = File.read(File.join(source_dir, "OpSurfaceView.kt"))
surface_touch = File.read(File.join(source_dir, "OpSurfaceViewEditorTouch.kt"))
native = File.read(File.join(source_dir, "OpNative.kt"))
callbacks = File.read(File.join(source_dir, "OpCallbacksImpl.kt"))
controller = File.read(File.join(source_dir, "BackgroundGenerationController.kt"))
service = File.read(File.join(source_dir, "BackgroundGenerationService.kt"))
state = File.read(File.join(source_dir, "BackgroundWorkState.kt"))
notifications = File.read(File.join(source_dir, "BackgroundGenerationNotifications.kt"))
state_tests = File.read(
  File.join(player_dir, "app/src/test/kotlin/tech/zseven/openpencil/BackgroundWorkStateTest.kt"),
)

permissions = REXML::XPath.match(manifest, "/manifest/uses-permission").map do |permission|
  permission.attributes["android:name"]
end
%w[
  android.permission.FOREGROUND_SERVICE
  android.permission.FOREGROUND_SERVICE_DATA_SYNC
  android.permission.POST_NOTIFICATIONS
  android.permission.WAKE_LOCK
].each do |permission|
  raise "background generation permission missing: #{permission}" unless permissions.include?(permission)
end

service_element = REXML::XPath.first(
  manifest,
  "/manifest/application/service[@android:name='.BackgroundGenerationService']",
)
raise "background generation service missing" unless service_element
raise "background service must not be exported" unless service_element.attributes["android:exported"] == "false"
raise "background service must use dataSync" unless service_element.attributes["android:foregroundServiceType"] == "dataSync"
raise "background service must stay in the app process" if service_element.attributes["android:process"]

frame_index = native.index("external fun nativeFrame")
has_work_index = native.index("external fun nativeHasBackgroundWork")
tick_index = native.index("external fun nativeBackgroundTick")
pointer_index = native.index("external fun nativePointer")
raise "background JNI methods missing" unless [frame_index, has_work_index, tick_index, pointer_index].all?
raise "background JNI order drifted" unless frame_index < has_work_index && has_work_index < tick_index && tick_index < pointer_index
raise "background tick must carry monotonic time" unless native.include?("nativeBackgroundTick(engine: Long, nowMs: Long): Boolean")

raise "Activity must declare visible ownership in onStart" unless activity.match?(/onStart.*?setActivityVisible\(this, true\)/m)
raise "Activity must probe before onPause returns" unless activity.match?(/onPause.*?prepareForBackground\(\).*?super\.onPause/m)
raise "Activity must close the start exemption in onStop" unless activity.match?(/onStop.*?setActivityVisible\(this, false\)/m)
raise "notification permission must be contextual" unless activity.include?("setBackgroundWorkActivationHandler(::requestBackgroundNotificationPermission)")
raise "notification denial must not stop the FGS" unless activity.include?("FGS remains visible in Task Manager")

raise "successful foreground frames must observe work and may request permission" unless surface.match?(/nativeFrame.*?0 -> \{.*?observeBackgroundGeneration\(allowPermissionPrompt = true\)/m)
raise "failed or suspended frames must not run shell bridges" unless surface.match?(/when \(status\).*?GPU_ERROR -> recoverGpu\(\).*?0 -> \{.*?syncSystemChromeAppearance\(\).*?ime\.sync\(\).*?drainCopyText\(\).*?pollShellAction\(\)/m)
raise "surface teardown must close the frame gate before suspend" unless surface.match?(/surfaceDestroyed.*?closeSurfaceFrameGate\(\).*?nativeSuspend\(engine\).*?markSurfaceSuspended\(engine\)/m)
raise "onPause probe must not launch notification permission UI" unless surface.match?(/prepareForBackground.*?observeBackgroundGeneration\(allowPermissionPrompt = false\)/m)
raise "surface resume must close the background gate first" unless surface.match?(/markSurfaceResuming\(context, engine\).*?attachOrResume/m)
raise "surface readiness must open only after attach/resume" unless surface.match?(/if \(!attachOrResume.*?return.*?openSurfaceFrameGate\(\)/m)
raise "surface frame requests need a generation token" unless surface.include?("surfaceFrameEpoch") && surface.include?("scheduledFrameEpoch")
raise "queued frame requests must capture and recheck their generation" unless surface.match?(/fun requestFrame\(\).*?requestedEpoch = surfaceFrameEpoch.*?requestFrame\(requestedEpoch\).*?private fun requestFrame\(requestedEpoch: Long\).*?requestedEpoch != surfaceFrameEpoch.*?postFrameCallback/m)
raise "queued frame callbacks must recheck their generation" unless surface.match?(/frameCallback.*?requestedEpoch = scheduledFrameEpoch.*?requestedEpoch != surfaceFrameEpoch.*?nativeFrame/m)
raise "frame-gate close must cancel and invalidate a queued callback" unless surface.match?(/closeSurfaceFrameGate.*?surfaceReady = false.*?surfaceFrameEpoch = nextSurfaceFrameEpoch\(\).*?removeFrameCallback\(frameCallback\).*?frameScheduled = false/m)
raise "delayed frame wakes must retain their original generation" unless surface.match?(/scheduleFrame\(delayMs: Long\).*?requestedEpoch = surfaceFrameEpoch.*?scheduleFrameForEpoch\(delayMs, requestedEpoch\)/m) && surface.match?(/scheduleFrameForEpoch.*?postDelayed\(\{ requestFrame\(requestedEpoch\) \}, delayMs\)/m)
raise "queued viewport updates must retain their original generation" unless surface.match?(/scheduleViewportUpdate\(requestedEpoch: Long\).*?requestedEpoch != surfaceFrameEpoch.*?viewportUpdateEpoch = requestedEpoch/m) && surface.match?(/onPreDraw.*?requestedEpoch = viewportUpdateEpoch.*?requestedEpoch != surfaceFrameEpoch/m)
raise "surfaceChanged must retry an unopened same-size Surface" unless surface.match?(/surfaceChanged.*?if \(!surfaceReady \|\| extentChanged\).*?markSurfaceResuming\(context, engine\).*?if \(attachedOnce\) \{\s*cancelStreamsBeforeSuspend\(\)\s*OpNative\.nativeSuspend\(engine\)\s*\}.*?attachOrResume\(holder\.surface\).*?openSurfaceFrameGate\(\)/m)
raise "GPU recovery must close before validity checks and gate background work" unless surface.match?(/recoverGpu.*?closeSurfaceFrameGate\(\).*?markSurfaceResuming\(context, engine\).*?holder\.surface.*?isValid/m)
# Every suspend barrier cancels the live gesture stream (uptime clock) and
# clears local touch tracking BEFORE the blocking nativeSuspend.
raise "surface teardown must cancel a pending long press before suspend" unless surface.match?(/surfaceDestroyed.*?editorTouch\.resetTracking\(\).*?cancelStreamsBeforeSuspend\(\).*?nativeSuspend\(engine\)/m)
raise "a stale long press must not enter JNI while suspended" unless surface_touch.match?(/fun fireLongPress.*?!view\.isFrameGateOpen \|\| view\.editorEngine\(\) == 0L\) return.*?nativeEditorRightPress/m)
raise "Activity destruction must close frames before suspend" unless surface.match?(/fun destroy\(\).*?closeSurfaceFrameGate\(\).*?nativeSuspend\(engine\).*?markSurfaceSuspended\(engine\).*?releaseView/m)
raise "destroy must release through the process owner" unless surface.include?("BackgroundGenerationController.releaseView(context, engine, this)")

raise "native callback receiver must not strongly own a View" unless callbacks.include?("WeakReference<OpSurfaceView>")
raise "callback receiver must support View adoption" unless callbacks.include?("fun attach(view: OpSurfaceView)") && callbacks.include?("fun detach(view: OpSurfaceView)")
raise "remote image fetch must survive View recreation" unless callbacks.include?("OpenPencilRemoteImage") && callbacks.include?("nativeRemoteImageResult")
raise "controller must retain the stable callback with the engine" unless controller.include?("BackgroundEngineLease") && controller.include?("receiver.attach(view)")
raise "resume/tick ownership must share one monitor" unless controller.match?(/markSurfaceResuming.*?synchronized\(monitor\)/m) && controller.match?(/pumpBackground.*?synchronized\(monitor\)/m)
raise "detached completion must have bounded cleanup" unless controller.include?("RETAINED_RESULT_TIMEOUT_MINUTES") && controller.include?("expireRetainedEngine")
raise "generation state must mint process-monotonic epochs" unless state.include?("lastIssuedEpoch") && state.include?("startServiceEpoch") && state.include?("stopServiceEpoch")
raise "stale timeout must compare its service epoch" unless state.match?(/serviceTimedOut\(epoch: Long\).*?epoch != serviceEpoch/m)

raise "service must not restart around a stale native handle" unless service.include?("return START_NOT_STICKY")
raise "background JNI must run off the Service main thread" unless service.include?("newSingleThreadScheduledExecutor") && service.include?("OpenPencilBackgroundGeneration")
raise "service background path must use render-free tick" unless controller.include?("OpNative.nativeBackgroundTick")
raise "service background path must not render frames" if service.include?("nativeFrame") || controller.include?("nativeFrame")
raise "worker cadence must remain low" unless service.include?("BACKGROUND_TICK_INTERVAL_MS = 500L")
raise "service start Intent must carry its epoch" unless controller.include?("EXTRA_SERVICE_EPOCH") && service.include?("getLongExtra(EXTRA_SERVICE_EPOCH")
raise "a stale service command must keep its immutable Intent epoch" unless service.match?(/requestedEpoch.*?requestedEpoch != currentEpoch.*?stopSelfResult\(startId\)/m) && service.include?("reportStartFailure(requestedEpoch)")
raise "target 35 timeout must consume an exact start-id" unless service.match?(/onTimeout.*?takeStartId\(startId\).*?stopRunImmediately\(run\).*?reportTimeout\(run\.epoch\)/m)
raise "stale stop must consume only an exact service epoch" unless service.match?(/requestEpochStop.*?takeEpoch\(serviceEpoch\)/m)
raise "service stop must preserve newer Android start-ids" unless service.include?("stopSelfResult(run.startId)")
raise "controller must never globally stop a newer service" if controller.include?("stopService(")
raise "service must release wake lock on every teardown" unless service.match?(/onDestroy.*?releaseWakeLock\(\)/m)
raise "wake lock must be bounded" unless service.include?("acquire(WAKE_LOCK_TIMEOUT_MS)") && service.include?("WAKE_LOCK_RENEW_INTERVAL_MS")
raise "wake lock must only follow suspended pump ownership" unless service.match?(/!BackgroundGenerationController\.needsBackgroundPump\(serviceEpoch\).*?releaseWakeLock\(\).*?renewWakeLockIfNeeded\(\)/m)
raise "wake-lock failure must not silently kill the scheduled pump" unless service.match?(/try \{\s*renewWakeLockIfNeeded\(\).*?catch \(error: RuntimeException\).*?requestPlatformStop\(serviceEpoch\)/m)
raise "static Service ownership must be weak" unless service.include?("WeakReference<BackgroundGenerationService>")
raise "JVM tests must interleave old completion with a new service run" unless state_tests.include?("delayed_completed_transition_cannot_stop_the_next_generation")
raise "JVM tests must interleave old timeout reporting with a new generation" unless state_tests.include?("delayed_timeout_report_cannot_pause_or_stop_the_next_generation")

raise "ongoing notification must return to the editor" unless notifications.include?("returnToEditorIntent(context)")
raise "ongoing notification must remain visible" unless notifications.include?(".setOngoing(true)")
raise "foreground completion must not always notify" unless controller.include?("if (transition.notifyCompletion)")
raise "foreground recovery must clear a stale paused notification" unless controller.match?(/setActivityVisible.*?visible.*?dismissPaused/m) && notifications.include?("cancel(PAUSED_ID)")
raise "notification must not expose an unwired cancel action" if notifications.include?("ACTION_CANCEL_GENERATION")

puts "Android background generation contract validates"
