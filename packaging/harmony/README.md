# OpenPencil HarmonyOS Player

A thin ArkUI shell around the OpenPencil Rust engine, targeting **HarmonyOS 5
(API 12+)** as ONE app for `phone`, `tablet`, and `2in1` (PC). The engine
paints the entire UI inside a single `XComponent` surface; ArkTS forwards
lifecycle, raw pointers, keys, the IME, and the file pickers, and nothing
else. Gestures are interpreted by the engine (single-finger tap selects,
single-finger drag pans, two-finger pinch zooms around the midpoint) exactly
as on Android and iOS.

The full editor starts with the engine's canonical untitled `.op` document.
The engine-painted **Open File** action asks this shell to present the system
document picker; selecting an `.op` or `.pen` file validates and opens it in
the existing engine. The **Export** action renders PNG, JPEG, SVG, or PDF and
presents the system save picker; WebP is hidden, exactly as on iOS, because
the pinned mobile Skia archive has no WebP encoder.
The engine-painted **Import Image or SVG** action presents one system document
picker for PNG, JPEG, GIF, WebP, or SVG. The shell preserves the picked file
name and rejects payloads above 32 MiB before returning them to Rust.
The desktop-class **Code** panel reuses that frozen-export protocol for
framework source files and generated/AI bundle ZIPs.

## Layout

```text
packaging/harmony/
├── AppScope/app.json5                     bundleName tech.zseven.openpencil
├── build-profile.json5                    products, SDK floor, signing placeholder
├── hvigorfile.ts / oh-package.json5
├── entry/
│   ├── build-profile.json5                prebuilt-.so packaging (no CMake target)
│   ├── oh-package.json5                   wires the libopenpencil.so type package
│   ├── libs/arm64-v8a/libopenpencil.so    ← produced by scripts/build-ohos.sh
│   └── src/main/
│       ├── module.json5                   deviceTypes [phone, tablet, 2in1]
│       ├── cpp/types/libopenpencil/       index.d.ts: the NAPI contract
│       ├── ets/entryability/EntryAbility.ets
│       ├── ets/pages/Index.ets            the XComponent + shell-action sink
│       ├── ets/common/*.ets               engine host, input, IME, pickers, dialogs
│       └── resources/{base,en_US,zh_CN}/
└── Tests/*.rb                             local source + packaged-artifact gate
```

## Prerequisites

1. **DevEco Studio 5** with the **HarmonyOS 5.0.0(12) SDK** installed through
   *SDK Manager*. Select its **Ets (ArkTS)**, **Toolchains**, and **Native**
   components; the project profile is pinned to
   `compatibleSdkVersion: "5.0.0(12)"`.
2. Export the matching SDK's OpenHarmony root (the directory that contains
   `native/`) for the Rust build:

   ```bash
   # Replace <HarmonyOS-5.0.0-sdk> with the SDK Manager installation.
   export OHOS_NDK_HOME="<HarmonyOS-5.0.0-sdk>/openharmony"
   export PATH="$OHOS_NDK_HOME/native/llvm/bin:$PATH"
   ```
3. A **Huawei developer account** for signing (see *Signing* below). Nothing
   installs on a real device or on the emulator unsigned.

## Build the engine library

The ArkTS shell does **not** compile any native code. It consumes a prebuilt
`libopenpencil.so` (crate `op-engine-napi`), built by the OHOS build script:

```bash
# From the repository root, with OHOS_NDK_HOME exported.
bash scripts/build-ohos.sh
```

After Cargo succeeds, the script verifies that the new module contains the
`editorImportImageOrSvg`, `hasBackgroundWork`, `backgroundTick`, and the four
timestamped editor-pointer (`editorPressAt` / `editorMoveAt` /
`editorReleaseAt` / `editorCancelGestureAt`) NAPI registration markers. It then installs the library through a temporary file
and an atomic rename to:

```text
packaging/harmony/entry/libs/arm64-v8a/libopenpencil.so
```

A failed build, marker check, or copy leaves the previous HAP payload
untouched. That does **not** make an old payload releasable: the project
contract below reads the packaged ELF and fails with `stale HarmonyOS native
artifact` until all three markers are present. Do not assemble a HAP while
that gate is red.

hvigor packages everything under `entry/libs/<abi>/` into the HAP, so no
`externalNativeOptions`/CMake entry is declared in `entry/build-profile.json5`.
Only `arm64-v8a` is shipped: every HarmonyOS 5 device is 64-bit ARM. If you
add `x86_64` for the emulator, drop it in `entry/libs/x86_64/`.

