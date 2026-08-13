import CoreGraphics

/// Keeps layout-only UIKit passes from becoming engine viewport resizes.
enum ViewportChange {
    static func requiresResize(
        currentSize: CGSize,
        currentScale: CGFloat,
        nextSize: CGSize,
        nextScale: CGFloat
    ) -> Bool {
        currentSize != nextSize || currentScale != nextScale
    }
}

/// Coordinates UIKit's independently ordered bounds and safe-area callbacks.
/// Bounds changes wait for the matching safe-area callback, or for two stable
/// display frames when iPad resizing legitimately leaves safe insets unchanged.
final class ViewportConvergence {
    enum Signal {
        case layout
        case safeArea
    }

    private(set) var isPending = false
    private var layoutSeen = false
    private var safeAreaSeen = false
    private var lastSample: ViewportSample?
    private var stableFrameCount = 0

    func reset() {
        isPending = false
        layoutSeen = false
        safeAreaSeen = false
        lastSample = nil
        stableFrameCount = 0
    }

    func signal(_ signal: Signal, sample: ViewportSample) {
        if !isPending {
            isPending = true
            layoutSeen = false
            safeAreaSeen = false
        }
        switch signal {
        case .layout:
            layoutSeen = true
        case .safeArea:
            safeAreaSeen = true
        }
        // A callback begins or extends the epoch, so stability must be observed
        // again on display frames after the newest UIKit value is visible.
        lastSample = sample
        stableFrameCount = 0
    }

    /// Returns the one tuple that may be committed on this display frame.
    func displayFrame(sample: ViewportSample) -> ViewportSample? {
        guard isPending else { return nil }
        if sample == lastSample {
            stableFrameCount += 1
        } else {
            lastSample = sample
            stableFrameCount = 1
        }

        // Every path requires two stable display-frame samples. This also
        // protects paired and safe-area-only epochs when UIKit delivers a late
        // correction on the following animation frame.
        guard stableFrameCount >= 2, layoutSeen || safeAreaSeen else { return nil }

        reset()
        return sample
    }
}

struct ViewportSample: Equatable {
    var size: CGSize
    var scale: CGFloat
    var insets: ViewportInsets
}

/// Gates input while UIKit and the engine temporarily expose different
/// coordinate spaces during a coalesced viewport transition.
struct ViewportGeometryGate {
    private(set) var committed: ViewportSample?
    private(set) var isPending = false

    mutating func observe(_ sample: ViewportSample) -> Bool {
        guard let committed else { return false }
        guard sample != committed else { return false }
        let becamePending = !isPending
        isPending = true
        return becamePending
    }

    mutating func commit(_ sample: ViewportSample, succeeded: Bool) {
        guard succeeded else {
            isPending = true
            return
        }
        committed = sample
        isPending = false
    }
}

/// Remembers physical touches suppressed during a viewport transition until
/// UIKit reports their terminal event. A successful resize must not revive the
/// tail of an already-cancelled finger sequence.
struct SuppressedTouchSet<Key: Hashable> {
    private var keys: Set<Key> = []

    var isEmpty: Bool { keys.isEmpty }

    mutating func suppress<S: Sequence>(_ values: S) where S.Element == Key {
        keys.formUnion(values)
    }

    func contains(_ key: Key) -> Bool {
        keys.contains(key)
    }

    mutating func finish<S: Sequence>(_ values: S) where S.Element == Key {
        keys.subtract(values)
    }
}
