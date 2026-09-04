enum GenerationBackgroundEffect: Equatable {
    case requestProtection
    case startPump
    case stopPump
    case cancelWork
    case finishProtection(success: Bool)
}

/// Pure lifecycle state for one user-started generation. Keeping UIKit and
/// BackgroundTasks out of this type makes the exactly-once transitions easy
/// to exercise without an iOS runtime.
struct GenerationBackgroundState {
    private enum Protection {
        case idle
        case requested
        case running
        case closed
    }

    private(set) var workActive = false
    private(set) var backgrounded = false
    private var protection = Protection.idle
    private var pumpRunning = false
    private var workCancellationSent = false
    private var terminated = false

    mutating func observeWork(
        active: Bool,
        allowProtectionRequest: Bool = true
    ) -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        workActive = active

        if !active {
            var effects = stopPumpIfNeeded()
            workCancellationSent = false
            switch protection {
            case .requested, .running:
                protection = .idle
                effects.append(.finishProtection(success: true))
            case .closed:
                protection = .idle
            case .idle:
                break
            }
            return effects
        }

        var effects: [GenerationBackgroundEffect] = []
        if protection == .idle, allowProtectionRequest {
            protection = .requested
            effects.append(.requestProtection)
        }
        effects.append(contentsOf: reconcilePump())
        return effects
    }

    mutating func protectionStarted() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [.finishProtection(success: false)] }
        switch protection {
        case .requested:
            protection = .running
            return reconcilePump()
        case .idle:
            return [.finishProtection(success: !workActive)]
        case .closed:
            return [.finishProtection(success: false)]
        case .running:
            return []
        }
    }

    mutating func protectionFailed() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        switch protection {
        case .requested, .running:
            protection = .closed
            return stopPumpIfNeeded()
        case .idle, .closed:
            return []
        }
    }

    mutating func didEnterBackground() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        backgrounded = true
        return reconcilePump()
    }

    mutating func willEnterForeground() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        backgrounded = false
        return stopPumpIfNeeded()
    }

    mutating func protectionExpired() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        switch protection {
        case .requested, .running:
            protection = .closed
            var effects = stopPumpIfNeeded()
            effects.append(contentsOf: cancelWorkIfNeeded())
            effects.append(.finishProtection(success: false))
            return effects
        case .idle, .closed:
            return []
        }
    }

    mutating func teardown() -> [GenerationBackgroundEffect] {
        guard !terminated else { return [] }
        terminated = true
        backgrounded = false
        var effects = stopPumpIfNeeded()
        effects.append(contentsOf: cancelWorkIfNeeded())
        workActive = false
        if protection == .requested || protection == .running {
            effects.append(.finishProtection(success: false))
        }
        protection = .closed
        return effects
    }

    private mutating func reconcilePump() -> [GenerationBackgroundEffect] {
        let shouldRun = workActive && backgrounded && protection == .running
        if shouldRun, !pumpRunning {
            pumpRunning = true
            return [.startPump]
        }
        if !shouldRun {
            return stopPumpIfNeeded()
        }
        return []
    }

    private mutating func stopPumpIfNeeded() -> [GenerationBackgroundEffect] {
        guard pumpRunning else { return [] }
        pumpRunning = false
        return [.stopPump]
    }

    private mutating func cancelWorkIfNeeded() -> [GenerationBackgroundEffect] {
        guard workActive, !workCancellationSent else { return [] }
        workCancellationSent = true
        return [.cancelWork]
    }
}
