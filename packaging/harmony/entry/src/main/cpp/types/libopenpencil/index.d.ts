/**
 * Type declaration for the prebuilt Rust NAPI module `libopenpencil.so`
 * (crate `op-engine-napi`).
 *
 * CONTRACT: every function below is the Android JNI surface
 * (`packaging/android/.../OpNative.kt`) translated to camelCase with
 * the `native` prefix removed — same arguments, same order, same semantics,
 * same status codes (0 = OK, -1 = closing/unknown handle).
 *
 * Two platform substitutions:
 *   - `android.view.Surface` becomes the XComponent id (`string`). The
 *     native side resolves it with `OH_NativeXComponent` and owns the
 *     surface for the lifetime of the attachment.
 *   - `ByteArray` becomes `ArrayBuffer`; `FloatArray` becomes `Float32Array`.
 *
 * The engine handle is a `number`. NAPI turns an int64 into a JS double,
 * which is exact up to 2^53; aarch64 user-space pointers stay inside 48
 * bits, so a raw handle round-trips losslessly. `0` means failure, exactly
 * like the JNI bridge.
 *
 * Threading: `frame`/`suspend` are blocking barriers onto the engine thread,
 * as on Android. Upcalls in `OpCallbacks` are delivered through NAPI
 * thread-safe functions and therefore land on the ArkTS thread.
 */

/** Engine to shell upcalls. Mirrors `OpCallbacks.kt`. */
export interface OpCallbacks {
  /**
   * A mutation (pointer / resize / attach / resume / text edit / caret
   * blink) requested a redraw. `hasNextWake` schedules a timed frame.
   */
  onNeedsRedraw(hasNextWake: boolean, nextWakeMs: number): void;

  /** A runtime diagnostic (document load / layout / GPU failures). */
  onRuntimeError(kind: number, message: string, source: string | null): void;

  /** Inline text-edit focus transition: show/hide the system keyboard. */
  onInputFocusChanged(focused: boolean, inputKind: number, returnKeyHint: number): void;

  /**
   * A paint pass recorded a remote image miss; fetch the URL and push the
   * bytes back via `remoteImageResult`.
   */
  onRemoteImageRequest(requestId: number, url: string): void;

  /** Returns the decrypted device key, null only when it does not exist. */
  onCredentialLoad(): ArrayBuffer | null;

  /** Atomically stores the first device key; an existing value wins. */
  onCredentialStoreIfAbsent(value: ArrayBuffer): boolean;
}

/** Owned surrounding-text snapshot filled by `textGetState`. */
export interface OpTextState {
  text: string;
  selectionStart: number;
  selectionEnd: number;
  hasComposing: boolean;
  composingStart: number;
  composingEnd: number;
}

// ---- Lifecycle ------------------------------------------------------------

/** `mode`: 0 = viewer, 1 = full editor. Returns 0 on failure. */
export const create: (
  doc: ArrayBuffer,
  w: number,
  h: number,
  dpr: number,
  receiver: OpCallbacks,
  storageRoot: string,
  mode: number
) => number;

export const lastError: (engine: number) => string;
/** `xcomponentId` is the ArkTS `XComponent({ id })` value. */
export const attachSurface: (engine: number, xcomponentId: string) => number;
export const suspend: (engine: number) => number;
export const resume: (engine: number, xcomponentId: string | null) => number;
export const resize: (engine: number, w: number, h: number, dpr: number) => number;
export const resizeWithSafeArea: (
  engine: number,
  w: number,
  h: number,
  dpr: number,
  t: number,
  r: number,
  b: number,
  l: number
) => number;
export const setSafeArea: (engine: number, t: number, r: number, b: number, l: number) => number;
export const setKeyboard: (engine: number, h: number) => number;
export const prefersLightSystemIcons: (engine: number) => boolean;
/**
 * Blocking barrier; the TRUE frame status. The OHOS build pumps frames from
 * the native vsync, so the shell does not call this on a timer — it stays on
 * the ABI for parity and for forced single-shot redraws.
 */
export const frame: (engine: number, tMs: number) => number;
export const pointer: (
  engine: number,
  id: number,
  phase: number,
  x: number,
  y: number,
  tMs: number
) => number;
export const destroy: (engine: number) => void;

// ---- Text editing + IME ---------------------------------------------------

