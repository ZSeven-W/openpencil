import Foundation

/// Host-side coverage for the durable half of a picker-backed save: a
/// destination the user picked once has to survive as an opaque handle the
/// engine can hand back, and rewriting it has to replace the file's contents
/// rather than append to or truncate them.

private func check(_ condition: Bool, _ message: String) {
    if !condition {
        FileHandle.standardError.write(Data("DocumentSaveBinding test failed: \(message)\n".utf8))
        exit(1)
    }
}

private func scratchDirectory() -> URL {
    let url = FileManager.default.temporaryDirectory
        .appendingPathComponent("op-save-binding-\(UUID().uuidString)", isDirectory: true)
    try! FileManager.default.createDirectory(at: url, withIntermediateDirectories: true)
    return url
}

@main
enum DocumentSaveBindingTests {
    static func main() {
        roundTripsAPickedDestination()
        refusesGarbageAndVanishedDestinations()
        rewritingReplacesTheWholeFile()
        print("DocumentSaveBinding tests passed")
    }

    /// The handle the engine stores must resolve back to the same file.
    static func roundTripsAPickedDestination() {
        let directory = scratchDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let picked = directory.appendingPathComponent("poster.op")
        try! Data("first".utf8).write(to: picked)

        guard let handle = DocumentSaveBinding.handle(for: picked) else {
            check(false, "a picked destination must produce a handle")
            return
        }
        check(!handle.isEmpty, "the handle must not be empty")
        check(
            handle.utf8.count <= DocumentSaveBinding.maximumHandleBytes,
            "the handle must fit the engine's cap"
        )
        check(Data(base64Encoded: handle) != nil, "the handle must be base64 bookmark data")

        guard let destination = DocumentSaveBinding.resolve(handle) else {
            check(false, "a fresh handle must resolve")
            return
        }
        defer { destination.release() }
        check(
            destination.url.resolvingSymlinksInPath() == picked.resolvingSymlinksInPath(),
            "the handle must resolve to the picked file"
        )
    }

    /// A handle that no longer names a file is the signal to re-prompt, so it
    /// must come back nil rather than throw or resolve to nothing.
    static func refusesGarbageAndVanishedDestinations() {
        check(DocumentSaveBinding.resolve("not base64 at all!") == nil, "garbage must not resolve")
        check(DocumentSaveBinding.resolve("") == nil, "an empty handle must not resolve")

        let directory = scratchDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let doomed = directory.appendingPathComponent("deleted.op")
        try! Data("bytes".utf8).write(to: doomed)
        let handle = DocumentSaveBinding.handle(for: doomed)!
        try! FileManager.default.removeItem(at: doomed)
        check(
            DocumentSaveBinding.resolve(handle) == nil,
            "a destination the user deleted must fall back to the picker"
        )
    }

    /// A second save must leave the destination holding exactly the new bytes
    /// — the classic bug here is a shorter document leaving the old tail
    /// behind.
    static func rewritingReplacesTheWholeFile() {
        let directory = scratchDirectory()
        defer { try? FileManager.default.removeItem(at: directory) }
        let destination = directory.appendingPathComponent("bound.op")
        try! Data(repeating: 0x41, count: 4096).write(to: destination)
        let staged = directory.appendingPathComponent("staged.op")
        try! Data("{\"version\":\"1.0.0\"}".utf8).write(to: staged)

        try! DocumentSaveBinding.writeStaged(staged, into: destination)
        let written = try! Data(contentsOf: destination)
        check(
            written == Data("{\"version\":\"1.0.0\"}".utf8),
            "a rewrite must replace the destination's whole contents"
        )
        check(
            FileManager.default.fileExists(atPath: staged.path),
            "the staging file stays the shell's to clean up"
        )
    }
}
