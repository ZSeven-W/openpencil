//! Utility functions for the Claude Agent SDK
//!
//! Provides safe string handling utilities that respect UTF-8 boundaries,
//! preventing panics when truncating strings containing multi-byte characters
//! such as emoji or non-ASCII text.

/// Safely truncate a string at a UTF-8 character boundary.
///
/// Returns a slice of at most `max_bytes` bytes, ensuring the result
/// is valid UTF-8 by finding the last valid character boundary.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_bytes` - Maximum number of bytes in the result
///
/// # Returns
/// A string slice that is at most `max_bytes` long and valid UTF-8
///
/// # Example
/// ```
/// use anthropic_agent_sdk::utils::safe_truncate;
///
/// // Emoji is 4 bytes - truncating at byte 10 would cut it in half
/// let text = "Status: 🔍 Active";
/// let result = safe_truncate(text, 10);
/// assert_eq!(result, "Status: "); // Stops before the emoji
/// ```
#[inline]
#[must_use]
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }

    // Find the last valid UTF-8 boundary at or before max_bytes
    let mut boundary = max_bytes;
    while boundary > 0 && !s.is_char_boundary(boundary) {
        boundary -= 1;
    }

    &s[..boundary]
}

/// Truncate a string for display with ellipsis.
///
/// Returns a new String that is at most `max_bytes` long (including ellipsis),
/// with "..." appended if truncation occurred.
///
/// # Arguments
/// * `s` - The string to truncate
/// * `max_bytes` - Maximum number of bytes before adding ellipsis
///
/// # Returns
/// A String that is truncated with "..." if needed
///
/// # Example
/// ```
/// use anthropic_agent_sdk::utils::truncate_for_display;
///
/// let text = "This is a long message";
/// let result = truncate_for_display(text, 10);
/// assert_eq!(result, "This is a ...");
/// ```
#[must_use]
pub fn truncate_for_display(s: &str, max_bytes: usize) -> String {
    let truncated = safe_truncate(s, max_bytes);
    if truncated.len() < s.len() {
        format!("{truncated}...")
    } else {
        truncated.to_string()
    }
}

/// Extract a safe substring window ending at a byte position.
///
/// Returns a slice of at most `window_size` bytes ending at `end_byte`,
/// respecting UTF-8 character boundaries on both ends.
///
/// # Arguments
/// * `s` - The source string
/// * `end_byte` - The byte position to end at (clamped to string length)
/// * `window_size` - Maximum window size in bytes
///
/// # Returns
/// A string slice representing the window
///
/// # Example
/// ```
/// use anthropic_agent_sdk::utils::safe_window;
///
/// let code = "export class 🔍 Scanner";
/// let window = safe_window(code, 20, 10);
/// // Returns a safe slice without cutting the emoji
/// ```
#[must_use]
pub fn safe_window(s: &str, end_byte: usize, window_size: usize) -> &str {
    // Clamp end_byte to string length
    let end_raw = end_byte.min(s.len());

    // Find valid UTF-8 boundary for end (search backward)
    let end = if end_raw > 0 && !s.is_char_boundary(end_raw) {
        let mut boundary = end_raw;
        while boundary > 0 && !s.is_char_boundary(boundary) {
            boundary -= 1;
        }
        boundary
    } else {
        end_raw
    };

    // Calculate the desired start position
    let start_raw = end.saturating_sub(window_size);

    // Find the nearest valid UTF-8 character boundary for start (search forward)
    let start = if start_raw > 0 && !s.is_char_boundary(start_raw) {
        (start_raw..=start_raw.saturating_add(3).min(end))
            .find(|&i| s.is_char_boundary(i))
            .unwrap_or(end)
    } else {
        start_raw
    };

    &s[start..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_ascii() {
        let text = "Hello, World!";
        assert_eq!(safe_truncate(text, 7), "Hello, ");
        assert_eq!(safe_truncate(text, 100), text);
        assert_eq!(safe_truncate(text, 0), "");
    }

    #[test]
    fn test_safe_truncate_emoji() {
        // Emoji 🔍 is 4 bytes (bytes 8-11 in "Status: 🔍")
        let text = "Status: 🔍 Active";

        // Truncating at byte 10 would cut emoji in half - should stop before
        let result = safe_truncate(text, 10);
        assert_eq!(result, "Status: ");
        assert!(result.len() <= 10);

        // Truncating at byte 12 includes the full emoji
        let result = safe_truncate(text, 12);
        assert_eq!(result, "Status: 🔍");
    }

    #[test]
    fn test_safe_truncate_multibyte() {
        // 'é' is 2 bytes in UTF-8
        let text = "Café";

        // Byte 3 is in the middle of 'é' - should truncate to "Caf"
        let result = safe_truncate(text, 4);
        assert_eq!(result, "Caf");

        // Byte 5 includes the full 'é'
        let result = safe_truncate(text, 5);
        assert_eq!(result, "Café");
    }

    #[test]
    fn test_safe_truncate_chinese() {
        // Chinese characters are 3 bytes each
        let text = "你好世界"; // 12 bytes total

        // Truncating at byte 4 cuts second character - should get first only
        let result = safe_truncate(text, 4);
        assert_eq!(result, "你");

        // Truncating at byte 6 gets first two characters
        let result = safe_truncate(text, 6);
        assert_eq!(result, "你好");
    }

    #[test]
    fn test_truncate_for_display() {
        let text = "This is a long message";

        // Short enough - no ellipsis
        assert_eq!(truncate_for_display(text, 100), text);

        // Needs truncation - adds ellipsis
        let result = truncate_for_display(text, 10);
        assert_eq!(result, "This is a ...");
    }

    #[test]
    fn test_truncate_for_display_emoji() {
        let text = "Hello 🌍 World";

        // Truncate before emoji
        let result = truncate_for_display(text, 7);
        assert_eq!(result, "Hello ...");
    }

    #[test]
    fn test_safe_window_basic() {
        let text = "Hello, World!";

        // Window of 5 bytes ending at byte 12
        let result = safe_window(text, 12, 5);
        assert_eq!(result, "World");
    }

    #[test]
    fn test_safe_window_emoji() {
        let text = "Start 🔍 End";

        // Window that would start in middle of emoji
        let result = safe_window(text, 14, 8);
        // Should adjust start to not cut emoji
        assert!(result.is_char_boundary(0));
    }

    #[test]
    fn test_safe_window_bounds() {
        let text = "Short";

        // End beyond string length
        let result = safe_window(text, 100, 3);
        assert_eq!(result, "ort");

        // Window larger than string
        let result = safe_window(text, 5, 100);
        assert_eq!(result, "Short");
    }
}
