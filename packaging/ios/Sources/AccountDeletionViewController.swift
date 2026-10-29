import Combine
import SafariServices
import UIKit

/// The receipt is retained when this screen closes. Only the server's secure
/// page confirms deletion; returning from Safari merely refreshes its status.
final class AccountDeletionViewController: UIViewController, SFSafariViewControllerDelegate {
    private let region: SsoRegion
    private let deviceID: String
    private let coordinator = AccountDeletionCoordinator.shared
    private let statusLabel = UILabel()
    private let continueButton = UIButton(type: .system)
    private let retryButton = UIButton(type: .system)
    private var observation: AnyCancellable?
    private var hasAttempted = false

    init(region: SsoRegion, deviceID: String) {
        self.region = region
        self.deviceID = deviceID
        super.init(nibName: nil, bundle: nil)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) { fatalError("init(coder:) is not supported") }

    override func viewDidLoad() {
        super.viewDidLoad()
        title = AccountDeletionText.title
        view.backgroundColor = .systemBackground
        let scroll = UIScrollView()
        scroll.translatesAutoresizingMaskIntoConstraints = false
        view.addSubview(scroll)
        let stack = UIStackView()
        stack.axis = .vertical
        stack.spacing = 20
        stack.translatesAutoresizingMaskIntoConstraints = false
        scroll.addSubview(stack)
        let explanation = UILabel()
        explanation.text = AccountDeletionText.explanation
        for label in [explanation, statusLabel] {
            label.font = .preferredFont(forTextStyle: .body)
            label.adjustsFontForContentSizeCategory = true
            label.numberOfLines = 0
        }
        statusLabel.textColor = .secondaryLabel
        continueButton.addTarget(self, action: #selector(continuePressed), for: .touchUpInside)
        continueButton.tintColor = .systemRed
        continueButton.accessibilityIdentifier = "confirmOpenPencilDeletion"
        retryButton.setTitle(AccountDeletionText.retry, for: .normal)
        retryButton.addTarget(self, action: #selector(retryPressed), for: .touchUpInside)
        for button in [continueButton, retryButton] {
            button.titleLabel?.font = .preferredFont(forTextStyle: .headline)
            button.titleLabel?.adjustsFontForContentSizeCategory = true
            button.titleLabel?.numberOfLines = 0
            button.heightAnchor.constraint(greaterThanOrEqualToConstant: 44).isActive = true
        }
        [explanation, statusLabel, continueButton, retryButton].forEach(stack.addArrangedSubview)
        NSLayoutConstraint.activate([
            scroll.topAnchor.constraint(equalTo: view.safeAreaLayoutGuide.topAnchor),
            scroll.bottomAnchor.constraint(equalTo: view.safeAreaLayoutGuide.bottomAnchor),
            scroll.leadingAnchor.constraint(equalTo: view.leadingAnchor),
            scroll.trailingAnchor.constraint(equalTo: view.trailingAnchor),
            stack.topAnchor.constraint(equalTo: scroll.contentLayoutGuide.topAnchor, constant: 24),
            stack.bottomAnchor.constraint(equalTo: scroll.contentLayoutGuide.bottomAnchor, constant: -24),
            stack.leadingAnchor.constraint(equalTo: scroll.contentLayoutGuide.leadingAnchor, constant: 24),
            stack.trailingAnchor.constraint(equalTo: scroll.contentLayoutGuide.trailingAnchor, constant: -24),
            stack.widthAnchor.constraint(equalTo: scroll.frameLayoutGuide.widthAnchor, constant: -48),
        ])
        observation = coordinator.objectWillChange.sink { [weak self] _ in
            DispatchQueue.main.async { self?.refresh() }
        }
        refresh()
    }

    private func refresh() {
        statusLabel.text = coordinator.receipt == nil && !hasAttempted
            ? nil : coordinator.message
        continueButton.setTitle(
            coordinator.canRenewRequest ? AccountDeletionText.renew
                : (coordinator.receipt == nil ? AccountDeletionText.confirm : AccountDeletionText.openPage),
            for: .normal
        )
        continueButton.isEnabled = !coordinator.busy
            && (coordinator.serverState != .deleting || coordinator.canRenewRequest)
        retryButton.isHidden = coordinator.receipt == nil
        retryButton.isEnabled = !coordinator.busy
    }

    @objc private func continuePressed() {
        hasAttempted = true
        coordinator.begin(region: region, deviceID: deviceID) { [weak self] url in
            guard let self, let url, self.presentedViewController == nil else { return }
            let safari = SFSafariViewController(url: url)
            safari.delegate = self
            self.present(safari, animated: true)
        }
    }

    @objc private func retryPressed() { coordinator.retry() }

    func safariViewControllerDidFinish(_ controller: SFSafariViewController) {
        coordinator.retry()
    }
}
