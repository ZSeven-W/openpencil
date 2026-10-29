import SafariServices
import UIKit

/// Privacy access shared by signed-out authentication and account settings.
enum AppPrivacy {
    static var title: String {
        NSLocalizedString("privacy.title", value: "Privacy Policy", comment: "Privacy policy link")
    }

    /// Only the two published first-party policy pages can be opened here.
    /// Use the active authentication origin, not a next-launch preference.
    static func policyURL(for ssoOrigin: URL) -> URL {
        let host = ssoOrigin.host?.lowercased() == "sso.zseven.cn"
            ? "op.zseven.cn" : "op.zseven.tech"
        return URL(string: "https://\(host)/privacy/ios.html")!
    }

    static func makeLink(target: Any?, action: Selector) -> UIButton {
        let button = AuthTheme.makeLink(title: title, target: target, action: action)
        button.configuration?.titleTextAttributesTransformer = UIConfigurationTextAttributesTransformer {
            var attributes = $0
            attributes.font = UIFont.preferredFont(forTextStyle: .footnote)
            return attributes
        }
        button.titleLabel?.adjustsFontForContentSizeCategory = true
        button.heightAnchor.constraint(greaterThanOrEqualToConstant: 44).isActive = true
        button.accessibilityIdentifier = "privacyPolicy"
        return button
    }

    static func present(from presenter: UIViewController, ssoOrigin: URL) {
        guard presenter.presentedViewController == nil else { return }
        presenter.view.endEditing(true)
        presenter.present(SFSafariViewController(url: policyURL(for: ssoOrigin)), animated: true)
    }
}
