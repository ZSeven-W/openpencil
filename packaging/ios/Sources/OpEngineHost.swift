import Foundation
import Metal
import QuartzCore
import UIKit
/// Main-thread owner of the engine, Metal surface, lifecycle, and shell bridges.
final class OpEngineHost: NSObject {
    weak var view: OpPlayerView?
    var authStorageURL: URL?
    var authConfigured = false

    private(set) var engine: OpaquePointer?
    private weak var surfaceLayer: CAMetalLayer?
    private var logicalSize = CGSize.zero
    private var scale: CGFloat = 1
    private var viewportInsets = ViewportInsets(top: 0, right: 0, bottom: 0, left: 0)
    private var viewportSynchronized = false
    private var isSuspended = false
    private var isAlive = true
    private var displayLink: CADisplayLink?
    private let displayLinkTarget = OpDisplayLinkTarget()
    private var observers: [NSObjectProtocol] = []
    private lazy var documentExportCoordinator = DocumentExportCoordinator(host: self)
    private lazy var documentSaveCoordinator = DocumentSaveCoordinator(host: self)
    private lazy var imageImportCoordinator = ImageImportCoordinator(host: self)
    // Internal (not private) so OpEngineHost+Pointer.swift's `editor*At` wrappers can observe engine work.
    lazy var generationBackgroundCoordinator = GenerationBackgroundCoordinator(host: self)
    /// Editor mode (full desktop chrome) vs bare viewer.
    let editorMode: Bool
    private var imeFocused = false
    private var prefersLightSystemIcons: Bool?
    private var didReportSystemChromeFailure = false

