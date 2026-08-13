import UIKit
import WebKit

/// Full-screen, app-owned browser for one engine-provided sign-in request.
/// The controller never manufactures a login URL and never reports auth
/// success; the Rust auth flow remains authoritative and dismisses this view
/// after its own status poll completes.
final class EmbeddedLoginWebViewController: UIViewController {
    private let request: EmbeddedLoginRequest
    private let onCancel: (UInt64?) -> Void
    private let webView: WKWebView
    private let progressView = UIProgressView(progressViewStyle: .bar)
    private let errorLabel = UILabel()
    private let externalBrowserLabel = UILabel()
    private var progressObservation: NSKeyValueObservation?
    private var cancellationReported = false
    private var isFinishing = false
    private var pendingExternalURL: URL?
    private var userNavigationChainActive = false

    init(request: EmbeddedLoginRequest, onCancel: @escaping (UInt64?) -> Void) {
        self.request = request
        self.onCancel = onCancel

        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = request.dataStorePolicy == .persistent
            ? .default()
            : .nonPersistent()
        configuration.preferences.javaScriptCanOpenWindowsAutomatically = false
        configuration.defaultWebpagePreferences.allowsContentJavaScript = true
        configuration.defaultWebpagePreferences.preferredContentMode = .mobile
        webView = WKWebView(frame: .zero, configuration: configuration)
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        title = NSLocalizedString(
            "embeddedLogin.title",
            value: "Sign In",
            comment: "Embedded login title"
        )
        navigationItem.leftBarButtonItem = UIBarButtonItem(
            barButtonSystemItem: .close,
            target: self,
            action: #selector(cancelButtonPressed)
        )
        navigationItem.leftBarButtonItem?.accessibilityLabel = NSLocalizedString(
            "embeddedLogin.close",
            value: "Close sign-in",
            comment: "Embedded login close button"
        )
        navigationItem.largeTitleDisplayMode = .never
        view.backgroundColor = .systemBackground

        webView.navigationDelegate = self
        webView.uiDelegate = self
        webView.allowsBackForwardNavigationGestures = true
        webView.allowsLinkPreview = false
        webView.scrollView.contentInsetAdjustmentBehavior = .automatic
        webView.translatesAutoresizingMaskIntoConstraints = false

        progressView.translatesAutoresizingMaskIntoConstraints = false
        progressView.isHidden = true

        errorLabel.translatesAutoresizingMaskIntoConstraints = false
        errorLabel.font = .preferredFont(forTextStyle: .body)
        errorLabel.textColor = .secondaryLabel
        errorLabel.textAlignment = .center
        errorLabel.numberOfLines = 0
        errorLabel.isHidden = true
        errorLabel.text = NSLocalizedString(
            "embeddedLogin.loadError",
            value: "Unable to load the sign-in page. Check your connection and try again.",
            comment: "Embedded login load error"
        )

        externalBrowserLabel.translatesAutoresizingMaskIntoConstraints = false
        externalBrowserLabel.font = .preferredFont(forTextStyle: .footnote)
        externalBrowserLabel.textColor = .secondaryLabel
        externalBrowserLabel.textAlignment = .center
        externalBrowserLabel.numberOfLines = 2
        externalBrowserLabel.isHidden = true
        externalBrowserLabel.text = NSLocalizedString(
            "embeddedLogin.externalBrowser",
            value: "Continue signing in in your browser, then return to OpenPencil.",
            comment: "External identity provider sign-in guidance"
        )

        view.addSubview(webView)
        view.addSubview(errorLabel)
        view.addSubview(externalBrowserLabel)
        view.addSubview(progressView)
        NSLayoutConstraint.activate([
            webView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            webView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            webView.topAnchor.constraint(equalTo: view.topAnchor),
            webView.bottomAnchor.constraint(equalTo: view.bottomAnchor),
            progressView.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            progressView.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            progressView.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
            errorLabel.leadingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.leadingAnchor,
                constant: 24
            ),
            errorLabel.trailingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.trailingAnchor,
                constant: -24
            ),
            errorLabel.centerYAnchor.constraint(equalTo: view.safeAreaLayoutGuide.centerYAnchor),
            externalBrowserLabel.leadingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.leadingAnchor,
                constant: 24
            ),
            externalBrowserLabel.trailingAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.trailingAnchor,
                constant: -24
            ),
            externalBrowserLabel.bottomAnchor.constraint(
                equalTo: view.safeAreaLayoutGuide.bottomAnchor,
                constant: -12
            ),
        ])

        progressObservation = webView.observe(\.estimatedProgress, options: [.new]) {
            [weak self] webView, _ in
            guard let self else { return }
            self.progressView.progress = Float(webView.estimatedProgress)
            self.progressView.isHidden = webView.estimatedProgress >= 1
        }
        webView.load(URLRequest(
            url: request.initialURL,
            cachePolicy: .useProtocolCachePolicy,
            timeoutInterval: 60
        ))
    }

    deinit {
        progressObservation?.invalidate()
        webView.stopLoading()
        webView.navigationDelegate = nil
        webView.uiDelegate = nil
    }

    /// Called by the host after Rust reports a terminal auth status. This does
    /// not emit a cancellation back into an already-completed flow.
    func finishFromHost(animated: Bool) {
        guard !isFinishing else { return }
        isFinishing = true
        pendingExternalURL = nil
        webView.stopLoading()
        dismiss(animated: animated)
    }

    /// Called during engine teardown. The view is removed without trying to
    /// re-enter or cancel an engine that is already being destroyed.
    func finishForTeardown(animated: Bool) {
        finishFromHost(animated: animated)
    }

    @objc private func cancelButtonPressed() {
        guard !isFinishing else { return }
        isFinishing = true
        pendingExternalURL = nil
        webView.stopLoading()
        if !cancellationReported {
            cancellationReported = true
            onCancel(request.flowID)
        }
        dismiss(animated: true)
    }

    private func showNavigationError() {
        errorLabel.isHidden = false
        webView.isHidden = true
        progressView.isHidden = true
    }
}

