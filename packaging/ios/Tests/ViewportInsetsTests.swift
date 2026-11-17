import CoreGraphics

@main
enum ViewportInsetsTests {
    static func main() {
        testDisplayDivisions()
        assertInsets(
            ViewportInsets(top: 59, right: 0, bottom: 34, left: 0),
            input: (top: 59, right: 0, bottom: 34, left: 0),
            size: CGSize(width: 402, height: 874)
        )
        assertInsets(
            ViewportInsets(top: 0, right: 59, bottom: 21, left: 59),
            input: (top: 0, right: 59, bottom: 21, left: 59),
            size: CGSize(width: 874, height: 402)
        )
        assertInsets(
            ViewportInsets(top: 400, right: 300, bottom: 400, left: 300),
            input: (top: 900, right: 700, bottom: 900, left: 700),
            size: CGSize(width: 600, height: 800)
        )
        assertInsets(
            ViewportInsets(top: 0, right: 0, bottom: 0, left: 0),
            input: (top: .nan, right: -4, bottom: .infinity, left: -.infinity),
            size: CGSize(width: 600, height: 800)
        )
    }

    private static func testDisplayDivisions() {
        let inner = CGRect(x: 0, y: 0, width: 768, height: 860)
        let inset = ViewportInsets(top: 25, right: 36, bottom: 20, left: 0)
        let hinge = CGRect(x: 376, y: 0, width: 16, height: 860)
        precondition(inset.foldLayout([], in: inner, rightToLeft: false).editor == inset)
        precondition(
            inset.foldLayout([hinge], in: inner, rightToLeft: false).editor
                == ViewportInsets(top: 25, right: 392, bottom: 20, left: 0)
        )
        let symmetric = ViewportInsets(top: 25, right: 0, bottom: 20, left: 0)
        precondition(
            symmetric.foldLayout([hinge], in: inner, rightToLeft: true).editor
                == ViewportInsets(top: 25, right: 0, bottom: 20, left: 392)
        )

        let tabletop = CGRect(x: 10, y: 20, width: 860, height: 768)
        let horizontal = CGRect(x: 10, y: 396, width: 860, height: 16)
        let tabletopInset = ViewportInsets(top: 32, right: 12, bottom: 20, left: 37)
        let layout = tabletopInset.foldLayout([horizontal], in: tabletop, rightToLeft: false)
        precondition(layout.editor == ViewportInsets(top: 392, right: 12, bottom: 20, left: 37))
        precondition(layout.preview == CGRect(x: 37, y: 32, width: 811, height: 344))

        // A narrow window on one side of the fold already has room to
        // lay out its controls without an extra inset.
        let outer = CGRect(x: 0, y: 0, width: 360, height: 860)
        let outerLayout = inset.foldLayout([hinge], in: outer, rightToLeft: false)
        precondition(outerLayout.editor == inset && outerLayout.preview == nil)
    }

    private static func assertInsets(
        _ expected: ViewportInsets,
        input: (top: CGFloat, right: CGFloat, bottom: CGFloat, left: CGFloat),
        size: CGSize
    ) {
        let actual = ViewportInsets.clamped(
            top: input.top,
            right: input.right,
            bottom: input.bottom,
            left: input.left,
            to: size
        )
        precondition(actual == expected, "expected \(expected), got \(actual)")
    }
}
