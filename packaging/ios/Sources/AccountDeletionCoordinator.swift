import Combine
import CryptoKit
import Foundation
import Security
import UIKit

/// Owns receipt verification and crash-safe local cleanup. Browser dismissal,
/// sign-out and auth errors are never interpreted as deletion completion.
final class AccountDeletionCoordinator: NSObject, ObservableObject, URLSessionTaskDelegate {
    enum Gate { case checking, editor, recovery, restartRequired }
    static let shared = AccountDeletionCoordinator()

    @Published private(set) var gate: Gate = .checking
    @Published private(set) var busy = false
    @Published private(set) var message = AccountDeletionText.checking
    @Published private(set) var receipt: AccountDeletionReceipt?
    @Published private(set) var serverState: AccountDeletionServerState?
    @Published private(set) var canRenewRequest = false
    @Published private(set) var isPreparingCleanup = false

    private let players = NSHashTable<OpPlayerView>.weakObjects()
    private var bootstrapped = false
    private var hasStartedEditor = false
    private var pollTimer: Timer?
    private var pollAttempt = 0
    private var task: URLSessionDataTask?
    private var foregroundObserver: NSObjectProtocol?
    private lazy var session: URLSession = {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.timeoutIntervalForRequest = 15
        configuration.timeoutIntervalForResource = 30
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.urlCache = nil
        configuration.requestCachePolicy = .reloadIgnoringLocalCacheData
        return URLSession(configuration: configuration, delegate: self, delegateQueue: nil)
    }()

    private static func paths() throws -> AccountDeletionLocalPaths {
        AccountDeletionLocalPaths.inSandbox(container: URL(fileURLWithPath: NSHomeDirectory()))
    }

    private static func store() throws -> AccountDeletionStore {
        AccountDeletionStore(directory: try paths().applicationSupport
            .appendingPathComponent("OpenPencilDeletion", isDirectory: true))
    }

    func register(_ player: OpPlayerView) {
        precondition(Thread.isMainThread)
        hasStartedEditor = true
        players.add(player)
        if isPreparingCleanup { player.teardownEngine() }
    }

