import Foundation

private func expect(_ condition: Bool, _ message: String) {
    if !condition { fatalError(message) }
}

private func expectFailure(_ message: String, _ operation: () throws -> Void) {
    do { try operation(); fatalError(message) } catch {}
}

@main
struct AccountDeletionStateTests {
    static func main() throws {
        let origin = URL(string: "https://sso.zseven.tech")!
        let requestID = "9c954dfa-3f80-4ed0-82b5-3c9dfbb6edee"
        let deviceID = "test-openpencil-device"
        let secret = Data(repeating: 122, count: 32).base64EncodedString()
            .replacingOccurrences(of: "+", with: "-").replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
        let deletionURL = "https://sso.zseven.tech/openpencil/delete-account?request=\(requestID)"
        let body: [String: Any] = [
            "request_id": requestID, "device_id": deviceID, "scope": "openpencil",
            "state": "awaiting_confirmation", "deletion_url": deletionURL,
            "expires_at": "2030-10-29T09:00:00Z",
        ]
        func response(_ changes: [String: Any] = [:], removing: String? = nil) throws -> AccountDeletionResponse {
            var next = body.merging(changes) { _, new in new }
            if let removing { next.removeValue(forKey: removing) }
            return try JSONDecoder().decode(AccountDeletionResponse.self, from: JSONSerialization.data(withJSONObject: next))
        }
        let receipt = try response().makeReceipt(origin: origin, deviceID: deviceID, secret: secret, now: Date(timeIntervalSince1970: 0))
        expect(receipt.phase == .awaitingServer, "create must not authorize cleanup")
        expect(try response().verifiedState(for: receipt) == .awaitingConfirmation, "pending response is pending")
        for key in ["request_id", "device_id", "scope", "state", "expires_at"] {
            expectFailure("missing \(key) accepted") { _ = try response(removing: key) }
        }
        let mismatches: [[String: Any]] = [
            ["request_id": UUID().uuidString.lowercased()], ["device_id": "another-device"],
            ["scope": "zseven"], ["scope": "rish"], ["expires_at": "invalid-date"],
            ["deletion_url": deletionURL + "&extra=1"],
        ]
        for mismatch in mismatches {
            expectFailure("mismatched receipt accepted") { _ = try response(mismatch).verifiedState(for: receipt) }
        }
        for state in ["cancelled", "failed", "idle", "signed_out", "completed"] {
            expectFailure("unknown or cancellation state accepted") { _ = try response(["state": state]) }
        }
        for badURL in [
            deletionURL.replacingOccurrences(of: ".tech", with: ".cn"),
            deletionURL.replacingOccurrences(of: "https:", with: "http:"),
            deletionURL.replacingOccurrences(of: "/openpencil/", with: "/account/"),
            deletionURL + "#deleted", deletionURL + "&request=\(requestID)",
            deletionURL.replacingOccurrences(of: "sso.zseven.tech", with: "sso.zseven.tech:443"),
            deletionURL.replacingOccurrences(of: "sso.zseven.tech", with: "user@sso.zseven.tech"),
        ] {
            expectFailure("untrusted deletion URL accepted") {
                _ = try response(["deletion_url": badURL]).makeReceipt(origin: origin, deviceID: deviceID, secret: secret, now: .distantPast)
            }
        }
        expectFailure("initial deleted state accepted") {
            _ = try response(["state": "deleted"]).makeReceipt(origin: origin, deviceID: deviceID, secret: secret, now: .distantPast)
        }
        expectFailure("creation without deletion URL accepted") {
            _ = try response(removing: "deletion_url").makeReceipt(origin: origin, deviceID: deviceID, secret: secret, now: .distantPast)
        }
        expectFailure("expired creation accepted") {
            _ = try response().makeReceipt(origin: origin, deviceID: deviceID, secret: secret, now: .distantFuture)
        }
        let confirmed = try response(["state": "deleting", "expires_at": "2030-11-05T09:00:00Z"], removing: "deletion_url")
        let refreshed = try confirmed.refreshedReceipt(receipt)
        expect(refreshed.expiresAt > receipt.expiresAt, "confirmation must refresh receipt lifetime")
        expect(refreshed.phase == .awaitingServer, "processing must not authorize cleanup")
        let deleted = try response(["state": "deleted"], removing: "deletion_url")
        expect(try deleted.verifiedState(for: receipt) == .deleted, "valid exact receipt should confirm deletion")
        let httpURL = origin.appendingPathComponent("api/v1/openpencil/deletion-requests/\(requestID)")
        let deletedData = try JSONSerialization.data(withJSONObject: body.merging(["state": "deleted"]) { _, new in new })
        for status in [301, 302, 401, 403, 404, 409, 429, 500, 503] {
            expectFailure("HTTP \(status) authorized deletion") {
                _ = try AccountDeletionResponse.decode(data: deletedData, statusCode: status, responseURL: httpURL, requestURL: httpURL)
            }
        }
        expectFailure("redirected HTTP success accepted") {
            _ = try AccountDeletionResponse.decode(data: deletedData, statusCode: 200, responseURL: URL(string: "https://sso.zseven.cn"), requestURL: httpURL)
        }
        expect(AccountDeletionError.rejected(404).permitsReceiptRenewal, "404 should permit explicit re-verification")
        expect(AccountDeletionError.rejected(410).permitsReceiptRenewal, "410 should permit explicit re-verification")
        for status in [200, 301, 401, 403, 409, 429, 500, 503] {
            expect(!AccountDeletionError.rejected(status).permitsReceiptRenewal, "other HTTP result renewed receipt")
        }
        expect(!AccountDeletionError.transport.permitsReceiptRenewal, "offline receipt must not be silently replaced")
        expectFailure("empty response accepted") {
            _ = try AccountDeletionResponse.decode(data: Data(), statusCode: 200, responseURL: httpURL, requestURL: httpURL)
        }
        expectFailure("oversized response accepted") {
            _ = try AccountDeletionResponse.decode(data: Data(repeating: 32, count: 20_000), statusCode: 200, responseURL: httpURL, requestURL: httpURL)
        }
        try testRecovery(receipt: receipt)
        print("AccountDeletionState tests passed")
    }

