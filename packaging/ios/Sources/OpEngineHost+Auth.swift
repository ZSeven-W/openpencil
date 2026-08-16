import UIKit

/// SSO glue: regional auth configuration, the native-login URL bridge, and
/// the account-center snapshot/sign-out calls. Split from `OpEngineHost.swift`
/// purely for the 800-line file cap.
extension OpEngineHost {
    /// Applies the user's persisted engine locale after engine creation.
    func applyPersistedLocale(engine: OpaquePointer) {
        guard let code = EngineLanguage.storedPreference else { return }
        let bytes = Data(code.utf8)
        let status = bytes.withUnsafeBytes { raw in
            op_editor_set_locale(
                engine,
                raw.bindMemory(to: UInt8.self).baseAddress,
                raw.count
            )
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_set_locale", engine: engine)
        }
    }

    /// Native 15-language sheet for the editor chrome; the engine applies
    /// the choice immediately and the preference re-applies on launch.
    func presentLanguagePicker() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode,
              let view, let presenter = view.nearestViewController() else { return }
        let current = currentLocaleCode(engine: engine)
        let sheet = UIAlertController(
            title: NSLocalizedString(
                "language.pickerTitle",
                value: "Language",
                comment: "Language picker title"
            ),
            message: nil,
            preferredStyle: .actionSheet
        )
        for entry in EngineLanguage.all {
            let title = entry.code == current ? "✓ \(entry.name)" : entry.name
            sheet.addAction(UIAlertAction(title: title, style: .default) { [weak self] _ in
                guard let self, let engine = self.engine else { return }
                let bytes = Data(entry.code.utf8)
                let status = bytes.withUnsafeBytes { raw in
                    op_editor_set_locale(
                        engine,
                        raw.bindMemory(to: UInt8.self).baseAddress,
                        raw.count
                    )
                }
                if status == OpStatus_Ok {
                    EngineLanguage.savePreference(entry.code)
                }
                self.requestImmediateFrame()
            })
        }
        sheet.addAction(UIAlertAction(
            title: NSLocalizedString("common.cancel", value: "Cancel", comment: "Cancel"),
            style: .cancel
        ))
        sheet.popoverPresentationController?.sourceView = view
        presenter.present(sheet, animated: true)
    }

    private func currentLocaleCode(engine: OpaquePointer) -> String? {
        var required = 0
        guard
            op_editor_locale_code(engine, nil, 0, &required) == OpStatus_Ok,
            required > 0, required <= 64
        else { return nil }
        var bytes = [UInt8](repeating: 0, count: required)
        let status = bytes.withUnsafeMutableBufferPointer { buffer in
            op_editor_locale_code(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard status == OpStatus_Ok else { return nil }
        return String(decoding: bytes.prefix(required), as: UTF8.self)
    }

    /// Installs the mobile auth runtime only when secure local storage can be
    /// prepared. `NotReady` means this build has no compatible auth archive;
    /// the Rust UI remains in its honest unavailable/stub state.
    func configureMobileAuth(engine: OpaquePointer, storageURL: URL) {
        // Lazy region lock: the proprietary runtime initializes once per
        // process, so a fresh install (no persisted credential) defers
        // configuration until the first sign-in — the login region then
        // reflects the latest IP verdict and any user switch made before
        // that first tap. Returning users configure at startup so their
        // session restores immediately.
        guard AuthStorage.hasPersistedCredential(at: storageURL) else {
            SsoRegionStore.refreshDetectedRegionAsync()
            return
        }
        // The region comes from the gateway's own IP verdict: a stored
        // answer resolves immediately, a first launch waits for the probe.
        SsoRegionStore.resolveForStartup { [weak self] region in
            guard let self, self.engine == engine else { return }
            self.configureMobileAuthNow(engine: engine, storageURL: storageURL, region: region)
        }
    }

    /// `RequestLogin` shell action: configure the auth runtime for the
    /// resolved region if this process has not yet, then start the device
    /// flow. Unavailability (stub build) surfaces as a native alert — the
    /// engine-painted login modal never opens on touch chrome.
    func startLoginFlow() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        if authConfigured {
            beginLoginNow(engine: engine)
            return
        }
        guard let storageURL = authStorageURL ?? AuthStorage.prepare() else {
            presentLoginUnavailable()
            return
        }
        SsoRegionStore.resolveForStartup { [weak self] region in
            guard let self, self.engine == engine else { return }
            self.configureMobileAuthNow(engine: engine, storageURL: storageURL, region: region)
            self.beginLoginNow(engine: engine)
        }
    }

    private func beginLoginNow(engine: OpaquePointer) {
        let status = op_editor_begin_login(engine)
        if status == OpStatus_NotReady {
            presentLoginUnavailable()
            return
        }
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_begin_login", engine: engine)
        }
        requestImmediateFrame()
    }

    private func presentLoginUnavailable() {
        guard let view, let presenter = view.nearestViewController() else { return }
        let alert = UIAlertController(
            title: NSLocalizedString(
                "nativeLogin.unavailableTitle",
                value: "Sign-in unavailable",
                comment: "Auth unavailable"
            ),
            message: NSLocalizedString(
                "nativeLogin.unavailableBody",
                value: "This build has no sign-in backend.",
                comment: "Auth unavailable detail"
            ),
            preferredStyle: .alert
        )
        alert.addAction(UIAlertAction(title: "OK", style: .default))
        presenter.present(alert, animated: true)
    }

    private func configureMobileAuthNow(
        engine: OpaquePointer,
        storageURL: URL,
        region: SsoRegion
    ) {
        let deviceName = UIDevice.current.name
        let appVersion = Bundle.main.object(
            forInfoDictionaryKey: "CFBundleShortVersionString"
        ) as? String ?? "unknown"
        let storage = Data(storageURL.path.utf8)
        let device = Data(deviceName.utf8)
        let version = Data(appVersion.utf8)
        guard !storage.isEmpty, !device.isEmpty, !version.isEmpty else { return }
        let status = storage.withUnsafeBytes { storageBytes in
            device.withUnsafeBytes { deviceBytes in
                version.withUnsafeBytes { versionBytes in
                    op_editor_configure_auth(
                        engine,
                        storageBytes.bindMemory(to: UInt8.self).baseAddress,
                        storageBytes.count,
                        deviceBytes.bindMemory(to: UInt8.self).baseAddress,
                        deviceBytes.count,
                        versionBytes.bindMemory(to: UInt8.self).baseAddress,
                        versionBytes.count,
                        region.authRegionCode
                    )
                }
            }
        }
        if status == OpStatus_Ok {
            authConfigured = true
        } else if status != OpStatus_NotReady {
            reportFailure(status, operation: "op_editor_configure_auth", engine: engine)
        }
        requestImmediateFrame()
    }

    /// Copies the borrowed Rust string into Swift-owned storage before any
    /// other engine call. The first query obtains the exact required size;
    /// the second call fills the caller-owned buffer.
    func copyLoginURL(engine: OpaquePointer) -> URL? {
        var required = 0
        guard
            op_editor_copy_login_url(engine, nil, 0, &required) == OpStatus_Ok,
            required > 0,
            required <= 16 * 1024
        else { return nil }
        var bytes = [UInt8](repeating: 0, count: required)
        let status = bytes.withUnsafeMutableBufferPointer { buffer in
            op_editor_copy_login_url(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard status == OpStatus_Ok, required <= bytes.count else { return nil }
        let text = String(decoding: bytes.prefix(required), as: UTF8.self)
        return URL(string: text)
    }

    /// User-originated close path. This runs on the engine's owner/main thread,
    /// outside the editor ABI call that originally queued the shell action.
    func cancelLoginFlow() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        let status = op_editor_cancel_login(engine)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_cancel_login", engine: engine)
        }
        requestImmediateFrame()
    }

    /// Decodes the engine's JSON account snapshot for the native account
    /// center. Two-phase copy mirroring the login-URL bridge.
    func accountSnapshot(engine: OpaquePointer) -> AccountSnapshot? {
        var required = 0
        guard
            op_editor_account_snapshot(engine, nil, 0, &required) == OpStatus_Ok,
            required > 0,
            required <= 64 * 1024
        else { return nil }
        var bytes = [UInt8](repeating: 0, count: required)
        let status = bytes.withUnsafeMutableBufferPointer { buffer in
            op_editor_account_snapshot(engine, buffer.baseAddress, buffer.count, &required)
        }
        guard status == OpStatus_Ok, required <= bytes.count else { return nil }
        return try? JSONDecoder().decode(
            AccountSnapshot.self,
            from: Data(bytes.prefix(required))
        )
    }

    /// Revokes the device session from the native account center.
    func signOutAccount() {
        precondition(Thread.isMainThread)
        guard let engine, editorMode else { return }
        let status = op_editor_auth_sign_out(engine)
        if status != OpStatus_Ok && status != OpStatus_Suspended {
            reportFailure(status, operation: "op_editor_auth_sign_out", engine: engine)
        }
        requestImmediateFrame()
    }
}
