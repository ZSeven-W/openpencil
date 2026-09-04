/// Converts UIKit's UTF-16 text positions into the UTF-8 byte offsets used by
/// the Rust editor ABI. UIKit can report a caret between any two UTF-16 code
/// units; malformed half-surrogate positions are rounded back to the nearest
/// valid Swift string boundary before the byte count is measured.
enum ImeSelectionOffsets {
    static func utf8Range(
        in text: String,
        utf16Start: Int,
        utf16End: Int
    ) -> Range<Int> {
        let lower = utf8Offset(in: text, utf16Offset: min(utf16Start, utf16End))
        let upper = utf8Offset(in: text, utf16Offset: max(utf16Start, utf16End))
        return lower..<upper
    }

    private static func utf8Offset(in text: String, utf16Offset: Int) -> Int {
        let utf16 = text.utf16
        var clamped = max(0, min(utf16Offset, utf16.count))
        while clamped > 0 {
            let utf16Index = utf16.index(utf16.startIndex, offsetBy: clamped)
            if let stringIndex = String.Index(utf16Index, within: text) {
                return text[..<stringIndex].utf8.count
            }
            clamped -= 1
        }
        return 0
    }
}
