@main
enum ImeSelectionOffsetsTests {
    static func main() {
        check("ni", 0, 2, 0..<2)
        check("中文", 0, 2, 0..<6)
        check("中a文", 1, 2, 3..<4)
        check("😀中", 2, 3, 4..<7)

        // A defensive half-surrogate caret rounds back instead of producing
        // an invalid byte boundary for Rust string slicing.
        check("😀中", 1, 1, 0..<0)

        // Out-of-range and reversed UIKit positions are normalized.
        check("中文", 99, -4, 0..<6)
    }

    private static func check(
        _ text: String,
        _ utf16Start: Int,
        _ utf16End: Int,
        _ expected: Range<Int>
    ) {
        let actual = ImeSelectionOffsets.utf8Range(
            in: text,
            utf16Start: utf16Start,
            utf16End: utf16End
        )
        precondition(
            actual == expected,
            "\(text.debugDescription) \(utf16Start)..<\(utf16End): got \(actual), expected \(expected)"
        )
    }
}
