/// Per-generation identity retained by the system launch-handler closure.
/// The closure owns only this small value object and a weak coordinator; a
/// late handler can therefore finish itself without touching a newer turn.
final class GenerationBackgroundRegistration {
    enum StartDecision: Equatable {
        case accept
        case finish(success: Bool)
    }

    let token: String
    let identifier: String
    private(set) var submitted = false
    private(set) var completion: Bool?

    init(token: String, identifierPrefix: String) {
        self.token = token
        identifier = identifierPrefix + token
    }

    func markSubmitted() {
        guard completion == nil else { return }
        submitted = true
    }

    func startDecision(
        deliveredIdentifier: String,
        isCurrentRegistration: Bool
    ) -> StartDecision {
        if let completion {
            return .finish(success: completion)
        }
        guard isCurrentRegistration, deliveredIdentifier == identifier else {
            return .finish(success: false)
        }
        submitted = false
        return .accept
    }

    /// Records the first terminal result and returns whether a still-pending
    /// scheduler request has to be cancelled.
    func finish(success: Bool) -> Bool {
        guard completion == nil else { return false }
        let shouldCancelRequest = submitted
        submitted = false
        completion = success
        return shouldCancelRequest
    }
}