export const textBegin: (engine: number, nodeId: string) => number;
export const textEnd: (engine: number) => number;
export const textInsert: (engine: number, text: string) => number;
export const textBackspace: (engine: number) => number;
export const textDeleteForward: (engine: number) => number;
export const textSetCaret: (engine: number, offset: number, extend: boolean) => number;
export const textSelectRange: (engine: number, anchor: number, focus: number) => number;
export const imeSetComposingText: (
  engine: number,
  text: string,
  selStart: number,
  selEnd: number
) => number;
export const imeCommitComposition: (engine: number) => number;
export const imeCancelComposition: (engine: number) => number;
/** Fills `out` in place, like the JNI out-parameter. */
export const textGetState: (engine: number, out: OpTextState) => number;
/** Fills `out` (length 4: x, y, w, h) in place. */
export const textCaretRect: (engine: number, out: Float32Array) => number;

// ---- Remote images / fonts / pages ----------------------------------------

export const remoteImageResult: (
  engine: number,
  requestId: number,
  bytes: ArrayBuffer | null
) => number;
export const registerFont: (engine: number, bytes: ArrayBuffer) => number;
/**
 * Chrome family: `false` switches to the full desktop layout (top bar, side
 * panels) for desktop-class devices (2in1/PC); `true` restores the mobile
 * touch chrome. Call once right after `create`.
 */
export const editorSetTouchChrome: (engine: number, enabled: boolean) => number;
/** Desktop-chrome hover: cursor motion without a pressed button. */
export const editorHover: (engine: number, x: number, y: number) => number;
/** Live hardware modifier bitmask (1 shift / 2 ctrl / 4 alt / 8 meta). */
export const keyModifiers: () => number;
/**
 * A hardware key as a raw HarmonyOS key code plus this crate's modifier
 * bitmask (1 shift / 2 ctrl / 4 alt / 8 meta), mapped onto an `OpKey_*`.
 * Returns `OpStatus.INVALID_ARG` (1) when the key has no editor binding —
 * the shell's signal to leave it to the IME as text.
 */
export const keyEvent: (engine: number, keyCode: number, modifiers: number) => number;
/**
 * Reports whether the shell's `ImeConduit` holds a system
 * `InputMethodController`. While it does NOT, the native key channel injects
 * printable characters itself (the 2in1 case: a SURFACE XComponent eats
 * hardware keys before the IME framework sees them, so `insertText` never
 * fires). While it DOES, the IME owns them and native injection would type
 * every character twice. Defaults to `false`.
 */
export const setImeConduitAttached: (attached: boolean) => void;
/** Panel-aware wheel scroll; zoom = Ctrl+wheel zoom at the cursor. */
export const editorWheel: (
  engine: number,
  x: number,
  y: number,
  dx: number,
  dy: number,
  zoom: boolean
) => number;
export const getPageCount: (engine: number) => number;
export const setActivePage: (engine: number, index: number) => number;

// ---- Full-editor mode (desktop chrome) ------------------------------------

export const editorPress: (engine: number, x: number, y: number) => number;
export const editorMove: (engine: number, x: number, y: number) => number;
export const editorRelease: (engine: number, x: number, y: number) => number;
export const editorCancelGesture: (engine: number) => number;
export const editorBeginTransform: (engine: number, x: number, y: number) => number;
export const editorRightPress: (engine: number, x: number, y: number) => number;
export const editorPan: (engine: number, x: number, y: number, dx: number, dy: number) => number;
export const editorPinch: (engine: number, x: number, y: number, deltaY: number) => number;
export const editorText: (engine: number, text: string) => number;
export const editorKey: (engine: number, key: number) => number;
export const editorImePreedit: (
  engine: number,
  text: string,
  selStart: number,
  selEnd: number
) => number;
export const editorImeCommit: (engine: number, text: string) => number;
export const editorImeFocused: (engine: number) => boolean;
/** `region` is an OpAuthRegion value: 0 = China, 1 = Global. */
export const editorConfigureAuth: (
  engine: number,
  storageDir: string,
  deviceName: string,
  appVersion: string,
  region: number
) => number;
export const editorTakeLoginUrl: (engine: number) => string | null;
export const editorCancelLogin: (engine: number) => number;
export const editorTakeShellAction: (engine: number) => number;
export const editorOpenDocument: (engine: number, bytes: ArrayBuffer, name: string) => number;
export const editorExportFileName: (engine: number) => string | null;
export const editorExportToPath: (engine: number, path: string) => number;
export const editorCancelExport: (engine: number) => number;
export const editorAccountSnapshot: (engine: number) => string | null;
export const editorSignOut: (engine: number) => number;
export const editorBeginLogin: (engine: number) => number;
export const editorSetLocale: (engine: number, tag: string) => number;
export const editorLocaleCode: (engine: number) => string | null;
