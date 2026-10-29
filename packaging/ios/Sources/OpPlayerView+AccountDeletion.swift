import UIKit

extension OpPlayerView {
    /// Only called after a verified server receipt has been saved as pending
    /// local cleanup. Teardown persists final engine state before it is wiped.
    func prepareForAccountDeletion(completion: @escaping (Bool) -> Void) {
        precondition(Thread.isMainThread)
        isUserInteractionEnabled = false
        imeTextView.resignFirstResponder()
        nearestViewController()?.dismiss(animated: false)
        if !didTearDown {
            if let engine = host.engine { _ = op_cancel_background_work(engine) }
            host.signOutAccount()
        }
        host.quiesceDocumentSaves { [self] in
            teardownEngine()
            completion(host.teardownSucceeded)
        }
    }
}
