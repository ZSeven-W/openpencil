import Foundation

@main
enum BoundedDocumentReaderTests {
    static func main() throws {
        let exactLimit = Data([0, 1, 2, 3, 4, 5, 6, 7])
        let exactRead = try BoundedDocumentReader.read(
            stream: InputStream(data: exactLimit),
            reportedSize: nil,
            byteLimit: exactLimit.count
        )
        precondition(exactRead == exactLimit)

        expectTooLarge(reportedSize: exactLimit.count + 1, data: exactLimit)
        expectTooLarge(reportedSize: nil, data: exactLimit + Data([8]))
    }

    private static func expectTooLarge(reportedSize: Int?, data: Data) {
        do {
            _ = try BoundedDocumentReader.read(
                stream: InputStream(data: data),
                reportedSize: reportedSize,
                byteLimit: 8
            )
            preconditionFailure("Expected the bounded reader to reject the document")
        } catch BoundedDocumentReader.ReadError.tooLarge {
            // Expected.
        } catch {
            preconditionFailure("Unexpected bounded-reader error: \(error)")
        }
    }
}
