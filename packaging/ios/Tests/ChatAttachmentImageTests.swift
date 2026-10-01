import CoreGraphics
import Foundation
import ImageIO
import UniformTypeIdentifiers

@main
enum ChatAttachmentImageTests {
    static func main() {
        sniffsEveryEngineFormat()
        passesSmallEngineReadyImagesThrough()
        reencodesHeicOrOtherFormatsAsJpeg()
        shrinksOversizedImagesUnderTheCap()
        rejectsUndecodableBytes()
        sanitisesFileNames()
        print("ChatAttachmentImage tests passed")
    }

    private static func sniffsEveryEngineFormat() {
        precondition(ChatAttachmentImage.sniffMediaType(Data([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0])) == "image/png")
        precondition(ChatAttachmentImage.sniffMediaType(Data([0xFF, 0xD8, 0xFF, 0xE0])) == "image/jpeg")
        precondition(ChatAttachmentImage.sniffMediaType(Data("GIF89a..".utf8)) == "image/gif")
        precondition(ChatAttachmentImage.sniffMediaType(Data("RIFF\u{0}\u{0}\u{0}\u{0}WEBPVP8 ".utf8)) == "image/webp")
        precondition(ChatAttachmentImage.sniffMediaType(Data("....ftypheic".utf8)) == nil)
        precondition(ChatAttachmentImage.sniffMediaType(Data()) == nil)
    }

    private static func passesSmallEngineReadyImagesThrough() {
        let png = encode(makeImage(width: 32, height: 16, noise: false), type: .png)!
        let prepared = ChatAttachmentImage.prepare(png, suggestedName: "Screenshot 1.png")!
        precondition(prepared.data == png, "an engine-ready image must not be re-encoded")
        precondition(prepared.mediaType == "image/png")
        precondition(prepared.fileName == "Screenshot 1.png")
    }

    private static func reencodesHeicOrOtherFormatsAsJpeg() {
        let image = makeImage(width: 64, height: 32, noise: false)
        // HEIC encoding needs hardware support; TIFF exercises the same
        // "ImageIO can read it, the engine cannot" path everywhere.
        let source = encode(image, type: UTType.heic) ?? encode(image, type: .tiff)!
        precondition(ChatAttachmentImage.sniffMediaType(source) == nil)
        let prepared = ChatAttachmentImage.prepare(source, suggestedName: "IMG_0001.HEIC")!
        precondition(prepared.mediaType == "image/jpeg")
        precondition(ChatAttachmentImage.sniffMediaType(prepared.data) == "image/jpeg")
        precondition(prepared.fileName == "IMG_0001.jpg")
        let decoded = CGImageSourceCreateWithData(prepared.data as CFData, nil)
            .flatMap { CGImageSourceCreateImageAtIndex($0, 0, nil) }!
        precondition(decoded.width == 64 && decoded.height == 32, "small photos keep their size")
    }

    private static func shrinksOversizedImagesUnderTheCap() {
        let png = encode(makeImage(width: 2_600, height: 2_000, noise: true), type: .png)!
        precondition(png.count > ChatAttachmentImage.maximumBytes, "fixture must exceed the cap")
        let prepared = ChatAttachmentImage.prepare(png, suggestedName: nil)!
        precondition(prepared.data.count <= ChatAttachmentImage.maximumBytes)
        precondition(prepared.mediaType == "image/jpeg")
        precondition(prepared.fileName == "photo.jpg")
    }

    private static func rejectsUndecodableBytes() {
        precondition(ChatAttachmentImage.prepare(Data("not an image".utf8), suggestedName: "a.png") == nil)
        precondition(ChatAttachmentImage.prepare(Data(), suggestedName: nil) == nil)
    }

    private static func sanitisesFileNames() {
        precondition(ChatAttachmentImage.fileName("a/b\\c\n.webp", mediaType: "image/webp") == "abc.webp")
        precondition(ChatAttachmentImage.fileName("  ", mediaType: "image/gif") == "photo.gif")
        precondition(ChatAttachmentImage.fileName(".hidden", mediaType: "image/jpeg") == ".hidden.jpg")
        let long = String(repeating: "x", count: 300)
        precondition(ChatAttachmentImage.fileName(long, mediaType: "image/png").count == 104)
    }

    private static func makeImage(width: Int, height: Int, noise: Bool) -> CGImage {
        var pixels = [UInt8](repeating: 0, count: width * height * 4)
        var seed: UInt32 = 0x9E37_79B9
        for index in stride(from: 0, to: pixels.count, by: 4) {
            if noise {
                seed = seed &* 1_664_525 &+ 1_013_904_223
                pixels[index] = UInt8(truncatingIfNeeded: seed >> 24)
                pixels[index + 1] = UInt8(truncatingIfNeeded: seed >> 16)
                pixels[index + 2] = UInt8(truncatingIfNeeded: seed >> 8)
            } else {
                pixels[index] = 200
                pixels[index + 1] = 80
                pixels[index + 2] = 40
            }
            pixels[index + 3] = 255
        }
        let context = CGContext(
            data: &pixels,
            width: width,
            height: height,
            bitsPerComponent: 8,
            bytesPerRow: width * 4,
            space: CGColorSpaceCreateDeviceRGB(),
            bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue
        )!
        return context.makeImage()!
    }

    private static func encode(_ image: CGImage, type: UTType) -> Data? {
        let output = NSMutableData()
        guard let destination = CGImageDestinationCreateWithData(
            output,
            type.identifier as CFString,
            1,
            nil
        ) else { return nil }
        CGImageDestinationAddImage(destination, image, nil)
        guard CGImageDestinationFinalize(destination) else { return nil }
        return output as Data
    }
}
