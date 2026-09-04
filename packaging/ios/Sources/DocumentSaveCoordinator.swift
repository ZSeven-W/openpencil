import Foundation
import UIKit

/// Bridges the engine's Save / Save As shell action to the system document
/// picker.
///
/// `.op` is a document format, not an export format, so a save has to land
/// somewhere the user can find it. The engine stays the only writer of
/// canonical `.op` bytes: it streams them into an app-private staging file
/// (so a multi-megabyte document never crosses the C ABI as `Data`), and the
/// picker places that file at the destination the user chose. Only after the
/// destination really received the bytes does `op_editor_commit_save` mark
/// the document saved.
///
/// The picker opens on `NSDocumentDirectory` — "On My iPhone ▸ OpenPencil",
/// the Files-visible home this app already publishes — so accepting the
/// default keeps the pre-picker behaviour, and the user can now also place
/// the document in iCloud Drive or any file provider.
///
/// A committed save is bookmarked, so the *next* plain Save rewrites the same
/// file with no picker at all. A bookmark that no longer resolves, or a
/// destination that refuses the write, falls back to the picker rather than
/// reporting a failed save.
///
/// Every terminal path either commits or cancels the engine's pending save
/// and removes the staging directory.
final class DocumentSaveCoordinator: NSObject {
    private static let maximumFilenameBytes = 4 * 1024
    private static let maximumPathBytes = 16 * 1024
    private static let maximumTargetBytes = 64 * 1024

    private weak var host: OpEngineHost?
    private weak var activePicker: UIDocumentPickerViewController?
    private var stagingDirectory: URL?
    private var stagedFile: URL?
    private var stagedFilename: String?
    private var hasPendingEngineRequest = false
    private var saveErrorPending = false
    private let writeQueue = DispatchQueue(label: "tech.zseven.openpencil.save", qos: .userInitiated)

    init(host: OpEngineHost) {
        self.host = host
        super.init()
    }

    /// Tells a freshly created engine that this shell can present a save
    /// picker, so Save / Save As emit `OpShellAction_SaveDocument` instead of
    /// painting the engine's own name dialog.
    ///
    /// `NSDocumentDirectory` is still handed to `op_create` and still matters:
    /// it is where legacy documents are migrated to, where the picker opens by
    /// default, and where a backgrounded shell-bound document's shadow copy
    /// lands.
    static func declareCapability(engine: OpaquePointer, host: OpEngineHost) {
        let status = op_editor_configure_save_picker(engine, true)
        if status != OpStatus_Ok {
            host.reportFailure(
                status,
                operation: "op_editor_configure_save_picker",
                engine: engine
            )
        }
    }

