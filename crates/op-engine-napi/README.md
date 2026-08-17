# op-engine-napi

The OpenHarmony (OHOS) binding for the OpenPencil engine — built as
**`libopenpencil.so`**, the native module the ArkTS app in `packaging/harmony`
loads:

```ts
import native from 'libopenpencil.so';
```

It is the OHOS twin of `crates/op-engine-jni` (Android): the same
`op-engine-ffi` C ABI, the same one-engine-thread marshalling, the same
tombstoning handle table — reached through Node-API instead of JNI.

---

## Architecture

```text
ArkTS shell (packaging/harmony)
   │  import native from 'libopenpencil.so'
   ▼
op-engine-napi                        ← this crate
   ├── module.rs        Node-API module surface + XComponent adoption at load
   ├── bindings*.rs     The exported functions (split like the JNI natives)
   ├── callbacks.rs     engine → ArkTS upcalls (napi threadsafe functions)
   ├── window.rs        Surface borrow bookkeeping per XComponent id
   ├── xcomponent.rs    Hand-rolled OH_NativeXComponent NDK declarations
   ├── hilog.rs         HiLog writer (the twin of Android's alog.rs)
   └── action.rs        Platform-free constants + event mappings (host-tested)
   ▼
op-engine-ffi (C ABI)  →  op-host-native / op-editor-* / jian-skia
```

Two pieces are **imported, not re-implemented**:

* `op_engine_jni::engine_thread` — the engine-thread queue (post / blocking
  call / drain-on-close / deferred callback-origin destroy).
* `op_engine_jni::registry` — the monotonic, tombstoning handle table.

