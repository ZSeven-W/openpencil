import Foundation

enum AccountDeletionText {
    static var title: String { text("deletion.title", "Delete OpenPencil Account") }
    static var explanation: String { text("deletion.explanation", "Permanently delete your OpenPencil cloud account and data, and all designs and settings stored inside this app on this device. Your shared ZSeven account, wallet, and other ZSeven products will remain. Copies exported to other Files locations will remain. You will sign in on the secure confirmation page before deletion starts.") }
    static var confirm: String { text("deletion.confirm", "Continue to Confirmation") }
    static var checking: String { text("deletion.checking", "Checking deletion status…") }
    static var awaitingConfirmation: String { text("deletion.awaiting", "Open the secure page, sign in to the same ZSeven account, and confirm deleting only OpenPencil. Closing the page does not confirm deletion.") }
    static var pending: String { text("deletion.pending", "OpenPencil deletion is processing. This may take several minutes while active sessions expire. You can close this screen; status will be checked when the app is active again.") }
    static var cleaning: String { text("deletion.cleaning", "Cloud deletion is confirmed. Removing this app’s local OpenPencil data…") }
    static var completed: String { text("deletion.completed", "Your OpenPencil account and this app’s local data have been deleted. Close and reopen OpenPencil to start fresh. Your shared ZSeven account, wallet, other products, and exported copies remain.") }
    static var requestFailure: String { text("deletion.requestFailure", "Could not start deletion. Check your connection and try again. No local data was deleted.") }
    static var verificationFailure: String { text("deletion.verificationFailure", "Deletion completion could not be verified. Local data is preserved. Check your connection and try again.") }
    static var recoveryFailure: String { text("deletion.recoveryFailure", "Account deletion needs attention before the editor can open. Retry to finish cleanup, or close and reopen the app. Completion has not been reported.") }
    static var retry: String { text("deletion.retry", "Check Again") }
    static var openPage: String { text("deletion.openPage", "Open Secure Deletion Page") }
    static var renew: String { text("deletion.renew", "Restart Verification") }
    static var continueEditing: String { text("deletion.continueEditing", "Continue Editing") }

    private static func text(_ key: String, _ fallback: String) -> String {
        NSLocalizedString(key, value: fallback, comment: "OpenPencil account deletion")
    }
}
