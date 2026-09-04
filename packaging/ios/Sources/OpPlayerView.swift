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
    /// Matches the Rust touch-pan slop, in logical UIKit points.
    private static let editorLongPressSlop: CGFloat = 8

    let host: OpEngineHost
    private var touchIDs: [ObjectIdentifier: UInt32] = [:]
    private var nextTouchID: UInt32 = 1
    private var keyboardObservers: [NSObjectProtocol] = []
    var didTearDown = false
    private var documentPickerPresented = false
    private var documentOpenErrorPending = false
    weak var nativeLoginController: NativeLoginViewController?
    private var systemChromeStyle: UIUserInterfaceStyle?
    private let viewportConvergence = ViewportConvergence()
    private var viewportDisplayLink: CADisplayLink?
    private var geometryGate = ViewportGeometryGate()
    private var suppressedTouches = SuppressedTouchSet<ObjectIdentifier>()

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

    // ---- IME bridge (delegate methods in OpPlayerView+ImeBridge.swift) --
    let imeTextView = ImeConduitTextView()
    var wasComposing = false

    /// Long-press edit menu over engine text inputs (iOS 16+). Stored as
    /// the protocol type so the property itself needs no availability.
    private var editMenuInteraction: UIInteraction?

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
        if host.editorMode, #available(iOS 16.0, *) {
            let interaction = UIEditMenuInteraction(delegate: self)
            addInteraction(interaction)
            editMenuInteraction = interaction
        }
        // The editor stays laid out against the stable window viewport.
        // Floating and split keyboards are local occluders, not a full-width
        // bottom inset; UIKit's guide treats them as dismissed by default.
        keyboardLayoutGuide.followsUndockedKeyboard = false

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
        viewportDisplayLink?.invalidate()
    }

    // MARK: - IME bridge

    private func setupImeTextView() {
        // Plain backspace on the EMPTY conduit never reaches the delegate
        // (there is nothing to change), so the key must be forwarded from
        // deleteBackward itself — otherwise engine inputs can type but
        // never delete.
        imeTextView.onEmptyDeleteBackward = { [weak self] in
            guard let self, self.host.editorMode else { return }
            self.host.editorKey(Int32(OpKey_Backspace))
            self.host.requestImmediateFrame()
        }
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

    // MARK: - Page overlay

    // MARK: - Layout

    override func layoutSubviews() {
        super.layoutSubviews()
        guard !didTearDown, bounds.width > 0, bounds.height > 0 else { return }
        if convergeInitialViewportIfNeeded() {
            updateKeyboardOcclusionFromLayoutGuide()
            return
        }
        signalViewportConvergence(.layout)
        updateKeyboardOcclusionFromLayoutGuide()
    }

    override func safeAreaInsetsDidChange() {
        super.safeAreaInsetsDidChange()
        guard !didTearDown else { return }
        if convergeInitialViewportIfNeeded() {
            scheduleKeyboardOcclusionUpdate()
            return
        }
        signalViewportConvergence(.safeArea)
        scheduleKeyboardOcclusionUpdate()
    }

    private func signalViewportConvergence(_ signal: ViewportConvergence.Signal) {
        let sample = currentViewportSample()
        if geometryGate.observe(sample) {
            cancelTouchesForViewportTransition()
        }
        viewportConvergence.signal(signal, sample: sample)
        guard viewportDisplayLink == nil else { return }
        let link = CADisplayLink(target: self, selector: #selector(viewportDisplayFrame(_:)))
        link.add(to: .main, forMode: .common)
        viewportDisplayLink = link
    }

    /// The first surface attachment does not have an old responsive layout to
    /// protect, so use UIKit's current tuple immediately. Later transitions go
    /// through the epoch barrier below.
    private func convergeInitialViewportIfNeeded() -> Bool {
        guard host.engine == nil, bounds.width > 0, bounds.height > 0 else { return false }
        viewportDisplayLink?.invalidate()
        viewportDisplayLink = nil
        viewportConvergence.reset()
        let sample = currentViewportSample()
        let succeeded = convergeViewport(sample)
        geometryGate.commit(sample, succeeded: succeeded)
        return succeeded
    }

    @objc private func viewportDisplayFrame(_ link: CADisplayLink) {
        guard !didTearDown else {
            link.invalidate()
            viewportDisplayLink = nil
            return
        }
        let current = currentViewportSample()
        if geometryGate.observe(current) {
            cancelTouchesForViewportTransition()
        }
        guard let sample = viewportConvergence.displayFrame(sample: current) else { return }
        let succeeded = convergeViewport(sample)
        geometryGate.commit(sample, succeeded: succeeded)
        if succeeded {
            link.invalidate()
            viewportDisplayLink = nil
        } else {
            viewportConvergence.signal(.layout, sample: current)
        }
    }

    private func currentViewportSample() -> ViewportSample {
        let displayScale = window?.screen.scale ?? UIScreen.main.scale
        return ViewportSample(
            size: bounds.size,
            scale: displayScale,
            insets: ViewportInsets.clamped(
                top: safeAreaInsets.top,
                right: safeAreaInsets.right,
                bottom: safeAreaInsets.bottom,
                left: safeAreaInsets.left,
                to: bounds.size
            )
        )
    }

    private func convergeViewport(_ sample: ViewportSample) -> Bool {
        guard !didTearDown, sample.size.width > 0, sample.size.height > 0 else { return false }
        let surface = metalLayer
        contentScaleFactor = sample.scale
        surface.contentsScale = sample.scale
        surface.device = surface.device ?? MTLCreateSystemDefaultDevice()
        surface.pixelFormat = .bgra8Unorm
        surface.framebufferOnly = false
        surface.presentsWithTransaction = false
        surface.drawableSize = CGSize(
            width: (sample.size.width * sample.scale).rounded(),
            height: (sample.size.height * sample.scale).rounded()
        )

        return host.configure(
            surface: surface,
            logicalSize: sample.size,
            scale: sample.scale,
            safeArea: UIEdgeInsets(
                top: sample.insets.top,
                left: sample.insets.left,
                bottom: sample.insets.bottom,
                right: sample.insets.right
            )
        )
    }

    private func cancelTouchesForViewportTransition() {
        longPressWork?.cancel()
        suppressedTouches.suppress(touchIDs.keys)
        if host.editorMode {
            host.editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())
            resetEditorTouchTracking()
        } else {
            for (key, id) in touchIDs {
                guard let touch = activeTouch(for: key) else { continue }
                dispatch(touch, id: id, phase: Int32(OpPointerPhase_Cancel.rawValue))
            }
            touchIDs.removeAll()
            storedTouches.removeAll()
        }
    }

    override func didMoveToWindow() {
        super.didMoveToWindow()
        if window == nil, superview == nil {
            teardownEngine()
        } else {
            applySystemChromeStyle()
            setNeedsLayout()
        }
    }

    /// UIKit names the appearance that produces light system glyphs `.dark`
    /// and the appearance that produces dark glyphs `.light`.
    func updateSystemChrome(prefersLightIcons: Bool) {
        precondition(Thread.isMainThread)
        let next: UIUserInterfaceStyle = prefersLightIcons ? .dark : .light
        guard next != systemChromeStyle else { return }
        systemChromeStyle = next
        applySystemChromeStyle()
    }

    private func applySystemChromeStyle() {
        guard let window, let systemChromeStyle else { return }
        guard window.overrideUserInterfaceStyle != systemChromeStyle else { return }
        window.overrideUserInterfaceStyle = systemChromeStyle
        window.rootViewController?.setNeedsStatusBarAppearanceUpdate()
    }

    func teardownEngine() {
        guard !didTearDown else { return }
        didTearDown = true
        viewportDisplayLink?.invalidate()
        viewportDisplayLink = nil
        nativeLoginController?.finishForTeardown(animated: false)
        nativeLoginController = nil
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

    func nearestViewController() -> UIViewController? {
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
        if geometryGate.isPending || !suppressedTouches.isEmpty {
            suppressedTouches.suppress(touches.map(ObjectIdentifier.init))
            return
        }
        for touch in touches {
            let key = ObjectIdentifier(touch)
            guard !suppressedTouches.contains(key) else { continue }
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
        if geometryGate.isPending {
            suppressedTouches.suppress(touches.map(ObjectIdentifier.init))
            return
        }
        for touch in touches {
            let key = ObjectIdentifier(touch)
            guard !suppressedTouches.contains(key), let id = touchIDs[key] else { continue }
            if host.editorMode {
                editorTouchMoved(touch, key: ObjectIdentifier(touch))
            } else {
                dispatch(touch, id: id, phase: Int32(OpPointerPhase_Move.rawValue))
            }
        }
    }

    override func touchesEnded(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesEnded(touches, with: event)
        let keys = touches.map(ObjectIdentifier.init)
        if geometryGate.isPending || keys.contains(where: suppressedTouches.contains) {
            suppressedTouches.suppress(keys)
            suppressedTouches.finish(keys)
            return
        }
        if host.editorMode {
            for touch in touches {
                let key = ObjectIdentifier(touch)
                editorTouchEnded(touch, key: key)
                // editorTouchEnded may suppress a surviving finger while
                // ending a two-finger transform. If UIKit delivers both Ends
                // in this same batch, clear the key after its own terminal
                // event so no completed touch can poison the next clean Down.
                suppressedTouches.finish([key])
            }
        } else {
            finish(touches, phase: Int32(OpPointerPhase_Up.rawValue))
        }
    }

    override func touchesCancelled(_ touches: Set<UITouch>, with event: UIEvent?) {
        super.touchesCancelled(touches, with: event)
        let keys = touches.map(ObjectIdentifier.init)
        if geometryGate.isPending || keys.contains(where: suppressedTouches.contains) {
            suppressedTouches.finish(keys)
            return
        }
        longPressWork?.cancel()
        if host.editorMode {
            // UIKit cancellation is not a release: committing the host's
            // release ladder here can apply a drag/drop or a deferred tap
            // that the system explicitly interrupted. The cancelled set has
            // no trustworthy touch timestamp, so the Cancel carries the
            // frame pump's own monotonic domain.
            host.editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())
            resetEditorTouchTracking()
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
        host.dispatchPointer(
            id: id,
            phase: phase,
            point: touch.location(in: self),
            timeMs: touchTimestampMs(touch)
        )
    }

    /// `UITouch.timestamp` (seconds since the device boot) converted to
    /// the engine's millisecond clock. Same boot-uptime domain as the
    /// frame pump's `CACurrentMediaTime`, so touch events and frames
    /// share one monotonic timeline.
    private func touchTimestampMs(_ touch: UITouch) -> UInt64 {
        UInt64((touch.timestamp * 1_000).rounded(.down))
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
            host.editorPressAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))
        } else if touchIDs.count == 2 {
            // Second finger: pan + pinch take over. Track BOTH touches
            // (the primary was registered earlier, the new one now).
            longPressWork?.cancel()
            // The first finger has already entered the ordinary editor press
            // ladder. Cancel that capture before multi-touch starts so a
            // marquee/node drag cannot survive a two-finger gesture.
            host.editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())
            twoFingerActive = true
            pinchTouches.removeAll()
            for (candidateKey, _) in touchIDs {
                if let candidate = activeTouch(for: candidateKey) {
                    pinchTouches[candidateKey] = candidate
                }
            }
            updatePinchState()
            // Ownership is captured at second-finger Down, before any Move,
            // and remains stable if the midpoint later crosses a safe band.
            host.editorBeginTransform(x: lastMidpoint.x, y: lastMidpoint.y)
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
                let pinchDelta = PinchZoomDelta.wheelDelta(
                    previousDistance: previousDistance,
                    currentDistance: lastPinchDistance
                )
                if dx != 0 || dy != 0 {
                    host.editorPan(x: lastMidpoint.x, y: lastMidpoint.y, dx: dx, dy: dy)
                }
                if pinchDelta != 0 {
                    host.editorPinch(x: lastMidpoint.x, y: lastMidpoint.y, delta: pinchDelta)
                }
                host.requestImmediateFrame()
            }
            return
        }
        guard key == primaryTouchKey else { return }
        lastKnownPoint = point
        host.editorMoveAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))
        // Movement beyond the slop cancels the long-press candidate.
        if let work = longPressWork, !longPressFired {
            let slop = Self.editorLongPressSlop
            let deltaX = point.x - downPoint.x
            let deltaY = point.y - downPoint.y
            if deltaX * deltaX + deltaY * deltaY > slop * slop {
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
            // Ends the explicit direct-transform ownership captured above.
            host.editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())
            // The surviving finger belongs to the same physical two-finger
            // sequence. Suppress it through its terminal event so its final Up
            // cannot re-enter the single-finger release ladder. A later clean
            // Down starts the next editor gesture.
            suppressedTouches.suppress(touchIDs.keys)
            resetEditorTouchTracking()
            return
        }
        guard key == primaryTouchKey else { return }
        longPressWork?.cancel()
        longPressWork = nil
        if !longPressFired {
            host.editorReleaseAt(x: point.x, y: point.y, timeMs: touchTimestampMs(touch))
        }
        primaryTouchKey = nil
        longPressFired = false
        host.requestImmediateFrame()
    }

    private func resetEditorTouchTracking() {
        longPressWork?.cancel()
        longPressWork = nil
        primaryTouchKey = nil
        longPressFired = false
        twoFingerActive = false
        touchIDs.removeAll()
        storedTouches.removeAll()
        pinchTouches.removeAll()
        downPoint = .zero
        lastKnownPoint = .zero
        lastMidpoint = .zero
        lastPinchDistance = 0
    }

    private func armLongPress(at point: CGPoint) {
        longPressWork?.cancel()
        let work = DispatchWorkItem { [weak self] in
            guard let self, self.longPressWork != nil else { return }
            self.longPressFired = true
            // The Down at press time already ran the engine's press ladder,
            // so the engine's IME focus reflects THIS touch: focused means
            // the finger is holding an editable text field — offer Paste
            // instead of the right-click context menu.
            if self.presentPasteMenuIfEditingText(at: self.lastKnownPoint) {
                // The press capture opened at Down must not leak while the
                // release is suppressed by `longPressFired`.
                self.host.editorCancelGestureAt(timeMs: OpEngineHost.syntheticCancelNowMs())
            } else {
                self.host.editorRightPress(x: self.lastKnownPoint.x, y: self.lastKnownPoint.y)
            }
            self.host.requestImmediateFrame()
        }
        longPressWork = work
        DispatchQueue.main.asyncAfter(deadline: .now() + 0.5, execute: work)
    }

    /// Presents the system edit menu (Paste) at `point` when an engine text
    /// input holds the IME and the pasteboard has text. Returns `false` to
    /// let the caller fall back to the engine's long-press (right-click)
    /// path — including on iOS 15, which lacks `UIEditMenuInteraction`.
    private func presentPasteMenuIfEditingText(at point: CGPoint) -> Bool {
        guard host.editorMode,
              host.editorImeFocusedNow(),
              UIPasteboard.general.hasStrings
        else { return false }
        if #available(iOS 16.0, *),
           let interaction = editMenuInteraction as? UIEditMenuInteraction {
            let configuration = UIEditMenuConfiguration(identifier: nil, sourcePoint: point)
            interaction.presentEditMenu(with: configuration)
            return true
        }
        return false
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

    private func keyboardFrameDidChange(_ notification: Notification) {
        // The notification only schedules convergence. Its frame is in screen
        // coordinates and is easy to mis-convert for Stage Manager windows;
        // UIKeyboardLayoutGuide publishes the same transition directly in this
        // view's coordinate space. Defer one run-loop turn so UIKit can install
        // the guide's target geometry for the keyboard animation.
        _ = notification
        scheduleKeyboardOcclusionUpdate()
    }

    private func scheduleKeyboardOcclusionUpdate() {
        guard !didTearDown else { return }
        setNeedsLayout()
        DispatchQueue.main.async { [weak self] in
            guard let self, !self.didTearDown else { return }
            self.layoutIfNeeded()
            self.updateKeyboardOcclusionFromLayoutGuide()
        }
    }

    private func updateKeyboardOcclusionFromLayoutGuide() {
        guard !didTearDown else { return }
        let height = KeyboardOcclusion.bottomHeight(
            bounds: bounds,
            guideFrame: keyboardLayoutGuide.layoutFrame,
            safeAreaBottom: safeAreaInsets.bottom,
            firstResponder: imeTextView.isFirstResponder
        )
        host.updateKeyboardHeight(height)
    }
}


