package tech.zseven.openpencil

import org.junit.Assert.assertArrayEquals
import org.junit.Assert.assertThrows
import org.junit.Test

class CollaborationCredentialEnvelopeTest {
    @Test
    fun roundTripsTheOnlyAcceptedShape() {
        val nonce = ByteArray(CollaborationCredentialEnvelope.NONCE_BYTES) { it.toByte() }
        val ciphertext = ByteArray(CollaborationCredentialEnvelope.CIPHERTEXT_BYTES) {
            (it + 20).toByte()
        }

        val decoded = CollaborationCredentialEnvelope.decode(
            CollaborationCredentialEnvelope.encode(nonce, ciphertext),
        )

        assertArrayEquals(nonce, decoded.nonce)
        assertArrayEquals(ciphertext, decoded.ciphertext)
    }

    @Test
    fun rejectsWrongVersionNonceLengthCiphertextLengthAndTrailingData() {
        val valid = CollaborationCredentialEnvelope.encode(
            ByteArray(CollaborationCredentialEnvelope.NONCE_BYTES),
            ByteArray(CollaborationCredentialEnvelope.CIPHERTEXT_BYTES),
        )
        val mutations = listOf(
            valid.copyOf().also { it[4] = 2 },
            valid.copyOf().also { it[5] = 11 },
            valid.copyOf().also { it[7] = 47 },
            valid + byteArrayOf(0),
        )

        mutations.forEach { envelope ->
            assertThrows(IllegalArgumentException::class.java) {
                CollaborationCredentialEnvelope.decode(envelope)
            }
        }
    }

    @Test
    fun rejectsWrongMagicAndTruncation() {
        val valid = CollaborationCredentialEnvelope.encode(
            ByteArray(CollaborationCredentialEnvelope.NONCE_BYTES),
            ByteArray(CollaborationCredentialEnvelope.CIPHERTEXT_BYTES),
        )
        assertThrows(IllegalArgumentException::class.java) {
            CollaborationCredentialEnvelope.decode(valid.copyOf().also { it[0] = 0 })
        }
        assertThrows(IllegalArgumentException::class.java) {
            CollaborationCredentialEnvelope.decode(valid.copyOf(valid.size - 1))
        }
    }
}
