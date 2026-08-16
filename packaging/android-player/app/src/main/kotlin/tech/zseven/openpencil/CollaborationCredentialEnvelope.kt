package tech.zseven.openpencil

import java.nio.ByteBuffer
import java.nio.ByteOrder

/** Strict, versioned envelope around one AES-GCM encrypted X25519 secret. */
internal object CollaborationCredentialEnvelope {
    const val PRIVATE_KEY_BYTES = 32
    const val NONCE_BYTES = 12
    const val TAG_BYTES = 16
    const val CIPHERTEXT_BYTES = PRIVATE_KEY_BYTES + TAG_BYTES

    private const val VERSION: Byte = 1
    private const val HEADER_BYTES = 8
    private const val TOTAL_BYTES = HEADER_BYTES + NONCE_BYTES + CIPHERTEXT_BYTES
    private val MAGIC = byteArrayOf(0x4f, 0x50, 0x43, 0x4b) // OPCK

    data class Payload(val nonce: ByteArray, val ciphertext: ByteArray)

    fun encode(nonce: ByteArray, ciphertext: ByteArray): ByteArray {
        require(nonce.size == NONCE_BYTES) { "invalid collaboration credential nonce" }
        require(ciphertext.size == CIPHERTEXT_BYTES) {
            "invalid collaboration credential ciphertext"
        }
        return ByteBuffer.allocate(TOTAL_BYTES)
            .order(ByteOrder.BIG_ENDIAN)
            .put(MAGIC)
            .put(VERSION)
            .put(NONCE_BYTES.toByte())
            .putShort(CIPHERTEXT_BYTES.toShort())
            .put(nonce)
            .put(ciphertext)
            .array()
    }

    fun decode(envelope: ByteArray): Payload {
        require(envelope.size == TOTAL_BYTES) { "invalid collaboration credential envelope" }
        val buffer = ByteBuffer.wrap(envelope).order(ByteOrder.BIG_ENDIAN)
        val magic = ByteArray(MAGIC.size).also(buffer::get)
        require(magic.contentEquals(MAGIC)) { "invalid collaboration credential magic" }
        require(buffer.get() == VERSION) { "unsupported collaboration credential version" }
        require(buffer.get().toInt() and 0xff == NONCE_BYTES) {
            "invalid collaboration credential nonce length"
        }
        require(buffer.short.toInt() and 0xffff == CIPHERTEXT_BYTES) {
            "invalid collaboration credential ciphertext length"
        }
        val nonce = ByteArray(NONCE_BYTES).also(buffer::get)
        val ciphertext = ByteArray(CIPHERTEXT_BYTES).also(buffer::get)
        require(!buffer.hasRemaining()) { "trailing collaboration credential data" }
        return Payload(nonce, ciphertext)
    }
}
