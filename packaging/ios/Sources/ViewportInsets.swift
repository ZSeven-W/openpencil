import CoreGraphics

/// Sanitizes platform insets before they cross the viewport ABI. Keeping the
/// tuple conversion pure makes rotation and Stage Manager edge cases testable
/// without a live engine.
struct ViewportInsets: Equatable {
    let top: CGFloat
    let right: CGFloat
    let bottom: CGFloat
    let left: CGFloat

    static func clamped(
        top: CGFloat,
        right: CGFloat,
        bottom: CGFloat,
        left: CGFloat,
        to size: CGSize
    ) -> ViewportInsets {
        var top = finiteInset(top, extent: size.height)
        var bottom = finiteInset(bottom, extent: size.height)
        var left = finiteInset(left, extent: size.width)
        var right = finiteInset(right, extent: size.width)
        scalePair(&top, &bottom, extent: size.height)
        scalePair(&left, &right, extent: size.width)
        return ViewportInsets(top: top, right: right, bottom: bottom, left: left)
    }

    private static func finiteInset(_ value: CGFloat, extent: CGFloat) -> CGFloat {
        guard value.isFinite, extent.isFinite, extent > 0 else { return 0 }
        return max(0, min(value, extent))
    }

    private static func scalePair(_ first: inout CGFloat, _ second: inout CGFloat, extent: CGFloat) {
        let sum = first + second
        guard sum > extent, sum > 0 else { return }
        let factor = extent / sum
        first *= factor
        second *= factor
    }
}
