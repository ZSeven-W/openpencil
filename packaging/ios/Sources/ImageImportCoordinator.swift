import Foundation
import UIKit
import UniformTypeIdentifiers

/// Owns the system picker round trip for the engine-painted
/// "Import Image or SVG" action.
///
/// Picker bytes are read off the main thread through the same 32 MiB bounded
/// reader used for documents, then returned to Rust on the engine's owner
/// thread. Rust remains the only document mutator and re-checks collaboration
/// permission when the result arrives.
final class ImageImportCoordinator: NSObject {
    private weak var host: OpEngineHost?
    private weak var activePicker: UIDocumentPickerViewController?
    private weak var activeAlert: UIAlertController?
    private var activeReadToken: UUID?
    private var importErrorPending = false
    private let readQueue = DispatchQueue(
        label: "tech.zseven.openpencil.image-import",
        qos: .userInitiated
    )

    init(host: OpEngineHost) {
        self.host = host
        super.init()
    }

    /// Shell action 12: present one bounded PNG/JPEG/GIF/WebP/SVG picker.
    func beginImport() {
        precondition(Thread.isMainThread)
        guard
            activePicker == nil,
            activeReadToken == nil,
            let view = host?.view,
            !view.didTearDown,
            let presenter = view.nearestViewController(),
            presenter.presentedViewController == nil
        else { return }

        let types = [
            UTType.png,
            UTType.jpeg,
            UTType.gif,
            UTType(filenameExtension: "webp"),
            UTType(filenameExtension: "svg"),
        ].compactMap { $0 }
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: types, asCopy: true)
        picker.delegate = self
        picker.allowsMultipleSelection = false
        picker.modalPresentationStyle = .formSheet
        activePicker = picker
        presenter.present(picker, animated: true)
        picker.presentationController?.delegate = self
    }

    /// Teardown invalidates worker completions before dismissing owned UIKit.
    func cancelForTeardown() {
        precondition(Thread.isMainThread)
        activeReadToken = nil
        importErrorPending = false
        if let picker = activePicker {
            picker.delegate = nil
            picker.presentationController?.delegate = nil
            picker.dismiss(animated: false)
        }
        activePicker = nil
        activeAlert?.dismiss(animated: false)
        activeAlert = nil
    }

    private func readPickedFile(_ url: URL) {
        precondition(Thread.isMainThread)
        let token = UUID()
        activeReadToken = token
        let filename = url.lastPathComponent
        readQueue.async { [weak self] in
            let scoped = url.startAccessingSecurityScopedResource()
            defer {
                if scoped { url.stopAccessingSecurityScopedResource() }
            }
            let result = Result { try BoundedDocumentReader.read(from: url) }
            DispatchQueue.main.async { [weak self] in
                self?.completeRead(
                    token: token,
                    filename: filename,
                    result: result
                )
            }
        }
    }

    private func completeRead(
        token: UUID,
        filename: String,
        result: Result<Data, Error>
    ) {
        precondition(Thread.isMainThread)
        guard activeReadToken == token else { return }
        activeReadToken = nil
        switch result {
        case .success(let data):
            returnPickedBytes(data, filename: filename)
        case .failure(let error):
            NSLog("OpenPencil could not read image %@: %@", filename, error.localizedDescription)
            presentImportError()
        }
    }

    private func returnPickedBytes(_ data: Data, filename: String) {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else { return }
        let name = Data(filename.utf8)
        let status = data.withUnsafeBytes { dataBytes in
            name.withUnsafeBytes { nameBytes in
                op_editor_import_image_or_svg(
                    engine,
                    dataBytes.bindMemory(to: UInt8.self).baseAddress,
                    dataBytes.count,
                    nameBytes.bindMemory(to: UInt8.self).baseAddress,
                    nameBytes.count
                )
            }
        }
        if status == OpStatus_Ok {
            host.requestImmediateFrame()
            return
        }
        host.reportFailure(status, operation: "op_editor_import_image_or_svg", engine: engine)
        // Busy is the collaboration race gate. Rust painted the precise
        // rejection notice, so a second generic UIKit alert would obscure it.
        host.requestImmediateFrame()
        if status != OpStatus_Busy {
            presentImportError()
        }
    }

    private func finishPicker() {
        precondition(Thread.isMainThread)
        activePicker?.delegate = nil
        activePicker?.presentationController?.delegate = nil
        activePicker = nil
    }

    private func presentImportError(attempt: Int = 0) {
        precondition(Thread.isMainThread)
        if attempt == 0 {
            guard !importErrorPending else { return }
            importErrorPending = true
            DispatchQueue.main.async { [weak self] in
                self?.presentImportError(attempt: 1)
            }
            return
        }
        guard importErrorPending else { return }
        guard let view = host?.view, !view.didTearDown,
              let presenter = view.nearestViewController()
        else {
            importErrorPending = false
            return
        }
        guard presenter.presentedViewController == nil else {
            guard attempt < 20 else {
                importErrorPending = false
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
                self?.presentImportError(attempt: attempt + 1)
            }
            return
        }

        let alert = UIAlertController(
            title: NSLocalizedString(
                "imageImport.error.title",
                comment: "Image import failure title"
            ),
            message: NSLocalizedString(
                "imageImport.error.body",
                comment: "Image import failure message"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("OK", comment: "Dismiss alert"),
            style: .default,
            handler: { [weak self] _ in
                self?.importErrorPending = false
                self?.activeAlert = nil
            }
        ))
        activeAlert = alert
        presenter.present(alert, animated: true)
    }
}

extension ImageImportCoordinator: UIDocumentPickerDelegate {
    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        finishPicker()
    }

    func documentPicker(
        _ controller: UIDocumentPickerViewController,
        didPickDocumentsAt urls: [URL]
    ) {
        guard let url = urls.first else {
            finishPicker()
            return
        }
        finishPicker()
        readPickedFile(url)
    }
}

extension ImageImportCoordinator: UIAdaptivePresentationControllerDelegate {
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
        finishPicker()
    }
}
