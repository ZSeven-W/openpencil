package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class ChatAttachmentImageTest {
    private fun bytes(vararg values: Int) = ByteArray(values.size) { values[it].toByte() }

    @Test
    fun sniffsEveryEngineFormat() {
        assertEquals("image/png", ChatAttachmentImage.sniffMediaType(bytes(0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0)))
        assertEquals("image/jpeg", ChatAttachmentImage.sniffMediaType(bytes(0xFF, 0xD8, 0xFF, 0xE0)))
        assertEquals("image/gif", ChatAttachmentImage.sniffMediaType("GIF89a..".toByteArray()))
        assertEquals("image/webp", ChatAttachmentImage.sniffMediaType("RIFF\u0000\u0000\u0000\u0000WEBPVP8 ".toByteArray()))
        assertNull(ChatAttachmentImage.sniffMediaType("....ftypheic".toByteArray()))
        assertNull(ChatAttachmentImage.sniffMediaType(ByteArray(0)))
    }

    @Test
    fun onlyUnknownOrOversizedBytesAreReencoded() {
        assertFalse(ChatAttachmentImage.needsReencode(bytes(0xFF, 0xD8, 0xFF, 0xE0)))
        assertTrue(ChatAttachmentImage.needsReencode("....ftypheic".toByteArray()))
        val oversized = ByteArray(ChatAttachmentImage.MAX_BYTES + 1).also {
            it[0] = 0xFF.toByte(); it[1] = 0xD8.toByte(); it[2] = 0xFF.toByte()
        }
        assertTrue(ChatAttachmentImage.needsReencode(oversized))
    }

    @Test
    fun sampleSizeKeepsTheLongestEdgeAtOrAboveTheTarget() {
        assertEquals(1, ChatAttachmentImage.sampleSizeFor(4000, 3000, 4096))
        assertEquals(2, ChatAttachmentImage.sampleSizeFor(8192, 6144, 4096))
        assertEquals(4, ChatAttachmentImage.sampleSizeFor(3000, 16384, 4096))
        assertEquals(1, ChatAttachmentImage.sampleSizeFor(10, 10, 4096))
    }

    @Test
    fun fileNamesAreSanitisedAndMatchTheSentBytes() {
        assertEquals("IMG_0001.jpg", ChatAttachmentImage.fileName("IMG_0001.HEIC", "image/jpeg"))
        assertEquals("abc.webp", ChatAttachmentImage.fileName("a/b\\c\n.webp", "image/webp"))
        assertEquals("photo.gif", ChatAttachmentImage.fileName("  ", "image/gif"))
        assertEquals("photo.png", ChatAttachmentImage.fileName(null, "image/png"))
        assertEquals("photo.png", ChatAttachmentImage.fileName("1000000021.png", "image/png"))
        assertEquals(".hidden.jpg", ChatAttachmentImage.fileName(".hidden", "image/jpeg"))
        assertEquals(104, ChatAttachmentImage.fileName("x".repeat(300), "image/png").length)
    }
}
