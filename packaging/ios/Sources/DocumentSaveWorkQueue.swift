import Foundation

/// Main-thread admissions plus a serial worker. Quiescing closes admissions
/// immediately and calls back on the main thread after every admitted write.
final class DocumentSaveWorkQueue {
    private let queue = DispatchQueue(label: "tech.zseven.openpencil.save", qos: .userInitiated)
    private var accepting = true

    @discardableResult
    func submit(_ operation: @escaping () -> Void) -> Bool {
        precondition(Thread.isMainThread)
        guard accepting else { return false }
        queue.async(execute: operation)
        return true
    }

    func quiesce(completion: @escaping () -> Void) {
        precondition(Thread.isMainThread)
        accepting = false
        queue.async { DispatchQueue.main.async(execute: completion) }
    }
}
