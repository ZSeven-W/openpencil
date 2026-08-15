# OpenPencil iOS Player

This directory is a source-only SwiftUI/UIKit shell for `op-engine-ffi`. XcodeGen creates the project; the Rust archive remains an external build input and is not copied into this directory.

The app loads `Resources/ppt-demo.op` by default: a six-slide, 16:9 OpenPencil presentation deck. Pass the launch argument `-doc sample` to fall back to `Resources/sample.op` (a byte-for-byte copy of `crates/op-editor-core/assets/scene_templates/daily-sign-card.op`); more generally, `-doc <name>` selects any bundled `<name>.op` document. `OpPlayerView` owns its `CAMetalLayer`; the pointer passed to OpenPencil is borrowed and is valid until `op_suspend`/`op_destroy` returns. UIKit, CADisplayLink, all engine calls, and all callback reactions run on the main thread. Callback payloads are copied synchronously, then reactions are dispatched asynchronously so callbacks never re-enter the ABI.

Gestures are interpreted by the engine, not the shell: single-finger tap selects the topmost node under the finger, single-finger drag pans, two-finger pinch zooms around the pinch midpoint. The engine paints the document's active page with the exact painter the desktop editor canvas uses. In editor mode, the engine-painted **Open File** action asks this thin shell to present the system Files picker; selecting an `.op` or `.pen` file validates and opens it in the existing engine, while a rejected file leaves the current document unchanged.

## Build inputs

From the repository root, build the archive matching the destination if it is
not already present:

```bash
# `editor` enables the full desktop chrome (viewer-only without it).
cargo build -p op-engine-ffi --release --target aarch64-apple-ios-sim --features metal,editor
cargo build -p op-engine-ffi --release --target aarch64-apple-ios --features metal,editor
```

The resulting archives are relative to the repository root:

- Simulator: `target/aarch64-apple-ios-sim/release/libop_engine_ffi.a`
- Device: `target/aarch64-apple-ios/release/libop_engine_ffi.a`

Authentication is an optional, explicit final-link input. A normal source
checkout leaves `OP_AUTH_ARCHIVE` empty and uses the public stub. For local
login work, build a **Debug** engine against a private ABI-v2 or ABI-v3 archive:

```bash
scripts/build-mobile-auth-dev.sh \
  --platform ios-simulator \
  --archive /absolute/path/to/libop_auth.a \
  --abi 3
```

Then pass that exact archive to Xcode as `OP_AUTH_ARCHIVE` and repeat the ABI
with `OPENPENCIL_DEV_OP_AUTH_ABI_VERSION`. The project pre-build gate accepts
an unsigned archive only for `CONFIGURATION=Debug`. Release linking accepts
only the adopted ABI-v3 matrix under `crates/op-auth-bridge/prebuilt/`: every
target is SHA-pinned and Ed25519-signed, while the source-owned
`AUTH-RELEASE-POLICY` pins the exact complete matrix and private build identity.
The matrix's signed `VERSION` and `openpencil_revision` record its own release
provenance; they do not have to equal the consuming OpenPencil version or
commit. Never copy a local private archive into this source tree or into an
Xcode resource phase.

The protected production flow rebuilds, audits, and signs all ten immutable
ABI-v3 candidates together, including `aarch64-apple-ios`,
`aarch64-apple-ios-sim`, both Android targets, and all six desktop targets.
App Store/TestFlight builds verify the adopted policy, the complete signed
matrix, and Cargo's actual ABI-v3 link selection before staging the exact
device archive for the final Xcode link. A missing target, policy, digest,
signature, or hardening mismatch, incomplete matrix, ABI downgrade, or stub
fallback stops the release before upload. OpenPencil-only source changes and
version bumps may reuse the adopted matrix; an op-platform, ABI, toolchain, or
hardening change requires a newly signed matrix and reviewed policy adoption.

## App Store / TestFlight release

The formal `v*` Rust release calls `.github/workflows/ios-app-store.yml` with
the exact release ref and source commit. The iOS upload is intentionally
independent of the GitHub Release asset job: an App Store Connect or review
failure does not prevent the desktop and Android artifacts from being
published. The same workflow also has a manual entry point for uploading a
replacement build from an explicitly selected source SHA/ref pair. GitHub exposes that
manual entry point only after the workflow path has been registered on the
repository's default branch.

The protected `testflight` environment owns the Apple Distribution
certificate, App Store provisioning profile, and App Store Connect API key.
The reviewed App Store encryption questionnaire concluded that no export
compliance documents are required. `project.yml` therefore source-controls
`ITSAppUsesNonExemptEncryption=NO`; the app and release workflow deliberately
do not carry an `ITSEncryptionExportComplianceCode`. Keep this declaration in
sync with the shipping cryptography and storefront availability before
changing algorithms or distribution regions.

Every upload derives a monotonically increasing three-component build number
from UTC epoch minutes. The workflow validates the exact remote ref, signed
ten-target Auth matrix, Xcode/iOS SDK floor, archived bundle identifier, signing
entitlements, encryption declarations, and final upload result before it can
report success. It uploads the build to App Store Connect/TestFlight but does
not automatically submit a store review.