The ArkTS side is compiled against `entry/src/main/cpp/types/libopenpencil/index.d.ts`.
That file is the **contract with the Rust crate** — every function is the
Android JNI surface (`packaging/android/.../OpNative.kt`) with the `native`
prefix removed and the first letter lower-cased (`nativeCreate` → `create`,
`nativeEditorTakeShellAction` → `editorTakeShellAction`, …), same arguments
and semantics. Two platform substitutions: `android.view.Surface` becomes the
XComponent id (`string`, resolved natively through `OH_NativeXComponent`), and
`ByteArray`/`FloatArray` become `ArrayBuffer`/`Float32Array`. The engine
handle is a `number` (an int64 crossing NAPI becomes a JS double, exact up to
2^53; aarch64 user-space pointers stay inside 48 bits).

Foreground frames are coalesced onto ArkUI `displaySync`; `frame()` has one
call site and is stopped before the XComponent surface is suspended. The
ArkTS shell also drains engine shell actions after every interaction plus on a
100 ms safety-net timer.

## Background generation

When render-free AI generation becomes active, the foreground shell asks
HarmonyOS for a transient suspension delay with `requestSuspendDelay`. On page
hide or surface loss it first stops `displaySync`, then suspends the GPU
surface, and only then starts a 100 ms `backgroundTick` timer. That tick pumps
chat/image-search services and persists generated document revisions without
calling `frame()` or touching the suspended GPU surface.

This is intentionally a **finite** continuation window, not an indefinite
keepalive. HarmonyOS grants at most 3 minutes normally and at most 1 minute on
low battery, and may refuse a request when the app's transient-task quota is
unavailable. Completion, foreground resume, teardown, and the system expiry
callback all stop the timer and cancel the grant immediately. Regular 100 ms
ticks have already persisted each applied revision, so the deadline callback
does no additional tool or file work. It leaves unfinished generation pending;
returning to OpenPencil resumes it and shows an honest "background time ended"
notice.

Transient suspension delay needs no manifest permission or persistent
notification. OpenPencil does not mislabel AI generation as a `DATA_TRANSFER`
continuous task, and does not use `TASK_KEEPING`, which is limited to eligible
2-in-1 computing-task scenarios.

## Signing

`build-profile.json5` deliberately ships with an **empty** `signingConfigs`
array — no certificate, profile, or key material is checked in. In DevEco
Studio, open *File ▸ Project Structure ▸ Signing Configs* and either enable
*Automatically generate signature* (requires a logged-in Huawei developer
account) or point the config at your own `.cer` / `.p7b` / `.p12`. DevEco
writes the resulting `signingConfigs` entry back into `build-profile.json5`;
do not commit that entry.

## Build and install

The `hvigorw` / `hvigorw.bat` wrapper scripts are generated by DevEco Studio
the first time the project is opened (they are not checked in); the pinned
hvigor + plugin versions live in `hvigor/hvigor-config.json5`.

```bash
cd packaging/harmony

# One-time: fetch the proprietary native sign-in SDK .har packages
# (Douyin OpenSDK + Alipay AFServiceSDK) into entry/libs/, then install.
# The entry module references them as file: dependencies, so `ohpm install`
# fails until this has run once.
bash scripts/fetch-vendor-sdks.sh
ohpm install --all

# Debug HAP
hvigorw assembleHap --mode module -p product=default -p buildMode=debug

# Release HAP (requires a release signing config)
hvigorw assembleHap --mode module -p product=default -p buildMode=release

# Install on a connected device / emulator
hdc list targets
hdc install entry/build/default/outputs/default/entry-default-signed.hap

# Launch and follow the log
hdc shell aa start -a EntryAbility -b tech.zseven.openpencil
hdc hilog | grep OpenPencilPlayer
```

Building from DevEco Studio (*Build ▸ Build Hap(s)/APP(s)*) is equivalent and
handles signing interactively.

## Per-device-type notes

- **Phone** — portrait or landscape following the user's rotation setting
  (`orientation: "unspecified"`, mirroring the Android manifest, which
  declares no `screenOrientation` and handles the config change itself). The
  window is edge-to-edge: the surface spans the status bar, navigation area,
  and any display cutout, while the four safe-area bands are forwarded to the
  engine separately so only its interactive chrome avoids system UI. Showing
  the keyboard never resizes the editor — IME occlusion is a separate channel
  (`setKeyboard`), matching Android's `adjustNothing`.
- **Tablet** — same code path. Multi-window and free-form resizes publish a
  new viewport tuple through `WindowBridge`; the engine re-lays out and picks
  its own responsive size class. A connected physical keyboard is forwarded
  through `onKeyEvent` (named keys plus Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y /
  Ctrl+D).
