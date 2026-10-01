import Foundation
import ImageIO
import UniformTypeIdentifiers

/// Platform-free preparation of one photo-picker image for
/// `op_editor_attach_chat_image`.
///
/// The engine sniffs PNG, JPEG, GIF, and WebP magic bytes and rejects
/// anything above 5 MiB. Photos from the library are frequently HEIC (or a
/// huge camera JPEG), so the shell re-encodes those to JPEG — applying the
/// EXIF orientation and shrinking the longest edge until the payload fits —
/// before the bytes cross the ABI. Engine-ready images pass through
/// untouched. Pure Foundation / ImageIO so the rules are unit-tested on the
/// host without UIKit.
enum ChatAttachmentImage {
    /// Mirrors `op_editor_core::chat::MAX_ATTACHMENT_BYTES`.
    static let maximumBytes = 5 * 1024 * 1024
    /// Longest edge tried first when a photo has to be re-encoded; a chat
    /// attachment is a model input, not an archive copy.
    static let initialMaxPixelSize = 4096
    private static let minimumMaxPixelSize = 512
    private static let jpegQuality = 0.85

    struct Prepared: Equatable {
        let data: Data
        let mediaType: String
        let fileName: String
    }

    /// The engine-recognised media type of `data`, from its magic bytes.
    static func sniffMediaType(_ data: Data) -> String? {
        let bytes = [UInt8](data.prefix(12))
        if bytes.starts(with: [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
            return "image/png"
        }
        if bytes.starts(with: [0xFF, 0xD8, 0xFF]) {
            return "image/jpeg"
        }
        if bytes.starts(with: Array("GIF87a".utf8)) || bytes.starts(with: Array("GIF89a".utf8)) {
            return "image/gif"
        }
        if bytes.count >= 12,
           bytes[0..<4].elementsEqual("RIFF".utf8),
           bytes[8..<12].elementsEqual("WEBP".utf8) {
            return "image/webp"
        }
        return nil
    }

    /// Returns engine-ready bytes, or nil when the image cannot be decoded or
    /// cannot be brought under the size cap.
    static func prepare(_ data: Data, suggestedName: String?) -> Prepared? {
        if let mediaType = sniffMediaType(data), data.count <= maximumBytes {
            return Prepared(
                data: data,
                mediaType: mediaType,
                fileName: fileName(suggestedName, mediaType: mediaType)
            )
        }
        guard let jpeg = reencodeAsJPEG(data) else { return nil }
        return Prepared(
            data: jpeg,
            mediaType: "image/jpeg",
            fileName: fileName(suggestedName, mediaType: "image/jpeg")
        )
    }

    /// Decodes any ImageIO-readable image (HEIC, TIFF, oversized JPEG, …)
    /// and encodes it as an orientation-corrected JPEG under the cap.
    static func reencodeAsJPEG(_ data: Data) -> Data? {
        guard let source = CGImageSourceCreateWithData(data as CFData, nil),
              CGImageSourceGetCount(source) > 0
        else { return nil }
        var maxPixelSize = initialMaxPixelSize
        while maxPixelSize >= minimumMaxPixelSize {
            let options: [CFString: Any] = [
                kCGImageSourceCreateThumbnailFromImageAlways: true,
                kCGImageSourceCreateThumbnailWithTransform: true,
                kCGImageSourceShouldCacheImmediately: true,
                kCGImageSourceThumbnailMaxPixelSize: maxPixelSize,
            ]
            guard let image = CGImageSourceCreateThumbnailAtIndex(source, 0, options as CFDictionary),
                  let encoded = encodeJPEG(image)
            else { return nil }
            if encoded.count <= maximumBytes {
                return encoded
            }
            maxPixelSize = maxPixelSize * 3 / 4
        }
        return nil
    }

    private static func encodeJPEG(_ image: CGImage) -> Data? {
        let output = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(
            output,
            UTType.jpeg.identifier as CFString,
            1,
            nil
        ) else { return nil }
        let properties: [CFString: Any] = [kCGImageDestinationLossyCompressionQuality: jpegQuality]
        CGImageDestinationAddImage(destination, image, properties as CFDictionary)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return output as Data
    }

    /// A display name the engine accepts: no path separators or control
    /// characters, and an extension that matches the bytes actually sent.
    static func fileName(_ suggested: String?, mediaType: String) -> String {
        let cleaned = (suggested ?? "")
            .unicodeScalars
            .filter { !CharacterSet.controlCharacters.contains($0) && $0 != "/" && $0 != "\\" }
        var stem = String(String.UnicodeScalarView(cleaned))
            .trimmingCharacters(in: .whitespacesAndNewlines)
        if let dot = stem.lastIndex(of: "."), dot != stem.startIndex {
            stem = String(stem[..<dot])
        }
        if stem.isEmpty {
            stem = "photo"
        }
        let fileExtension: String
        switch mediaType {
        case "image/jpeg": fileExtension = "jpg"
        case "image/gif": fileExtension = "gif"
        case "image/webp": fileExtension = "webp"
        default: fileExtension = "png"
        }
        return "\(String(stem.prefix(100))).\(fileExtension)"
    }
}
