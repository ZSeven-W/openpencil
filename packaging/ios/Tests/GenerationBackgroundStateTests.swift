import Foundation

private func check(
    _ actual: [GenerationBackgroundEffect],
    _ expected: [GenerationBackgroundEffect],
    _ message: String
) {
    guard actual == expected else {
        FileHandle.standardError.write(Data(
            "GenerationBackgroundState test failed: \(message); got \(actual), expected \(expected)\n".utf8
        ))
        exit(1)
    }
}

@main
enum GenerationBackgroundStateTests {
    static func main() {
        foregroundStartAndLifecycleTransitionsAreExactlyOnce()
        expirationClosesOnlyTheCurrentGeneration()
        expirationThenTeardownDoesNotCancelTwice()
        aLateSystemHandlerIsClosedImmediately()
        teardownStopsAndClosesOnce()
        backgroundDiscoveryDoesNotCreateAUserInitiatedLease()
        acquisitionFailureDoesNotRetryInALoop()
        aRunningProtectionFailureStopsOnlyThePump()
        print("GenerationBackgroundState tests passed")
    }

    static func foregroundStartAndLifecycleTransitionsAreExactlyOnce() {
        var state = GenerationBackgroundState()
        check(state.observeWork(active: true), [.requestProtection], "active work requests once")
        check(state.observeWork(active: true), [], "repeated active work does not request twice")
        check(state.protectionStarted(), [], "foreground protection does not start a pump")
        check(state.didEnterBackground(), [.startPump], "background starts the render-free pump")
        check(state.didEnterBackground(), [], "a repeated background event is harmless")
        check(state.willEnterForeground(), [.stopPump], "foreground stops the pump first")
        check(state.willEnterForeground(), [], "a repeated foreground event is harmless")
        check(state.didEnterBackground(), [.startPump], "the same active task can background again")
        check(
            state.observeWork(active: false),
            [.stopPump, .finishProtection(success: true)],
            "completion stops the pump before releasing protection"
        )
        check(state.observeWork(active: false), [], "completion is emitted once")
        check(state.observeWork(active: true), [.requestProtection], "a new generation gets a new lease")
    }

    static func expirationClosesOnlyTheCurrentGeneration() {
        var state = GenerationBackgroundState()
        check(state.observeWork(active: true), [.requestProtection], "generation requests protection")
        check(state.didEnterBackground(), [], "a pending lease cannot pump")
        check(state.protectionStarted(), [.startPump], "a granted lease starts in background")
        check(
            state.protectionExpired(),
            [.stopPump, .cancelWork, .finishProtection(success: false)],
            "expiration stops the pump, cancels work, then reports failure"
        )
        check(state.protectionExpired(), [], "expiration is handled once")
        check(state.observeWork(active: true), [], "expired active work does not spin on requests")
        check(state.observeWork(active: false), [], "idle observation resets the closed cycle")
        check(state.observeWork(active: true), [.requestProtection], "the next generation may request")
    }

    static func aLateSystemHandlerIsClosedImmediately() {
        var state = GenerationBackgroundState()
        check(state.observeWork(active: true), [.requestProtection], "generation requests protection")
        check(
            state.observeWork(active: false),
            [.finishProtection(success: true)],
            "completion cancels a pending request"
        )
        check(
            state.protectionStarted(),
            [.finishProtection(success: true)],
            "a raced late task receives terminal success"
        )
    }

    static func expirationThenTeardownDoesNotCancelTwice() {
        var state = GenerationBackgroundState()
        _ = state.observeWork(active: true)
        _ = state.protectionStarted()
        _ = state.didEnterBackground()
        _ = state.protectionExpired()
        check(state.teardown(), [], "teardown does not repeat expiration cleanup")
    }

    static func teardownStopsAndClosesOnce() {
        var state = GenerationBackgroundState()
        _ = state.observeWork(active: true)
        _ = state.protectionStarted()
        _ = state.didEnterBackground()
        check(
            state.teardown(),
            [.stopPump, .cancelWork, .finishProtection(success: false)],
            "teardown stops the timer and work before closing the lease"
        )
        check(state.teardown(), [], "teardown is idempotent")
        check(state.willEnterForeground(), [], "terminal state ignores lifecycle events")
        check(state.observeWork(active: true), [], "terminal state never restarts work")

        var idle = GenerationBackgroundState()
        check(idle.teardown(), [], "idle teardown has no generation to cancel")
        check(idle.teardown(), [], "idle teardown remains idempotent")
    }

    static func backgroundDiscoveryDoesNotCreateAUserInitiatedLease() {
        var state = GenerationBackgroundState()
        check(state.didEnterBackground(), [], "idle backgrounding does nothing")
        check(
            state.observeWork(active: true, allowProtectionRequest: false),
            [],
            "work discovered after backgrounding cannot submit a foreground-only request"
        )
        check(state.willEnterForeground(), [], "foreground recovery needs no timer")
        check(
            state.observeWork(active: true),
            [.requestProtection],
            "foreground recovery may protect the still-running user generation"
        )
    }

    static func acquisitionFailureDoesNotRetryInALoop() {
        var state = GenerationBackgroundState()
        _ = state.observeWork(active: true)
        check(state.protectionFailed(), [], "a failed acquisition closes the cycle")
        check(state.observeWork(active: true), [], "active polling does not retry a failed lease")
        check(state.observeWork(active: false), [], "completion resets the failed cycle")
        check(state.observeWork(active: true), [.requestProtection], "a later generation retries")
    }

    static func aRunningProtectionFailureStopsOnlyThePump() {
        var state = GenerationBackgroundState()
        _ = state.observeWork(active: true)
        _ = state.protectionStarted()
        check(state.didEnterBackground(), [.startPump], "handoff may pump while backgrounded")
        check(
            state.protectionFailed(),
            [.stopPump],
            "handoff failure stops its pump without treating it as system cancellation"
        )
        check(state.protectionFailed(), [], "a repeated handoff failure is harmless")
        check(state.observeWork(active: true), [], "failed active work cannot reacquire in a loop")
        check(state.observeWork(active: false), [], "completion resets the failed handoff")
        check(state.observeWork(active: true), [.requestProtection], "a later generation may retry")
    }
}
