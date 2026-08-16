package tech.zseven.openpencil

import java.nio.file.Files
import java.util.concurrent.CountDownLatch
import java.util.concurrent.Executors
import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ExclusiveCredentialFileTest {
    @Test
    fun existingCredentialIsNeverReplaced() {
        val directory = Files.createTempDirectory("op-credential-existing")
        val target = directory.resolve("device.enc")
        val original = ByteArray(68) { 0x11 }
        Files.write(target, original)

        assertFalse(ExclusiveCredentialFile.installIfAbsent(target, ByteArray(68) { 0x22 }))
        assertArrayEquals(original, Files.readAllBytes(target))
    }

    @Test
    fun concurrentCreatorsProduceExactlyOneUnchangedWinner() {
        val directory = Files.createTempDirectory("op-credential-race")
        val target = directory.resolve("device.enc")
        val workers = 16
        val start = CountDownLatch(1)
        val pool = Executors.newFixedThreadPool(workers)
        try {
            val candidates = (1..workers).map { marker -> ByteArray(68) { marker.toByte() } }
            val futures = candidates.map { candidate ->
                pool.submit<Boolean> {
                    start.await()
                    ExclusiveCredentialFile.installIfAbsent(target, candidate)
                }
            }
            start.countDown()
            assertEquals(1, futures.count { it.get() })

            val stored = Files.readAllBytes(target)
            assertTrue(candidates.any { candidate -> candidate.contentEquals(stored) })
            assertArrayEquals(stored, Files.readAllBytes(target))
        } finally {
            pool.shutdownNow()
        }
    }
}
