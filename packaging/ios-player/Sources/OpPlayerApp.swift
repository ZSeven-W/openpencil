import SwiftUI

@main
struct OpenPencilPlayerApp: App {
    var body: some Scene {
        WindowGroup {
            // Edge-to-edge: the Metal view spans the full screen; the
            // engine consumes the safe-area insets and lays the chrome
            // out against the usable rectangle.
            OpPlayerContainer()
                .ignoresSafeArea()
        }
    }
}

private struct OpPlayerContainer: UIViewRepresentable {
    /// Launch with `-editor` for the full desktop chrome (default), or
    /// `-viewer` for the bare document viewer.
    private static let editorMode: Bool = {
        let arguments = ProcessInfo.processInfo.arguments
        if arguments.contains("-viewer") { return false }
        return true
    }()

    func makeUIView(context: Context) -> OpPlayerView {
        OpPlayerView(editorMode: Self.editorMode)
    }

    func updateUIView(_ view: OpPlayerView, context: Context) {
        view.setNeedsLayout()
    }

    static func dismantleUIView(_ view: OpPlayerView, coordinator: ()) {
        view.teardownEngine()
    }
}
