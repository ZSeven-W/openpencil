import CoreGraphics

@main
enum KeyboardOcclusionTests {
    static func main() {
        let bounds = CGRect(x: 0, y: 0, width: 1_210, height: 834)

        assertHeight(
            0,
            guide: CGRect(x: 0, y: 809, width: 1_210, height: 25),
            safeAreaBottom: 25,
            firstResponder: true,
            bounds: bounds
        )
        assertHeight(
            370,
            guide: CGRect(x: 0, y: 464, width: 1_210, height: 370),
            safeAreaBottom: 25,
            firstResponder: true,
            bounds: bounds
        )
        assertHeight(
            0,
            guide: CGRect(x: 445, y: 400, width: 320, height: 260),
            safeAreaBottom: 25,
            firstResponder: true,
            bounds: bounds
        )
        assertHeight(
            0,
            guide: CGRect(x: 0, y: 464, width: 1_210, height: 370),
            safeAreaBottom: 25,
            firstResponder: false,
            bounds: bounds
        )

        // Stage Manager: both bounds and guide may have non-zero local origins;
        // the result depends only on their intersection in one coordinate space.
        let stageBounds = CGRect(x: 40, y: 20, width: 800, height: 600)
        assertHeight(
            240,
            guide: CGRect(x: 40, y: 380, width: 800, height: 240),
            safeAreaBottom: 20,
            firstResponder: true,
            bounds: stageBounds
        )

        // A phone's Home Indicator safe area is not a keyboard. When a
        // bottom-docked keyboard appears, UIKit reports the complete covered
        // height including that safe area; the engine removes the overlap once.
        let phonePortrait = CGRect(x: 0, y: 0, width: 402, height: 874)
        assertHeight(
            0,
            guide: CGRect(x: 0, y: 840, width: 402, height: 34),
            safeAreaBottom: 34,
            firstResponder: true,
            bounds: phonePortrait
        )
        assertHeight(
            335,
            guide: CGRect(x: 0, y: 539, width: 402, height: 335),
            safeAreaBottom: 34,
            firstResponder: true,
            bounds: phonePortrait
        )

        // Landscape keeps the side cutout in the safe-area channel and only
        // exposes a narrow bottom Home Indicator baseline here.
        let phoneLandscape = CGRect(x: 0, y: 0, width: 874, height: 402)
        assertHeight(
            0,
            guide: CGRect(x: 0, y: 381, width: 874, height: 21),
            safeAreaBottom: 21,
            firstResponder: true,
            bounds: phoneLandscape
        )
    }

    private static func assertHeight(
        _ expected: CGFloat,
        guide: CGRect,
        safeAreaBottom: CGFloat,
        firstResponder: Bool,
        bounds: CGRect
    ) {
        let actual = KeyboardOcclusion.bottomHeight(
            bounds: bounds,
            guideFrame: guide,
            safeAreaBottom: safeAreaBottom,
            firstResponder: firstResponder
        )
        precondition(actual == expected, "expected \(expected), got \(actual)")
    }
}
