import napi from 'libopenpencil.so';

/**
 * The C-ABI engine surface, marshalled by `crates/op-engine-napi`. Every
 * exported function dispatches onto the engine's dedicated thread; a
 * closed/unknown handle returns `OpStatus.CLOSING`.
 *
 * The names mirror `OpNative.kt` with the `native` prefix removed
 * (nativeCreate -> create, nativeEditorTakeShellAction ->
 * editorTakeShellAction, ...). The engine handle is a `number` (see
 * `cpp/types/libopenpencil/index.d.ts` for why that is lossless).
 */
export default napi;

/** Raw statuses returned by the engine ABI. */
export enum OpStatus {
  OK = 0,
  CLOSING = -1,
  /**
   * `OpStatus::InvalidArg`. From `keyEvent` it means "this key has no editor
   * binding" — the shell must leave it to the IME as text rather than
   * consuming it.
   */
  INVALID_ARG = 1,
  /** `OpStatus::GpuError` discriminant. */
  GPU_ERROR = 4,
  /** Collaboration or another engine-owned operation currently blocks it. */
  BUSY = 8,
}

/**
 * Shell actions the engine hands to the platform shell, pinned to
 * `OpShellAction` in `crates/op-engine-ffi/include/op_engine.h`.
 */
export enum OpShellAction {
  NONE = 0,
  OPEN_DOCUMENT = 1,
  OPEN_LOGIN_WEBVIEW = 2,
  CLOSE_LOGIN_WEBVIEW = 3,
  EXPORT_DOCUMENT = 4,
  OPEN_ACCOUNT_CENTER = 5,
  REQUEST_LOGIN = 6,
  OPEN_LANGUAGE_PICKER = 7,
  /**
   * The TopBar's painted traffic-light dots. Only desktop-class (2in1)
   * shells see these: they hid the platform title bar, so the engine's own
   * dots are the window controls. Touch chrome paints none.
   */
  WINDOW_CLOSE = 8,
  WINDOW_MINIMIZE = 9,
  WINDOW_ZOOM = 10,
  /**
   * Save / Save As through `DocumentViewPicker.save`. Emitted only after
   * `editorConfigureSavePicker(engine, true)`: HarmonyOS public directories
   * are not path-writable, so the engine cannot place a document anywhere
   * the user's file manager can reach on its own.
   */
  SAVE_DOCUMENT = 11,
  /** Pick one PNG, JPEG, GIF, WebP, or SVG and return bounded bytes. */
  IMPORT_IMAGE_OR_SVG = 12,
}

/** Pointer phases mirroring the C ABI `OpPointerPhase`. */
export enum OpPointerPhase {
  DOWN = 0,
  MOVE = 1,
  UP = 2,
  CANCEL = 3,
}

/** Editor key codes (mirror `KEY_*` in op-engine-ffi/src/editor.rs). */
export enum OpKey {
  BACKSPACE = 1,
  DELETE = 2,
  ENTER = 3,
  ESCAPE = 4,
  DUPLICATE = 5,
  UNDO = 6,
  REDO = 7,
  ARROW_UP = 9,
  ARROW_DOWN = 10,
  ARROW_LEFT = 11,
  ARROW_RIGHT = 12,
}

/** Engine mode passed to `create`. */
export enum OpEngineMode {
  VIEWER = 0,
  EDITOR = 1,
}
