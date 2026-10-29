import SwiftUI
import SafariServices

/// No engine exists behind this screen until pending cleanup has succeeded.
struct AccountDeletionRecoveryView: View {
    @ObservedObject var coordinator: AccountDeletionCoordinator
    @State private var verificationURL: URL?
    @State private var showsVerification = false

    var body: some View {
        ScrollView {
            VStack(spacing: 24) {
                Text(AccountDeletionText.title).font(.title2).bold()
                Text(coordinator.message).multilineTextAlignment(.center)
                if coordinator.busy || coordinator.gate == .checking {
                    ProgressView()
                } else if coordinator.gate == .recovery {
                    Button(AccountDeletionText.retry) { coordinator.retry() }
                        .buttonStyle(.borderedProminent)
                    if coordinator.canRenewRequest {
                        Button(AccountDeletionText.renew) {
                            coordinator.restartVerification { url in
                                verificationURL = url
                                showsVerification = url != nil
                            }
                        }
                    }
                    if coordinator.receipt?.phase == .awaitingServer {
                        Button(AccountDeletionText.continueEditing) { coordinator.continueEditing() }
                    }
                }
            }
            .padding(32)
        }
        .sheet(isPresented: $showsVerification, onDismiss: { coordinator.retry() }) {
            if let url = verificationURL { AccountDeletionBrowser(url: url) }
        }
    }
}

private struct AccountDeletionBrowser: UIViewControllerRepresentable {
    let url: URL

    func makeUIViewController(context: Context) -> SFSafariViewController {
        SFSafariViewController(url: url)
    }

    func updateUIViewController(_ controller: SFSafariViewController, context: Context) {}
}
