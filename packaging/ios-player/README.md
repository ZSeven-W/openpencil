# OpenPencil iOS Player

This directory is a source-only SwiftUI/UIKit shell for `op-engine-ffi`. XcodeGen creates the project; the Rust archive remains an external build input and is not copied into this directory.

The app loads `Resources/sample.op` (a byte-for-byte copy of `crates/op-editor-core/assets/scene_templates/daily-sign-card.op`). `OpPlayerView` owns its `CAMetalLayer`; the pointer passed to OpenPencil is borrowed and is valid until `op_suspend`/`op_destroy` returns. UIKit, CADisplayLink, all engine calls, and all callback reactions run on the main thread. Callback payloads are copied synchronously, then reactions are dispatched asynchronously so callbacks never re-enter the ABI.

Gestures are interpreted by the engine, not the shell: single-finger tap selects the topmost node under the finger, single-finger drag pans, two-finger pinch zooms around the pinch midpoint. The engine paints the document's active page with the exact painter the desktop editor canvas uses. In editor mode, the engine-painted **Open File** action asks this thin shell to present the system Files picker; selecting an `.op` or `.pen` file validates and opens it in the existing engine, while a rejected file leaves the current document unchanged.

## Build inputs

From the repository root, build the archive matching the destination if it is not already present:

```bash
cd /Users/kayshen/Workspace/ZSeven-W/openpencil
# `editor` enables the full desktop chrome (viewer-only without it).
cargo build -p op-engine-ffi --release --target aarch64-apple-ios-sim --features metal,editor
cargo build -p op-engine-ffi --release --target aarch64-apple-ios --features metal,editor
```

The resulting archives are:

- Simulator: `/Users/kayshen/Workspace/ZSeven-W/openpencil/target/aarch64-apple-ios-sim/release/libop_engine_ffi.a`
- Device: `/Users/kayshen/Workspace/ZSeven-W/openpencil/target/aarch64-apple-ios/release/libop_engine_ffi.a`

Generate the project (do this again after changing `project.yml`):

```bash
cd /Users/kayshen/Workspace/ZSeven-W/openpencil/packaging/ios-player
xcodegen generate --spec project.yml
```

## Simulator build and run

On this host, pass the SDK, destination, Rust archive, and linker flags explicitly. Link the archive by path: `-lop_engine_ffi` can select the adjacent simulator dylib and leave the app with a non-redistributable local dependency. Replace `<sim-id>` with an installed iOS simulator UUID:

```bash
cd /Users/kayshen/Workspace/ZSeven-W/openpencil/packaging/ios-player
xcodebuild \
  -project OpenPencilPlayer.xcodeproj \
  -scheme OpenPencilPlayer \
  -configuration Release \
  -sdk iphonesimulator26.4 \
  -destination 'platform=iOS Simulator,id=<sim-id>' \
  -derivedDataPath "$PWD/.derived-data" \
  HEADER_SEARCH_PATHS=/Users/kayshen/Workspace/ZSeven-W/openpencil/crates/op-engine-ffi/include \
  OTHER_LDFLAGS='/Users/kayshen/Workspace/ZSeven-W/openpencil/target/aarch64-apple-ios-sim/release/libop_engine_ffi.a -lc++ -framework CoreFoundation -framework CoreGraphics -framework CoreText -framework ImageIO -framework MobileCoreServices -framework UIKit -framework Foundation -framework Metal -framework QuartzCore' \
  build

xcrun simctl install <sim-id> "$PWD/.derived-data/Build/Products/Release-iphonesimulator/OpenPencilPlayer.app"
xcrun simctl launch <sim-id> dev.openpencil.player
```

## Real-device build

Use the device archive and replace `<device-id>` with the attached phone's destination identifier. Signing values may be supplied by the orchestrator or selected in Xcode:

```bash
cd /Users/kayshen/Workspace/ZSeven-W/openpencil/packaging/ios-player
xcodebuild \
  -project OpenPencilPlayer.xcodeproj \
  -scheme OpenPencilPlayer \
  -configuration Release \
  -sdk iphoneos26.4 \
  -destination 'platform=iOS,id=<device-id>' \
  -derivedDataPath "$PWD/.derived-data-device" \
  HEADER_SEARCH_PATHS=/Users/kayshen/Workspace/ZSeven-W/openpencil/crates/op-engine-ffi/include \
  OTHER_LDFLAGS='/Users/kayshen/Workspace/ZSeven-W/openpencil/target/aarch64-apple-ios/release/libop_engine_ffi.a -lc++ -framework CoreFoundation -framework CoreGraphics -framework CoreText -framework ImageIO -framework MobileCoreServices -framework UIKit -framework Foundation -framework Metal -framework QuartzCore' \
  build
```

## Coordinate and lifecycle contract

The engine viewport is `view.bounds.size` in logical UIKit points. `CAMetalLayer.drawableSize` is `bounds × contentsScale` in physical pixels, but touch locations are passed directly from `UITouch.location(in:)` without multiplying by scale. Therefore pointer input and all returned geometry share surface-logical points with a top-left origin.

`layoutSubviews` configures the Metal layer, creates the engine once, attaches the borrowed layer, and calls `op_resize` for later bounds/scale changes (including rotation). Safe-area and keyboard occlusion are separate logical-point channels. Backgrounding suspends the borrowed surface; foregrounding resumes it. Teardown synchronously suspends and destroys the engine.

CADisplayLink is paused before every frame. A redraw callback caused by a mutation arms the next display tick (the viewer engine schedules no timed wakes of its own). Touch timestamps and frame timestamps both use `CACurrentMediaTime() × 1000`.

## Source-only validation

This does not generate a project or link an app. It checks the YAML/resource contract, compiles the bridging header, and type-checks every Swift source against the iOS simulator SDK and the checked-in `op_engine.h`:

```bash
cd /Users/kayshen/Workspace/ZSeven-W/openpencil
bash packaging/ios-player/Tests/validate_sources.sh
```
