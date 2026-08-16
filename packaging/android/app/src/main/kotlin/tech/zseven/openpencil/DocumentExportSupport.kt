package tech.zseven.openpencil

import java.util.Locale

/**
 * Pure helpers for the frozen-export save flow. Mirrors the iOS
 * `DocumentExportCoordinator`'s filename discipline so both mobile shells
 * accept exactly the engine-derived names and nothing else.
 */
internal object DocumentExportSupport {
    /** Matches the engine-side 4 KiB export file-name ABI cap. */
    const val MAX_FILENAME_BYTES = 4 * 1024

    /**
     * Returns the filename when it is safe to place inside an app-private
     * staging directory: no traversal, no path separators, no control
     * characters, a non-empty stem and extension, and within the ABI cap.
     */
    fun validatedFilename(filename: String?): String? {
        if (filename.isNullOrEmpty()) return null
        if (filename == "." || filename == "..") return null
        if (filename.contains('/') || filename.contains('\\')) return null
        if (filename.any { it.isISOControl() }) return null
        if (filename.toByteArray(Charsets.UTF_8).size > MAX_FILENAME_BYTES) return null
        val dot = filename.lastIndexOf('.')
        if (dot <= 0 || dot == filename.lastIndex) return null
        return filename
    }

    /** SAF create-document MIME for the staged file's extension. */
    fun mimeTypeFor(filename: String): String =
        when (filename.substringAfterLast('.', "").lowercase(Locale.ROOT)) {
            "png" -> "image/png"
            "jpg", "jpeg" -> "image/jpeg"
            "webp" -> "image/webp"
            "svg" -> "image/svg+xml"
            "pdf" -> "application/pdf"
            else -> "application/octet-stream"
        }
}
