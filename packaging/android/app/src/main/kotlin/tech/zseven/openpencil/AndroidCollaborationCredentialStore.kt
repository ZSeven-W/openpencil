package tech.zseven.openpencil

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.io.File
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Files
import java.nio.file.LinkOption
import java.nio.file.StandardOpenOption
import java.security.KeyStore
import javax.crypto.AEADBadTagException
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

private const val ANDROID_KEY_STORE = "AndroidKeyStore"
private const val WRAPPING_KEY_ALIAS =
    "tech.zseven.openpencil.collaboration.device-key-wrapping-v1"
private const val CIPHER_TRANSFORMATION = "AES/GCM/NoPadding"
private const val GCM_TAG_BITS = CollaborationCredentialEnvelope.TAG_BYTES * 8

/**
 * Stores the collaboration X25519 identity as an authenticated ciphertext in
 * `noBackupFilesDir`. Its AES-GCM wrapping key is non-exportable and remains
 * inside Android Keystore.
 *
 * A missing ciphertext is represented by `null` only when the wrapping alias
 * is also absent. Keystore, IO, malformed-envelope, authentication, and
 * inconsistent-state failures throw and must be mapped to a fail-closed
 * native result; this class never rotates a damaged identity.
 */
internal class AndroidCollaborationCredentialStore(context: Context) {
    private val noBackupRoot = context.applicationContext.noBackupFilesDir
    private val directory = File(noBackupRoot, "collaboration")
    private val credentialFile = File(directory, "device-x25519-v1.enc")
    private val lockFile = File(directory, ".credential.lock")
    private val associatedData =
        "$WRAPPING_KEY_ALIAS:envelope-v1".toByteArray(StandardCharsets.UTF_8)

    fun load(): ByteArray? = withStoreLock {
        if (!credentialFile.exists()) {
            requireWrappingKeyAbsent()
            return@withStoreLock null
        }
        requireRegularFile(credentialFile)
        if (credentialFile.length() != ENVELOPE_BYTES.toLong()) {
            throw IOException("invalid collaboration credential envelope length")
        }
        val envelope = Files.readAllBytes(credentialFile.toPath())
        val payload = try {
            CollaborationCredentialEnvelope.decode(envelope)
        } finally {
            envelope.fill(0)
        }
        val key = loadExistingWrappingKey()
        val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
        cipher.init(Cipher.DECRYPT_MODE, key, GCMParameterSpec(GCM_TAG_BITS, payload.nonce))
        cipher.updateAAD(associatedData)
        try {
            val secret = cipher.doFinal(payload.ciphertext)
            if (secret.size != CollaborationCredentialEnvelope.PRIVATE_KEY_BYTES) {
                secret.fill(0)
                throw IOException("invalid collaboration credential plaintext length")
            }
            secret
        } catch (error: AEADBadTagException) {
            throw IOException("collaboration credential authentication failed", error)
        } finally {
            payload.nonce.fill(0)
            payload.ciphertext.fill(0)
        }
    }

    /** Atomically installs a first credential; an existing value always wins. */
    fun storeIfAbsent(secret: ByteArray) = withStoreLock {
        if (secret.size != CollaborationCredentialEnvelope.PRIVATE_KEY_BYTES) {
            throw IOException("invalid collaboration credential length")
        }
        if (credentialFile.exists()) {
            requireRegularFile(credentialFile)
            return@withStoreLock
        }

        val key = createWrappingKey()
        val cipher = Cipher.getInstance(CIPHER_TRANSFORMATION)
        cipher.init(Cipher.ENCRYPT_MODE, key)
        val nonce = cipher.iv
        if (nonce.size != CollaborationCredentialEnvelope.NONCE_BYTES) {
            nonce.fill(0)
            throw IOException("invalid Android Keystore GCM nonce length")
        }
        cipher.updateAAD(associatedData)
        val ciphertext = cipher.doFinal(secret)
        val envelope = try {
            CollaborationCredentialEnvelope.encode(nonce, ciphertext)
        } finally {
            nonce.fill(0)
            ciphertext.fill(0)
        }
        try {
            if (!ExclusiveCredentialFile.installIfAbsent(credentialFile.toPath(), envelope)) {
                requireRegularFile(credentialFile)
            }
        } finally {
            envelope.fill(0)
        }
    }

