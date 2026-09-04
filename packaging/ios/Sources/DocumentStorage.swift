import Foundation

/// The user-visible directory saved `.op` documents land in.
///
/// `UIFileSharingEnabled` + `LSSupportsOpeningDocumentsInPlace` (both in
/// Info.plist) publish exactly this directory to the Files app under
/// "On My iPhone ▸ OpenPencil", so anything the engine writes here is
/// browsable, sharable, and openable outside the app. The private
/// Application Support root `AuthStorage.prepare()` returns stays what it
/// was built for — tokens and config — and stays excluded from backup.
enum DocumentStorage {
    /// Creates (if needed) and returns `NSDocumentDirectory`.
    ///
    /// Deliberately leaves the directory's backup resource value alone:
    /// these are the user's own design files, and the iCloud/iTunes backup
    /// they get by default is what a user expects for documents they can
    /// see. Only the private auth root opts out.
    static func prepare(fileManager: FileManager = .default) -> URL? {
        do {
            return try fileManager.url(
                for: .documentDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
        } catch {
            NSLog("OpenPencil could not prepare its documents directory: %@", error.localizedDescription)
            return nil
        }
    }
}
