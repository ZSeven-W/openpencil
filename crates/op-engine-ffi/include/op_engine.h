/*
 * OpenPencil Engine C ABI
 *
 * This header has one ABI shape in every build. Rust feature flags and debug
 * assertions never add, remove, or alter declarations in this contract.
 *
 * Coordinates: pointer input uses surface-logical pixels with a top-left
 * origin. Safe-area and keyboard occlusion are separate logical-point
 * channels.
 *
 * Tail-growth versioning: OpCreateDesc.size must be at least
 * offsetof(OpCreateDesc, dpr) + sizeof(float) (36 bytes on the supported
 * 64-bit hosts). OpCallbacks.size must be at least sizeof(size_t) (8 bytes
 * on the supported 64-bit hosts). Missing trailing fields are zero/null; a
 * size larger than the current sizeof(struct) is invalid.
 *
 * Threading: an engine is owned by exactly one thread — the thread that
 * called op_create. Every other call must run on that thread or
 * OpStatus_WrongThread is returned. Callbacks fire synchronously inside
 * the call that caused them; the shell copies the payload and reacts
 * asynchronously, and must not re-enter the engine synchronously.
 *
 * Rendering: the engine paints the document's active page with the exact
 * painter the desktop editor canvas uses. op_attach_surface hands the
 * engine a borrowed platform surface — a CAMetalLayer* on iOS, an
 * ANativeWindow* on Android — which the shell must keep alive until
 * op_suspend / op_destroy returns. op_frame presents one frame;
 * op_frame_cpu renders into caller-owned RGBA8888 storage (used by tests
 * and debug tooling).
 */


#ifndef OP_ENGINE_H
#define OP_ENGINE_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Stable status values returned by every OpenPencil C ABI call. */
typedef enum OpStatus {
    OpStatus_Ok = 0,
    OpStatus_InvalidArg = 1,
    OpStatus_BadDocument = 2,
    OpStatus_LayoutError = 3,
    OpStatus_GpuError = 4,
    OpStatus_OutOfMemory = 5,
    OpStatus_WrongThread = 6,
    OpStatus_Suspended = 7,
    OpStatus_Busy = 8,
    OpStatus_NoFocus = 9,
    OpStatus_NotReady = 10,
    OpStatus_Poisoned = 11,
} OpStatus;

typedef struct OpEngine OpEngine;

/* Logical safe-area insets (top-left origin, logical points). */
typedef struct OpInsets {
    float top;
    float right;
    float bottom;
    float left;
} OpInsets;

/* Runtime diagnostic payload copied synchronously into the callback. */
typedef struct OpRuntimeError {
    int32_t kind;
    const uint8_t *message_ptr;
    size_t message_len;
    const uint8_t *source_ptr;
    size_t source_len;
} OpRuntimeError;

typedef void (*OpNeedsRedraw)(void *user_data, bool has_next_wake, uint64_t next_wake_ms);
typedef void (*OpRuntimeErrorCallback)(void *user_data, const OpRuntimeError *error);
typedef void (*OpInputFocusChanged)(void *user_data, bool focused, int32_t input_kind, int32_t return_key_hint);
typedef void (*OpRemoteImageRequest)(void *user_data, uint64_t request_id, const uint8_t *url_ptr, size_t url_len);
/* Platform secure-store callbacks may run on a collaboration worker thread.
 * The shell must make them thread-safe and keep user_data alive until
 * op_destroy returns. Credentials are exactly 32 bytes. Load: 0=found,
 * 1=missing, negative=failure. */
typedef int32_t (*OpCredentialLoad)(void *user_data, uint8_t *out, size_t capacity, size_t *out_len);
typedef int32_t (*OpCredentialStoreIfAbsent)(void *user_data, const uint8_t *value, size_t value_len);

/* Callback table. Future callbacks grow only at the tail. */
typedef struct OpCallbacks {
    size_t size;
    void *user_data;
    OpNeedsRedraw needs_redraw;
    OpRuntimeErrorCallback runtime_error;
    OpInputFocusChanged input_focus_changed;
    OpRemoteImageRequest remote_image_request;
    OpCredentialLoad credential_load;
    OpCredentialStoreIfAbsent credential_store_if_absent;
} OpCallbacks;

