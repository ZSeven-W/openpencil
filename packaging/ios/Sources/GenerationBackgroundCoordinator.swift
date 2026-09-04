import BackgroundTasks
import Foundation
import UIKit

/// Keeps a user-started AI generation alive after UIKit backgrounds the app.
/// The engine stays suspended and only its render-free owner-thread pump runs.
final class GenerationBackgroundCoordinator {
    static let permittedIdentifier = "tech.zseven.openpencil.generation.*"
    private static let continuedIdentifierPrefix = "tech.zseven.openpencil.generation."

    private weak var host: OpEngineHost?
    private var state = GenerationBackgroundState()
    private var pumpTimer: DispatchSourceTimer?
    private var continuedRegistration: GenerationBackgroundRegistration?
    private var continuedTask: BGTask?
    private var applicationProtectionIdentifier = UIBackgroundTaskIdentifier.invalid
    private var applicationProtectionToken: String?

    init(host: OpEngineHost) {
        self.host = host
    }

    func observeEngineWork() {
        precondition(Thread.isMainThread)
        refreshWork(allowProtectionRequest: UIApplication.shared.applicationState != .background)
    }

    func didEnterBackground() {
        precondition(Thread.isMainThread)
        // Close a completion race before deciding whether to start the pump.
        refreshWork(allowProtectionRequest: false)
        apply(state.didEnterBackground())
    }

    func willEnterForeground() {
        precondition(Thread.isMainThread)
        // Stop render-free ticks before the host reattaches its Metal surface.
        apply(state.willEnterForeground())
    }

    func teardown() {
        precondition(Thread.isMainThread)
        apply(state.teardown())
    }

    private func refreshWork(allowProtectionRequest: Bool) {
        guard let host, let engine = host.engine else { return }
        var active = false
        let status = op_has_background_work(engine, &active)
        guard status == OpStatus_Ok else {
            host.reportFailure(status, operation: "op_has_background_work", engine: engine)
            return
        }
        apply(state.observeWork(
            active: active,
            allowProtectionRequest: allowProtectionRequest
        ))
    }

    private func apply(_ effects: [GenerationBackgroundEffect]) {
        for effect in effects {
            switch effect {
            case .requestProtection:
                requestProtection()
            case .startPump:
                startPump()
            case .stopPump:
                stopPump()
            case .cancelWork:
                cancelEngineWork()
            case let .finishProtection(success):
                finishProtection(success: success)
            }
        }
    }

