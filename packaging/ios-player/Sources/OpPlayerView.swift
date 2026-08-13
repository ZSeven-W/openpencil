import Metal
import QuartzCore
import UIKit
import UniformTypeIdentifiers

/// The OpenPencil player surface: a `CAMetalLayer`-backed view that owns
/// the engine host, forwards touches (the engine interprets tap / pan /
/// pinch / long-press in editor mode), bridges the system keyboard/IME,
/// and tracks safe-area + keyboard occlusion changes.
final class OpPlayerView: UIView, UITextViewDelegate, UIDocumentPickerDelegate {
    override class var layerClass: AnyClass { CAMetalLayer.self }

    let host: OpEngineHost
    private var touchIDs: [ObjectIdentifier: UInt32] = [:]
    private var nextTouchID: UInt32 = 1
    private var keyboardObservers: [NSObjectProtocol] = []
    private var didTearDown = false
    private var documentPickerPresented = false
    private var documentOpenErrorPending = false

    // ---- Editor-mode gesture state (mirrors the Android shell) --------
    private var primaryTouchKey: ObjectIdentifier?
    private var longPressWork: DispatchWorkItem?
    private var longPressFired = false
    private var downPoint = CGPoint.zero
    private var lastKnownPoint = CGPoint.zero
    private var twoFingerActive = false
    private var pinchTouches: [ObjectIdentifier: UITouch] = [:]
    private var lastMidpoint = CGPoint.zero
    private var lastPinchDistance: CGFloat = 0

    // ---- IME bridge ----------------------------------------------------
    private let imeTextView = UITextView()
    private var wasComposing = false
    private var composingSelection = NSRange(location: 0, length: 0)

    private var metalLayer: CAMetalLayer {
        guard let layer = layer as? CAMetalLayer else {
            preconditionFailure("OpPlayerView must be backed by CAMetalLayer")
        }
        return layer
    }

    init(editorMode: Bool) {
        host = OpEngineHost(editorMode: editorMode)
        super.init(frame: .zero)
        commonInit()
    }

    override convenience init(frame: CGRect) {
        self.init(editorMode: false)
    }

    required init?(coder: NSCoder) {
        host = OpEngineHost(editorMode: false)
        super.init(coder: coder)
        commonInit()
    }

    private func commonInit() {
        isMultipleTouchEnabled = true
        isOpaque = true
        backgroundColor = .black
        contentMode = .redraw
        host.view = self
        setupImeTextView()

        let center = NotificationCenter.default
        keyboardObservers.append(center.addObserver(
            forName: UIResponder.keyboardWillChangeFrameNotification,
            object: nil,
            queue: .main
        ) { [weak self] notification in
            self?.keyboardFrameDidChange(notification)
        })
        keyboardObservers.append(center.addObserver(
            forName: UIResponder.keyboardWillHideNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.host.updateKeyboardHeight(0)
        })
    }

    deinit {
        keyboardObservers.forEach(NotificationCenter.default.removeObserver)
        longPressWork?.cancel()
    }

    // MARK: - IME bridge

    private func setupImeTextView() {
        // A 1×1 offscreen text view that can still become first responder:
        // the engine owns the text, this view is only the IME conduit.
        imeTextView.frame = CGRect(x: -40, y: -40, width: 1, height: 1)
        imeTextView.delegate = self
        imeTextView.autocorrectionType = .no
        imeTextView.autocapitalizationType = .none
        imeTextView.smartInsertDeleteType = .no
        imeTextView.keyboardType = .default
        imeTextView.returnKeyType = .default
        // This offscreen view is only an IME conduit. The iPad hardware-
        // keyboard assistant would otherwise appear as a separate light bar
        // over the engine-painted composer even though it has no actions.
        imeTextView.inputAssistantItem.leadingBarButtonGroups = []
        imeTextView.inputAssistantItem.trailingBarButtonGroups = []
        imeTextView.isHidden = false
        imeTextView.isUserInteractionEnabled = false
        addSubview(imeTextView)
    }

    /// The engine's IME focus flipped; show or hide the system keyboard.
    func imeFocusChanged(_ focused: Bool) {
        if focused {
            if !imeTextView.isFirstResponder {
                imeTextView.becomeFirstResponder()
            }
        } else {
            if imeTextView.isFirstResponder {
                imeTextView.resignFirstResponder()
            }
        }
    }

