import CoreGraphics

@main
enum ViewportInsetsTests {
    static func main() {
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
