import Foundation

@main
struct DocumentSaveWorkQueueTests {
    static func main() {
        let queue = DocumentSaveWorkQueue()
        let releaseWrite = DispatchSemaphore(value: 0)
        let writeFinished = DispatchSemaphore(value: 0)
        var completed = false
        precondition(queue.submit {
            releaseWrite.wait()
            writeFinished.signal()
        })
        let start = Date()
        queue.quiesce {
            precondition(Thread.isMainThread)
            precondition(writeFinished.wait(timeout: .now()) == .success)
            completed = true
        }
        precondition(Date().timeIntervalSince(start) < 0.2, "quiescing blocked the UI thread")
        precondition(!completed, "cleanup ran before the active save finished")
        precondition(!queue.submit { fatalError("late write admitted") })
        releaseWrite.signal()
        let deadline = Date().addingTimeInterval(3)
        while !completed && Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
        precondition(completed, "quiescence callback was not delivered")
        var repeated = false
        queue.quiesce { repeated = true }
        while !repeated && Date() < deadline {
            RunLoop.main.run(until: Date().addingTimeInterval(0.01))
        }
        precondition(repeated, "repeated teardown must still complete")
        print("DocumentSaveWorkQueue tests passed")
    }
}
