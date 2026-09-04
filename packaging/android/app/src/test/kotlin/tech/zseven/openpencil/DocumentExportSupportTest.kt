package tech.zseven.openpencil

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class DocumentExportSupportTest {
    @Test
    fun engineDerivedNamesPassThroughUnchanged() {
        assertEquals(
            "untitled-导航栏.png",
            DocumentExportSupport.validatedFilename("untitled-导航栏.png"),
        )
        assertEquals(
            "openpencil-slides.pdf",
            DocumentExportSupport.validatedFilename("openpencil-slides.pdf"),
        )
    }

    @Test
    fun traversalSeparatorsAndControlCharactersAreRejected() {
        assertNull(DocumentExportSupport.validatedFilename(null))
        assertNull(DocumentExportSupport.validatedFilename(""))
        assertNull(DocumentExportSupport.validatedFilename("."))
        assertNull(DocumentExportSupport.validatedFilename(".."))
        assertNull(DocumentExportSupport.validatedFilename("a/b.png"))
        assertNull(DocumentExportSupport.validatedFilename("a\\b.png"))
        // Spaces are legitimate: export names derive from node names.
        assertEquals(
            "sign card.png",
            DocumentExportSupport.validatedFilename("sign card.png"),
        )
        assertNull(DocumentExportSupport.validatedFilename("bad\nname.png"))
    }

    @Test
    fun namesWithoutARealStemAndExtensionAreRejected() {
        assertNull(DocumentExportSupport.validatedFilename("noextension"))
        assertNull(DocumentExportSupport.validatedFilename("trailingdot."))
        assertNull(DocumentExportSupport.validatedFilename(".hidden"))
        assertNull(
            DocumentExportSupport.validatedFilename(
                "x".repeat(DocumentExportSupport.MAX_FILENAME_BYTES) + ".png",
            ),
        )
    }

    @Test
    fun mimeTypesFollowTheStagedExtension() {
        assertEquals("image/png", DocumentExportSupport.mimeTypeFor("a.png"))
        assertEquals("image/jpeg", DocumentExportSupport.mimeTypeFor("a.jpg"))
        assertEquals("image/jpeg", DocumentExportSupport.mimeTypeFor("a.JPEG"))
        assertEquals("image/webp", DocumentExportSupport.mimeTypeFor("a.webp"))
        assertEquals("image/svg+xml", DocumentExportSupport.mimeTypeFor("a.svg"))
        assertEquals("application/pdf", DocumentExportSupport.mimeTypeFor("a.pdf"))
        assertEquals("application/zip", DocumentExportSupport.mimeTypeFor("component.zip"))
        assertEquals("text/html", DocumentExportSupport.mimeTypeFor("component.html"))
        listOf("tsx", "vue", "svelte", "dart", "swift", "kt").forEach { suffix ->
            assertEquals("text/plain", DocumentExportSupport.mimeTypeFor("component.$suffix"))
        }
        assertEquals("application/octet-stream", DocumentExportSupport.mimeTypeFor("a.unknown"))
    }
}