    /// This gate runs before the first OpPlayerView/engine is constructed.
    func bootstrap() {
        precondition(Thread.isMainThread)
        guard !bootstrapped else { return }
        bootstrapped = true
        foregroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.didBecomeActiveNotification, object: nil, queue: .main
        ) { [weak self] _ in self?.retry() }
        do {
            receipt = try Self.store().load()
            guard receipt != nil else { gate = .editor; return }
            retry()
        } catch {
            gate = .recovery
            message = AccountDeletionText.recoveryFailure
        }
    }

    func continueEditing() {
        guard receipt?.phase == .awaitingServer, gate == .recovery else { return }
        gate = .editor
        schedulePoll()
    }

    func begin(region: SsoRegion, deviceID: String, completion: @escaping (URL?) -> Void) {
        precondition(Thread.isMainThread)
        let canRecover = gate == .recovery && canRenewRequest && receipt?.phase == .awaitingServer
        guard !busy, gate == .editor || canRecover else { completion(nil); return }
        if let receipt, !canRenewRequest {
            message = AccountDeletionText.pending
            completion(receipt.phase == .awaitingServer ? receipt.deletionURL : nil)
            return
        }
        // A renewal retains the original region/device binding even if sign-out
        // or a different account is now displayed by the editor.
        let requestOrigin = receipt?.origin ?? region.origin
        let requestDevice = receipt?.deviceID ?? deviceID
        guard !requestDevice.isEmpty else { message = AccountDeletionText.requestFailure; completion(nil); return }
        var random = [UInt8](repeating: 0, count: 32)
        guard SecRandomCopyBytes(kSecRandomDefault, random.count, &random) == errSecSuccess else {
            message = AccountDeletionText.requestFailure
            completion(nil)
            return
        }
        let secret = Data(random).base64EncodedString().replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_").replacingOccurrences(of: "=", with: "")
        let challenge = SHA256.hash(data: Data(secret.utf8)).map { String(format: "%02x", $0) }.joined()
        var request = URLRequest(url: requestOrigin.appendingPathComponent("api/v1/openpencil/deletion-requests"))
        request.httpMethod = "POST"
        request.setValue("application/json", forHTTPHeaderField: "Content-Type")
        request.httpBody = try? JSONSerialization.data(withJSONObject: [
            "device_id": requestDevice, "receipt_challenge": challenge,
        ])
        busy = true
        message = AccountDeletionText.checking
        perform(request) { [weak self] result in
            guard let self else { completion(nil); return }
            self.busy = false
            do {
                let response = try result.get()
                let receipt = try response.makeReceipt(
                    origin: requestOrigin, deviceID: requestDevice, secret: secret, now: Date()
                )
                // Never open the destructive web flow until its recovery receipt is durable.
                try Self.store().save(receipt)
                self.receipt = receipt
                self.canRenewRequest = false
                self.serverState = .awaitingConfirmation
                self.message = AccountDeletionText.awaitingConfirmation
                self.schedulePoll()
                completion(receipt.deletionURL)
            } catch {
                self.message = AccountDeletionText.requestFailure
                completion(nil)
            }
        }
    }

    func restartVerification(completion: @escaping (URL?) -> Void) {
        guard let receipt, canRenewRequest else { completion(nil); return }
        begin(
            region: receipt.origin == SsoRegion.china.origin ? .china : .global,
            deviceID: receipt.deviceID, completion: completion
        )
    }

    func retry() {
        precondition(Thread.isMainThread)
        guard bootstrapped, !busy, gate != .restartRequired else { return }
        pollTimer?.invalidate()
        pollTimer = nil
        do {
            if receipt == nil { receipt = try Self.store().load() }
            guard let receipt else {
                gate = .editor
                return
            }
            if receipt.phase == .localCleanupPending {
                finishLocalCleanup()
                return
            }
            var request = URLRequest(url: receipt.origin.appendingPathComponent(
                "api/v1/openpencil/deletion-requests/\(receipt.requestID)"
            ))
            request.setValue("Bearer \(receipt.secret)", forHTTPHeaderField: "Authorization")
            busy = true
            perform(request) { [weak self] result in
                guard let self else { return }
                self.busy = false
                do {
                    let response = try result.get()
                    let state = try response.verifiedState(for: receipt)
                    let refreshed = try response.refreshedReceipt(receipt)
                    try Self.store().save(refreshed)
                    self.receipt = refreshed
                    self.canRenewRequest = false
                    self.serverState = state
                    switch state {
                    case .deleted:
                        self.receipt = try Self.store().markCleanupPending(refreshed)
                        self.finishLocalCleanup()
                    case .awaitingConfirmation, .deleting:
                        self.message = state == .deleting
                            ? AccountDeletionText.pending : AccountDeletionText.awaitingConfirmation
                        if self.gate == .checking { self.gate = .editor }
                        self.schedulePoll()
                    }
                } catch {
                    if (error as? AccountDeletionError)?.permitsReceiptRenewal == true {
                        self.canRenewRequest = true
                    }
                    self.message = AccountDeletionText.verificationFailure
                    if self.gate == .checking { self.gate = .recovery }
                    self.schedulePoll()
                }
            }
        } catch {
            gate = .recovery
            message = AccountDeletionText.recoveryFailure
        }
    }

    private func schedulePoll() {
        guard receipt?.phase == .awaitingServer, gate != .restartRequired,
              UIApplication.shared.applicationState == .active else { return }
        pollTimer?.invalidate()
        pollAttempt += 1
        let delay = min(60.0, 5.0 * Double(min(pollAttempt, 12)))
        pollTimer = Timer.scheduledTimer(withTimeInterval: delay, repeats: false) { [weak self] _ in
            guard UIApplication.shared.applicationState == .active else { return }
            self?.retry()
        }
    }

    private func finishLocalCleanup() {
        guard receipt?.phase == .localCleanupPending else { return }
        busy = true
        isPreparingCleanup = true
        message = AccountDeletionText.cleaning
        pollTimer?.invalidate()
        pollTimer = nil
        task?.cancel()
        // All scenes must stop before files are erased. The global auth runtime
        // is intentionally not reused after this process has hosted an editor.
        let group = DispatchGroup()
        var teardownSucceeded = true
        for player in players.allObjects {
            group.enter()
            player.prepareForAccountDeletion { success in
                teardownSucceeded = teardownSucceeded && success
                group.leave()
            }
        }
        group.notify(queue: .main) { [self] in
            gate = .recovery
            isPreparingCleanup = false
            guard teardownSucceeded else {
                busy = false
                message = AccountDeletionText.recoveryFailure
                return
            }
            eraseLocalData()
        }
    }

    private func eraseLocalData() {
        do {
            guard let receipt else { throw AccountDeletionError.invalidReceipt }
            try AccountDeletionLocalCleanup.finish(
                receipt: receipt, store: Self.store(),
                removeFiles: { try Self.paths().removeFiles() },
                removeCredentials: Self.removeCollaborationKey,
                clearPreferences: {
                    URLCache.shared.removeAllCachedResponses()
                    if let identifier = Bundle.main.bundleIdentifier {
                        UserDefaults.standard.removePersistentDomain(forName: identifier)
                    }
                }
            )
            self.receipt = nil
            message = AccountDeletionText.completed
            gate = hasStartedEditor ? .restartRequired : .editor
        } catch {
            // The durable marker remains, so the next launch retries before auth init.
            message = AccountDeletionText.recoveryFailure
        }
        busy = false
    }

    private static func removeCollaborationKey() throws {
        let status = SecItemDelete([
            kSecClass: kSecClassGenericPassword,
            kSecAttrService: "tech.zseven.openpencil.collaboration",
            kSecAttrAccount: "device-noise-x25519-v1",
        ] as CFDictionary)
        guard status == errSecSuccess || status == errSecItemNotFound else {
            throw AccountDeletionError.keychainFailure
        }
    }

    private func perform(
        _ request: URLRequest,
        completion: @escaping (Result<AccountDeletionResponse, Error>) -> Void
    ) {
        task = session.dataTask(with: request) { data, response, error in
            let result: Result<AccountDeletionResponse, Error> = Result {
                guard error == nil, let http = response as? HTTPURLResponse,
                      let data else { throw AccountDeletionError.transport }
                return try AccountDeletionResponse.decode(
                    data: data, statusCode: http.statusCode, responseURL: http.url, requestURL: request.url
                )
            }
            DispatchQueue.main.async { completion(result) }
        }
        task?.resume()
    }

    func urlSession(
        _ session: URLSession, task: URLSessionTask,
        willPerformHTTPRedirection response: HTTPURLResponse,
        newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void
    ) {
        // Receipt secrets are never sent through a redirect, even to another first-party host.
        completionHandler(nil)
    }
}