Generate the project (do this again after changing `project.yml`):

```bash
cd packaging/ios-player
xcodegen generate --spec project.yml
```

## Simulator build and run

On this host, pass the SDK, destination, Rust archive, and linker flags explicitly. Link the archive by path: `-lop_engine_ffi` can select the adjacent simulator dylib and leave the app with a non-redistributable local dependency. Replace `<sim-id>` with an installed iOS simulator UUID:

```bash
cd packaging/ios-player
xcodebuild \
  -project OpenPencilPlayer.xcodeproj \
  -scheme OpenPencilPlayer \
  -configuration Release \
  -sdk iphonesimulator26.4 \
  -destination 'platform=iOS Simulator,id=<sim-id>' \
  -derivedDataPath "$PWD/.derived-data" \
  HEADER_SEARCH_PATHS="$PWD/../../crates/op-engine-ffi/include" \
  OTHER_LDFLAGS="$PWD/../../target/aarch64-apple-ios-sim/release/libop_engine_ffi.a -lc++ -framework CoreFoundation -framework CoreGraphics -framework CoreText -framework ImageIO -framework MobileCoreServices -framework UIKit -framework Foundation -framework Metal -framework QuartzCore -framework Security" \
  build

xcrun simctl install <sim-id> "$PWD/.derived-data/Build/Products/Release-iphonesimulator/OpenPencilPlayer.app"
xcrun simctl launch <sim-id> tech.zseven.openpencil
```

## Real-device build

Use the device archive and replace `<device-id>` with the attached phone's destination identifier. Signing values may be supplied by the orchestrator or selected in Xcode:

```bash
cd packaging/ios-player
xcodebuild \
  -project OpenPencilPlayer.xcodeproj \
  -scheme OpenPencilPlayer \
  -configuration Release \
  -sdk iphoneos26.4 \
  -destination 'platform=iOS,id=<device-id>' \
  -derivedDataPath "$PWD/.derived-data-device" \
  HEADER_SEARCH_PATHS="$PWD/../../crates/op-engine-ffi/include" \
  OTHER_LDFLAGS="$PWD/../../target/aarch64-apple-ios/release/libop_engine_ffi.a -lc++ -framework CoreFoundation -framework CoreGraphics -framework CoreText -framework ImageIO -framework MobileCoreServices -framework UIKit -framework Foundation -framework Metal -framework QuartzCore -framework Security" \
  build
```

For an authenticated local simulator build, use the Debug engine path and add:

```text
-configuration Debug
OP_AUTH_ARCHIVE=/absolute/path/to/libop_auth.a
OPENPENCIL_DEV_OP_AUTH_ABI_VERSION=3
```

Do not add those development settings to a Release build; the gate rejects
them before the final link.

## Coordinate and lifecycle contract

The engine viewport is `view.bounds.size` in logical UIKit points. `CAMetalLayer.drawableSize` is `bounds × contentsScale` in physical pixels, but touch locations are passed directly from `UITouch.location(in:)` without multiplying by scale. Therefore pointer input and all returned geometry share surface-logical points with a top-left origin.

The Metal surface stays edge-to-edge, including the status-bar, cutout, and Home Indicator regions. UIKit safe-area insets are forwarded separately so the engine places interactive chrome and content inside the usable rectangle without turning those regions into a padded outer container; the surrounding canvas backdrop remains visually continuous. The engine exposes whether that backdrop needs light system glyphs, and the shell updates `window.overrideUserInterfaceStyle` only when the preference changes. This keeps the status bar and Home Indicator legible when the editor switches between Light and Dark themes without imposing a separate native theme on the Rust UI. The SwiftUI root explicitly ignores all safe-area regions, including the keyboard, so presenting the IME never shifts or resizes the whole editor. Only a bottom-docked software keyboard is forwarded as local occlusion; floating and split iPad keyboards do not create a full-width inset.

The first layout configures the Metal layer, creates the engine, and attaches the borrowed layer with UIKit's current viewport tuple. Later layout and safe-area callbacks enter an epoch barrier; every path requires the same tuple on two consecutive display frames, including bounds-only iPad/Stage Manager changes where safe insets remain constant and UIKit may omit a safe-area callback. The resulting bounds, scale, and insets are sent through one atomic viewport call, so a delayed safe-area callback cannot expose a transient responsive size class derived from new bounds with stale insets. Keyboard occlusion remains a separate logical-point channel. Backgrounding suspends the borrowed surface; foregrounding resumes it. Teardown synchronously suspends and destroys the engine.

CADisplayLink is paused before every frame. A redraw callback caused by a mutation arms the next display tick (the viewer engine schedules no timed wakes of its own). Touch timestamps and frame timestamps both use `CACurrentMediaTime() × 1000`.

## Source-only validation

This does not generate a project or link an app. It checks the YAML/resource contract, compiles the bridging header, and type-checks every Swift source against the iOS simulator SDK and the checked-in `op_engine.h`:

```bash
bash packaging/ios-player/Tests/validate_sources.sh
```
