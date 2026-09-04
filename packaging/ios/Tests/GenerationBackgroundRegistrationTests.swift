import Foundation

private func check(_ condition: Bool, _ message: String) {
    guard condition else {
        FileHandle.standardError.write(Data(
            "GenerationBackgroundRegistration test failed: \(message)\n".utf8
        ))
        exit(1)
    }
}

@main
enum GenerationBackgroundRegistrationTests {
    static func main() {
        uniqueTokensProduceUniqueIdentifiers()
        onlyTheCurrentMatchingHandlerCanStart()
        anOldHandlerCannotOccupyANewGeneration()
        pendingCancellationIsExactlyOnce()
        print("GenerationBackgroundRegistration tests passed")
    }

    static func uniqueTokensProduceUniqueIdentifiers() {
        let prefix = "tech.zseven.openpencil.generation."
        let first = GenerationBackgroundRegistration(token: "first", identifierPrefix: prefix)
        let second = GenerationBackgroundRegistration(token: "second", identifierPrefix: prefix)
        check(first.identifier != second.identifier, "each generation needs a unique identifier")
        check(first.identifier == prefix + "first", "identifier must preserve the wildcard prefix")
    }

    static func onlyTheCurrentMatchingHandlerCanStart() {
        let registration = GenerationBackgroundRegistration(token: "one", identifierPrefix: "op.")
        check(
            registration.startDecision(
                deliveredIdentifier: "op.other",
                isCurrentRegistration: true
            ) == .finish(success: false),
            "a mismatched task identifier must fail"
        )
        check(
            registration.startDecision(
                deliveredIdentifier: registration.identifier,
                isCurrentRegistration: false
            ) == .finish(success: false),
            "a noncurrent registration must fail"
        )
        check(
            registration.startDecision(
                deliveredIdentifier: registration.identifier,
                isCurrentRegistration: true
            ) == .accept,
            "the current matching registration may start"
        )
    }

    static func anOldHandlerCannotOccupyANewGeneration() {
        let old = GenerationBackgroundRegistration(token: "old", identifierPrefix: "op.")
        old.markSubmitted()
        check(old.finish(success: true), "finishing a pending old request must cancel it")

        let new = GenerationBackgroundRegistration(token: "new", identifierPrefix: "op.")
        check(
            old.startDecision(
                deliveredIdentifier: old.identifier,
                isCurrentRegistration: false
            ) == .finish(success: true),
            "a late old handler receives only its recorded terminal result"
        )
        check(new.completion == nil, "the new generation remains untouched")
    }

    static func pendingCancellationIsExactlyOnce() {
        let registration = GenerationBackgroundRegistration(token: "one", identifierPrefix: "op.")
        registration.markSubmitted()
        check(registration.finish(success: false), "first finish cancels the pending request")
        check(!registration.finish(success: true), "second finish must not cancel or overwrite")
        check(registration.completion == false, "the first terminal result wins")
    }
}
