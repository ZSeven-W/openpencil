# OpenPencilPlayer (Android)

A thin Android host for the OpenPencil engine, consuming `crates/op-engine-jni`'s
`dev.openpencil.player.OpNative` C-ABI surface. The engine renders through
EGL/GLES onto a `SurfaceView`; the shell owns lifecycle, insets, touch, and
the frame pump. Gestures are interpreted by the engine: single-finger tap
selects the topmost node under the finger, single-finger drag pans, two-finger
pinch zooms around the pinch midpoint. The engine paints the document's
active page with the exact painter the desktop editor canvas uses.

## Build + install

Requires the Android SDK + NDK and `cargo-ndk` (`cargo install cargo-ndk`).
Uses the Gradle wrapper (Gradle 8.14.3); point Gradle at a JDK 17+ (Android
Studio's bundled JBR works):

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/<version>"

# Build the cdylib into jniLibs (arm64-v8a + x86_64), then install:
# `editor` forwards to op-engine-ffi/editor (full desktop chrome).
cargo ndk -t arm64-v8a -t x86_64 -o packaging/android-player/app/src/main/jniLibs \
  build -p op-engine-jni --features gl,editor
cd packaging/android-player && ./gradlew installDebug && cd -
```

## Run

```bash
adb shell am start -n dev.openpencil.player/.MainActivity
adb logcat -s OpenPencilPlayer:V OpJni:V AndroidRuntime:E libEGL:W
```

The app loads the bundled PowerPoint demo at `assets/ppt-demo.op` by default: a
six-slide, 16:9 OpenPencil presentation deck derived from
`crates/op-editor-core/assets/scene_templates/slide-deck.op` and pinned to the
`corporate-blue-light` style guide. Pass the asset name without the `.op`
suffix in the existing `doc` intent extra to load another bundled document, for
example:

```bash
adb shell am start -n dev.openpencil.player/.MainActivity --es doc sample
```

## What the shell does / does not own

- `OpSurfaceView` — create/attach/resume/suspend/resize on
  `SurfaceHolder` events, Choreographer frame pump driven by the
  `onNeedsRedraw` upcall, one suspend→resume GPU-error recovery per
  surface generation, touch forwarding in logical px (top-left origin),
  inset replay after create/attach/resize.
- `MainActivity` — transparent edge-to-edge window with a continuous dark
  backdrop; four-sided system-bar/cutout insets move only interactive editor
  chrome while IME occlusion remains a separate logical-pixel channel. This
  covers portrait/landscape cutouts plus gesture and three-button navigation
  without padding or resizing the `SurfaceView`; `nativeDestroy` runs on
  destroy.
- `OpNative` / `OpCallbacks` — the JNI surface contract with
  `crates/op-engine-jni` (engine-thread upcalls, blocking barriers for
  lifecycle calls).

Everything else — document loading/layout, painting, gesture
interpretation, fit-to-view, selection — lives in the engine
(`crates/op-engine-ffi`).