    init(editorMode: Bool) {
        self.editorMode = editorMode
        super.init()
        displayLinkTarget.host = self
        let link = CADisplayLink(target: displayLinkTarget, selector: #selector(OpDisplayLinkTarget.tick(_:)))
        link.add(to: .main, forMode: .common)
        link.isPaused = true
        displayLink = link

        let center = NotificationCenter.default
        observers.append(center.addObserver(
            forName: UIApplication.didEnterBackgroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.suspendForBackground()
        })
        observers.append(center.addObserver(
            forName: UIApplication.willEnterForegroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.resumeFromForeground()
        })
    }

    convenience override init() {
        self.init(editorMode: false)
    }

    deinit {
        displayLink?.invalidate()
        observers.forEach(NotificationCenter.default.removeObserver)
    }

    func configure(
        surface: CAMetalLayer,
        logicalSize size: CGSize,
        scale newScale: CGFloat,
        safeArea insets: UIEdgeInsets
    ) -> Bool {
        precondition(Thread.isMainThread)
        guard isAlive, size.width > 0, size.height > 0, newScale > 0 else { return false }
        let nextInsets = ViewportInsets.clamped(
            top: insets.top,
            right: insets.right,
            bottom: insets.bottom,
            left: insets.left,
            to: size
        )
        let viewportChanged = ViewportChange.requiresResize(
            currentSize: logicalSize,
            currentScale: scale,
            nextSize: size,
            nextScale: newScale
        ) || viewportInsets != nextInsets
        surfaceLayer = surface
        logicalSize = size
        scale = newScale
        viewportInsets = nextInsets

        if engine == nil {
            createAndAttach(surface: surface)
            return updateEngineViewport()
        } else if viewportChanged || !viewportSynchronized {
            // Keyboard-layout-guide convergence can schedule a UIKit layout
            // without changing this Metal view's viewport. Do not turn that
            // layout pass into an engine resize (and an automatic camera fit).
            return updateEngineViewport()
        }
        return true
    }

    func teardown() {
        precondition(Thread.isMainThread)
        guard isAlive else { return }
        displayLink?.isPaused = true
        displayLink?.invalidate()
        displayLink = nil
        documentExportCoordinator.cancelForTeardown()
        documentSaveCoordinator.cancelForTeardown()
        imageImportCoordinator.cancelForTeardown()
        generationBackgroundCoordinator.teardown()

        if let engine {
            // A live press/move ladder must never cross the suspend barrier.
            cancelGesturesBeforeSuspend()
            let suspendStatus = op_suspend(engine)
            if suspendStatus != OpStatus_Ok {
                NSLog("op_suspend failed with status %d", suspendStatus.rawValue)
            }
            isSuspended = true
            let destroyStatus = op_destroy(engine)
            if destroyStatus != OpStatus_Ok {
                reportFailure(destroyStatus, operation: "op_destroy", engine: nil)
            }
            self.engine = nil
        }
        isAlive = false
        observers.forEach(NotificationCenter.default.removeObserver)
        observers.removeAll()
    }

    private func createAndAttach(surface: CAMetalLayer) {
        // The full editor starts with the engine's canonical blank document.
        // `-doc <name>` remains an explicit bundled-document override; the
        // viewer-only shell keeps `ppt-demo` as its fallback.
        let explicitDocName: String? = {
            let args = ProcessInfo.processInfo.arguments
            if let index = args.firstIndex(of: "-doc"), index + 1 < args.count {
                return args[index + 1]
            }
            return nil
        }()
        let document: Data
        if editorMode && explicitDocName == nil {
            document = Data()
        } else {
            let docName = explicitDocName ?? "ppt-demo"
            guard
                let documentURL = Bundle.main.url(forResource: docName, withExtension: "op"),
                let bundledDocument = try? Data(contentsOf: documentURL)
            else {
                NSLog("OpenPencil Player could not load bundled document %@.op", docName)
                return
            }
            document = bundledDocument
        }
        guard let storageURL = AuthStorage.prepare() else {
            NSLog("OpenPencil Player could not prepare its private storage")
            return
        }

        let assetBase = Data((Bundle.main.resourceURL?.path ?? "").utf8)
        let storageRoot = Data(storageURL.path.utf8)
        guard !storageRoot.isEmpty else {
            NSLog("OpenPencil Player resolved an empty private storage path")
            return
        }
        // Saves must land where the user can find them: NSDocumentDirectory
        // is what Files shows under "On My iPhone -> OpenPencil". An empty
        // value leaves the engine on its private fallback rather than
        // failing to start.
        let documentsRoot = Data((DocumentStorage.prepare()?.path ?? "").utf8)
        var callbacks = makeCallbacks()
        var created: OpaquePointer?

        let status = document.withUnsafeBytes { documentBytes in
            assetBase.withUnsafeBytes { assetBytes in
                storageRoot.withUnsafeBytes { storageBytes in
                    documentsRoot.withUnsafeBytes { documentsBytes in
                        withUnsafePointer(to: &callbacks) { callbacksPointer in
                            var desc = OpCreateDesc()
                            desc.size = MemoryLayout<OpCreateDesc>.size
                            desc.doc_ptr = documentBytes.bindMemory(to: UInt8.self).baseAddress
                            desc.doc_len = documentBytes.count
                            desc.width = Float(logicalSize.width)
                            desc.height = Float(logicalSize.height)
                            desc.dpr = Float(scale)
                            desc.callbacks = callbacksPointer
                            desc.asset_base_ptr = assetBytes.bindMemory(to: UInt8.self).baseAddress
                            desc.asset_base_len = assetBytes.count
                            desc.mode = editorMode ? 1 : 0
                            desc.storage_root_ptr = storageBytes.bindMemory(to: UInt8.self).baseAddress
                            desc.storage_root_len = storageBytes.count
                            desc.documents_root_ptr = documentsBytes
                                .bindMemory(to: UInt8.self).baseAddress
                            desc.documents_root_len = documentsBytes.count
                            return op_create(&desc, &created)
                        }
                    }
                }
            }
        }

        guard status == OpStatus_Ok, let created else {
            reportFailure(status, operation: "op_create", engine: nil)
            return
        }
        engine = created
        if editorMode { DocumentSaveCoordinator.declareCapability(engine: created, host: self) }
        registerBundledFonts(engine: created)
        var surfaceDesc = OpSurfaceDesc()
        surfaceDesc.size = MemoryLayout<OpSurfaceDesc>.size
        surfaceDesc.handle = Unmanaged.passUnretained(surface).toOpaque()
        let attach = op_attach_surface(created, &surfaceDesc)
        guard attach == OpStatus_Ok else {
            reportFailure(attach, operation: "op_attach_surface", engine: created)
            _ = op_destroy(created)
            engine = nil
            return
        }
        isSuspended = false
        authStorageURL = storageURL
        configureMobileAuth(engine: created, storageURL: storageURL)
        applyPersistedLocale(engine: created)
        UpdateChecker.checkOncePerDay { [weak self] in
            self?.view?.nearestViewController()
        }
        syncSystemChromeStyle()
    }


    /// Registers every bundled `fonts/*.ttf` into the engine's font
    /// registry (mirrors the Android shell's asset staging).
    private func registerBundledFonts(engine: OpaquePointer) {
        guard let fontDir = Bundle.main.resourceURL?.appendingPathComponent("fonts") else { return }
        let files = (try? FileManager.default.contentsOfDirectory(
            at: fontDir,
            includingPropertiesForKeys: nil
        )) ?? []
        for file in files where file.pathExtension == "ttf" || file.pathExtension == "otf" {
            guard let data = try? Data(contentsOf: file) else { continue }
            let status = data.withUnsafeBytes { bytes in
                op_register_font(engine, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
            }
            if status != OpStatus_Ok {
                NSLog("op_register_font(%@) failed with %d", file.lastPathComponent, status.rawValue)
            }
        }
    }

    // MARK: - Editor ABI forwarding

    func editorPress(x: CGFloat, y: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_press(engine, Float(x), Float(y))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_press", engine: engine)
        } else if status == OpStatus_Ok {
            drainShellActions()
            generationBackgroundCoordinator.observeEngineWork()
        }
    }

    func editorMove(x: CGFloat, y: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_move(engine, Float(x), Float(y))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_move", engine: engine)
        }
    }

