import Foundation
import QuartzCore

/// Timestamped editor-pointer forwarding — the `op_editor_*_at` C entry
/// points.
///
/// The plain `editorPress` / `editorMove` / `editorRelease` /
/// `editorCancelGesture` wrappers stay in `OpEngineHost.swift` (source
/// and binary compatibility); every LIVE touch routes through these
/// `_at` variants instead, so the engine receives each event's factual
/// monotonic timestamp:
/// - Down / Move / Up carry `UITouch.timestamp * 1000` (the same
///   boot-uptime domain as the frame pump's `CACurrentMediaTime`), and
/// - Cancels for which no trustworthy touch timestamp exists
///   (`touchesCancelled`, long-press / geometry / two-finger
///   takeovers) carry `CACurrentMediaTime * 1000`.
///
/// The engine keeps its global clock monotonic and forwards the raw
/// event time separately into the preview runtime, so an out-of-order
/// event never regresses a clock while velocity-sensing gestures still
/// measure the event pair's own delta.
extension OpEngineHost {
    func editorPressAt(x: CGFloat, y: CGFloat, timeMs: UInt64) {
        guard let engine, editorMode else { return }
        let status = op_editor_press_at(engine, Float(x), Float(y), timeMs)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_press_at", engine: engine)
        } else if status == OpStatus_Ok {
            drainShellActions()
            generationBackgroundCoordinator.observeEngineWork()
        }
    }

    func editorMoveAt(x: CGFloat, y: CGFloat, timeMs: UInt64) {
        guard let engine, editorMode else { return }
        let status = op_editor_move_at(engine, Float(x), Float(y), timeMs)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_move_at", engine: engine)
        }
    }

    func editorReleaseAt(x: CGFloat, y: CGFloat, timeMs: UInt64) {
        guard let engine, editorMode else { return }
        let status = op_editor_release_at(engine, Float(x), Float(y), timeMs)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_release_at", engine: engine)
        }
        if status == OpStatus_Ok { generationBackgroundCoordinator.observeEngineWork() }
    }

    func editorCancelGestureAt(timeMs: UInt64) {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        let status = op_editor_cancel_gesture_at(engine, timeMs)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_cancel_gesture_at", engine: engine)
        } else if status == OpStatus_Ok {
            requestImmediateFrame()
        }
    }

    /// Viewer-mode generic pointer forwarding (`op_pointer`). Lives beside
    /// the timestamped editor entries so every raw pointer crossing the FFI
    /// does so through this single seam.
    func dispatchPointer(id: UInt32, phase: Int32, point: CGPoint, timeMs: UInt64) {
        precondition(Thread.isMainThread)
        guard let engine else { return }
        let status = op_pointer(engine, id, phase, Float(point.x), Float(point.y), timeMs)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_pointer", engine: engine)
        }
    }

    /// Cancels any live gesture stream before a suspend barrier
    /// (`didEnterBackground`, teardown): Editor streams through
    /// `op_editor_cancel_gesture_at`, viewer streams through the global
    /// generic pointer CANCEL. Issued via the RAW entry points on purpose —
    /// the wrappers resume the display link on success, and the very next
    /// step here suspends the engine. Idempotent while idle; MUST run
    /// BEFORE `op_suspend`.
    func cancelGesturesBeforeSuspend() {
        precondition(Thread.isMainThread)
        guard let engine else { return }
        let nowMs = OpEngineHost.syntheticCancelNowMs()
        if editorMode {
            let status = op_editor_cancel_gesture_at(engine, nowMs)
            if status != OpStatus_Ok && status != OpStatus_Suspended {
                reportFailure(status, operation: "op_editor_cancel_gesture_at", engine: engine)
            }
        } else {
            // Generic pointer Cancel is global in the engine; coordinates
            // and pointer id are intentionally ignored for cancellation.
            let status = op_pointer(
                engine,
                0,
                Int32(OpPointerPhase_Cancel.rawValue),
                0,
                0,
                nowMs
            )
            if status != OpStatus_Ok && status != OpStatus_Suspended {
                reportFailure(status, operation: "op_pointer", engine: engine)
            }
        }
    }

    /// Synthetic-Cancel clock: `CACurrentMediaTime` in milliseconds —
    /// the frame pump's monotonic domain, used when no trustworthy
    /// `UITouch` timestamp is available (`touchesCancelled`, long-press
    /// / geometry / two-finger takeovers). Deliberately spelled out (not
    /// `Self.nowMilliseconds()`) so the two sites cannot drift apart.
    static func syntheticCancelNowMs() -> UInt64 {
        UInt64((CACurrentMediaTime() * 1_000).rounded(.down))
    }
}