Both are pure `std` with no JNI in them. They live in the JNI crate for
historical reasons; sharing them is deliberate, because a byte-identical copy
of 850 lines of teardown-ordering logic is exactly the kind of twin this
workspace has been burned by before (see `crates/CLAUDE.md`, "Shared host
logic"). If a third mobile binding ever appears, lift them into their own
crate rather than copying.

### Everything OHOS-specific is target-gated

Every module except `action` carries
`#![cfg(all(target_os = "linux", target_env = "ohos"))]`, so on macOS/Linux
this crate is an inert stub that still compiles, lints, and runs its contract
tests. `cargo check --workspace` and `cargo test --workspace` stay green
without an NDK.

---

## ArkTS-visible API

Names are the Kotlin `OpNative` contract translated to camelCase with the
`native` prefix dropped (`nativeEditorPress` → `editorPress`), in the same
order, followed by the OHOS-only additions. **This table is the contract**: a
unit test (`contract_tests`) fails the build if an exported name is missing
from it, or if a name is exported that it does not list.

Common conventions:

* `engine` is the handle from `create` (`0` means failure).
* An `i32` return is an `OpStatus` (`0` = Ok, `1` = InvalidArg, … `10` =
  NotReady) unless stated otherwise. **`-1` is `STATUS_CLOSING`**: the handle
  is unknown or torn down, so nothing was dispatched.
* Coordinates are surface-**logical** pixels, top-left origin.
* Sizes/insets are logical points; `dpr` converts to physical pixels.

### Lifecycle

| Function | Arguments | Returns |
| --- | --- | --- |
| `create` | `doc: Uint8Array \| null, w: number, h: number, dpr: number, callbacks: EngineCallbacks, storageRoot: string, mode: number` | `number` — engine handle, `0` on failure |
| `lastError` | `engine: number` | `string` — last error text; pass `0` to read the create failure |
| `attachSurface` | `engine: number, xcomponentId: string` | `number` |
| `suspend` | `engine: number` | `number` — blocking barrier; the GPU surface is gone when it returns |
| `resume` | `engine: number, xcomponentId: string \| null` | `number` — `null` is rejected with `-1` |
| `resize` | `engine: number, w: number, h: number, dpr: number` | `number` |
| `resizeWithSafeArea` | `engine: number, w, h, dpr, t, r, b, l: number` | `number` |
| `setSafeArea` | `engine: number, t, r, b, l: number` | `number` |
| `setKeyboard` | `engine: number, h: number` | `number` |
| `prefersLightSystemIcons` | `engine: number` | `boolean` — false on any failure |
| `frame` | `engine: number, tMs: number` | `number` — blocking barrier; the TRUE frame status |
| `pointer` | `engine: number, id: number, phase: number, x, y: number, tMs: number` | `number` |
| `destroy` | `engine: number` | `void` |

`mode` is `0` for the viewer and `1` for the full editor. In editor mode `doc`
may be `null` to open the blank starter; the viewer always needs bytes.
`storageRoot` must be an absolute app-private directory — editor mode rejects
a missing one.

`EngineCallbacks` is a plain object; every field is optional and every handler
runs on ArkUI's event loop (never inside the engine call that produced it):

| Field | Signature |
| --- | --- |
| `onNeedsRedraw` | `(hasNextWake: boolean, nextWakeMs: number) => void` |
| `onRuntimeError` | `(kind: number, message: string, source: string \| null) => void` |
| `onInputFocusChanged` | `(focused: boolean, inputKind: number, returnKeyHint: number) => void` |
| `onRemoteImageRequest` | `(requestId: number, url: string) => void` |

### Text editing + IME

| Function | Arguments | Returns |
| --- | --- | --- |
| `textBegin` | `engine: number, nodeId: string` | `number` |
| `textEnd` | `engine: number` | `number` |
| `textInsert` | `engine: number, text: string` | `number` |
| `textBackspace` | `engine: number` | `number` |
| `textDeleteForward` | `engine: number` | `number` |
| `textSetCaret` | `engine: number, offset: number, extend: boolean` | `number` |
| `textSelectRange` | `engine: number, anchor: number, focus: number` | `number` |
| `imeSetComposingText` | `engine: number, text: string, selStart: number, selEnd: number` | `number` |
| `imeCommitComposition` | `engine: number` | `number` |
| `imeCancelComposition` | `engine: number` | `number` |
| `textGetState` | `engine: number` | `TextState` |
| `textCaretRect` | `engine: number` | `number[]` — `[x, y, w, h]`, empty on failure |

Offsets here are **UTF-16 code units**. `TextState` is
`{ status, text, selectionStart, selectionEnd, hasComposing, composingStart, composingEnd }`;
only trust the fields when `status === 0`.

### Remote images / fonts / pages

| Function | Arguments | Returns |
| --- | --- | --- |
| `remoteImageResult` | `engine: number, requestId: number, bytes: Uint8Array \| null` | `number` — `null`/empty reports a failed fetch |
| `registerFont` | `engine: number, bytes: Uint8Array` | `number` |
| `getPageCount` | `engine: number` | `number` — the count, or an `OpStatus`/`-1` |
| `setActivePage` | `engine: number, index: number` | `number` |

### Full-editor mode (`--features editor`)

| Function | Arguments | Returns |
| --- | --- | --- |
| `editorPress` | `engine: number, x, y: number` | `number` |
| `editorMove` | `engine: number, x, y: number` | `number` |
| `editorRelease` | `engine: number, x, y: number` | `number` |
| `editorCancelGesture` | `engine: number` | `number` |
| `editorBeginTransform` | `engine: number, x, y: number` | `number` — pass the second finger's Down MIDPOINT |
| `editorRightPress` | `engine: number, x, y: number` | `number` |
| `editorPan` | `engine: number, x, y, dx, dy: number` | `number` — only after `editorBeginTransform` |
| `editorPinch` | `engine: number, x, y, deltaY: number` | `number` — positive zooms in; only after `editorBeginTransform` |
| `editorText` | `engine: number, text: string` | `number` |
| `editorKey` | `engine: number, key: number` | `number` — an `OpKey_*` code |
| `editorImePreedit` | `engine: number, text: string, selStart: number, selEnd: number` | `number` — offsets are BYTES here |
| `editorImeCommit` | `engine: number, text: string` | `number` |
| `editorImeFocused` | `engine: number` | `boolean` — show/hide the system keyboard |
| `editorConfigureAuth` | `engine: number, storageDir: string, deviceName: string, appVersion: string, region: number` | `number` |
| `editorTakeLoginUrl` | `engine: number` | `string \| null` — CONSUMES the pending URL |
| `editorCancelLogin` | `engine: number` | `number` |
| `editorTakeShellAction` | `engine: number` | `number` — an `OpShellAction`; negative = engine failure |
| `editorOpenDocument` | `engine: number, bytes: Uint8Array, name: string \| null` | `number` |
| `editorExportFileName` | `engine: number` | `string \| null` — does NOT consume the export |
| `editorExportToPath` | `engine: number, path: string` | `number` — target must not exist |
| `editorCancelExport` | `engine: number` | `number` |
| `editorAccountSnapshot` | `engine: number` | `string \| null` — JSON, re-read per call |
| `editorSignOut` | `engine: number` | `number` |
| `editorBeginLogin` | `engine: number` | `number` — `10` (NotReady) = stub backend, see below |
| `editorSetLocale` | `engine: number, tag: string` | `number` — BCP-47 |
| `editorLocaleCode` | `engine: number` | `string \| null` |

`region` is an `OpAuthRegion`: `0` = China, `1` = Global. An unknown value
falls back to China rather than being forwarded.

Shell action codes (`editorTakeShellAction`), from `op_engine.h`:
`0` None · `1` OpenDocument · `2` OpenLoginWebView · `3` CloseLoginWebView ·
`4` ExportDocument · `5` OpenAccountCenter · `6` RequestLogin ·
`7` OpenLanguagePicker.

Editor key codes (`editorKey`): `1` Backspace · `2` Delete · `3` Enter ·
`4` Escape · `5` Duplicate · `6` Undo · `7` Redo · `9` ArrowUp ·
`10` ArrowDown · `11` ArrowLeft · `12` ArrowRight.

### OHOS-only additions

No Android counterpart — these cover the XComponent surface model and the
2-in-1 / PC form factors where ArkUI delivers mouse and hardware-key events.

| Function | Arguments | Returns |
| --- | --- | --- |
| `setXcomponentListener` | `callback: ((event: string, xcomponentId: string, width: number, height: number) => void) \| null` | `void` |
| `pixelSize` | `engine: number` | `number[]` — `[width, height]` physical, empty on failure |
| `touchEvent` | `engine: number, id: number, touchType: number, x, y: number, tMs: number` | `number` |
| `editorOpenDocumentPath` | `engine: number, path: string` | `number` |
| `mouseMove` | `engine: number, x, y: number` | `number` |
| `mouseButton` | `engine: number, x, y: number, button: number, pressed: boolean` | `number` |
| `mouseWheel` | `engine: number, x, y, dx, dy: number, zoom: boolean` | `number` |
| `keyEvent` | `engine: number, keyCode: number, modifiers: number` | `number` — `1` (InvalidArg) = no editor binding, route as text |
| `clipboardSetText` | `engine: number, text: string` | `number` — pastes shell-read pasteboard text |
| `clipboardGetText` | `engine: number` | `string \| null` — **RESERVED, always null** |

* `setXcomponentListener` `event` is `"created"`, `"changed"`, or
  `"destroyed"`; the size is in PHYSICAL pixels. `"destroyed"` is a
  notification — the binding has already suspended any engine bound to that
  surface synchronously, because the framework only guarantees the window
  until its own callback returns. Do not race it with your own `suspend`.
* `touchEvent` takes the **ArkUI `TouchType`** ordinal (Down 0, Up 1, Move 2,
  Cancel 3) and translates it; the raw `pointer` takes `OpPointerPhase`
  (Down 0, Move 1, Up 2, Cancel 3). Up and Move are swapped between the two —
  which is exactly why `touchEvent` exists. Forward every changed touch with
  its finger id so pinch works.
* `mouseButton` takes the ArkTS `MouseButton` ordinal (0 left, 1 right,
  2 middle). Right-button DOWN becomes the context press; right-button UP and
  every other button are accepted as no-ops (`0`), not failures.
* `mouseWheel` brackets each event with begin-transform → pan/pinch →
  cancel-gesture, because the engine's pan/pinch only apply while a transform
  is captured. That bracket also cancels an in-flight pointer drag.
* `keyEvent` takes a raw HarmonyOS key code plus a modifier bitmask
  (`1` shift, `2` ctrl, `4` alt, `8` meta) and maps it to an `OpKey_*`.
  Printable characters have no binding — send them through `editorText` /
  `editorImeCommit`.
* `clipboardGetText` never returns text: `op_engine.h` has no channel for the
  engine to hand a selection back to the shell (the Android player has the
  same gap). Treat `null` as "copy unsupported"; do not branch on it.

---

## Threading rules

1. **One engine, one thread.** `create` spawns a dedicated engine thread and
   every entry point marshals onto it. Engine pointers are never dereferenced
   anywhere else.
2. **NAPI entry points block the ArkUI main thread** for the duration of the
   engine call — the same contract as the Android player's `nativeFrame`
   barrier. Keep per-call work bounded.
3. **Callbacks are asynchronous.** Node-API cannot call JS synchronously from
   a foreign thread, so every upcall is queued through a threadsafe function
   and its payload is an owned copy made on the engine thread. Never assume a
   handler ran before the call that triggered it returned.
4. **A dead handle is `-1`, never a crash.** Handles are monotonic and
   tombstoned; a destroyed or unknown handle is rejected before any dispatch.
5. **Destroy is ordered.** `destroy` tombstones the handle, drains the queue
   (completing every parked caller with `Closing`), then runs `op_destroy` →
   surface unbind → callback-context free strictly last on the engine thread.
   A destroy initiated from inside a callback defers instead of blocking.
6. **Surface teardown is synchronous.** `OnSurfaceDestroyed` suspends the
   bound engine on its own thread before returning to the framework.

---

## Building

```sh
export OHOS_NDK_HOME="$HOME/command-line-tools/sdk/default/openharmony"
scripts/build-ohos.sh          # aarch64, release, --features gl,editor
```

Features: `gl` (EGL/GLES rendering), `editor` (full editor mode),
`mobile-auth-dev` (debug-only local auth), `pinned-skia-binaries`
(**unusable on OHOS** — no prebuilt Skia archive exists for these triples).

`scripts/ohos/README.md` documents the toolchain, the Skia cross-compile
contract, and the linker wrappers.

Host-side checks (no NDK needed):

```sh
cargo check --workspace
cargo test -p op-engine-napi
cargo clippy --workspace --all-targets -- -D warnings
```

---

## Known limitations

### Authentication reports unavailable

`op-auth`'s private build matrix has no OHOS target, so an OHOS build links
the public stub. `editorBeginLogin` returns `OpStatus::NotReady` (`10`).
Surface that natively — "sign-in is not available on HarmonyOS yet" — instead
of opening a login UI. `editorConfigureAuth`, `editorAccountSnapshot`,
`editorSignOut`, and `editorTakeLoginUrl` are wired and safe to call; they
simply never produce a signed-in account on this platform.

### Collaboration uses the engine's own key store

Secure-store callbacks must answer synchronously from a collaboration worker
thread, which Node-API cannot do. Both `credential_*` callbacks are therefore
left null, and the engine falls back to its own `CollabRuntime` key store
instead of a platform keystore.

### Copy has no engine channel

See `clipboardGetText` above.

---

## UNVERIFIED-UNTIL-NDK

No OpenHarmony NDK is installed on the machine this was developed on. What
**is** verified, and what is not:

**Verified locally**

* `cargo check --workspace`, `cargo test -p op-engine-napi`,
  `cargo fmt --all --check`, and
  `cargo clippy --workspace --all-targets -- -D warnings` on macOS.
* Every OHOS-gated module type-checks AND passes `clippy -D warnings` for
  `aarch64-unknown-linux-ohos`, with and without `--features editor`, against
  stubbed `op-engine-ffi` / `op-engine-jni` crates. This covers the whole
  napi-ohos surface (`#[napi]` signatures, `#[napi(object)]` shapes,
  threadsafe functions, module registration) and the hand-rolled NDK
  declarations.
* `napi-ohos = "1.2.0"` and `napi-derive-ohos = "1.2.0"` exist on crates.io
  (MIT) and compile for `aarch64-unknown-linux-ohos`.
* The NDK constants are transcribed from upstream headers, not guessed:
  `OH_NativeXComponent_TouchEventType` and the callback table from
  `arkui_ace_engine/interfaces/native/native_interface_xcomponent.h`, the key
  codes from `multimodalinput_input/interfaces/kits/c/input/oh_key_code.h`,
  and the HiLog levels from `hiviewdfx_hilog/.../hilog/log.h`.

**Not verified**

1. **The real cross-build.** `cargo build -p op-engine-napi --target
   aarch64-unknown-linux-ohos` has never run: everything downstream of
   `skia-bindings`' build script is unexecuted, and Skia must be compiled from
   source for OHOS.
2. **`attachSurface` cannot succeed yet.** The engine's EGL surface backend is
   `target_os = "android"`-gated in `vendor/jian` and `op-engine-ffi`, so
   `op_attach_surface` returns `NotReady` (`10`) on OHOS. The fix is a
   `vendor/jian` submodule change plus a two-line gate widening in
   `op-engine-ffi`; `scripts/ohos/README.md` spells out all three edits. Until
   then the module builds, loads, and drives everything except presenting
   frames.
3. **Link names.** `libace_ndk.z.so` and `libhilog_ndk.z.so` are declared via
   `#[link(name = "ace_ndk.z")]` / `#[link(name = "hilog_ndk.z")]` and are
   unproven until a real link step runs.
4. **Module registration.** Reading `__NATIVE_XCOMPONENT_OBJ__` out of the
   module exports and `napi_unwrap`-ing it to an `OH_NativeXComponent*`
   compiles, but has never been exercised by an ArkTS runtime — including the
   assumption that ArkUI re-runs module registration per XComponent.
5. **The engine's mobile paths on OHOS.** The export-staging and
   shell-action arms in `op-engine-ffi` were `target_os = "ios"/"android"`
   only; they are now widened with `target_env = "ohos"` (the same code
   already compiles on the host through those `cfg`s' `test` arm), but no OHOS
   run has confirmed the behaviour end to end.
6. **`x86_64-unknown-linux-ohos`.** Wired for the emulator, never built.
7. **HiLog domain `0xF000`.** Chosen, not mandated; adjust if the app reserves
   a different domain.
