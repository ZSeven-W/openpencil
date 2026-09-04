import Foundation

/// The durable half of a picker-backed save: turning the URL the document
/// picker handed back into something the engine can round-trip, and back
/// again on the next save.
///
/// The engine deliberately treats this as an opaque string (see
/// `DocumentBinding::Shell` in `op-engine-ffi/src/editor_document.rs`) — a
/// picked destination is not a path the engine can open. On iOS the token is
/// base64-encoded bookmark data, which survives app relaunches and keeps
/// working when the user later moves or renames the file in Files.
enum DocumentSaveBinding {
    /// Bookmarks larger than this are refused rather than handed to the
    /// engine, which caps the handle at 64 KiB.
    static let maximumHandleBytes = 48 * 1024

    /// A resolved destination plus the security scope that has to be closed.
    struct Destination {
        let url: URL
        private let scoped: Bool

        init(url: URL, scoped: Bool) {
            self.url = url
            self.scoped = scoped
        }

        func release() {
            if scoped {
                url.stopAccessingSecurityScopedResource()
            }
        }
    }

    /// Encodes `url` as a handle the engine can store.
    ///
    /// The caller must already hold access to `url` (inside the picker
    /// delegate callback, or inside `startAccessingSecurityScopedResource`),
    /// because bookmarking a document outside the app sandbox is itself a
    /// scoped operation.
    static func handle(for url: URL) -> String? {
        do {
            let data = try url.bookmarkData(
                options: [],
                includingResourceValuesForKeys: nil,
                relativeTo: nil
            )
            guard data.count <= maximumHandleBytes else {
                NSLog("OpenPencil save bookmark is too large (%d bytes)", data.count)
                return nil
            }
            return data.base64EncodedString()
        } catch {
            NSLog("OpenPencil could not bookmark the save destination: %@", error.localizedDescription)
            return nil
        }
    }

    /// Resolves a handle back into a writable URL, opening its security
    /// scope. The caller MUST call `release()` on the result.
    ///
    /// Returns nil for a handle that no longer resolves — the file was
    /// deleted, or its provider is gone — which is the signal to fall back to
    /// the picker rather than report a failed save.
    static func resolve(_ handle: String) -> Destination? {
        guard let data = Data(base64Encoded: handle) else { return nil }
        var isStale = false
        let url: URL
        do {
            url = try URL(
                resolvingBookmarkData: data,
                options: [],
                relativeTo: nil,
                bookmarkDataIsStale: &isStale
            )
        } catch {
            NSLog(
                "OpenPencil could not resolve the bound save destination: %@",
                error.localizedDescription
            )
            return nil
        }
        // A stale bookmark still resolves; the URL is simply the file's new
        // home. The next successful save re-bookmarks it, so nothing has to
        // be repaired here.
        let scoped = url.startAccessingSecurityScopedResource()
        guard FileManager.default.fileExists(atPath: url.path) else {
            if scoped { url.stopAccessingSecurityScopedResource() }
            return nil
        }
        return Destination(url: url, scoped: scoped)
    }

    /// Replaces `destination`'s contents with the staged file's bytes,
    /// coordinated so a provider-backed file (iCloud Drive, a third-party
    /// provider) sees one atomic revision rather than a partial write.
    static func writeStaged(_ staged: URL, into destination: URL) throws {
        let bytes = try Data(contentsOf: staged, options: .mappedIfSafe)
        var coordinatorError: NSError?
        var writeError: Error?
        NSFileCoordinator().coordinate(
            writingItemAt: destination,
            options: .forReplacing,
            error: &coordinatorError
        ) { url in
            do {
                try bytes.write(to: url, options: .atomic)
            } catch {
                writeError = error
            }
        }
        if let coordinatorError { throw coordinatorError }
        if let writeError { throw writeError }
    }
}
