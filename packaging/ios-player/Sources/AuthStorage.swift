import Foundation

enum AuthStorage {
    /// Creates the per-app auth directory with an iOS data-protection class
    /// that remains usable after the user's first unlock. Tokens are local
    /// runtime state and must never enter iCloud/iTunes device backups.
    static func prepare(fileManager: FileManager = .default) -> URL? {
        do {
            let applicationSupport = try fileManager.url(
                for: .applicationSupportDirectory,
                in: .userDomainMask,
                appropriateFor: nil,
                create: true
            )
            let directory = applicationSupport
                .appendingPathComponent("OpenPencil", isDirectory: true)
                .appendingPathComponent("Auth", isDirectory: true)
            try fileManager.createDirectory(
                at: directory,
                withIntermediateDirectories: true,
                attributes: [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication]
            )
            var resourceValues = URLResourceValues()
            resourceValues.isExcludedFromBackup = true
            var mutableDirectory = directory
            try mutableDirectory.setResourceValues(resourceValues)
            try fileManager.setAttributes(
                [.protectionKey: FileProtectionType.completeUntilFirstUserAuthentication],
                ofItemAtPath: directory.path
            )
            return directory
        } catch {
            NSLog("OpenPencil could not prepare auth storage: %@", error.localizedDescription)
            return nil
        }
    }
}