    private func requestProtection() {
        precondition(Thread.isMainThread)
        let token = UUID().uuidString.lowercased()
        if #available(iOS 26.0, *) {
            requestContinuedProtection(token: token)
        } else {
            requestFallbackProtection(token: token)
        }
    }

    @available(iOS 26.0, *)
    private func requestContinuedProtection(token: String) {
        let registration = GenerationBackgroundRegistration(
            token: token,
            identifierPrefix: Self.continuedIdentifierPrefix
        )
        let scheduler = BGTaskScheduler.shared
        let registered = scheduler.register(
            forTaskWithIdentifier: registration.identifier,
            using: DispatchQueue.main
        ) { [weak self, registration] task in
            guard let self else {
                task.setTaskCompleted(success: registration.completion ?? false)
                return
            }
            self.handleContinuedTask(task, registration: registration)
        }
        guard registered else {
            _ = registration.finish(success: false)
            apply(state.protectionFailed())
            NSLog("Could not register iOS continued-processing task %@", registration.identifier)
            return
        }

        continuedRegistration = registration
        let request = BGContinuedProcessingTaskRequest(
            identifier: registration.identifier,
            title: NSLocalizedString("backgroundGeneration.title", comment: "Background generation title"),
            subtitle: NSLocalizedString(
                "backgroundGeneration.subtitle",
                comment: "Background generation subtitle"
            )
        )
        request.strategy = .fail
        // Default resources provide CPU and network access. Metal remains
        // detached, so this task deliberately never requests background GPU.
        do {
            try scheduler.submit(request)
            registration.markSubmitted()
        } catch {
            failContinuedProtection(registration: registration)
            NSLog("Could not submit iOS continued-processing task: %@", error.localizedDescription)
            return
        }

        // The scheduler handler is asynchronous on the main queue. A short
        // UIApplication grant bridges successful submission to handler
        // delivery if the user backgrounds immediately after pressing Send.
        guard startApplicationProtection(
            token: token,
            name: "OpenPencil continued-processing handoff"
        ) else {
            failContinuedProtection(registration: registration)
            NSLog("Could not acquire iOS continued-processing handoff")
            return
        }
    }

    @available(iOS 26.0, *)
    private func handleContinuedTask(
        _ deliveredTask: BGTask,
        registration: GenerationBackgroundRegistration
    ) {
        precondition(Thread.isMainThread)
        guard let task = deliveredTask as? BGContinuedProcessingTask else {
            deliveredTask.setTaskCompleted(success: false)
            if continuedRegistration === registration {
                failContinuedProtection(registration: registration)
            }
            return
        }
        let decision = registration.startDecision(
            deliveredIdentifier: task.identifier,
            isCurrentRegistration: continuedRegistration === registration && continuedTask == nil
        )
        guard decision == .accept else {
            if case let .finish(success) = decision {
                task.setTaskCompleted(success: success)
            }
            return
        }

        continuedTask = task
        // The engine has no honest denominator for a streaming/tool-driven
        // turn. Foundation explicitly represents that as indeterminate rather
        // than inventing a percentage from elapsed time or poll counts.
        task.progress.totalUnitCount = -1
        task.progress.completedUnitCount = 0
        task.expirationHandler = { [weak self, weak task, registration] in
            DispatchQueue.main.async {
                guard let self, let task else { return }
                self.expireContinuedTask(task, registration: registration)
            }
        }

        // The continued task now owns background runtime. End the finite
        // handoff before reconciling the already-running state machine.
        endApplicationProtection(token: registration.token)
        apply(state.protectionStarted())
    }

    private func expireContinuedTask(
        _ task: BGTask,
        registration: GenerationBackgroundRegistration
    ) {
        precondition(Thread.isMainThread)
        guard continuedTask === task, continuedRegistration === registration else { return }
        apply(state.protectionExpired())
    }

    private func failContinuedProtection(
        registration: GenerationBackgroundRegistration
    ) {
        guard continuedRegistration === registration else { return }
        // Stop any pump driven by the handoff before releasing that grant.
        apply(state.protectionFailed())
        endApplicationProtection(token: registration.token)
        continuedRegistration = nil
        if registration.finish(success: false) {
            BGTaskScheduler.shared.cancel(
                taskRequestWithIdentifier: registration.identifier
            )
        }
    }

    private func requestFallbackProtection(token: String) {
        if !startApplicationProtection(token: token, name: "OpenPencil generation") {
            apply(state.protectionFailed())
        }
    }

    @discardableResult
    private func startApplicationProtection(token: String, name: String) -> Bool {
        guard applicationProtectionIdentifier == .invalid else { return false }
        applicationProtectionToken = token
        let identifier = UIApplication.shared.beginBackgroundTask(withName: name) { [weak self] in
            DispatchQueue.main.async {
                self?.expireApplicationProtection(token: token)
            }
        }
        guard identifier != .invalid else {
            applicationProtectionToken = nil
            return false
        }
        applicationProtectionIdentifier = identifier
        apply(state.protectionStarted())
        return true
    }

    private func expireApplicationProtection(token: String) {
        precondition(Thread.isMainThread)
        guard applicationProtectionToken == token else { return }
        apply(state.protectionExpired())
    }

    private func endApplicationProtection(token: String? = nil) {
        if let token, applicationProtectionToken != token { return }
        let identifier = applicationProtectionIdentifier
        applicationProtectionIdentifier = .invalid
        applicationProtectionToken = nil
        if identifier != .invalid {
            UIApplication.shared.endBackgroundTask(identifier)
        }
    }

    private func startPump() {
        precondition(Thread.isMainThread)
        guard pumpTimer == nil else { return }
        let timer = DispatchSource.makeTimerSource(queue: .main)
        timer.schedule(
            deadline: .now(),
            repeating: .milliseconds(100),
            leeway: .milliseconds(25)
        )
        timer.setEventHandler { [weak self] in
            self?.pumpOnce()
        }
        pumpTimer = timer
        timer.resume()
    }

    private func stopPump() {
        precondition(Thread.isMainThread)
        guard let timer = pumpTimer else { return }
        pumpTimer = nil
        timer.setEventHandler {}
        timer.cancel()
    }

    private func pumpOnce() {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else {
            teardown()
            return
        }
        var active = false
        let status = op_background_tick(engine, OpEngineHost.nowMilliseconds(), &active)
        guard status == OpStatus_Ok else {
            host.reportFailure(status, operation: "op_background_tick", engine: engine)
            apply(state.protectionExpired())
            return
        }
        apply(state.observeWork(active: active, allowProtectionRequest: false))
    }

    private func cancelEngineWork() {
        precondition(Thread.isMainThread)
        guard let host, let engine = host.engine else { return }
        let status = op_cancel_background_work(engine)
        if status != OpStatus_Ok {
            host.reportFailure(status, operation: "op_cancel_background_work", engine: engine)
        } else {
            host.requestImmediateFrame()
        }
    }

    private func finishProtection(success: Bool) {
        precondition(Thread.isMainThread)
        if let registration = continuedRegistration {
            continuedRegistration = nil
            if registration.finish(success: success) {
                BGTaskScheduler.shared.cancel(
                    taskRequestWithIdentifier: registration.identifier
                )
            }
        }
        if let task = continuedTask {
            continuedTask = nil
            task.expirationHandler = nil
            if #available(iOS 26.0, *),
               success,
               let task = task as? BGContinuedProcessingTask
            {
                // A terminal 1/1 is truthful after indeterminate work ends.
                task.progress.totalUnitCount = 1
                task.progress.completedUnitCount = 1
            }
            task.setTaskCompleted(success: success)
        }
        endApplicationProtection()
    }
}
