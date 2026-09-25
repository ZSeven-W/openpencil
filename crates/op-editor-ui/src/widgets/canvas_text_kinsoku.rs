//! Line-start prohibition (kinsoku) for the canvas text wrapper.
//!
//! The jian layout measures text through the skia paragraph shaper,
//! whose ICU line breaker never starts a line with closing punctuation
//! such as `。` or `，`. The paint-side greedy wrapper broke per CJK
//! character with no such rule, so a sentence whose final `。` just
//! missed the width painted the stop alone on a new line — a line the
//! layout never reserved. Carrying the preceding character down with
//! the punctuation (oikomi-free "push-out") matches what ICU does.

/// Characters that must not begin a line: closing brackets and quotes,
/// sentence and clause punctuation, the prolonged-sound mark and the
/// small kana (JIS X 4051 classes cl-02/04/05/06/11 plus the common
/// ASCII and fullwidth forms).
pub(super) fn is_no_line_start(ch: char) -> bool {
    matches!(
        ch,
        '。' | '，'
            | '、'
            | '．'
            | '：'
            | '；'
            | '！'
            | '？'
            | '）'
            | '］'
            | '｝'
            | '〉'
            | '》'
            | '」'
            | '』'
            | '】'
            | '〕'
            | '〗'
            | '〙'
            | '’'
            | '”'
            | '…'
            | '‥'
            | '・'
            | 'ー'
            | '々'
            | 'ぁ'
            | 'ぃ'
            | 'ぅ'
            | 'ぇ'
            | 'ぉ'
            | 'っ'
            | 'ゃ'
            | 'ゅ'
            | 'ょ'
            | 'ァ'
            | 'ィ'
            | 'ゥ'
            | 'ェ'
            | 'ォ'
            | 'ッ'
            | 'ャ'
            | 'ュ'
            | 'ョ'
            | '.'
            | ','
            | ':'
            | ';'
            | '!'
            | '?'
            | ')'
            | ']'
            | '}'
    )
}

/// Split `current` for a break that would otherwise start the next line
/// with the prohibited `next`. Returns the text that must move down to
/// the new line ahead of `next` (the last character of `current`), or
/// an empty string when moving it would leave `current` empty or would
/// itself start the line with a prohibited character — in those cases
/// the plain break is the least-bad option.
pub(super) fn carry_for_line_start(current: &mut String, next: char) -> String {
    if !is_no_line_start(next) {
        return String::new();
    }
    let mut chars = current.char_indices().rev();
    let Some((last_at, last)) = chars.next() else {
        return String::new();
    };
    if chars.next().is_none() || last == ' ' || is_no_line_start(last) {
        return String::new();
    }
    let carried = current[last_at..].to_string();
    current.truncate(last_at);
    carried
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_stop_pulls_its_preceding_character_down() {
        let mut current = String::from("评论内容很长");
        let carried = carry_for_line_start(&mut current, '。');
        assert_eq!(current, "评论内容很");
        assert_eq!(carried, "长");
    }

    #[test]
    fn an_ordinary_character_breaks_plainly() {
        let mut current = String::from("评论内容");
        assert!(carry_for_line_start(&mut current, '很').is_empty());
        assert_eq!(current, "评论内容");
    }

    #[test]
    fn a_single_character_line_is_never_emptied() {
        let mut current = String::from("好");
        assert!(carry_for_line_start(&mut current, '。').is_empty());
        assert_eq!(current, "好");
    }

    #[test]
    fn a_carried_character_that_is_itself_prohibited_stays() {
        let mut current = String::from("真的吗？");
        assert!(carry_for_line_start(&mut current, '」').is_empty());
        assert_eq!(current, "真的吗？");
    }
}
