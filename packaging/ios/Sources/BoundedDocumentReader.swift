import Foundation

/// Reads a picker URL without allowing provider metadata or a large file to
/// bypass the editor ABI's document-size limit.
enum BoundedDocumentReader {
    // Parsing temporarily retains the picker Data, the ABI UTF-8 copy, and
    // decoded JSON at the same time. Keep the shell cap well below the engine's
    // generic 256 MiB ABI ceiling so a valid input cannot exhaust a phone.
    static let maximumBytes = 32 * 1024 * 1024

    private static let readChunkBytes = 64 * 1024

    enum ReadError: LocalizedError {
        case tooLarge
        case outOfMemory
        case unavailable

        var errorDescription: String? {
            switch self {
            case .tooLarge:
                return "The selected document exceeds the 32 MiB mobile limit."
            case .outOfMemory:
                return "There is not enough memory to open the selected document."
            case .unavailable:
                return "The selected document could not be read."
            }
        }
    }

    static func read(from url: URL) throws -> Data {
        let reportedSize = (try? url.resourceValues(forKeys: [.fileSizeKey]))?.fileSize
        if let reportedSize, reportedSize > maximumBytes {
            throw ReadError.tooLarge
        }

        guard let stream = InputStream(url: url) else {
            throw ReadError.unavailable
        }

        return try read(stream: stream, reportedSize: reportedSize, byteLimit: maximumBytes)
    }

    /// Kept internal so the unknown-size and exact-limit paths can be tested
    /// with an in-memory stream without allocating a 32 MiB fixture.
    static func read(
        stream: InputStream,
        reportedSize: Int?,
        byteLimit: Int
    ) throws -> Data {
        precondition(byteLimit > 0)
        if let reportedSize, reportedSize > byteLimit {
            throw ReadError.tooLarge
        }

        let initialCapacity = min(
            max(reportedSize ?? readChunkBytes, readChunkBytes),
            byteLimit
        )
        guard var storage = malloc(initialCapacity) else {
            throw ReadError.outOfMemory
        }
        var ownsStorage = true
        defer {
            if ownsStorage { free(storage) }
        }

        stream.open()
        defer { stream.close() }

        var count = 0
        var capacity = initialCapacity
        while true {
            if count == capacity {
                if capacity == byteLimit {
                    var extraByte: UInt8 = 0
                    let extraCount = stream.read(&extraByte, maxLength: 1)
                    if extraCount < 0 {
                        throw stream.streamError ?? ReadError.unavailable
                    }
                    if extraCount > 0 {
                        throw ReadError.tooLarge
                    }
                    break
                }

                let nextCapacity = capacity <= byteLimit / 2
                    ? capacity * 2
                    : byteLimit
                guard let resized = realloc(storage, nextCapacity) else {
                    throw ReadError.outOfMemory
                }
                storage = resized
                capacity = nextCapacity
            }

            let destination = storage
                .advanced(by: count)
                .assumingMemoryBound(to: UInt8.self)
            let bytesRead = stream.read(destination, maxLength: capacity - count)
            if bytesRead < 0 {
                throw stream.streamError ?? ReadError.unavailable
            }
            if bytesRead == 0 { break }
            count += bytesRead
        }

        if count == 0 { return Data() }
        ownsStorage = false
        return Data(bytesNoCopy: storage, count: count, deallocator: .free)
    }
}
