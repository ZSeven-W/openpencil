package tech.zseven.openpencil

/**
 * Resolves Android's composing-text cursor convention in UTF-16 units.
 *
 * Positive positions are relative to the end of the composing text (`1`
 * means immediately after it); zero and negative positions are relative to
 * its start. Misbehaving IMEs may supply extreme values, so calculate in
 * [Long] before clamping to the composing range instead of allowing [Int]
 * arithmetic to wrap around.
 */
internal fun composingCursorUtf16(text: CharSequence, newCursorPosition: Int): Int {
    val utf16Length = text.length.toLong()
    val requested = if (newCursorPosition > 0) {
        utf16Length + newCursorPosition.toLong() - 1L
    } else {
        newCursorPosition.toLong()
    }
    return requested.coerceIn(0L, utf16Length).toInt()
}
