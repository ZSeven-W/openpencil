import Foundation

enum AccountDeletionError: Error {
    case invalidResponse
    case invalidReceipt
    case unsafePath
    case keychainFailure
    case transport
    case rejected(Int)

    var permitsReceiptRenewal: Bool {
        if case .rejected(let status) = self { return status == 404 || status == 410 }
        return false
    }
}

/// The receipt secret never enters a URL or the engine's storage tree.
struct AccountDeletionReceipt: Codable, Equatable {
    enum Phase: String, Codable { case awaitingServer, localCleanupPending }
    let requestID: String
    let deviceID: String
    let origin: URL
    let secret: String
    let deletionURL: URL
    var expiresAt: Date
    var phase: Phase

    func validate() throws {
        guard Self.isPinnedOrigin(origin),
              UUID(uuidString: requestID)?.uuidString.lowercased() == requestID,
              !deviceID.isEmpty, deviceID.utf8.count <= 512,
              secret.utf8.count == 43,
              secret.allSatisfy({ $0.isASCII && ($0.isLetter || $0.isNumber || $0 == "-" || $0 == "_") }),
              Self.isDeletionURL(deletionURL, origin: origin, requestID: requestID)
        else { throw AccountDeletionError.invalidReceipt }
        let base64 = secret.replacingOccurrences(of: "-", with: "+")
            .replacingOccurrences(of: "_", with: "/") + "="
        guard let bytes = Data(base64Encoded: base64), bytes.count == 32,
              bytes.base64EncodedString().replacingOccurrences(of: "+", with: "-")
                .replacingOccurrences(of: "/", with: "_")
                .replacingOccurrences(of: "=", with: "") == secret
        else { throw AccountDeletionError.invalidReceipt }
    }

    static func isPinnedOrigin(_ url: URL) -> Bool {
        ["https://sso.zseven.cn", "https://sso.zseven.tech"].contains(url.absoluteString)
    }

    static func isDeletionURL(_ url: URL, origin: URL, requestID: String) -> Bool {
        guard let parts = URLComponents(url: url, resolvingAgainstBaseURL: false),
              parts.scheme == "https", parts.host == origin.host,
              parts.port == nil, parts.user == nil, parts.password == nil,
              parts.fragment == nil, parts.percentEncodedPath == "/openpencil/delete-account",
              parts.queryItems == [URLQueryItem(name: "request", value: requestID)]
        else { return false }
        return true
    }
}

enum AccountDeletionServerState: String, Decodable {
    case awaitingConfirmation = "awaiting_confirmation"
    case deleting
    case deleted
}

/// Required fields are decoded strictly. An HTTP success alone proves nothing.
struct AccountDeletionResponse: Decodable {
    let requestID: String
    let deviceID: String
    let scope: String
    let state: AccountDeletionServerState
    let deletionURL: URL?
    let expiresAt: String

    enum CodingKeys: String, CodingKey {
        case requestID = "request_id", deviceID = "device_id", scope, state
        case deletionURL = "deletion_url", expiresAt = "expires_at"
    }

    private var expiry: Date? {
        let formatter = ISO8601DateFormatter()
        if let date = formatter.date(from: expiresAt) { return date }
        formatter.formatOptions.insert(.withFractionalSeconds)
        return formatter.date(from: expiresAt)
    }

    static func decode(
        data: Data, statusCode: Int, responseURL: URL?, requestURL: URL?
    ) throws -> AccountDeletionResponse {
        guard responseURL == requestURL, responseURL != nil, data.count <= 16 * 1024 else {
            throw AccountDeletionError.invalidResponse
        }
        guard (200..<300).contains(statusCode) else { throw AccountDeletionError.rejected(statusCode) }
        return try JSONDecoder().decode(Self.self, from: data)
    }

    func makeReceipt(origin: URL, deviceID expectedDevice: String, secret: String, now: Date) throws
        -> AccountDeletionReceipt {
        guard scope == "openpencil", deviceID == expectedDevice,
              state == .awaitingConfirmation,
              let deletionURL, let expiresAt = expiry, expiresAt > now
        else { throw AccountDeletionError.invalidResponse }
        let receipt = AccountDeletionReceipt(
            requestID: requestID, deviceID: deviceID, origin: origin, secret: secret,
            deletionURL: deletionURL, expiresAt: expiresAt, phase: .awaitingServer
        )
        try receipt.validate()
        return receipt
    }

    func verifiedState(for receipt: AccountDeletionReceipt) throws -> AccountDeletionServerState {
        try receipt.validate()
        guard requestID == receipt.requestID, deviceID == receipt.deviceID,
              scope == "openpencil", expiry != nil,
              deletionURL == nil || deletionURL == receipt.deletionURL
        else { throw AccountDeletionError.invalidResponse }
        return state
    }

    func refreshedReceipt(_ receipt: AccountDeletionReceipt) throws -> AccountDeletionReceipt {
        _ = try verifiedState(for: receipt)
        guard let expiry else { throw AccountDeletionError.invalidResponse }
        var next = receipt
        next.expiresAt = expiry
        return next
    }
}

/// Atomic, private, backup-excluded storage outside every deletion target.
struct AccountDeletionStore {
    let directory: URL
    private var receiptURL: URL { directory.appendingPathComponent("receipt.json") }

