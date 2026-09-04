import SwiftUI

@main
struct OpenPencilPlayerApp: App {
    var body: some Scene {
        WindowGroup {
            // Edge-to-edge: the Metal view spans the full screen; the
            // engine consumes the safe-area insets and lays the chrome
            // out against the usable rectangle. Include the keyboard region
            // explicitly so SwiftUI never resizes or lifts the whole editor;
            // docked keyboard occlusion is delivered through its own channel.
            OpPlayerContainer()
                .ignoresSafeArea(.all, edges: .all)
                // Native provider sign-in SDKs (Douyin / Alipay) return via
                // the app's URL schemes; whatever they do not consume routes
                // through the universal-link handler.
                .onOpenURL { url in
                    if NativeProviderCallbacks.handle(url) { return }
                    UniversalLinkRouter.handle(url)
                }
                // WeChat returns through the app's universal link; hand the
                // activity to its SDK first, then fall back to the in-app
                // universal-link router.
                .onContinueUserActivity(NSUserActivityTypeBrowsingWeb) { activity in
                    if WechatNativeSignIn.handleUniversalLink(activity) { return }
                    if let url = activity.webpageURL {
                        UniversalLinkRouter.handle(url)
                    }
                }
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