/// Long-press edit menu over engine-painted text inputs. The engine owns
/// the text, so the menu offers exactly the action the shell can deliver
/// through the ABI: Paste, forwarded via `op_editor_paste_text` into
/// whichever input holds the IME.
@available(iOS 16.0, *)
extension OpPlayerView: UIEditMenuInteractionDelegate {
    func editMenuInteraction(
        _ interaction: UIEditMenuInteraction,
        menuFor configuration: UIEditMenuConfiguration,
        suggestedActions: [UIMenuElement]
    ) -> UIMenu? {
        let paste = UIAction(
            title: NSLocalizedString("editMenu.paste", comment: "Edit-menu paste action")
        ) { [weak self] _ in
            guard let self, !self.didTearDown else { return }
            // Reading `.string` here is user-initiated (they tapped Paste),
            // so the system paste transparency notice is expected UX.
            guard let text = UIPasteboard.general.string, !text.isEmpty else { return }
            self.host.editorPasteText(text)
            self.host.requestImmediateFrame()
        }
        return UIMenu(children: [paste])
    }
}

/// IME conduit: a 1×1 offscreen text view whose only job is forwarding
/// keyboard/IME events to the engine. `deleteBackward` fires even when the
/// conduit is empty — the delegate path does not — so plain backspace is
/// forwarded from here. Chinese IMEs (讯飞/百度-style) delete through this
/// UIKeyInput entry without consulting shouldChangeTextIn at all, so this
/// override is the ONLY reliable delete path outside a composition.
final class ImeConduitTextView: UITextView {
    var onEmptyDeleteBackward: (() -> Void)?

    override func deleteBackward() {
        // While composing, the IME owns the edit: shrinking the marked
        // text flows back through textViewDidChange as a preedit update.
        if markedTextRange != nil {
            super.deleteBackward()
            return
        }
        // Outside a composition the conduit is kept empty and every
        // backspace belongs to the engine. Defensively drop any residual
        // conduit text first — an accumulated conduit would otherwise
        // swallow deletes into invisible text and the engine would never
        // hear the key.
        if !text.isEmpty {
            text = ""
        }
        onEmptyDeleteBackward?()
    }
}
