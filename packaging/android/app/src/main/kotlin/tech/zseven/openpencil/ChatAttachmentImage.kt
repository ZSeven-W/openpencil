package tech.zseven.openpencil

/**
 * Platform-free rules for preparing one photo-picker image for
 * [OpNative.nativeEditorAttachChatImage].
 *
 * The engine sniffs PNG, JPEG, GIF, and WebP magic bytes and rejects anything
 * above 5 MiB. Gallery photos are often HEIC or a camera JPEG far above the
 * cap, so those are re-encoded to JPEG first (see [ChatAttachmentPicker]);
 * engine-ready images pass through untouched. Kept free of Android types so
 * the rules run as JVM unit tests.
 */
object ChatAttachmentImage {
    /** Mirrors `op_editor_core::chat::MAX_ATTACHMENT_BYTES`. */
    const val MAX_BYTES = 5 * 1024 * 1024

    /** Raw picker input ceiling, matching the shell's other picker reads. */
    const val MAX_INPUT_BYTES = 32 * 1024 * 1024

    /** Longest edge tried first when a photo has to be re-encoded. */
    const val INITIAL_MAX_EDGE = 4096
    const val MIN_MAX_EDGE = 512
    const val JPEG_QUALITY = 85

    /** The engine-recognised media type of [bytes], from its magic bytes. */
    fun sniffMediaType(bytes: ByteArray): String? = when {
        bytes.startsWith(0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A) -> "image/png"
        bytes.startsWith(0xFF, 0xD8, 0xFF) -> "image/jpeg"
        bytes.startsWithAscii("GIF87a") || bytes.startsWithAscii("GIF89a") -> "image/gif"
        bytes.size >= 12 && bytes.startsWithAscii("RIFF") &&
            String(bytes, 8, 4, Charsets.US_ASCII) == "WEBP" -> "image/webp"
        else -> null
    }

    /** True when the bytes cannot be handed to the engine as they are. */
    fun needsReencode(bytes: ByteArray): Boolean =
        bytes.size > MAX_BYTES || sniffMediaType(bytes) == null

    /** Power-of-two decode sample size that brings the longest edge near [maxEdge]. */
    fun sampleSizeFor(width: Int, height: Int, maxEdge: Int): Int {
        var sample = 1
        val longest = maxOf(width, height)
        while (longest / (sample * 2) >= maxEdge) sample *= 2
        return sample
    }

    /**
     * A display name the engine accepts: no path separators or control
     * characters, and an extension that matches the bytes actually sent.
     */
    fun fileName(suggested: String?, mediaType: String): String {
        var stem = suggested.orEmpty()
            .filter { !it.isISOControl() && it != '/' && it != '\\' }
            .trim()
        val dot = stem.lastIndexOf('.')
        if (dot > 0) stem = stem.substring(0, dot)
        // The Photo Picker names items by their opaque media id
        // ("1000000021.png"); that reads as noise on the chip.
        if (stem.isEmpty() || stem.all { it.isDigit() }) stem = "photo"
        val extension = when (mediaType) {
            "image/jpeg" -> "jpg"
            "image/gif" -> "gif"
            "image/webp" -> "webp"
            else -> "png"
        }
        return "${stem.take(100)}.$extension"
    }

    private fun ByteArray.startsWith(vararg prefix: Int): Boolean =
        size >= prefix.size && prefix.indices.all { this[it] == prefix[it].toByte() }

    private fun ByteArray.startsWithAscii(prefix: String): Boolean =
        startsWith(*prefix.map { it.code }.toIntArray())
}