    func editorRelease(x: CGFloat, y: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_release(engine, Float(x), Float(y))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_release", engine: engine)
        }
        if status == OpStatus_Ok { generationBackgroundCoordinator.observeEngineWork() }
    }

    func editorCancelGesture() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        let status = op_editor_cancel_gesture(engine)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_cancel_gesture", engine: engine)
        } else if status == OpStatus_Ok {
            requestImmediateFrame()
        }
    }

    func editorBeginTransform(x: CGFloat, y: CGFloat) {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        let status = op_editor_begin_transform(engine, Float(x), Float(y))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_begin_transform", engine: engine)
        }
    }

    func editorRightPress(x: CGFloat, y: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_right_press(engine, Float(x), Float(y))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_right_press", engine: engine)
        }
    }

    func editorPan(x: CGFloat, y: CGFloat, dx: CGFloat, dy: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_pan(engine, Float(x), Float(y), Float(dx), Float(dy))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_pan", engine: engine)
        }
    }

    func editorPinch(x: CGFloat, y: CGFloat, delta: CGFloat) {
        guard let engine, editorMode else { return }
        let status = op_editor_pinch(engine, Float(x), Float(y), Float(delta))
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_pinch", engine: engine)
        }
    }

    func editorText(_ text: String) {
        guard let engine, editorMode else { return }
        let data = Data(text.utf8)
        let status = data.withUnsafeBytes { bytes in
            op_editor_text(engine, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_text", engine: engine)
        }
    }

    func editorKey(_ key: Int32) {
        guard let engine, editorMode else { return }
        let status = op_editor_key(engine, key)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_key", engine: engine)
        }
        if status == OpStatus_Ok { generationBackgroundCoordinator.observeEngineWork() }
    }

    func editorImePreedit(_ text: String, selection: Range<Int>) {
        guard let engine, editorMode else { return }
        let data = Data(text.utf8)
        let status = data.withUnsafeBytes { bytes in
            op_editor_ime_preedit(
                engine,
                bytes.bindMemory(to: UInt8.self).baseAddress,
                bytes.count,
                selection.lowerBound,
                selection.upperBound
            )
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_ime_preedit", engine: engine)
        }
    }

    /// Pastes clipboard text into whichever engine text input owns the
    /// keyboard (the long-press edit menu's Paste action). No-op without
    /// a focused input.
    func editorPasteText(_ text: String) {
        guard let engine, editorMode, !text.isEmpty else { return }
        let data = Data(text.utf8)
        let status = data.withUnsafeBytes { bytes in
            op_editor_paste_text(engine, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_paste_text", engine: engine)
        }
    }

    /// Synchronous IME-focus probe for gesture-time decisions. The polled
    /// `imeFocused` mirror updates only after frames, so the long-press
    /// handler asks the engine directly.
    func editorImeFocusedNow() -> Bool {
        guard let engine, editorMode else { return false }
        var focused = false
        return op_editor_ime_focused(engine, &focused) == OpStatus_Ok && focused
    }

    func editorImeCommit(_ text: String) {
        guard let engine, editorMode else { return }
        let data = Data(text.utf8)
        let status = data.withUnsafeBytes { bytes in
            op_editor_ime_commit(engine, bytes.bindMemory(to: UInt8.self).baseAddress, bytes.count)
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_ime_commit", engine: engine)
        }
    }

    /// Replaces the current editor document after the system file picker has
    /// copied the selected bytes. The ABI validates before committing, so a
    /// malformed document leaves the current canvas untouched.
    func openDocument(_ document: Data, filename: String) {
        precondition(Thread.isMainThread)
        guard let engine, editorMode, !document.isEmpty else {
            view?.showDocumentOpenError()
            return
        }
        let name = Data(filename.utf8)
        let status = document.withUnsafeBytes { documentBytes in
            name.withUnsafeBytes { nameBytes in
                op_editor_open_document(
                    engine,
                    documentBytes.bindMemory(to: UInt8.self).baseAddress,
                    documentBytes.count,
                    nameBytes.bindMemory(to: UInt8.self).baseAddress,
                    nameBytes.count
                )
            }
        }
        guard status == OpStatus_Ok else {
            reportFailure(status, operation: "op_editor_open_document", engine: engine)
            view?.showDocumentOpenError()
            return
        }
        imeFocused = false
        view?.imeFocusChanged(false)
        requestImmediateFrame()
    }

    /// Drains one-shot requests emitted by engine-painted chrome (deferred so UIKit is never entered inside an editor ABI call stack).
    func drainShellActions() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        for _ in 0..<8 {
            var action = Int32(OpShellAction_None.rawValue)
            let status = op_editor_take_shell_action(engine, &action)
            guard status == OpStatus_Ok else {
                if status != OpStatus_Suspended {
                    reportFailure(status, operation: "op_editor_take_shell_action", engine: engine)
                }
                return
            }
            guard action != Int32(OpShellAction_None.rawValue) else { return }
            if action == Int32(OpShellAction_OpenDocument.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.view?.showDocumentPicker()
                }
            } else if action == Int32(OpShellAction_OpenLoginWebView.rawValue) {
                guard let url = copyLoginURL(engine: engine) else {
                    cancelLoginFlow()
                    continue
                }
                DispatchQueue.main.async { [weak self] in
                    self?.view?.showNativeLogin(url: url)
                }
            } else if action == Int32(OpShellAction_CloseLoginWebView.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.view?.closeNativeLoginFromHost()
                }
            } else if action == Int32(OpShellAction_RequestLogin.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.startLoginFlow()
                }
            } else if action == Int32(OpShellAction_OpenLanguagePicker.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.presentLanguagePicker()
                }
            } else if action == Int32(OpShellAction_OpenAccountCenter.rawValue) {
                guard let snapshot = accountSnapshot(engine: engine) else { continue }
                DispatchQueue.main.async { [weak self] in
                    self?.view?.showAccountCenter(snapshot: snapshot)
                }
            } else if action == Int32(OpShellAction_ExportDocument.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.documentExportCoordinator.beginExport()
                }
            } else if action == Int32(OpShellAction_SaveDocument.rawValue) {
                DispatchQueue.main.async { [weak self] in
                    self?.documentSaveCoordinator.beginSave()
                }
            } else if action == Int32(OpShellAction_ImportImageOrSvg.rawValue) {
                DispatchQueue.main.async { [weak self] in self?.imageImportCoordinator.beginImport() }
            }
        }
    }

    /// Polls the engine's IME focus after every frame; the view shows or
    /// hides the system keyboard on transitions.
    func syncImeFocus() {
        guard let engine, editorMode else { return }
        var focused = false
        let status = op_editor_ime_focused(engine, &focused)
        guard status == OpStatus_Ok else { return }
        if focused != imeFocused {
            imeFocused = focused
            DispatchQueue.main.async { [weak self] in
                self?.view?.imeFocusChanged(focused)
            }
        }
    }

    /// Outbound clipboard bridge: engine copy buttons (collab invite /
    /// share address, MCP config, chat copy) queue text that the desktop
    /// runner drains into the OS clipboard; here the same queue lands on
    /// `UIPasteboard`. The two-phase probe/copy contract mirrors
    /// `copyLoginURL`; NotReady is the common per-frame case.
    private func drainCopyText() {
        guard let engine, editorMode else { return }
        var required = 0
        var status = op_editor_take_copy_text(engine, nil, 0, &required)
        guard status == OpStatus_Ok, required > 0 else { return }
        var buffer = [UInt8](repeating: 0, count: required)
        status = buffer.withUnsafeMutableBufferPointer { bytes in
            op_editor_take_copy_text(engine, bytes.baseAddress, bytes.count, &required)
        }
        guard status == OpStatus_Ok, let text = String(bytes: buffer, encoding: .utf8) else {
            return
        }
        UIPasteboard.general.string = text
    }

    // MARK: - Remote images

    /// `remote_image_request` upcall: the paint pass hit a remote image
    /// miss; fetch the bytes and push them back into the engine.
    func deferRemoteImageRequest(requestID: UInt64, url: String) {
        guard engine != nil else { return }
        guard let remoteURL = URL(string: url) else { return }
        let task = URLSession.shared.dataTask(with: remoteURL) { [weak self] data, _, error in
            guard let self, let engine = self.engine, let data, error == nil else { return }
            let status = data.withUnsafeBytes { bytes in
                op_remote_image_result(
                    engine,
                    requestID,
                    bytes.bindMemory(to: UInt8.self).baseAddress,
                    bytes.count
                )
            }
            if status == OpStatus_Ok {
                DispatchQueue.main.async { [weak self] in
                    self?.requestImmediateFrame()
                }
            }
        }
        task.resume()
    }

    // MARK: - Callbacks

    private func makeCallbacks() -> OpCallbacks {
        var callbacks = OpCallbacks()
        callbacks.size = MemoryLayout<OpCallbacks>.size
        callbacks.user_data = Unmanaged.passUnretained(self).toOpaque()
        callbacks.needs_redraw = opPlayerNeedsRedraw
        callbacks.runtime_error = opPlayerRuntimeError
        callbacks.input_focus_changed = opPlayerInputFocusChanged
        callbacks.remote_image_request = opPlayerRemoteImageRequest
        // iOS uses the transport's Security.framework Keychain backend
        // directly. The shell-provided credential bridge is Android-only.
        callbacks.credential_load = nil
        callbacks.credential_store_if_absent = nil
        return callbacks
    }

    private func updateEngineViewport() -> Bool {
        guard let engine else {
            viewportSynchronized = false
            return false
        }
        let status = op_resize_with_safe_area(
            engine,
            Float(logicalSize.width),
            Float(logicalSize.height),
            Float(scale),
            Float(viewportInsets.top),
            Float(viewportInsets.right),
            Float(viewportInsets.bottom),
            Float(viewportInsets.left)
        )
        if status != OpStatus_Ok {
            reportFailure(status, operation: "op_resize_with_safe_area", engine: engine)
        }
        viewportSynchronized = status == OpStatus_Ok
        return viewportSynchronized
    }

    func updateKeyboardHeight(_ height: CGFloat) {
        precondition(Thread.isMainThread)
        guard let engine else { return }
        let clamped = max(0, min(height, logicalSize.height))
        let status = op_set_keyboard(engine, Float(clamped))
        if status != OpStatus_Ok {
            reportFailure(status, operation: "op_set_keyboard", engine: engine)
        }
    }

    func displayLinkDidFire(_ link: CADisplayLink) {
        precondition(Thread.isMainThread)
        link.isPaused = true
        guard let engine, !isSuspended else { return }
        let status = op_frame(engine, Self.nowMilliseconds())
        if status == OpStatus_GpuError {
            scheduleWake(at: Self.nowMilliseconds() + 17)
        } else if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_frame", engine: engine)
        }
        generationBackgroundCoordinator.observeEngineWork()
        if editorMode {
            syncImeFocus()
            drainCopyText()
            drainShellActions()
        }
        syncSystemChromeStyle()
    }

    /// Keeps native status-bar and Home Indicator glyphs legible over the
    /// engine-painted edge-to-edge backdrop. The Rust editor owns its theme;
    /// UIKit only needs the resulting light-vs-dark glyph preference.
    private func syncSystemChromeStyle() {
        guard let engine else { return }
        var prefersLight = false
        let status = op_prefers_light_system_icons(engine, &prefersLight)
        guard status == OpStatus_Ok else {
            if status != OpStatus_Suspended, !didReportSystemChromeFailure {
                didReportSystemChromeFailure = true
                reportFailure(status, operation: "op_prefers_light_system_icons", engine: engine)
            }
            return
        }
        didReportSystemChromeFailure = false
        guard prefersLight != prefersLightSystemIcons else { return }
        prefersLightSystemIcons = prefersLight
        view?.updateSystemChrome(prefersLightIcons: prefersLight)
    }

    private func suspendForBackground() {
        precondition(Thread.isMainThread)
        guard let engine, !isSuspended else { return }
        displayLink?.isPaused = true
        // Cancel first, suspend second: cancelGesturesBeforeSuspend issues
        // the raw entry points so it cannot re-light the just-paused
        // display link before the engine suspends.
        cancelGesturesBeforeSuspend()
        let status = op_suspend(engine)
        if status == OpStatus_Ok {
            isSuspended = true
            generationBackgroundCoordinator.didEnterBackground()
        } else {
            reportFailure(status, operation: "op_suspend", engine: engine)
        }
    }

    private func resumeFromForeground() {
        precondition(Thread.isMainThread)
        generationBackgroundCoordinator.willEnterForeground()
        guard let engine, isSuspended, let surfaceLayer else { return }
        var desc = OpSurfaceDesc()
        desc.size = MemoryLayout<OpSurfaceDesc>.size
        desc.handle = Unmanaged.passUnretained(surfaceLayer).toOpaque()
        let status = op_resume(engine, &desc)
        if status == OpStatus_Ok {
            isSuspended = false
            requestImmediateFrame()
        } else {
            reportFailure(status, operation: "op_resume", engine: engine)
        }
    }

    /// `needs_redraw` upcall: mutations (pointer / resize / attach /
    /// caret blink) resume the display link or schedule a timed wake.
    func deferNeedsRedraw(hasNextWake: Bool, nextWakeMilliseconds: UInt64) {
        DispatchQueue.main.async { [weak self] in
            guard let self, self.isAlive, !self.isSuspended else { return }
            if hasNextWake {
                self.scheduleWake(at: nextWakeMilliseconds)
            } else {
                self.displayLink?.isPaused = false
            }
        }
    }

    private func scheduleWake(at milliseconds: UInt64) {
        let now = Self.nowMilliseconds()
        if milliseconds <= now {
            displayLink?.isPaused = false
            return
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + Double(milliseconds - now) / 1_000) { [weak self] in
            guard let self, self.isAlive, !self.isSuspended else { return }
            self.displayLink?.isPaused = false
        }
    }

    func deferRuntimeError(_ message: String, source: String, kind: Int32) {
        DispatchQueue.main.async {
            let suffix = source.isEmpty ? "" : " [\(source)]"
            NSLog("OpenPencil runtime diagnostic kind=%d: %@%@", kind, message, suffix)
        }
    }

    func deferInputFocusChanged(focused: Bool, inputKind: Int32, returnKeyHint: Int32) {
        DispatchQueue.main.async { [weak self] in
            self?.view?.imeFocusChanged(focused)
        }
    }

    func requestImmediateFrame() {
        precondition(Thread.isMainThread)
        guard isAlive, !isSuspended else { return }
        displayLink?.isPaused = false
    }

    func reportFailure(_ status: OpStatus, operation: String, engine: OpaquePointer?) {
        let detail = lastError(engine: engine)
        if detail.isEmpty {
            NSLog("%@ failed with OpStatus %d", operation, status.rawValue)
        } else {
            NSLog("%@ failed with OpStatus %d: %@", operation, status.rawValue, detail)
        }
    }

    private func lastError(engine: OpaquePointer?) -> String {
        var required = 0
        guard op_last_error(engine, nil, 0, &required) == OpStatus_Ok, required > 0 else {
            return ""
        }
        var bytes = [UInt8](repeating: 0, count: required)
        let status = bytes.withUnsafeMutableBufferPointer { buffer in
            op_last_error(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard status == OpStatus_Ok else { return "" }
        return String(decoding: bytes.prefix(required), as: UTF8.self)
    }

    static func nowMilliseconds() -> UInt64 {
        UInt64((CACurrentMediaTime() * 1_000).rounded(.down))
    }
}

private final class OpDisplayLinkTarget: NSObject {
    weak var host: OpEngineHost?

    @objc func tick(_ link: CADisplayLink) {
        host?.displayLinkDidFire(link)
    }
}

private func host(from userData: UnsafeMutableRawPointer?) -> OpEngineHost? {
    guard let userData else { return nil }
    return Unmanaged<OpEngineHost>.fromOpaque(userData).takeUnretainedValue()
}

private func copiedString(_ pointer: UnsafePointer<UInt8>?, _ length: Int) -> String {
    guard let pointer, length > 0 else { return "" }
    return String(decoding: UnsafeBufferPointer(start: pointer, count: length), as: UTF8.self)
}

private func opPlayerNeedsRedraw(
    _ userData: UnsafeMutableRawPointer?,
    _ hasNextWake: Bool,
    _ nextWakeMilliseconds: UInt64
) {
    host(from: userData)?.deferNeedsRedraw(
        hasNextWake: hasNextWake,
        nextWakeMilliseconds: nextWakeMilliseconds
    )
}

private func opPlayerRuntimeError(
    _ userData: UnsafeMutableRawPointer?,
    _ error: UnsafePointer<OpRuntimeError>?
) {
    guard let value = error?.pointee else { return }
    let message = copiedString(value.message_ptr, value.message_len)
    let source = copiedString(value.source_ptr, value.source_len)
    host(from: userData)?.deferRuntimeError(message, source: source, kind: value.kind)
}

private func opPlayerInputFocusChanged(
    _ userData: UnsafeMutableRawPointer?,
    _ focused: Bool,
    _ inputKind: Int32,
    _ returnKeyHint: Int32
) {
    host(from: userData)?.deferInputFocusChanged(focused: focused, inputKind: inputKind, returnKeyHint: returnKeyHint)
}

private func opPlayerRemoteImageRequest(
    _ userData: UnsafeMutableRawPointer?,
    _ requestID: UInt64,
    _ urlPointer: UnsafePointer<UInt8>?,
    _ urlLength: Int
) {
    let url = copiedString(urlPointer, urlLength)
    host(from: userData)?.deferRemoteImageRequest(requestID: requestID, url: url)
}
