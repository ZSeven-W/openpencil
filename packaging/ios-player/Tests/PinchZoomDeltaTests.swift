import CoreGraphics
import Foundation

@main
enum PinchZoomDeltaTests {
    static func main() {
        assertScale(previous: 100, current: 200, expected: 2)
        assertScale(previous: 200, current: 100, expected: 0.5)
        assertScale(previous: 120, current: 150, expected: 1.25)

        let first = PinchZoomDelta.wheelDelta(previousDistance: 80, currentDistance: 100)
        let second = PinchZoomDelta.wheelDelta(previousDistance: 100, currentDistance: 160)
        let combined = PinchZoomDelta.wheelDelta(previousDistance: 80, currentDistance: 160)
        precondition(abs((first + second) - combined) < 0.001)

        assertIgnored(previous: 0, current: 100)
        assertIgnored(previous: 0.001, current: 100)
        assertIgnored(previous: 100, current: 0)
        assertIgnored(previous: .nan, current: 100)
        assertIgnored(previous: 100, current: .infinity)
    }

    private static func assertScale(previous: CGFloat, current: CGFloat, expected: CGFloat) {
        let delta = PinchZoomDelta.wheelDelta(
            previousDistance: previous,
            currentDistance: current
        )
        let actual = exp(delta * 0.0015)
        precondition(abs(actual - expected) < 0.000_001, "expected \(expected), got \(actual)")
    }

    private static func assertIgnored(previous: CGFloat, current: CGFloat) {
        let delta = PinchZoomDelta.wheelDelta(
            previousDistance: previous,
            currentDistance: current
        )
        precondition(delta == 0, "invalid distances must not zoom")
    }
}