/* Engine construction descriptor. doc_ptr/doc_len normally hold a non-empty
 * canonical .op document. Full-editor mode may pass NULL/0 to open the
 * canonical blank starter; viewer mode always requires document bytes.
 * asset_base is the v1 tail; mode is the v2 tail (0 = viewer, 1 = full
 * editor); storage_root is the v3 tail. Mobile editor shells must pass a
 * private app-sandbox directory before any runtime config is resolved. */
typedef struct OpCreateDesc {
    size_t size;
    const uint8_t *doc_ptr;
    size_t doc_len;
    float width;
    float height;
    float dpr;
    const OpCallbacks *callbacks;
    const uint8_t *asset_base_ptr;
    size_t asset_base_len;
    int32_t mode;
    const uint8_t *storage_root_ptr;
    size_t storage_root_len;
} OpCreateDesc;

/* Platform surface descriptor. iOS: borrowed CAMetalLayer*. Android:
 * borrowed ANativeWindow*. */
typedef struct OpSurfaceDesc {
    size_t size;
    void *handle;
} OpSurfaceDesc;

typedef enum OpPointerPhase {
    OpPointerPhase_Down = 0,
    OpPointerPhase_Move = 1,
    OpPointerPhase_Up = 2,
    OpPointerPhase_Cancel = 3,
} OpPointerPhase;

/* Work the editor asks its platform shell to perform. */
typedef enum OpShellAction {
    OpShellAction_None = 0,
    OpShellAction_OpenDocument = 1,
    OpShellAction_OpenLoginWebView = 2,
    OpShellAction_CloseLoginWebView = 3,
    OpShellAction_ExportDocument = 4,
} OpShellAction;

/* Surrounding-text snapshot for the shell's input connection. The text
 * pointer is BORROWED from the engine and valid only until the next engine
 * call; offsets are UTF-16 code units. */
typedef struct OpTextState {
    const uint8_t *text_ptr;
    size_t text_len;
    uint32_t selection_start;
    uint32_t selection_end;
    bool has_composing;
    uint32_t composing_start;
    uint32_t composing_end;
} OpTextState;

/* Create an engine from a versioned descriptor. `out` receives the
 * handle, or NULL on failure (read the reason via op_last_error with a
 * NULL engine). */
OpStatus op_create(const OpCreateDesc *desc, OpEngine **out);

/* Destroy an engine on its owner thread. */
OpStatus op_destroy(OpEngine *engine);

/* Copy the last error into a caller-owned byte buffer. With a NULL
 * engine, reports the op_create error. */
OpStatus op_last_error(OpEngine *engine, uint8_t *buffer, size_t length, size_t *required);

/* Select GPU mode using a borrowed platform surface. */
OpStatus op_attach_surface(OpEngine *engine, const OpSurfaceDesc *desc);

/* Suspend rendering and synchronously stop using a borrowed surface. */
OpStatus op_suspend(OpEngine *engine);

/* Resume rendering, optionally with a new borrowed GPU surface. */
OpStatus op_resume(OpEngine *engine, const OpSurfaceDesc *desc);

/* Pump and present one GPU frame. */
OpStatus op_frame(OpEngine *engine, uint64_t now_ms);

/* Pump and copy one CPU frame into caller-owned RGBA8888 storage.
 * `buffer` must cover `height * stride` bytes with
 * `stride >= physical_width * 4`. */
OpStatus op_frame_cpu(OpEngine *engine, uint64_t now_ms, uint8_t *buffer, size_t buffer_len, size_t stride);

/* Change the logical viewport and device-pixel ratio. */
OpStatus op_resize(OpEngine *engine, float width, float height, float dpr);

/* Atomically update logical viewport, DPR, and safe-area insets. Mobile
 * rotation/configuration paths should prefer this over separate calls. */
OpStatus op_resize_with_safe_area(OpEngine *engine, float width, float height, float dpr, float top, float right, float bottom, float left);