    private fun loadExistingWrappingKey(): SecretKey {
        val keyStore = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
        if (!keyStore.containsAlias(WRAPPING_KEY_ALIAS)) {
            throw IOException("collaboration wrapping key is missing")
        }
        return validateWrappingKey(keyStore.getKey(WRAPPING_KEY_ALIAS, null))
    }

    private fun requireWrappingKeyAbsent() {
        val keyStore = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
        if (keyStore.containsAlias(WRAPPING_KEY_ALIAS)) {
            throw IOException("collaboration credential is missing for its wrapping key")
        }
    }

    private fun createWrappingKey(): SecretKey {
        // Recheck inside the store call. Another runtime may have completed
        // its earlier load before this runtime generated its X25519 candidate.
        requireWrappingKeyAbsent()
        val generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, ANDROID_KEY_STORE)
        generator.init(
            KeyGenParameterSpec.Builder(
                WRAPPING_KEY_ALIAS,
                KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT,
            )
                .setKeySize(256)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM)
                .setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                .setRandomizedEncryptionRequired(true)
                .setUserAuthenticationRequired(false)
                .build(),
        )
        generator.generateKey()

        // Android Keystore remains canonical; never trust the generator's
        // returned object when another process could have initialized it.
        val canonical = KeyStore.getInstance(ANDROID_KEY_STORE).apply { load(null) }
        return validateWrappingKey(canonical.getKey(WRAPPING_KEY_ALIAS, null))
    }

    private fun validateWrappingKey(value: java.security.Key?): SecretKey {
        val key = value as? SecretKey
            ?: throw IOException("collaboration wrapping key has an invalid type")
        if (key.algorithm != KeyProperties.KEY_ALGORITHM_AES || key.encoded != null) {
            throw IOException("collaboration wrapping key is exportable or malformed")
        }
        return key
    }

    private fun <T> withStoreLock(block: () -> T): T {
        // FileChannel rejects overlapping locks inside one JVM instead of
        // waiting, so serialize there first; the file lock then covers other
        // app processes sharing the same sandbox.
        return synchronized(PROCESS_LOCK) {
            prepareDirectory()
            java.nio.channels.FileChannel.open(
                lockFile.toPath(),
                StandardOpenOption.CREATE,
                StandardOpenOption.WRITE,
                LinkOption.NOFOLLOW_LINKS,
            ).use { channel ->
                channel.lock().use { block() }
            }
        }
    }

    private fun prepareDirectory() {
        if (!noBackupRoot.isDirectory && !noBackupRoot.mkdirs() && !noBackupRoot.isDirectory) {
            throw IOException("could not prepare no-backup storage")
        }
        if (!directory.exists() && !directory.mkdir() && !directory.isDirectory) {
            throw IOException("could not prepare collaboration storage")
        }
        if (!directory.isDirectory || Files.isSymbolicLink(directory.toPath())) {
            throw IOException("collaboration storage is not a real directory")
        }
        if (directory.canonicalFile.parentFile != noBackupRoot.canonicalFile) {
            throw IOException("collaboration storage escaped the no-backup directory")
        }
    }

    private fun requireRegularFile(file: File) {
        if (
            Files.isSymbolicLink(file.toPath()) ||
            !Files.isRegularFile(file.toPath(), LinkOption.NOFOLLOW_LINKS)
        ) {
            throw IOException("collaboration credential is not a regular file")
        }
    }

    private companion object {
        val PROCESS_LOCK = Any()
        const val ENVELOPE_BYTES = 8 +
            CollaborationCredentialEnvelope.NONCE_BYTES +
            CollaborationCredentialEnvelope.CIPHERTEXT_BYTES
    }
}
