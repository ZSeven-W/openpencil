import UIKit

/// Signed-in profile snapshot decoded from `op_editor_account_snapshot`.
struct AccountSnapshot: Decodable {
    let signedIn: Bool
    let displayName: String?
    let username: String?
    let primaryEmail: String?
    let avatarUrl: String?
    let deviceId: String?

    enum CodingKeys: String, CodingKey {
        case signedIn = "signed_in"
        case displayName = "display_name"
        case username
        case primaryEmail = "primary_email"
        case avatarUrl = "avatar_url"
        case deviceId = "device_id"
    }
}

/// Platform-native account center for the mobile shell.
///
/// Profile data comes from the engine's account snapshot (the private auth
/// runtime never exposes its device token to the shell), region selection is
/// a next-launch preference, and deeper management — linked sign-in methods,
/// devices, email changes — opens the regional web account page in the
/// system browser.
final class AccountCenterViewController: UITableViewController {
    private enum Row {
        case region
        case manageInBrowser
        case signOut
    }

    private let snapshot: AccountSnapshot
    private let onSignOut: () -> Void
    private let rows: [Row] = [.region, .manageInBrowser, .signOut]
    private var avatarTask: URLSessionDataTask?
    private let avatarView = UIImageView()

    init(snapshot: AccountSnapshot, onSignOut: @escaping () -> Void) {
        self.snapshot = snapshot
        self.onSignOut = onSignOut
        super.init(style: .insetGrouped)
    }

    @available(*, unavailable)
    required init?(coder: NSCoder) {
        fatalError("init(coder:) is not supported")
    }

    deinit {
        avatarTask?.cancel()
    }

    override func viewDidLoad() {
        super.viewDidLoad()
        title = NSLocalizedString(
            "accountCenter.title",
            value: "Account",
            comment: "Account center title"
        )
        navigationItem.rightBarButtonItem = UIBarButtonItem(
            image: AuthTheme.lucideIcon("Icon-x", pointSize: 20),
            style: .plain,
            target: self,
            action: #selector(closePressed)
        )
        tableView.tableHeaderView = makeHeader()
        loadAvatar()
    }

    // MARK: - Header

    private func makeHeader() -> UIView {
        let header = UIView(frame: CGRect(x: 0, y: 0, width: 0, height: 148))

        avatarView.translatesAutoresizingMaskIntoConstraints = false
        avatarView.layer.cornerRadius = 32
        avatarView.clipsToBounds = true
        avatarView.backgroundColor = .secondarySystemFill
        avatarView.contentMode = .scaleAspectFill
        avatarView.image = AuthTheme.lucideIcon("Icon-user", pointSize: 64)
        avatarView.tintColor = .tertiaryLabel

        let nameLabel = UILabel()
        nameLabel.translatesAutoresizingMaskIntoConstraints = false
        nameLabel.font = .preferredFont(forTextStyle: .title2)
        nameLabel.text = snapshot.displayName ?? snapshot.username
            ?? NSLocalizedString(
                "accountCenter.signedOut",
                value: "Not signed in",
                comment: "Signed-out fallback"
            )

        let detailLabel = UILabel()
        detailLabel.translatesAutoresizingMaskIntoConstraints = false
        detailLabel.font = .preferredFont(forTextStyle: .footnote)
        detailLabel.textColor = .secondaryLabel
        detailLabel.text = snapshot.primaryEmail ?? snapshot.username ?? ""

        header.addSubview(avatarView)
        header.addSubview(nameLabel)
        header.addSubview(detailLabel)
        NSLayoutConstraint.activate([
            avatarView.widthAnchor.constraint(equalToConstant: 64),
            avatarView.heightAnchor.constraint(equalToConstant: 64),
            avatarView.centerXAnchor.constraint(equalTo: header.centerXAnchor),
            avatarView.topAnchor.constraint(equalTo: header.topAnchor, constant: 16),
            nameLabel.centerXAnchor.constraint(equalTo: header.centerXAnchor),
            nameLabel.topAnchor.constraint(equalTo: avatarView.bottomAnchor, constant: 8),
            detailLabel.centerXAnchor.constraint(equalTo: header.centerXAnchor),
            detailLabel.topAnchor.constraint(equalTo: nameLabel.bottomAnchor, constant: 2),
        ])
        return header
    }