/* Return the current physical pixel dimensions. */
OpStatus op_get_pixel_size(OpEngine *engine, uint32_t *width, uint32_t *height);

/* Deliver one pointer event (surface-logical coordinates, top-left
 * origin). `id` identifies the touch/finger; `phase` is an
 * OpPointerPhase value. */
OpStatus op_pointer(OpEngine *engine, uint32_t id, int32_t phase, float x, float y, uint64_t time_ms);

/* Update the four logical safe-area insets. */
OpStatus op_set_safe_area(OpEngine *engine, float top, float right, float bottom, float left);

/* Update the logical keyboard occlusion height. */
OpStatus op_set_keyboard(OpEngine *engine, float height);

/* Whether status/navigation bars should use light-colored icons. */
OpStatus op_prefers_light_system_icons(OpEngine *engine, bool *out);

/* Enter inline text-edit mode on the node with id `node_id` (the shell
 * then shows the system keyboard via input_focus_changed). */
OpStatus op_text_begin(OpEngine *engine, const uint8_t *node_id_ptr, size_t node_id_len);

/* Exit text-edit mode, committing the draft into the document. */
OpStatus op_text_end(OpEngine *engine);

/* Insert `text` at the caret, replacing any active selection. */
OpStatus op_text_insert(OpEngine *engine, const uint8_t *text_ptr, size_t text_len);

/* Delete the selection, or the character before the caret. */
OpStatus op_text_backspace(OpEngine *engine);

/* Delete the selection, or the character after the caret. */
OpStatus op_text_delete_forward(OpEngine *engine);

/* Place the caret at UTF-16 `offset`; `extend` keeps the selection anchor. */
OpStatus op_text_set_caret(OpEngine *engine, uint32_t offset, bool extend);

/* Drag-select: anchor at UTF-16 `anchor`, caret follows `focus`. */
OpStatus op_text_select_range(OpEngine *engine, uint32_t anchor, uint32_t focus);

/* Set the in-flight IME composition. `sel_start`/`sel_end` are UTF-16
 * offsets within the NEW composing text. */
OpStatus op_ime_set_composing_text(OpEngine *engine, const uint8_t *text_ptr, size_t text_len, uint32_t sel_start, uint32_t sel_end);

/* Commit the in-flight IME composition into the draft. */
OpStatus op_ime_commit_composition(OpEngine *engine);

/* Cancel the in-flight IME composition. */
OpStatus op_ime_cancel_composition(OpEngine *engine);

/* Snapshot the surrounding-text state (borrowed text pointer). */
OpStatus op_text_get_state(OpEngine *engine, OpTextState *out);

/* Caret rect in surface-logical points; `out` receives x, y, w, h. */
OpStatus op_text_caret_rect(OpEngine *engine, float *out);

/* Push fetched remote-image bytes back into the engine (empty = failed). */
OpStatus op_remote_image_result(OpEngine *engine, uint64_t request_id, const uint8_t *bytes_ptr, size_t bytes_len);

/* Register an imported TTF/OTF font and re-layout the document. */
OpStatus op_register_font(OpEngine *engine, const uint8_t *bytes_ptr, size_t bytes_len);

/* Number of document pages. */
OpStatus op_get_page_count(OpEngine *engine, uint32_t *out);

/* Switch to page `index` (0-based); the viewport re-fits. */
OpStatus op_set_active_page(OpEngine *engine, uint32_t index);

/* Full-editor mode (OpCreateDesc.mode == 1): the engine drives the
 * desktop chrome. Pointer press (single finger). */
OpStatus op_editor_press(OpEngine *engine, float x, float y);

/* Pointer move (single finger). */
OpStatus op_editor_move(OpEngine *engine, float x, float y);

/* Pointer release (single finger). */
OpStatus op_editor_release(OpEngine *engine, float x, float y);

/* Cancel the active pointer gesture without dispatching release actions. */
OpStatus op_editor_cancel_gesture(OpEngine *engine);

