/// Display width of one character, in wrap *units*. Wide scripts
/// (CJK, kana, Hangul, full-width punctuation, emoji) occupy two
/// units; everything else one. A deliberate estimate — transcript
/// bodies are clipped, so a mild over/under-shoot is safe.
pub(crate) fn char_display_units(c: char) -> u32 {
    let cp = c as u32;
    let wide = matches!(cp,
        0x1100..=0x115F   // Hangul Jamo
        | 0x2E80..=0x303E // CJK radicals + symbols
        | 0x3041..=0x33FF // Hiragana / Katakana / CJK marks
        | 0x3400..=0x4DBF // CJK Ext-A
        | 0x4E00..=0x9FFF // CJK Unified
        | 0xA000..=0xA4CF // Yi
        | 0xAC00..=0xD7A3 // Hangul syllables
        | 0xF900..=0xFAFF // CJK compat ideographs
        | 0xFE30..=0xFE4F // CJK compat forms
        | 0xFF00..=0xFF60 // full-width forms
        | 0xFFE0..=0xFFE6
        | 0x1F000..=0x1FAFF // emoji + symbols
    );
    if wide {
        2
    } else {
        1
    }
}

/// Greedy word-wrap `text` to lines no wider than `max_units`.
/// Existing `\n` always breaks; a run that overflows breaks at the
/// last space when there is one, otherwise mid-glyph.
pub(crate) fn wrap_units(text: &str, max_units: u32) -> Vec<String> {
    let budget = max_units.max(1);
    let mut out = Vec::new();
    for raw in text.split('\n') {
        let mut line = String::new();
        let mut line_units = 0u32;
        let mut last_space: Option<usize> = None;
        for c in raw.chars() {
            let u = char_display_units(c);
            if line_units + u > budget && !line.is_empty() {
                match last_space {
                    Some(idx) if idx > 0 => {
                        let rest: String = line[idx..].trim_start().to_string();
                        line.truncate(idx);
                        out.push(std::mem::take(&mut line));
                        line = rest;
                        line_units = line.chars().map(char_display_units).sum();
                    }
                    _ => {
                        out.push(std::mem::take(&mut line));
                        line_units = 0;
                    }
                }
                last_space = None;
            }
            if c == ' ' {
                last_space = Some(line.len());
            }
            line.push(c);
            line_units += u;
        }
        out.push(line);
    }
    if out.is_empty() {
        out.push(String::new());
    }
    out
}