    private func loadAvatar() {
        guard
            let raw = snapshot.avatarUrl,
            let url = URL(string: raw),
            url.scheme?.lowercased() == "https"
        else { return }
        let task = URLSession.shared.dataTask(with: url) { [weak self] data, _, _ in
            guard let data, let image = UIImage(data: data) else { return }
            DispatchQueue.main.async { self?.avatarView.image = image }
        }
        avatarTask = task
        task.resume()
    }

    // MARK: - Table

    override func tableView(_ tableView: UITableView, numberOfRowsInSection section: Int) -> Int {
        rows.count
    }

    override func tableView(
        _ tableView: UITableView,
        cellForRowAt indexPath: IndexPath
    ) -> UITableViewCell {
        let cell = UITableViewCell(style: .value1, reuseIdentifier: nil)
        switch rows[indexPath.row] {
        case .region:
            cell.textLabel?.text = NSLocalizedString(
                "accountCenter.region",
                value: "Sign-in Region",
                comment: "Region row"
            )
            cell.detailTextLabel?.text = SsoRegionStore.resolved().displayName
            cell.accessoryType = .disclosureIndicator
        case .manageInBrowser:
            cell.textLabel?.text = NSLocalizedString(
                "accountCenter.manage",
                value: "Manage Account in Browser",
                comment: "Web account row"
            )
            cell.accessoryType = .disclosureIndicator
        case .signOut:
            cell.textLabel?.text = NSLocalizedString(
                "accountCenter.signOut",
                value: "Sign Out",
                comment: "Sign-out row"
            )
            cell.textLabel?.textColor = .systemRed
        }
        return cell
    }

    override func tableView(_ tableView: UITableView, didSelectRowAt indexPath: IndexPath) {
        tableView.deselectRow(at: indexPath, animated: true)
        switch rows[indexPath.row] {
        case .region:
            presentRegionPicker()
        case .manageInBrowser:
            UIApplication.shared.open(
                SsoRegionStore.resolved().origin.appendingPathComponent("account"),
                options: [:]
            )
        case .signOut:
            confirmSignOut()
        }
    }

    // MARK: - Actions

    @objc private func closePressed() {
        dismiss(animated: true)
    }

    private func presentRegionPicker() {
        let sheet = UIAlertController(
            title: NSLocalizedString(
                "sso.region.pickerTitle",
                value: "Sign-in Region",
                comment: "Region picker title"
            ),
            message: NSLocalizedString(
                "sso.region.restartNote",
                value: "A region change takes effect after you restart OpenPencil.",
                comment: "Region restart note"
            ),
            preferredStyle: .actionSheet
        )
        for region in SsoRegion.allCases {
            sheet.addAction(UIAlertAction(title: region.displayName, style: .default) {
                [weak self] _ in
                SsoRegionStore.saveUserOverride(region)
                self?.tableView.reloadData()
            })
        }
        sheet.addAction(UIAlertAction(
            title: NSLocalizedString("common.cancel", value: "Cancel", comment: "Cancel"),
            style: .cancel
        ))
        sheet.popoverPresentationController?.sourceView = view
        present(sheet, animated: true)
    }

    private func confirmSignOut() {
        let alert = UIAlertController(
            title: NSLocalizedString(
                "accountCenter.signOutConfirmTitle",
                value: "Sign out of OpenPencil?",
                comment: "Sign-out confirmation"
            ),
            message: nil,
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("common.cancel", value: "Cancel", comment: "Cancel"),
            style: .cancel
        ))
        alert.addAction(UIAlertAction(
            title: NSLocalizedString(
                "accountCenter.signOut",
                value: "Sign Out",
                comment: "Sign-out row"
            ),
            style: .destructive
        ) { [weak self] _ in
            guard let self else { return }
            self.onSignOut()
            self.dismiss(animated: true)
        })
        present(alert, animated: true)
    }
}

/// App-owned navigation wrapper mirroring the login presentation.
func makeAccountCenterPresentation(
    snapshot: AccountSnapshot,
    onSignOut: @escaping () -> Void
) -> UINavigationController {
    let controller = AccountCenterViewController(snapshot: snapshot, onSignOut: onSignOut)
    let navigation = UINavigationController(rootViewController: controller)
    navigation.modalPresentationStyle = .formSheet
    return navigation
}
