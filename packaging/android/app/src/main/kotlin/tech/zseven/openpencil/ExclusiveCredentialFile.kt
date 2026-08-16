package tech.zseven.openpencil

import java.io.IOException
import java.nio.ByteBuffer
import java.nio.channels.FileChannel
import java.nio.file.FileAlreadyExistsException
import java.nio.file.LinkOption
import java.nio.file.Path
import java.nio.file.StandardOpenOption

/** Installs one credential without ever replacing an existing identity. */
internal object ExclusiveCredentialFile {
    /**
     * `CREATE_NEW` makes path creation an atomic winner decision. Callers
     * serialize readers with the same cross-process lock, so bytes cannot be
     * observed mid-write during normal operation. A process crash may leave a
     * short file; it is deliberately retained so later loads fail closed
     * instead of deleting it and silently rotating the device identity.
     */
    fun installIfAbsent(path: Path, bytes: ByteArray): Boolean {
        return try {
            FileChannel.open(
                path,
                StandardOpenOption.CREATE_NEW,
                StandardOpenOption.WRITE,
                LinkOption.NOFOLLOW_LINKS,
            ).use { channel ->
                val buffer = ByteBuffer.wrap(bytes)
                while (buffer.hasRemaining()) {
                    if (channel.write(buffer) <= 0) {
                        throw IOException("could not write collaboration credential")
                    }
                }
                channel.force(true)
            }
            true
        } catch (_: FileAlreadyExistsException) {
            false
        }
    }
}
