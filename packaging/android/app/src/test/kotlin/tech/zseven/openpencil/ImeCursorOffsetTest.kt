package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Test

class ImeCursorOffsetTest {
    @Test
    fun extreme_platform_positions_clamp_without_integer_wraparound() {
        val text = "中文"

        assertEquals(text.length, composingCursorUtf16(text, Int.MAX_VALUE))
        assertEquals(0, composingCursorUtf16(text, Int.MIN_VALUE))
    }

    @Test
    fun cursor_uses_utf16_units_for_cjk_and_supplementary_scalars() {
        val cjk = "中文"
        val emoji = "中😀文"

        assertEquals(2, cjk.length)
        assertEquals(cjk.length, composingCursorUtf16(cjk, 1))
        assertEquals(4, emoji.length)
        assertEquals(emoji.length, composingCursorUtf16(emoji, 1))
        assertEquals(0, composingCursorUtf16(emoji, 0))
    }

    @Test
    fun empty_and_out_of_range_positions_stay_inside_composing_text() {
        assertEquals(0, composingCursorUtf16("", 1))
        assertEquals(0, composingCursorUtf16("abc", -1))
        assertEquals(3, composingCursorUtf16("abc", 2))
    }
}
