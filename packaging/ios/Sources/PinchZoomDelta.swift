import CoreGraphics
import Foundation

/// Converts the platform pinch scale into the wheel delta expected by the
/// editor ABI. The engine applies `exp(delta * 0.0015)`, so using the
/// logarithm makes each gesture update reproduce the fingers' distance ratio.
enum PinchZoomDelta {
    private static let zoomExponentPerWheelUnit: CGFloat = 0.0015
    private static let minimumDistance: CGFloat = 0.001

    static func wheelDelta(previousDistance: CGFloat, currentDistance: CGFloat) -> CGFloat {
        guard previousDistance.isFinite,
              currentDistance.isFinite,
              previousDistance > minimumDistance,
              currentDistance > minimumDistance else {
            return 0
        }

        let ratio = currentDistance / previousDistance
        guard ratio.isFinite, ratio > 0 else { return 0 }
        let delta = log(ratio) / zoomExponentPerWheelUnit
        return delta.isFinite ? delta : 0
    }
}
