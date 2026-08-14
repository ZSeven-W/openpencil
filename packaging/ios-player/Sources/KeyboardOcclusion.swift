import CoreGraphics

/// Resolves the bottom-docked keyboard occlusion reported to the engine.
///
/// `UIKeyboardLayoutGuide.layoutFrame` is already expressed in the owning
/// view's coordinate space, so this remains correct for Split View and Stage
/// Manager windows whose origin does not match the screen origin. With
/// `followsUndockedKeyboard == false`, floating and split keyboards collapse
/// back to the bottom safe-area guide instead of shrinking the whole editor.
enum KeyboardOcclusion {
    private static let geometryTolerance: CGFloat = 1

    static func bottomHeight(
        bounds: CGRect,
        guideFrame: CGRect,
        safeAreaBottom: CGFloat,
        firstResponder: Bool
    ) -> CGFloat {
        guard firstResponder, isFinite(bounds), isFinite(guideFrame) else { return 0 }

        let intersection = bounds.intersection(guideFrame)
        guard !intersection.isNull, !intersection.isEmpty else { return 0 }
        guard abs(intersection.maxY - bounds.maxY) <= geometryTolerance else { return 0 }

        let height = max(0, min(bounds.height, bounds.maxY - intersection.minY))
        let safeArea = max(0, min(bounds.height, safeAreaBottom))
        // When no docked software keyboard is present, UIKit leaves the guide
        // at the bottom safe area. The iPad compact input assistant can also
        // produce a full-width keyboard notification while this guide remains
        // at that baseline; do not turn that assistant into a global inset.
        return height > safeArea + geometryTolerance ? height : 0
    }

    private static func isFinite(_ rect: CGRect) -> Bool {
        rect.origin.x.isFinite
            && rect.origin.y.isFinite
            && rect.size.width.isFinite
            && rect.size.height.isFinite
    }
}
