import Foundation
import UIKit

/// Bridges a frozen Rust export request to the system Files save UI.
///
/// Rust writes directly into an app-private, single-use staging directory so
/// large exports never cross the C ABI as a Swift `Data` value. The document
/// picker copies that file to the user's chosen destination; every terminal
/// picker path removes the staging directory.
final class DocumentExportCoordinator: NSObject {
    private static let maximumFilenameBytes = 4 * 1024
    private static let maximumPathBytes = 16 * 1024

    private weak var host: OpEngineHost?
    private weak var activePicker: UIDocumentPickerViewController?
    private var stagingDirectory: URL?
    private var hasPendingEngineRequest = false
    private var exportErrorPending = false

    init(host: OpEngineHost) {
        self.host = host
        super.init()
    }

    func beginExport() {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else { return }

        // A second shell action must not replace a file that the Files picker
        // is still copying. Discard only the newly frozen engine request.
        guard activePicker == nil, stagingDirectory == nil else {
            hasPendingEngineRequest = true
            cancelPendingEngineRequest()
            return
        }

        hasPendingEngineRequest = true
        guard let filename = copyExportFilename(engine: engine) else {
            cancelPendingEngineRequest()
            presentExportError()
            return
        }

        let stagedFile: URL
        do {
            stagedFile = try makeStagedFileURL(filename: filename)
        } catch {
            NSLog("OpenPencil could not prepare an export file: %@", error.localizedDescription)
            cancelPendingEngineRequest()
            cleanupStagingDirectory()
            presentExportError()
            return
        }

        let path = Data(stagedFile.path.utf8)
        guard !path.isEmpty, path.count <= Self.maximumPathBytes else {
            cancelPendingEngineRequest()
            cleanupStagingDirectory()
            presentExportError()
            return
        }
        let status = path.withUnsafeBytes { bytes in
            op_editor_export_to_path(
                engine,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count
            )
        }
        guard status == OpStatus_Ok else {
            host.reportFailure(status, operation: "op_editor_export_to_path", engine: engine)
            cancelPendingEngineRequest()
            cleanupStagingDirectory()
            presentExportError()
            return
        }

        // A successful atomic write consumes the frozen Rust request.
        hasPendingEngineRequest = false
        try? FileManager.default.setAttributes(
            [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
            ofItemAtPath: stagedFile.path
        )
        presentDocumentPicker(for: stagedFile)
    }

    func cancelForTeardown() {
        precondition(Thread.isMainThread)
        exportErrorPending = false
        cancelPendingEngineRequest()
        if let picker = activePicker {
            picker.delegate = nil
            picker.presentationController?.delegate = nil
            picker.dismiss(animated: false)
        }
        activePicker = nil
        cleanupStagingDirectory()
    }

    private func copyExportFilename(engine: OpaquePointer) -> String? {
        precondition(Thread.isMainThread)
        var required = 0
        let query = op_editor_copy_export_file_name(engine, nil, 0, &required)
        guard query == OpStatus_Ok else {
            host?.reportFailure(
                query,
                operation: "op_editor_copy_export_file_name",
                engine: engine
            )
            return nil
        }
        guard required > 0, required <= Self.maximumFilenameBytes else { return nil }

        var bytes = [UInt8](repeating: 0, count: required)
        let copy = bytes.withUnsafeMutableBufferPointer { buffer in
            op_editor_copy_export_file_name(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard copy == OpStatus_Ok, required > 0, required <= bytes.count else {
            if copy != OpStatus_Ok {
                host?.reportFailure(
                    copy,
                    operation: "op_editor_copy_export_file_name",
                    engine: engine
                )
            }
            return nil
        }
        guard let filename = String(bytes: bytes.prefix(required), encoding: .utf8) else {
            return nil
        }
        return Self.validatedFilename(filename)
    }

    /// Shared with `DocumentSaveCoordinator`, exactly as the Android and
    /// HarmonyOS shells share `DocumentExportSupport.validatedFilename`
    /// between their export and save flows.
    static func validatedFilename(_ filename: String) -> String? {
        guard
            !filename.isEmpty,
            filename != ".",
            filename != "..",
            !filename.contains("/"),
            !filename.contains("\\"),
            !filename.unicodeScalars.contains(where: CharacterSet.controlCharacters.contains),
            filename.lengthOfBytes(using: .utf8) <= maximumFilenameBytes,
            !URL(fileURLWithPath: filename).pathExtension.isEmpty
        else { return nil }
        return filename
    }

    private func makeStagedFileURL(filename: String) throws -> URL {
        precondition(Thread.isMainThread)
        let fileManager = FileManager.default
        let root = fileManager.temporaryDirectory
            .appendingPathComponent("OpenPencilExports", isDirectory: true)
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

    private func presentDocumentPicker(for stagedFile: URL) {
        precondition(Thread.isMainThread)
        guard
            let view = host?.view,
            let presenter = nearestViewController(from: view),
            presenter.presentedViewController == nil
        else {
            cleanupStagingDirectory()
            presentExportError()
            return
        }

        let picker = UIDocumentPickerViewController(forExporting: [stagedFile], asCopy: true)
        picker.delegate = self
        picker.allowsMultipleSelection = false
        picker.modalPresentationStyle = .formSheet
        activePicker = picker
        presenter.present(picker, animated: true)
        picker.presentationController?.delegate = self
    }

    private func cancelPendingEngineRequest() {
        precondition(Thread.isMainThread)
        guard hasPendingEngineRequest else { return }
        hasPendingEngineRequest = false
        guard let host, let engine = host.engine else { return }
        let status = op_editor_cancel_export(engine)
        if status != OpStatus_Ok && status != OpStatus_NotReady && status != OpStatus_Suspended {
            host.reportFailure(status, operation: "op_editor_cancel_export", engine: engine)
        }
    }

    private func finishPicker() {
        precondition(Thread.isMainThread)
        activePicker?.delegate = nil
        activePicker?.presentationController?.delegate = nil
        activePicker = nil
        cleanupStagingDirectory()
    }

    private func cleanupStagingDirectory() {
        guard let directory = stagingDirectory else { return }
        stagingDirectory = nil
        do {
            try FileManager.default.removeItem(at: directory)
        } catch {
            if (error as NSError).code != NSFileNoSuchFileError {
                NSLog(
                    "OpenPencil could not remove export staging directory: %@",
                    error.localizedDescription
                )
            }
        }
    }

    private func presentExportError(attempt: Int = 0) {
        precondition(Thread.isMainThread)
        if attempt == 0 {
            guard !exportErrorPending else { return }
            exportErrorPending = true
            DispatchQueue.main.async { [weak self] in
                self?.presentExportError(attempt: 1)
            }
            return
        }
        guard exportErrorPending else { return }
        guard let view = host?.view, let presenter = nearestViewController(from: view) else {
            exportErrorPending = false
            return
        }
        guard presenter.presentedViewController == nil else {
            guard attempt < 20 else {
                exportErrorPending = false
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
                self?.presentExportError(attempt: attempt + 1)
            }
            return
        }

        let alert = UIAlertController(
            title: NSLocalizedString("Unable to Export File", comment: "Export failure title"),
            message: NSLocalizedString(
                "OpenPencil could not create the requested export.",
                comment: "Export failure message"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("OK", comment: "Dismiss alert"),
            style: .default,
            handler: { [weak self] _ in self?.exportErrorPending = false }
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

extension DocumentExportCoordinator: UIDocumentPickerDelegate {
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        finishPicker()
    }

    func documentPicker(
        _ controller: UIDocumentPickerViewController,
        didPickDocumentsAt urls: [URL]
    ) {
        finishPicker()
    }
}

extension DocumentExportCoordinator: UIAdaptivePresentationControllerDelegate {
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
        finishPicker()
    }
}
