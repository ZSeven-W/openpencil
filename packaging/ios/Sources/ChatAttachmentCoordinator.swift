import Foundation
import PhotosUI
import UIKit
import UniformTypeIdentifiers

/// Owns the photo picker round trip for the Studio Home "Add screenshot"
/// button and the chat attach button (`OpShellAction_PickChatAttachment`).
///
/// The engine drains the request once per press, so a dismissed picker is a
/// silent no-op. The picked image is loaded and normalised off the main
/// thread (`ChatAttachmentImage`: HEIC / oversized photos become a JPEG under
/// the 5 MiB cap), then staged through `op_editor_attach_chat_image` on the
/// engine's owner thread. The Home thumbnail strip and the chat input strip
/// both read the staged attachment; the shell paints nothing itself.
final class ChatAttachmentCoordinator: NSObject {
    private weak var host: OpEngineHost?
    private weak var activePicker: PHPickerViewController?
    private var activeLoadToken: UUID?
    private let loadQueue = DispatchQueue(
        label: "tech.zseven.openpencil.chat-attachment",
        qos: .userInitiated
    )

    /// Engine-ready representations, in preference order. Anything else
    /// (HEIC, TIFF, RAW previews) falls back to `public.image` and is
    /// re-encoded.
    private static let directTypes: [UTType] = [
        .png,
        .jpeg,
        .gif,
        UTType(filenameExtension: "webp"),
    ].compactMap { $0 }

    init(host: OpEngineHost) {
        self.host = host
        super.init()
    }

    /// Shell action 13: present the system photo picker for one image.
    func beginPick() {
        precondition(Thread.isMainThread)
        guard
            activePicker == nil,
            activeLoadToken == nil,
            let view = host?.view,
            !view.didTearDown,
            let presenter = view.nearestViewController(),
            presenter.presentedViewController == nil
        else { return }

        // No photo-library entitlement is needed: the out-of-process picker
        // hands back only what the user selected. `.compatible` asks the
        // system to transcode HEIC when it can; the normaliser covers the
        // rest.
        var configuration = PHPickerConfiguration()
        configuration.filter = .images
        configuration.selectionLimit = 1
        configuration.preferredAssetRepresentationMode = .compatible
        let picker = PHPickerViewController(configuration: configuration)
        picker.delegate = self
        activePicker = picker
        presenter.present(picker, animated: true)
        picker.presentationController?.delegate = self
    }

    /// Teardown invalidates worker completions before dismissing the picker.
    func cancelForTeardown() {
        precondition(Thread.isMainThread)
        activeLoadToken = nil
        if let picker = activePicker {
            picker.delegate = nil
            picker.presentationController?.delegate = nil
            picker.dismiss(animated: false)
        }
        activePicker = nil
    }

    private func finishPicker() {
        precondition(Thread.isMainThread)
        activePicker?.delegate = nil
        activePicker?.presentationController?.delegate = nil
        activePicker = nil
    }

    private func load(_ provider: NSItemProvider) {
        precondition(Thread.isMainThread)
        let typeIdentifier = Self.directTypes
            .map(\.identifier)
            .first { provider.hasItemConformingToTypeIdentifier($0) }
            ?? UTType.image.identifier
        let suggestedName = provider.suggestedName
        let token = UUID()
        activeLoadToken = token
        provider.loadDataRepresentation(forTypeIdentifier: typeIdentifier) { [weak self] data, error in
            guard let self else { return }
            self.loadQueue.async { [weak self] in
                let prepared = data.flatMap {
                    ChatAttachmentImage.prepare($0, suggestedName: suggestedName)
                }
                if prepared == nil {
                    NSLog(
                        "OpenPencil could not prepare chat attachment (%@): %@",
                        typeIdentifier,
                        error?.localizedDescription ?? "undecodable or too large"
                    )
                }
                DispatchQueue.main.async { [weak self] in
                    self?.completeLoad(token: token, prepared: prepared)
                }
            }
        }
    }

    private func completeLoad(token: UUID, prepared: ChatAttachmentImage.Prepared?) {
        precondition(Thread.isMainThread)
        guard activeLoadToken == token else { return }
        activeLoadToken = nil
        guard let prepared else { return }
        attach(prepared)
    }

    private func attach(_ prepared: ChatAttachmentImage.Prepared) {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else { return }
        let mediaType = Data(prepared.mediaType.utf8)
        let fileName = Data(prepared.fileName.utf8)
        let status = prepared.data.withUnsafeBytes { dataBytes in
            mediaType.withUnsafeBytes { typeBytes in
                fileName.withUnsafeBytes { nameBytes in
                    op_editor_attach_chat_image(
                        engine,
                        dataBytes.bindMemory(to: UInt8.self).baseAddress,
                        dataBytes.count,
                        typeBytes.bindMemory(to: UInt8.self).baseAddress,
                        typeBytes.count,
                        nameBytes.bindMemory(to: UInt8.self).baseAddress,
                        nameBytes.count
                    )
                }
            }
        }
        if status != OpStatus_Ok {
            // Busy = the turn already holds the maximum attachments;
            // InvalidArg = the engine refused the bytes. Neither leaves
            // partial state behind, so logging is the whole response.
            host.reportFailure(status, operation: "op_editor_attach_chat_image", engine: engine)
        }
        host.requestImmediateFrame()
    }
}

extension ChatAttachmentCoordinator: PHPickerViewControllerDelegate {
    func picker(_ picker: PHPickerViewController, didFinishPicking results: [PHPickerResult]) {
        finishPicker()
        picker.dismiss(animated: true)
        // An empty result is the user's Cancel: the request was already
        // consumed by the drain, so there is nothing to undo.
        guard let provider = results.first?.itemProvider else { return }
        load(provider)
    }
}

extension ChatAttachmentCoordinator: UIAdaptivePresentationControllerDelegate {
    func presentationControllerDidDismiss(_ presentationController: UIPresentationController) {
        finishPicker()
    }
}
