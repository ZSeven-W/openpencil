import CoreGraphics

@main
enum ViewportChangeTests {
    static func main() {
        let portrait = sample(width: 402, height: 874, top: 59, bottom: 34)
        let landscapeOldInsets = sample(width: 874, height: 402, top: 59, bottom: 34)
        let landscape = sample(width: 874, height: 402, top: 0, bottom: 21)

        precondition(!ViewportChange.requiresResize(
            currentSize: portrait.size,
            currentScale: portrait.scale,
            nextSize: portrait.size,
            nextScale: portrait.scale
        ))
        precondition(ViewportChange.requiresResize(
            currentSize: portrait.size,
            currentScale: portrait.scale,
            nextSize: landscape.size,
            nextScale: landscape.scale
        ))

        callbackOrderCommitsFinalTuple(order: [.layout, .safeArea], final: landscape)
        callbackOrderCommitsFinalTuple(order: [.safeArea, .layout], final: landscape)

        // A late safe callback arriving after one frame must reset stability;
        // the old-inset tuple is never committed.
        let delayed = ViewportConvergence()
        delayed.signal(.layout, sample: landscapeOldInsets)
        precondition(delayed.displayFrame(sample: landscapeOldInsets) == nil)
        delayed.signal(.safeArea, sample: landscape)
        precondition(delayed.displayFrame(sample: landscape) == nil)
        precondition(delayed.displayFrame(sample: landscape) == landscape)

        // Even after both callbacks, a correction on the next frame resets the
        // epoch; the initially paired tuple is not committed prematurely.
        let pairedThenLate = ViewportConvergence()
        pairedThenLate.signal(.layout, sample: landscapeOldInsets)
        pairedThenLate.signal(.safeArea, sample: landscapeOldInsets)
        precondition(pairedThenLate.displayFrame(sample: landscapeOldInsets) == nil)
        pairedThenLate.signal(.safeArea, sample: landscape)
        precondition(pairedThenLate.displayFrame(sample: landscape) == nil)
        precondition(pairedThenLate.displayFrame(sample: landscape) == landscape)

        // iPad/Stage Manager may resize bounds without changing safe insets or
        // issuing safeAreaInsetsDidChange. Two stable frames provide a bounded
        // fallback instead of leaving the engine at the previous size.
        let boundsOnly = ViewportConvergence()
        boundsOnly.signal(.layout, sample: landscape)
        precondition(boundsOnly.displayFrame(sample: landscape) == nil)
        precondition(boundsOnly.displayFrame(sample: landscape) == landscape)

        // A safe-area-only change also waits for two stable display frames.
        let safeOnly = ViewportConvergence()
        safeOnly.signal(.safeArea, sample: landscape)
        precondition(safeOnly.displayFrame(sample: landscape) == nil)
        precondition(safeOnly.displayFrame(sample: landscape) == landscape)

        safeOnly.reset()
        precondition(!safeOnly.isPending)

        var gate = ViewportGeometryGate()
        gate.commit(portrait, succeeded: true)
        precondition(!gate.observe(portrait))
        precondition(!gate.isPending)
        precondition(gate.observe(landscape))
        precondition(gate.isPending)
        precondition(!gate.observe(landscape))
        gate.commit(landscape, succeeded: false)
        precondition(gate.isPending)
        gate.commit(landscape, succeeded: true)
        precondition(!gate.isPending)

        var suppressed = SuppressedTouchSet<Int>()
        suppressed.suppress([7, 9])
        precondition(!suppressed.isEmpty)
        precondition(suppressed.contains(7))
        // Viewport commit does not mutate this set: a late Move stays blocked.
        gate.commit(landscape, succeeded: true)
        precondition(suppressed.contains(7))
        // A new Down that arrives while an older physical sequence is still
        // suppressed joins that sequence instead of starting fresh input.
        suppressed.suppress([11])
        precondition(suppressed.contains(11))
        suppressed.finish([7])
        precondition(!suppressed.contains(7))
        precondition(suppressed.contains(9))
        suppressed.finish([9])
        precondition(!suppressed.contains(9))
        precondition(!suppressed.isEmpty)
        suppressed.finish([11])
        precondition(suppressed.isEmpty)

        // UIKit may report both fingers' Ends in one Set. The first handler
        // suppresses the still-present peer; finishing each key after its own
        // handler clears that peer when the loop reaches its terminal event.
        var sameBatch = SuppressedTouchSet<Int>()
        sameBatch.suppress([9])
        sameBatch.finish([7])
        precondition(sameBatch.contains(9))
        sameBatch.finish([9])
        precondition(sameBatch.isEmpty)
        // Empty suppression means the following clean Down is admissible.
        precondition(!sameBatch.contains(12))
    }

    private static func callbackOrderCommitsFinalTuple(
        order: [ViewportConvergence.Signal],
        final: ViewportSample
    ) {
        let coalescer = ViewportConvergence()
        for signal in order {
            coalescer.signal(signal, sample: final)
        }
        precondition(coalescer.displayFrame(sample: final) == nil)
        precondition(coalescer.displayFrame(sample: final) == final)
        precondition(!coalescer.isPending)
    }

    private static func sample(
        width: CGFloat,
        height: CGFloat,
        top: CGFloat,
        bottom: CGFloat
    ) -> ViewportSample {
        ViewportSample(
            size: CGSize(width: width, height: height),
            scale: 3,
            insets: ViewportInsets(top: top, right: 0, bottom: bottom, left: 0)
        )
    }
}