    private static func testRecovery(receipt: AccountDeletionReceipt) throws {
        let manager = FileManager.default
        let root = manager.temporaryDirectory.resolvingSymlinksInPath()
            .appendingPathComponent("op-deletion-tests-\(UUID().uuidString)", isDirectory: true)
        try manager.createDirectory(at: root, withIntermediateDirectories: true)
        defer { try? manager.removeItem(at: root) }
        let paths = AccountDeletionLocalPaths.inSandbox(container: root)
        let store = AccountDeletionStore(directory: paths.applicationSupport.appendingPathComponent("OpenPencilDeletion"))
        expect(try store.load() == nil, "fresh install must not be blocked by a missing receipt")
        for target in paths.targets {
            try manager.createDirectory(at: target, withIntermediateDirectories: true)
            try Data("private app data".utf8).write(to: target.appendingPathComponent("data.txt"))
        }
        let exported = root.appendingPathComponent("ExternalFiles/export.op")
        try manager.createDirectory(at: exported.deletingLastPathComponent(), withIntermediateDirectories: true)
        try Data("keep exported file".utf8).write(to: exported)
        try store.save(receipt)
        expect(try store.load() == receipt, "receipt must survive restart")
        var removedAnything = false
        expectFailure("awaiting receipt permitted local deletion") {
            try AccountDeletionLocalCleanup.finish(
                receipt: receipt, store: store,
                removeFiles: { removedAnything = true }, removeCredentials: {}, clearPreferences: {}
            )
        }
        expect(!removedAnything, "unconfirmed request performed side effects")
        let pending = try store.markCleanupPending(receipt)
        expect(try store.load()?.phase == .localCleanupPending, "cleanup marker must be durable")
        var removedCount = 0
        expectFailure("simulated mid-cleanup failure lost marker") {
            try AccountDeletionLocalCleanup.finish(receipt: pending, store: store, removeFiles: {
                try paths.removeFiles { url in
                    removedCount += 1
                    if removedCount == 2 { throw AccountDeletionError.transport }
                    try manager.removeItem(at: url)
                }
            }, removeCredentials: {}, clearPreferences: {})
        }
        expect(try store.load() == pending, "partial file deletion must retain marker")
        expectFailure("keychain failure lost marker") {
            try AccountDeletionLocalCleanup.finish(receipt: pending, store: store,
                removeFiles: { try paths.removeFiles() },
                removeCredentials: { throw AccountDeletionError.keychainFailure }, clearPreferences: {})
        }
        expect(try store.load() == pending, "credential failure must retain marker")
        var order: [String] = []
        try AccountDeletionLocalCleanup.finish(receipt: pending, store: store,
            removeFiles: { try paths.removeFiles(); order.append("files") },
            removeCredentials: { order.append("credentials") }, clearPreferences: { order.append("preferences") })
        expect(order == ["files", "credentials", "preferences"], "cleanup ordering changed")
        expect(try store.load() == nil, "success must clear receipt secret")
        expect(try Data(contentsOf: exported) == Data("keep exported file".utf8), "external export was modified")
        expect(paths.targets.allSatisfy { !manager.fileExists(atPath: $0.path) }, "app files remain")
        try paths.removeFiles()
        let externalDirectory = exported.deletingLastPathComponent()
        try manager.createSymbolicLink(at: paths.documents, withDestinationURL: externalDirectory)
        let reconstructedPaths = AccountDeletionLocalPaths.inSandbox(container: root)
        try reconstructedPaths.removeFiles()
        expect(manager.fileExists(atPath: exported.path), "Documents link followed to external files")
        try manager.removeItem(at: paths.applicationSupport.appendingPathComponent("OpenPencil"))
        try manager.createSymbolicLink(
            at: paths.applicationSupport.appendingPathComponent("OpenPencil"),
            withDestinationURL: externalDirectory
        )
        expectFailure("symlink parent allowed out-of-scope removal") { try paths.removeFiles() }
        expect(manager.fileExists(atPath: exported.path), "symlink target export was removed")
        try manager.removeItem(at: paths.applicationSupport.appendingPathComponent("OpenPencil"))
        try manager.createDirectory(at: paths.documents, withIntermediateDirectories: true)
        try manager.removeItem(at: store.directory)
        let protectedReceipt = paths.documents.appendingPathComponent("receipt.json")
        let pendingData = try JSONEncoder().encode(pending)
        try pendingData.write(to: protectedReceipt)
        try manager.createSymbolicLink(at: store.directory, withDestinationURL: paths.documents)
        expectFailure("symlinked store accepted a save") { try store.save(pending) }
        expectFailure("symlinked store accepted a load") { _ = try store.load() }
        expectFailure("symlinked store accepted clear") { try store.clear() }
        expectFailure("cleanup accepted a marker inside Documents") {
            try AccountDeletionLocalCleanup.finish(receipt: pending, store: store,
                removeFiles: { try paths.removeFiles() }, removeCredentials: {}, clearPreferences: {})
        }
        expect(try Data(contentsOf: protectedReceipt) == pendingData, "unsafe cleanup erased its recovery marker")
        try manager.removeItem(at: store.directory)
        try manager.createDirectory(at: store.directory, withIntermediateDirectories: true)
        try manager.createSymbolicLink(at: store.directory.appendingPathComponent("receipt.json"), withDestinationURL: exported)
        expectFailure("symlinked receipt file accepted") { _ = try store.load() }
        expectFailure("symlinked receipt file overwritten") { try store.save(pending) }
        expectFailure("symlinked receipt file cleared") { try store.clear() }
        expect(try Data(contentsOf: exported) == Data("keep exported file".utf8), "receipt symlink damaged exported file")
    }
}