    func textViewDidChange(_ textView: UITextView) {
        guard host.editorMode else { return }
        let marked = markedText(of: textView)
        if let marked {
            // Composition update: forward the marked text + caret.
            wasComposing = true
            let markedRange = textView.markedTextRange
            let selection = textView.selectedTextRange
            var selStart = 0
            var selEnd = 0
            if let markedRange, let selection {
                let startOffset = textView.offset(from: textView.beginningOfDocument, to: selection.start)
                let endOffset = textView.offset(from: textView.beginningOfDocument, to: selection.end)
                let markedStartOffset = textView.offset(from: textView.beginningOfDocument, to: markedRange.start)
                selStart = max(0, startOffset - markedStartOffset)
                selEnd = max(0, endOffset - markedStartOffset)
            }
            composingSelection = NSRange(location: selStart, length: selEnd - selStart)
            host.editorImePreedit(marked, selection: selStart..<selEnd)
        } else if wasComposing {
            // The composition committed: send the marked text that just
            // landed as the commit payload.
            wasComposing = false
            let committed = (textView.text as NSString).substring(with: composingSelection)
            host.editorImeCommit(committed.isEmpty ? textView.text : committed)
        }
        host.requestImmediateFrame()
    }

    func textView(
        _ textView: UITextView,
        shouldChangeTextIn range: NSRange,
        replacementText text: String
    ) -> Bool {
        guard host.editorMode else { return true }
        // While composing, the system owns the edit (preedit updates flow
        // through textViewDidChange).
        if markedText(of: textView) != nil {
            return true
        }
        if text == "\n" {
            host.editorKey(Int32(OpKey_Enter))
            host.requestImmediateFrame()
            return false
        }
        if range.length == 0 {
            // Plain insertion (typing / paste).
            host.editorText(text)
        } else {
            // Deletion at the caret.
            let atEnd = range.location + range.length >= (textView.text as NSString).length
            host.editorKey(Int32(atEnd ? OpKey_Backspace : OpKey_Delete))
        }
        host.requestImmediateFrame()
        return false
    }

    // MARK: - Page overlay

    // MARK: - Layout

    override func layoutSubviews() {
        super.layoutSubviews()
        guard !didTearDown, bounds.width > 0, bounds.height > 0 else { return }
        let surface = metalLayer
        let displayScale = window?.screen.scale ?? UIScreen.main.scale
        contentScaleFactor = displayScale
        surface.contentsScale = displayScale
        surface.device = surface.device ?? MTLCreateSystemDefaultDevice()
        surface.pixelFormat = .bgra8Unorm
        surface.framebufferOnly = false
        surface.presentsWithTransaction = false
        surface.drawableSize = CGSize(
            width: (bounds.width * displayScale).rounded(),
            height: (bounds.height * displayScale).rounded()
        )

        host.configure(surface: surface, logicalSize: bounds.size, scale: displayScale)
        host.updateSafeArea(safeAreaInsets)
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        guard !didTearDown else { return }
        host.updateSafeArea(safeAreaInsets)
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil, superview == nil {
            teardownEngine()
        } else {
            setNeedsLayout()
        }
    }

    func teardownEngine() {
        guard !didTearDown else { return }
        didTearDown = true
        host.teardown()
        keyboardObservers.forEach(NotificationCenter.default.removeObserver)
        keyboardObservers.removeAll()
    }

    // MARK: - Document picker

    /// Presents the system Files browser once for an engine shell request.
    /// Custom extensions keep `.op` and legacy `.pen` visible; `.data` is the
    /// fallback for providers that do not preserve a custom content type.
    func showDocumentPicker() {
        precondition(Thread.isMainThread)
        guard
            !didTearDown,
            !documentPickerPresented,
            let presenter = nearestViewController(),
            presenter.presentedViewController == nil
        else { return }

        let openPencil = UTType(filenameExtension: "op")
        let legacyPen = UTType(filenameExtension: "pen")
        let types = [openPencil, legacyPen].compactMap { $0 } + [.data]
        let picker = UIDocumentPickerViewController(forOpeningContentTypes: types, asCopy: true)
        picker.delegate = self
        picker.allowsMultipleSelection = false
        picker.modalPresentationStyle = .formSheet
        documentPickerPresented = true
        presenter.present(picker, animated: true)
    }

    func documentPickerWasCancelled(_ controller: UIDocumentPickerViewController) {
        documentPickerPresented = false
    }