- **2in1 (PC)** — the window is freely resizable; every resize goes through
  the same atomic `resizeWithSafeArea` call. Mouse press/move/release/hover
  are forwarded through `onMouse`/`onHover`, with right-press mapped to the
  engine's context action (touch reaches the same action via a 500 ms
  long-press). Cursor-shape control is intentionally not implemented — the
  engine paints its own affordances.

## Local verification

The local gate is a pair of Ruby contracts in the style of
`packaging/android/Tests/*.rb`. The project contract also inspects the exact
prebuilt ELF that hvigor will package, so it intentionally fails on a stale
library rather than validating sources alone:

```bash
ruby packaging/harmony/Tests/HarmonyProjectContractTests.rb
ruby packaging/harmony/Tests/HarmonyShellContractTests.rb
```

They pin the XComponent ↔ `libopenpencil.so` binding, the device-type list,
the bundle name, the render-free transient background-generation lifecycle,
all shell action codes against
`crates/op-engine-ffi/include/op_engine.h`, the empty-conduit backspace
contract, the bounded one-shot image/SVG picker, the 15-locale table
(cross-checked against the Android shell), the export format set without WebP,
and the rule that the ArkTS NAPI declaration matches the Android JNI surface
function-for-function.

The SSO section pins the parts that are easy to break silently: origins may
appear only in `SsoRegion.ets` (cross-checked against `SsoRegion.kt`, probe
target and mainland redirect host included), the configure is lazy for touch
chrome and eager for 2in1, the login presentation splits browser-vs-native on
`isDesktopClass()`, the JSON API routes match the Android client, cookies stay
an in-memory `Map`, and nothing in the auth path logs a credential.

## Limitations

- **Background generation is time-bounded by HarmonyOS.** It continues under
  a transient grant for up to 3 minutes normally (1 minute on low battery),
  then pauses safely until OpenPencil returns to the foreground. It is not a
  promise of unlimited background execution.
- **Sign-in is native, and splits by form factor.** Shell actions 2
  (OpenLoginWebView), 5 (OpenAccountCenter), and 6 (RequestLogin) are wired
  to the ZSeven SSO flow, mirroring the Android shell.
  - **2in1 (PC)** — the engine's verification URL is handed to the **system
    browser** (implicit `ohos.want.action.viewData` want): the user signs in
    and approves there, and the engine's background poll drives the close
    action. No native login page is built on a PC. The account center is a
    small native card (name, region, manage-in-browser, sign out).
  - **Phone / tablet** — a full native login page (ZSeven design: logo,
    labeled boxed inputs, gradient sign-in button, provider icon cards
    fetched from the pairing origin, region row) plus native
    registration / password-reset forms and a full-window account center.
    *This path compiles and is contract-pinned but has NOT been exercised on
    a phone emulator yet* — the available emulator is a 2in1.
  - A build with no `op-auth` backend answers `NotReady` (10); the shell then
    shows the "sign-in unavailable" notice and cancels the flow, exactly as
    before.
  - No SSO origin is hardcoded anywhere outside `ets/common/SsoRegion.ets`,
    which owns the two regional deployments and the IP probe; the Ruby
    contracts enforce that. Session cookies are memory-only and the device
    token never leaves the engine.
  - **Region switching needs a manual relaunch.** The auth runtime locks its
    origin at the first `configureAuth`, and `UIAbilityContext.restartApp` is
    API 22 while this module targets API 12 — so "restart now" terminates the
    ability and the user reopens the app.
- **Collaboration credentials are not stored.** `onCredentialLoad` returns
  null and `onCredentialStoreIfAbsent` returns false: HarmonyOS has no
  OpenPencil credential store yet, and storing a device key in plaintext
  would be worse than not having one.
- **WebP export is hidden**, matching iOS: the pinned mobile Skia archive
  ships no WebP encoder.
- **IME preedit is not painted in-canvas.** HarmonyOS's
  `InputMethodController` surfaces committed text (`insertText`) but not
  composing text in API 12, so CJK composition shows in the IME's own
  candidate window rather than inline. Wire `editorImePreedit` once
  `setPreviewText` is available on the minimum supported API level.
- **Mouse-wheel scrolling and Ctrl+wheel zoom are native-side work.** ArkUI
  exposes no wheel/axis callback for `XComponent` on API 12, and the Android
  shell has no wheel handling to mirror. The native layer should register
  `OH_NativeXComponent_RegisterUIAxisEventCallback` and map a plain wheel to
  `editor_pan` and Ctrl+wheel to `editor_pinch`, following the desktop host.
- **Untested on real hardware.** No HarmonyOS device, emulator, or DevEco
  installation was available while this shell was written; only the Ruby
  source contracts have been run. Expect first-run fixes around the
  XComponent id ↔ `OH_NativeXComponent` binding, avoid-area units, and
  IME event names.
