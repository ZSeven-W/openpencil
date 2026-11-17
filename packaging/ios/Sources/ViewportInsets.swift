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

    /// Keep the editor and a read-only design preview on opposite sides
    /// of the active hinge. The first intersecting division is the Duo fold.
    func foldLayout(
        _ divisions: [CGRect],
        in bounds: CGRect,
        rightToLeft: Bool
    ) -> (editor: ViewportInsets, preview: CGRect?) {
        let viewport = CGRect(
            x: bounds.minX + left,
            y: bounds.minY + top,
            width: bounds.width - left - right,
            height: bounds.height - top - bottom
        )
        for division in divisions {
            let overlap = viewport.intersection(division)
            guard !overlap.isEmpty else { continue }

            let first: CGRect
            let second: CGRect
            let firstPreferred: Bool
            if division.height > division.width {
                let leftWidth = overlap.minX - viewport.minX
                let rightWidth = viewport.maxX - overlap.maxX
                first = CGRect(x: viewport.minX, y: viewport.minY, width: leftWidth, height: viewport.height)
                second = CGRect(x: overlap.maxX, y: viewport.minY, width: rightWidth, height: viewport.height)
                firstPreferred = leftWidth > rightWidth || (leftWidth == rightWidth && !rightToLeft)
            } else {
                let topHeight = overlap.minY - viewport.minY
                let bottomHeight = viewport.maxY - overlap.maxY
                first = CGRect(x: viewport.minX, y: viewport.minY, width: viewport.width, height: topHeight)
                second = CGRect(x: viewport.minX, y: overlap.maxY, width: viewport.width, height: bottomHeight)
                firstPreferred = topHeight > bottomHeight
            }

            let editor = firstPreferred ? first : second
            guard !editor.isEmpty else { continue }
            let preview = firstPreferred ? second : first
            return (
                editor: ViewportInsets(
                    top: editor.minY - bounds.minY,
                    right: bounds.maxX - editor.maxX,
                    bottom: bounds.maxY - editor.maxY,
                    left: editor.minX - bounds.minX
                ),
                preview: preview.isEmpty ? nil : preview.offsetBy(dx: -bounds.minX, dy: -bounds.minY)
            )
        }
        return (editor: self, preview: nil)
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