/* Long-press -> right-click (context menus). */
OpStatus op_editor_right_press(OpEngine *engine, float x, float y);

/* Begin a two-finger transform at the second-finger Down midpoint. */
OpStatus op_editor_begin_transform(OpEngine *engine, float x, float y);

/* Two-finger pan after op_editor_begin_transform (midpoint deltas). */
OpStatus op_editor_pan(OpEngine *engine, float x, float y, float dx, float dy);

/* Pinch after op_editor_begin_transform (positive delta_y = zoom in). */
OpStatus op_editor_pinch(OpEngine *engine, float x, float y, float delta_y);

/* Printable text from the system keyboard. */
OpStatus op_editor_text(OpEngine *engine, const uint8_t *text_ptr, size_t text_len);

/* Non-printable key (see KEY_* constants below). */
OpStatus op_editor_key(OpEngine *engine, int32_t key);

/* IME preedit into the focused input (byte offsets within the preedit). */
OpStatus op_editor_ime_preedit(OpEngine *engine, const uint8_t *text_ptr, size_t text_len, size_t sel_start, size_t sel_end);

/* IME commit into the focused input. */
OpStatus op_editor_ime_commit(OpEngine *engine, const uint8_t *text_ptr, size_t text_len);

/* Whether the editor host currently holds the IME (show/hide keyboard). */
OpStatus op_editor_ime_focused(OpEngine *engine, bool *out);

/* Drain the next platform-shell action. File actions not owned by the mobile
 * shell stay queued for their host integration. */
OpStatus op_editor_take_shell_action(OpEngine *engine, int32_t *out);

/* Peek/copy the UTF-8 file name of the frozen export. NULL/0 reports the
 * required length without consuming it. The payload is not NUL-terminated. */
OpStatus op_editor_copy_export_file_name(OpEngine *engine, uint8_t *buffer, size_t capacity, size_t *required);

/* Atomically create the absolute UTF-8 staging path with the frozen export.
 * The target must not already exist. Success consumes the export; failure
 * leaves it retryable. */
OpStatus op_editor_export_to_path(OpEngine *engine, const uint8_t *path_ptr, size_t path_len);

/* Discard a frozen export when the platform save UI cannot be presented. */
OpStatus op_editor_cancel_export(OpEngine *engine);

/* Configure the real mobile auth backend. `storage_dir` must be a private
 * app-owned directory; device name and app version are display metadata. The
 * production SSO origin is pinned by the engine and is not shell-provided. */
OpStatus op_editor_configure_auth(OpEngine *engine, const uint8_t *storage_dir_ptr, size_t storage_dir_len, const uint8_t *device_name_ptr, size_t device_name_len, const uint8_t *app_version_ptr, size_t app_version_len);

/* Peek/copy the pending UTF-8 login URL. NULL/0 reports the required length
 * without consuming it. A complete copy consumes it; a short copy fails and
 * remains retryable. The payload is not NUL-terminated. */
OpStatus op_editor_copy_login_url(OpEngine *engine, uint8_t *buffer, size_t capacity, size_t *required);

/* Cancel an in-flight login after the user dismisses the embedded WebView. */
OpStatus op_editor_cancel_login(OpEngine *engine);

/* Parse and atomically install a platform-picked `.op` document. The optional
 * file name is UTF-8 display text; pass NULL/0 when unavailable. */
OpStatus op_editor_open_document(OpEngine *engine, const uint8_t *doc_ptr, size_t doc_len, const uint8_t *file_name_ptr, size_t file_name_len);

/* Editor key codes for op_editor_key. */
enum {
    OpKey_Backspace = 1,
    OpKey_Delete = 2,
    OpKey_Enter = 3,
    OpKey_Escape = 4,
    OpKey_Duplicate = 5,
    OpKey_Undo = 6,
    OpKey_Redo = 7,
    OpKey_ArrowUp = 9,
    OpKey_ArrowDown = 10,
    OpKey_ArrowLeft = 11,
    OpKey_ArrowRight = 12,
};

#ifdef __cplusplus
} /* extern "C" */
#endif

#endif /* OP_ENGINE_H */