extension EmbeddedLoginWebViewController: WKNavigationDelegate {
    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        guard !navigationAction.shouldPerformDownload, let url = navigationAction.request.url else {
            decisionHandler(.cancel)
            return
        }
        if navigationAction.targetFrame?.isMainFrame == false {
            decisionHandler(request.allowsSubframeNavigation(to: url) ? .allow : .cancel)
            return
        }
        let directlyUserInitiated = Self.isUserInitiated(navigationAction.navigationType)
        if directlyUserInitiated && request.allowsTopLevelNavigation(to: url) {
            userNavigationChainActive = true
        }
        switch request.topLevelDisposition(
            to: url,
            userInitiated: directlyUserInitiated || userNavigationChainActive
        ) {
        case .allowEmbedded:
            decisionHandler(.allow)
        case .openInitialURLExternally:
            // Re-enter through the original verification URI. Opening a
            // provider URL directly would skip the SSO pairing state/cookies
            // that produced the redirect and can violate embedded-UA policy.
            pendingExternalURL = request.initialURL
            userNavigationChainActive = false
            decisionHandler(.cancel)
            DispatchQueue.main.async { [weak self] in self?.openPendingExternalURL() }
        case .reject:
            if !request.allowsTopLevelNavigation(to: url) {
                userNavigationChainActive = false
            }
            decisionHandler(.cancel)
        }
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationResponse: WKNavigationResponse,
        decisionHandler: @escaping (WKNavigationResponsePolicy) -> Void
    ) {
        guard
            navigationResponse.canShowMIMEType,
            let url = navigationResponse.response.url
        else {
            decisionHandler(.cancel)
            return
        }
        let allowed = navigationResponse.isForMainFrame
            ? request.allowsTopLevelNavigation(to: url)
            : request.allowsSubframeNavigation(to: url)
        decisionHandler(allowed ? .allow : .cancel)
    }

    func webView(_ webView: WKWebView, didStartProvisionalNavigation navigation: WKNavigation?) {
        errorLabel.isHidden = true
        webView.isHidden = false
        progressView.isHidden = false
    }

    func webView(_ webView: WKWebView, didFinish navigation: WKNavigation?) {
        userNavigationChainActive = false
    }

    func webView(
        _ webView: WKWebView,
        didFailProvisionalNavigation navigation: WKNavigation?,
        withError error: Error
    ) {
        guard (error as NSError).code != NSURLErrorCancelled else { return }
        userNavigationChainActive = false
        showNavigationError()
    }

    func webView(_ webView: WKWebView, didFail navigation: WKNavigation?, withError error: Error) {
        guard (error as NSError).code != NSURLErrorCancelled else { return }
        userNavigationChainActive = false
        showNavigationError()
    }

    func webView(
        _ webView: WKWebView,
        didReceive challenge: URLAuthenticationChallenge,
        completionHandler: @escaping (URLSession.AuthChallengeDisposition, URLCredential?) -> Void
    ) {
        switch challenge.protectionSpace.authenticationMethod {
        case NSURLAuthenticationMethodHTTPBasic, NSURLAuthenticationMethodHTTPDigest:
            completionHandler(.cancelAuthenticationChallenge, nil)
        default:
            completionHandler(.performDefaultHandling, nil)
        }
    }
}

extension EmbeddedLoginWebViewController: WKUIDelegate {
    func webView(
        _ webView: WKWebView,
        createWebViewWith configuration: WKWebViewConfiguration,
        for navigationAction: WKNavigationAction,
        windowFeatures: WKWindowFeatures
    ) -> WKWebView? {
        guard navigationAction.targetFrame == nil, let url = navigationAction.request.url else {
            return nil
        }
        let directlyUserInitiated = Self.isUserInitiated(navigationAction.navigationType)
        switch request.topLevelDisposition(
            to: url,
            userInitiated: directlyUserInitiated || userNavigationChainActive
        ) {
        case .allowEmbedded:
            if directlyUserInitiated {
                userNavigationChainActive = true
            }
            webView.load(navigationAction.request)
        case .openInitialURLExternally:
            pendingExternalURL = request.initialURL
            userNavigationChainActive = false
            DispatchQueue.main.async { [weak self] in self?.openPendingExternalURL() }
        case .reject:
            break
        }
        return nil
    }

    private func openPendingExternalURL() {
        guard !isFinishing, let url = pendingExternalURL else { return }
        pendingExternalURL = nil
        externalBrowserLabel.isHidden = false
        UIApplication.shared.open(url, options: [:])
    }
}

private extension EmbeddedLoginWebViewController {
    static func isUserInitiated(_ type: WKNavigationType) -> Bool {
        switch type {
        case .linkActivated, .formSubmitted, .formResubmitted:
            return true
        default:
            return false
        }
    }
}

/// Wraps the browser in an app-owned navigation bar. UIKit supplies the
/// status-bar/home-indicator safe areas and a standard 44-point close target.
func makeEmbeddedLoginPresentation(
    request: EmbeddedLoginRequest,
    onCancel: @escaping (UInt64?) -> Void
) -> (UINavigationController, EmbeddedLoginWebViewController) {
    let browser = EmbeddedLoginWebViewController(request: request, onCancel: onCancel)
    let navigation = UINavigationController(rootViewController: browser)
    navigation.modalPresentationStyle = .fullScreen
    navigation.isModalInPresentation = true
    navigation.navigationBar.prefersLargeTitles = false
    return (navigation, browser)
}