    /// Shell action 11: stage the document, then rewrite or prompt.
    func beginSave() {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else { return }

        // A second shell action must not replace a staged file the picker is
        // still placing. Discard only the newly staged engine request.
        guard activePicker == nil, stagingDirectory == nil else {
            hasPendingEngineRequest = true
            cancelPendingEngineRequest(failed: false)
            return
        }

        hasPendingEngineRequest = true
        guard let filename = copySaveFilename(engine: engine) else {
            fail()
            return
        }

        let staged: URL
        do {
            staged = try makeStagedFileURL(filename: filename)
        } catch {
            NSLog("OpenPencil could not prepare a save file: %@", error.localizedDescription)
            fail()
            return
        }

        let path = Data(staged.path.utf8)
        guard !path.isEmpty, path.count <= Self.maximumPathBytes else {
            fail()
            return
        }
        let status = path.withUnsafeBytes { bytes in
            op_editor_stage_save_to_path(
                engine,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count
            )
        }
        guard status == OpStatus_Ok else {
            host.reportFailure(status, operation: "op_editor_stage_save_to_path", engine: engine)
            fail()
            return
        }
        stagedFile = staged
        stagedFilename = filename
        try? FileManager.default.setAttributes(
            [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
            ofItemAtPath: staged.path
        )

        // A document the user has already placed is rewritten in silence.
        if let handle = copySaveTarget(engine: engine),
           let destination = DocumentSaveBinding.resolve(handle) {
            rewrite(staged: staged, destination: destination, handle: handle, filename: filename)
            return
        }
        presentPicker(for: staged)
    }

    /// Teardown: never leave the engine believing a save is still in flight.
    func cancelForTeardown() {
        precondition(Thread.isMainThread)
        saveErrorPending = false
        cancelPendingEngineRequest(failed: false)
        if let picker = activePicker {
            picker.delegate = nil
            picker.presentationController?.delegate = nil
            picker.dismiss(animated: false)
        }
        activePicker = nil
        cleanupStagingDirectory()
    }

    // MARK: - Writing

    /// Copies the staged bytes into an already-bound destination off the main
    /// thread, then reports the outcome to the engine.
    ///
    /// A failure here re-presents the picker: the engine's pending save is
    /// still alive and the staged bytes are still on disk, so asking for a
    /// new destination is strictly better than telling the user their save
    /// failed for a dialog they never saw.
    private func rewrite(
        staged: URL,
        destination: DocumentSaveBinding.Destination,
        handle: String,
        filename: String
    ) {
        writeQueue.async { [weak self] in
            let result = Result {
                try DocumentSaveBinding.writeStaged(staged, into: destination.url)
            }
            let name = destination.url.lastPathComponent
            destination.release()
            DispatchQueue.main.async {
                guard let self else { return }
                switch result {
                case .success:
                    self.commit(handle: handle, displayName: name.isEmpty ? filename : name)
                case .failure(let error):
                    NSLog(
                        "OpenPencil could not rewrite the bound save destination: %@",
                        error.localizedDescription
                    )
                    self.presentPicker(for: staged)
                }
            }
        }
    }

    private func commit(handle: String, displayName: String) {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else {
            hasPendingEngineRequest = false
            cleanupStagingDirectory()
            return
        }
        let handleBytes = Data(handle.utf8)
        let nameBytes = Data(displayName.utf8)
        guard handleBytes.count <= Self.maximumTargetBytes,
              nameBytes.count <= Self.maximumFilenameBytes
        else {
            fail()
            return
        }
        let status = handleBytes.withUnsafeBytes { handlePointer in
            nameBytes.withUnsafeBytes { namePointer in
                op_editor_commit_save(
                    engine,
                    handlePointer.bindMemory(to: UInt8.self).baseAddress,
                    handlePointer.count,
                    namePointer.bindMemory(to: UInt8.self).baseAddress,
                    namePointer.count
                )
            }
        }
        hasPendingEngineRequest = false
        if status != OpStatus_Ok {
            host.reportFailure(status, operation: "op_editor_commit_save", engine: engine)
            cleanupStagingDirectory()
            presentSaveError()
            return
        }
        cleanupStagingDirectory()
    }

    // MARK: - Picker

    private func presentPicker(for stagedFile: URL) {
        precondition(Thread.isMainThread)
        guard
            let view = host?.view,
            let presenter = nearestViewController(from: view),
            presenter.presentedViewController == nil
        else {
            fail()
            return
        }

        let picker = UIDocumentPickerViewController(forExporting: [stagedFile], asCopy: true)
        picker.delegate = self
        picker.allowsMultipleSelection = false
        picker.modalPresentationStyle = .formSheet
        // "On My iPhone ▸ OpenPencil" is this app's published home, so the
        // default destination stays exactly where saves used to land.
        picker.directoryURL = DocumentStorage.prepare()
        activePicker = picker
        presenter.present(picker, animated: true)
        picker.presentationController?.delegate = self
    }

    private func finishPicker() {
        precondition(Thread.isMainThread)
        activePicker?.delegate = nil
        activePicker?.presentationController?.delegate = nil
        activePicker = nil
    }

    /// The picker placed the copy: bind it so the next Save is silent.
    private func bind(pickedURL: URL) {
        precondition(Thread.isMainThread)
        let scoped = pickedURL.startAccessingSecurityScopedResource()
        let handle = DocumentSaveBinding.handle(for: pickedURL)
        if scoped { pickedURL.stopAccessingSecurityScopedResource() }
        guard let handle else {
            // The bytes ARE at the destination; only the durable binding
            // failed. Marking the document saved without a handle would make
            // the next plain Save silently rewrite the wrong file, so cancel
            // instead and let the user save again.
            cancelPendingEngineRequest(failed: true)
            cleanupStagingDirectory()
            presentSaveError()
            return
        }
        commit(handle: handle, displayName: pickedURL.lastPathComponent)
    }

    // MARK: - Engine reads

    private func copySaveFilename(engine: OpaquePointer) -> String? {
        precondition(Thread.isMainThread)
        guard let raw = copyEngineString(
            engine: engine,
            operation: "op_editor_copy_save_file_name",
            cap: Self.maximumFilenameBytes,
            read: op_editor_copy_save_file_name
        ) else { return nil }
        return DocumentExportCoordinator.validatedFilename(raw)
    }

    /// The bound destination handle, or nil when the shell must prompt. A
    /// zero-length answer is the engine's documented "no binding yet".
    private func copySaveTarget(engine: OpaquePointer) -> String? {
        precondition(Thread.isMainThread)
        return copyEngineString(
            engine: engine,
            operation: "op_editor_copy_save_target",
            cap: Self.maximumTargetBytes,
            read: op_editor_copy_save_target
        )
    }

    private func copyEngineString(
        engine: OpaquePointer,
        operation: String,
        cap: Int,
        read: (OpaquePointer?, UnsafeMutablePointer<UInt8>?, Int, UnsafeMutablePointer<Int>?)
            -> OpStatus
    ) -> String? {
        var required = 0
        let query = read(engine, nil, 0, &required)
        guard query == OpStatus_Ok else {
            host?.reportFailure(query, operation: operation, engine: engine)
            return nil
        }
        guard required > 0, required <= cap else { return nil }

        var bytes = [UInt8](repeating: 0, count: required)
        let copy = bytes.withUnsafeMutableBufferPointer { buffer in
            read(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard copy == OpStatus_Ok, required > 0, required <= bytes.count else {
            if copy != OpStatus_Ok {
                host?.reportFailure(copy, operation: operation, engine: engine)
            }
            return nil
        }
        return String(bytes: bytes.prefix(required), encoding: .utf8)
    }

    // MARK: - Staging

    private func makeStagedFileURL(filename: String) throws -> URL {
        precondition(Thread.isMainThread)
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("OpenPencilSaves", isDirectory: true)
        let directory = root.appendingPathComponent(UUID().uuidString, isDirectory: true)
        try fileManager.createDirectory(
            at: directory,
            withIntermediateDirectories: true,
            attributes: [
                .protectionKey: FileProtectionType.completeUntilFirstUserAuthentication,
            ]
        )
        stagingDirectory = directory
        var values = URLResourceValues()
        values.isExcludedFromBackup = true
        var mutableDirectory = directory
        try mutableDirectory.setResourceValues(values)
        return directory.appendingPathComponent(filename, isDirectory: false)
    }

    private func cleanupStagingDirectory() {
        stagedFile = nil
        stagedFilename = nil
        guard let directory = stagingDirectory else { return }
        stagingDirectory = nil
        do {
            try FileManager.default.removeItem(at: directory)
        } catch {
            if (error as NSError).code != NSFileNoSuchFileError {
                NSLog(
                    "OpenPencil could not remove save staging directory: %@",
                    error.localizedDescription
                )
            }
        }
    }

    // MARK: - Failure paths

    /// Terminal failure: discard the engine's pending save and surface it.
    private func fail() {
        precondition(Thread.isMainThread)
        cancelPendingEngineRequest(failed: true)
        cleanupStagingDirectory()
        presentSaveError()
    }

    private func cancelPendingEngineRequest(failed: Bool) {
        precondition(Thread.isMainThread)
        guard hasPendingEngineRequest else { return }
        hasPendingEngineRequest = false
        guard let host, let engine = host.engine else { return }
        let status = op_editor_cancel_save(engine, failed)
        if status != OpStatus_Ok && status != OpStatus_NotReady && status != OpStatus_Suspended {
            host.reportFailure(status, operation: "op_editor_cancel_save", engine: engine)
        }
    }

    private func presentSaveError(attempt: Int = 0) {
        precondition(Thread.isMainThread)
        if attempt == 0 {
            guard !saveErrorPending else { return }
            saveErrorPending = true
            DispatchQueue.main.async { [weak self] in
                self?.presentSaveError(attempt: 1)
            }
            return
        }
        guard saveErrorPending else { return }
        guard let view = host?.view, let presenter = nearestViewController(from: view) else {
            saveErrorPending = false
            return
        }
        guard presenter.presentedViewController == nil else {
            guard attempt < 20 else {
                saveErrorPending = false
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
                self?.presentSaveError(attempt: attempt + 1)
            }
            return
        }

        let alert = UIAlertController(
            title: NSLocalizedString("Unable to Save Document", comment: "Save failure title"),
            message: NSLocalizedString(
                "OpenPencil could not save this document.",
                comment: "Save failure message"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("OK", comment: "Dismiss alert"),
            style: .default,
            handler: { [weak self] _ in self?.saveErrorPending = false }
        ))
        presenter.present(alert, animated: true)
    }

    private func nearestViewController(from view: UIView) -> UIViewController? {
        var responder: UIResponder? = view
        while let current = responder {
            if let controller = current as? UIViewController { return controller }
            responder = current.next
        }
        return view.window?.rootViewController
    }
}

extension DocumentSaveCoordinator: UIDocumentPickerDelegate {
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        finishPicker()
        // The user abandoned the save UI: the document keeps its changes and
        // its previous binding, and the next Save starts over.
        cancelPendingEngineRequest(failed: false)
        cleanupStagingDirectory()
    }

    func documentPicker(
        _ controller: UIDocumentPickerViewController,
        didPickDocumentsAt urls: [URL]
    ) {
        finishPicker()
        guard let picked = urls.first else {
            cancelPendingEngineRequest(failed: true)
            cleanupStagingDirectory()
            presentSaveError()
            return
        }
        bind(pickedURL: picked)
    }
}

extension DocumentSaveCoordinator: UIAdaptivePresentationControllerDelegate {
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
        finishPicker()
        cancelPendingEngineRequest(failed: false)
        cleanupStagingDirectory()
    }
}
