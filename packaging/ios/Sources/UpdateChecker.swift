import UIKit

/// Launch-time update check: reads the latest release tag from the public
/// GitHub repository, compares it with the running bundle version, and
/// offers to open the App Store page. Until the app has a live App Store
/// numeric id, the button falls back to the GitHub releases page.
enum UpdateChecker {
    /// Fill in after App Store release (the numeric id from App Store
    /// Connect); an empty value falls back to the releases page.
    private static let appStoreID = ""
    private static let releasesAPI =
        URL(string: "https://api.github.com/repos/ZSeven-W/openpencil/releases/latest")!
    private static let releasesPage =
        URL(string: "https://github.com/ZSeven-W/openpencil/releases/latest")!
    private static let lastCheckKey = "update.lastCheck"
    private static let checkInterval: TimeInterval = 24 * 60 * 60

    /// Once per day; a prompt appears only when the remote version is
    /// strictly newer. Never blocks launch — pure background fetch.
    static func checkOncePerDay(presenter: @escaping () -> UIViewController?) {
        let now = Date().timeIntervalSince1970
        let last = UserDefaults.standard.double(forKey: lastCheckKey)
        guard now - last >= checkInterval else { return }
        UserDefaults.standard.set(now, forKey: lastCheckKey)

        var request = URLRequest(url: releasesAPI)
        request.setValue("application/vnd.github+json", forHTTPHeaderField: "Accept")
        let task = URLSession.shared.dataTask(with: request) { data, response, _ in
            guard
                let data,
                let http = response as? HTTPURLResponse,
                (200..<300).contains(http.statusCode),
                let payload = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
                let tag = payload["tag_name"] as? String
            else { return }
            let remote = normalized(tag)
            let local = normalized(
                Bundle.main.object(forInfoDictionaryKey: "CFBundleShortVersionString")
                    as? String ?? "0"
            )
            guard isNewer(remote: remote, local: local) else { return }
            DispatchQueue.main.async {
                presentPrompt(version: tag, presenter: presenter)
            }
        }
        task.resume()
    }

    /// "v0.8.5" / "0.8.5" → [0, 8, 5].
    static func normalized(_ tag: String) -> [Int] {
        tag.trimmingCharacters(in: CharacterSet(charactersIn: "vV "))
            .split(separator: "-").first
            .map { $0.split(separator: ".").compactMap { Int($0) } } ?? []
    }

    static func isNewer(remote: [Int], local: [Int]) -> Bool {
        guard !remote.isEmpty, !local.isEmpty else { return false }
        let count = max(remote.count, local.count)
        for index in 0..<count {
            let r = index < remote.count ? remote[index] : 0
            let l = index < local.count ? local[index] : 0
            if r != l { return r > l }
        }
        return false
    }

    private static func presentPrompt(
        version: String,
        presenter: @escaping () -> UIViewController?
    ) {
        guard
            let controller = presenter(),
            controller.presentedViewController == nil
        else { return }
        let alert = UIAlertController(
            title: String(
                format: NSLocalizedString(
                    "update.availableTitle",
                    value: "Version %@ available",
                    comment: "Update prompt (%@ = version)"
                ),
                version
            ),
            message: NSLocalizedString(
                "update.availableBody",
                value: "A newer version of OpenPencil is available.",
                comment: "Update prompt body"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("sso.region.later", value: "Later", comment: "Defer"),
            style: .cancel
        ))
        alert.addAction(UIAlertAction(
            title: NSLocalizedString(
                "update.goToStore",
                value: "Update",
                comment: "Open the store"
            ),
            style: .default
        ) { _ in
            let destination = appStoreID.isEmpty
                ? releasesPage
                : URL(string: "itms-apps://apps.apple.com/app/id\(appStoreID)") ?? releasesPage
            UIApplication.shared.open(destination, options: [:])
        })
        controller.present(alert, animated: true)
    }
}