    private func validateLocation() throws {
        guard directory.isFileURL,
              directory.lastPathComponent == "OpenPencilDeletion",
              directory.deletingLastPathComponent().lastPathComponent == "Application Support",
              directory.resolvingSymlinksInPath().path == directory.standardizedFileURL.path
        else { throw AccountDeletionError.unsafePath }
        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: directory.path)
            guard attributes[.type] as? FileAttributeType == .typeDirectory else {
                throw AccountDeletionError.unsafePath
            }
        } catch let error as NSError {
            guard error.domain == NSCocoaErrorDomain,
                  [NSFileNoSuchFileError, NSFileReadNoSuchFileError].contains(error.code)
            else { throw error }
        }
        do {
            let attributes = try FileManager.default.attributesOfItem(atPath: receiptURL.path)
            guard attributes[.type] as? FileAttributeType == .typeRegular else {
                throw AccountDeletionError.unsafePath
            }
        } catch let error as NSError {
            guard error.domain == NSCocoaErrorDomain,
                  [NSFileNoSuchFileError, NSFileReadNoSuchFileError].contains(error.code)
            else { throw error }
        }
    }

    func load() throws -> AccountDeletionReceipt? {
        try validateLocation()
        guard FileManager.default.fileExists(atPath: receiptURL.path) else { return nil }
        let data = try Data(contentsOf: receiptURL)
        guard data.count <= 16 * 1024 else { throw AccountDeletionError.invalidReceipt }
        let receipt = try JSONDecoder().decode(AccountDeletionReceipt.self, from: data)
        try receipt.validate()
        return receipt
    }

    func save(_ receipt: AccountDeletionReceipt) throws {
        try receipt.validate()
        try validateLocation()
        let manager = FileManager.default
        try manager.createDirectory(at: directory, withIntermediateDirectories: true)
        try validateLocation()
        try manager.setAttributes([.posixPermissions: 0o700], ofItemAtPath: directory.path)
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var protectedDirectory = directory
        try protectedDirectory.setResourceValues(values)
        let data = try JSONEncoder().encode(receipt)
        #if os(iOS)
        try manager.setAttributes(
            [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
            ofItemAtPath: directory.path
        )
        try data.write(to: receiptURL, options: [.atomic, .completeFileProtectionUntilFirstUserAuthentication])
        #else
        try data.write(to: receiptURL, options: .atomic)
        #endif
        try manager.setAttributes([.posixPermissions: 0o600], ofItemAtPath: receiptURL.path)
    }

    /// Must succeed before the runtime is stopped or any local data is erased.
    func markCleanupPending(_ receipt: AccountDeletionReceipt) throws -> AccountDeletionReceipt {
        var next = receipt
        next.phase = .localCleanupPending
        try save(next)
        return next
    }

    func clear() throws {
        try validateLocation()
        if FileManager.default.fileExists(atPath: receiptURL.path) {
            try FileManager.default.removeItem(at: receiptURL)
        }
    }
}

/// Only app-owned paths are erased. Security-scoped external saves are absent.
struct AccountDeletionLocalPaths {
    let applicationSupport: URL
    let documents: URL
    let temporary: URL

    /// Canonicalize only the trusted system-provided app container, never a
    /// deletion target: resolving Documents itself could follow an external link.
    static func inSandbox(container: URL) -> AccountDeletionLocalPaths {
        let root = container.resolvingSymlinksInPath()
        return AccountDeletionLocalPaths(
            applicationSupport: root.appendingPathComponent("Library/Application Support", isDirectory: true),
            documents: root.appendingPathComponent("Documents", isDirectory: true),
            temporary: root.appendingPathComponent("tmp", isDirectory: true)
        )
    }

    var targets: [URL] {
        [applicationSupport.appendingPathComponent("OpenPencil/Auth", isDirectory: true),
         documents,
         temporary.appendingPathComponent("OpenPencilSaves", isDirectory: true),
         temporary.appendingPathComponent("OpenPencilExports", isDirectory: true)]
    }

    /// Idempotent removal lets a durable cleanup marker survive any partial failure.
    func removeFiles(remove: (URL) throws -> Void = { url in
        let manager = FileManager.default
        if manager.fileExists(atPath: url.path) || (try? url.resourceValues(forKeys: [.isSymbolicLinkKey]).isSymbolicLink) == true {
            try manager.removeItem(at: url)
        }
    }) throws {
        for target in targets {
            guard target.isFileURL, target.path != "/", target.pathComponents.count > 2 else {
                throw AccountDeletionError.unsafePath
            }
            // A symlink at the target can be unlinked safely; symlinked parents
            // could redirect deletion outside the sandbox and must be refused.
            let parent = target.deletingLastPathComponent().standardizedFileURL
            guard parent.resolvingSymlinksInPath() == parent else {
                throw AccountDeletionError.unsafePath
            }
            try remove(target)
        }
    }
}

enum AccountDeletionLocalCleanup {
    /// Every side effect must complete before the recovery marker is removed.
    static func finish(
        receipt: AccountDeletionReceipt,
        store: AccountDeletionStore,
        removeFiles: () throws -> Void,
        removeCredentials: () throws -> Void,
        clearPreferences: () throws -> Void
    ) throws {
        guard receipt.phase == .localCleanupPending, try store.load() == receipt else {
            throw AccountDeletionError.invalidReceipt
        }
        try removeFiles()
        try removeCredentials()
        try clearPreferences()
        try store.clear()
    }
}