    func documentPicker(
        _ controller: UIDocumentPickerViewController,
        didPickDocumentsAt urls: [URL]
    ) {
        documentPickerPresented = false
        guard let url = urls.first else { return }
        let filename = url.lastPathComponent
        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            let scoped = url.startAccessingSecurityScopedResource()
            defer {
                if scoped { url.stopAccessingSecurityScopedResource() }
            }
            do {
                let document = try BoundedDocumentReader.read(from: url)
                DispatchQueue.main.async { [weak self] in
                    guard let self, !self.didTearDown else { return }
                    self.host.openDocument(document, filename: filename)
                }
            } catch {
                NSLog("OpenPencil could not read %@: %@", filename, error.localizedDescription)
                DispatchQueue.main.async { [weak self] in
                    self?.showDocumentOpenError()
                }
            }
        }
    }

    func showDocumentOpenError() {
        precondition(Thread.isMainThread)
        guard !documentOpenErrorPending else { return }
        documentOpenErrorPending = true
        presentDocumentOpenError(attempt: 0)
    }

    private func presentDocumentOpenError(attempt: Int) {
        guard !didTearDown, let presenter = nearestViewController() else {
            documentOpenErrorPending = false
            return
        }
        // Document providers may finish their delegate callback just before
        // UIKit completes the picker's dismissal. Wait briefly so the error
        // is not silently lost behind that outgoing controller.
        if presenter.presentedViewController != nil {
            guard attempt < 20 else {
                documentOpenErrorPending = false
                return
            }
            DispatchQueue.main.asyncAfter(deadline: .now() + 0.15) { [weak self] in
                self?.presentDocumentOpenError(attempt: attempt + 1)
            }
            return
        }
        let alert = UIAlertController(
            title: NSLocalizedString("Unable to Open File", comment: "Document-open failure title"),
            message: NSLocalizedString(
                "Choose a valid OpenPencil .op or .pen file.",
                comment: "Document-open failure message"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(
            title: NSLocalizedString("OK", comment: "Dismiss alert"),
            style: .default,
            handler: { [weak self] _ in self?.documentOpenErrorPending = false }
        ))
        presenter.present(alert, animated: true)
    }

    private func nearestViewController() -> UIViewController? {
        var responder: UIResponder? = self
        while let current = responder {
            if let controller = current as? UIViewController { return controller }
            responder = current.next
        }
        return window?.rootViewController
    }

    // MARK: - Touch routing

    override func touchesBegan(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesBegan(touches, with: event)
        for touch in touches {
            let key = ObjectIdentifier(touch)
            let id = allocateTouchID()
            storeTouch(touch, key: key)
            touchIDs[key] = id
            if host.editorMode {
                editorTouchBegan(touch, key: key)
            } else {
                dispatch(touch, id: id, phase: Int32(OpPointerPhase_Down.rawValue))
            }
        }
    }

    override func touchesMoved(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesMoved(touches, with: event)
        for touch in touches {
            guard let id = touchIDs[ObjectIdentifier(touch)] else { continue }
            if host.editorMode {
                editorTouchMoved(touch, key: ObjectIdentifier(touch))
            } else {
                dispatch(touch, id: id, phase: Int32(OpPointerPhase_Move.rawValue))
            }
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        if host.editorMode {
            for touch in touches {
                editorTouchEnded(touch, key: ObjectIdentifier(touch))
            }
        } else {
            finish(touches, phase: Int32(OpPointerPhase_Up.rawValue))
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        longPressWork?.cancel()
        if host.editorMode {
            for touch in touches {
                editorTouchEnded(touch, key: ObjectIdentifier(touch))
            }
        } else {
            finish(touches, phase: Int32(OpPointerPhase_Cancel.rawValue))
        }
    }

    private func finish(_ touches: Set<UITouch>, phase: Int32) {
        for touch in touches {
            let key = ObjectIdentifier(touch)
            guard let id = touchIDs.removeValue(forKey: key) else { continue }
            dispatch(touch, id: id, phase: phase)
        }
    }

    private func dispatch(_ touch: UITouch, id: UInt32, phase: Int32) {
        // The engine viewport is bounds.size in logical UIKit points.
        // Metal's contentsScale affects drawable pixels only, so touch
        // points are not scaled.
        host.dispatchPointer(id: id, phase: phase, point: touch.location(in: self))
    }

    private func allocateTouchID() -> UInt32 {
        let candidate = nextTouchID
        nextTouchID = nextTouchID &+ 1
        if nextTouchID == 0 { nextTouchID = 1 }
        return candidate
    }

    // MARK: - Editor-mode gestures (press/move/release + long-press + pan/pinch)

    private func editorTouchBegan(_ touch: UITouch, key: ObjectIdentifier) {
        let point = touch.location(in: self)
        if primaryTouchKey == nil {
            primaryTouchKey = key
            downPoint = point
            lastKnownPoint = point
            longPressFired = false
            armLongPress(at: point)
            host.editorPress(x: point.x, y: point.y)
        } else if touchIDs.count == 2 {
            // Second finger: pan + pinch take over. Track BOTH touches
            // (the primary was registered earlier, the new one now).
            longPressWork?.cancel()
            twoFingerActive = true
            pinchTouches.removeAll()
            for (candidateKey, _) in touchIDs {
                if let candidate = activeTouch(for: candidateKey) {
                    pinchTouches[candidateKey] = candidate
                }
            }
            updatePinchState()
        }
    }

    private func editorTouchMoved(_ touch: UITouch, key: ObjectIdentifier) {
        let point = touch.location(in: self)
        if twoFingerActive {
            if pinchTouches.count >= 2 {
                let previousMid = lastMidpoint
                let previousDistance = lastPinchDistance
                updatePinchState()
                let dx = lastMidpoint.x - previousMid.x
                let dy = lastMidpoint.y - previousMid.y
                let distanceDelta = lastPinchDistance - previousDistance
                if dx != 0 || dy != 0 {
                    host.editorPan(x: lastMidpoint.x, y: lastMidpoint.y, dx: dx, dy: dy)
                }
                if distanceDelta != 0 {
                    host.editorPinch(x: lastMidpoint.x, y: lastMidpoint.y, delta: distanceDelta)
                }
                host.requestImmediateFrame()
            }
            return
        }
        guard key == primaryTouchKey else { return }
        lastKnownPoint = point
        host.editorMove(x: point.x, y: point.y)
        // Movement beyond the slop cancels the long-press candidate.
        if let work = longPressWork, !longPressFired {
            let slop: CGFloat = 12
            if abs(point.x - downPoint.x) > slop || abs(point.y - downPoint.y) > slop {
                work.cancel()
                longPressWork = nil
            }
        }
        host.requestImmediateFrame()
    }

    private func editorTouchEnded(_ touch: UITouch, key: ObjectIdentifier) {
        let point = touch.location(in: self)
        touchIDs.removeValue(forKey: key)
        storedTouches.removeValue(forKey: key)
        pinchTouches.removeValue(forKey: key)
        if twoFingerActive {
            // One finger lifted: re-arm single-finger tracking on the
            // remaining touch (if any).
            if let remainingKey = touchIDs.keys.first {
                primaryTouchKey = remainingKey
                twoFingerActive = false
                longPressWork?.cancel()
                lastKnownPoint = touch.location(in: self)
            } else {
                twoFingerActive = false
                primaryTouchKey = nil
            }
            return
        }
        guard key == primaryTouchKey else { return }
        longPressWork?.cancel()
        longPressWork = nil
        if !longPressFired {
            host.editorRelease(x: point.x, y: point.y)
        }
        primaryTouchKey = nil
        longPressFired = false
        host.requestImmediateFrame()
    }

    private func armLongPress(at point: CGPoint) {
        longPressWork?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.longPressWork != nil else { return }
            self.longPressFired = true
            self.host.editorRightPress(x: self.lastKnownPoint.x, y: self.lastKnownPoint.y)
            self.host.requestImmediateFrame()
        }
        longPressWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    /// UITouch objects are only valid while the touch is live; keep a
    /// per-key reference so the two-finger pinch can reach the primary
    /// touch from the second finger's begin event.
    private var storedTouches: [ObjectIdentifier: UITouch] = [:]

    private func storeTouch(_ touch: UITouch, key: ObjectIdentifier) {
        storedTouches[key] = touch
    }

    private func activeTouch(for key: ObjectIdentifier) -> UITouch? {
        storedTouches[key]
    }

    private func updatePinchState() {
        guard pinchTouches.count >= 2 else { return }
        let touches = Array(pinchTouches.values)
        let first = touches[0].location(in: self)
        let second = touches[1].location(in: self)
        lastMidpoint = CGPoint(x: (first.x + second.x) / 2, y: (first.y + second.y) / 2)
        let dx = first.x - second.x
        let dy = first.y - second.y
        lastPinchDistance = (dx * dx + dy * dy).squareRoot()
        lastKnownPoint = lastMidpoint
    }

    private func markedText(of textView: UITextView) -> String? {
        guard let range = textView.markedTextRange else { return nil }
        return textView.text(in: range)
    }

    private func keyboardFrameDidChange(_ notification: Notification) {
        guard
            let screenFrame = notification.userInfo?[UIResponder.keyboardFrameEndUserInfoKey] as? CGRect,
            let window
        else {
            host.updateKeyboardHeight(0)
            return
        }
        let frameInView = convert(window.convert(screenFrame, from: nil), from: window)
        let intersection = bounds.intersection(frameInView)
        let reachesBottom = !intersection.isNull && intersection.maxY >= bounds.maxY - 1
        let spansViewport = !intersection.isNull && intersection.width >= bounds.width * 0.9
        // The engine ABI currently models keyboard occlusion as a full-width
        // bottom inset. Floating/split iPad keyboards and the compact input
        // assistant cover only part of the canvas, so treating their height as
        // global would shift the entire Rust UI and chat composer upward.
        host.updateKeyboardHeight(reachesBottom && spansViewport ? intersection.height : 0)
    }
}
