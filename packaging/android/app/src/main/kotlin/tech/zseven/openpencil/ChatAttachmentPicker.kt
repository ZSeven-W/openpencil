package tech.zseven.openpencil

import android.graphics.Bitmap
import android.graphics.BitmapFactory
import android.graphics.Canvas
import android.graphics.Color
import android.graphics.ImageDecoder
import android.graphics.Matrix
import android.media.ExifInterface
import android.net.Uri
import android.os.Build
import android.provider.OpenableColumns
import android.util.Log
import androidx.activity.ComponentActivity
import androidx.activity.result.PickVisualMediaRequest
import androidx.activity.result.contract.ActivityResultContracts
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.IOException
import java.nio.ByteBuffer

private const val TAG = "OpenPencilPlayer"

/**
 * Answers `SHELL_ACTION_PICK_CHAT_ATTACHMENT` (Studio Home "Add screenshot" and
 * the chat attach button): presents the system Photo Picker for one image,
 * reads it off the main thread, normalises it for the engine (HEIC or
 * oversized photos become a JPEG under the 5 MiB cap), and stages it through
 * [OpSurfaceView.attachChatImage]. The engine drained the request when the
 * shell polled it, so a dismissed picker is a silent no-op.
 *
 * Must be constructed while the Activity is being created: the Activity
 * Result launcher is registered in the initializer.
 */
internal class ChatAttachmentPicker(
    private val activity: ComponentActivity,
    private val surface: () -> OpSurfaceView?,
) {
    private class Prepared(val bytes: ByteArray, val mediaType: String, val fileName: String)

    private var inProgress = false

    private val launcher = activity.registerForActivityResult(
        ActivityResultContracts.PickVisualMedia(),
    ) { uri ->
        if (uri == null) {
            Log.i(TAG, "chat attachment picker dismissed")
            inProgress = false
        } else {
            readAndAttach(uri)
        }
    }

    /** Shell action 13: present the Photo Picker once per engine request. */
    fun launch() {
        if (inProgress || activity.isFinishing || activity.isDestroyed) return
        inProgress = true
        try {
            launcher.launch(
                PickVisualMediaRequest(ActivityResultContracts.PickVisualMedia.ImageOnly),
            )
        } catch (e: Exception) {
            inProgress = false
            Log.w(TAG, "could not launch the chat attachment photo picker", e)
        }
    }

    private fun readAndAttach(uri: Uri) {
        val suggestedName = queryDisplayName(uri)
        Thread({
            val prepared = try {
                prepare(readBounded(uri), suggestedName)
            } catch (e: Exception) {
                Log.w(TAG, "could not read chat attachment", e)
                null
            } catch (e: OutOfMemoryError) {
                Log.w(TAG, "not enough memory to prepare chat attachment", e)
                null
            }
            activity.runOnUiThread {
                inProgress = false
                if (activity.isFinishing || activity.isDestroyed) return@runOnUiThread
                val view = surface() ?: return@runOnUiThread
                if (prepared == null) {
                    Log.w(TAG, "chat attachment is not a decodable image under the size cap")
                    return@runOnUiThread
                }
                val status = view.attachChatImage(prepared.bytes, prepared.mediaType, prepared.fileName)
                if (status == 0) {
                    Log.i(TAG, "chat attachment staged: ${prepared.mediaType}, ${prepared.bytes.size} bytes")
                } else if (status != OpNative.STATUS_CLOSING) {
                    // Busy = the turn already holds the maximum attachments;
                    // InvalidArg = the engine refused the bytes. Neither leaves
                    // partial state, so logging is the whole response.
                    Log.w(
                        TAG,
                        "chat attachment rejected, status=$status: " +
                            OpNative.nativeLastError(view.engine),
                    )
                }
            }
        }, "OpenPencilChatAttachment").start()
    }

    private fun prepare(bytes: ByteArray, suggestedName: String?): Prepared? {
        if (!ChatAttachmentImage.needsReencode(bytes)) {
            val mediaType = ChatAttachmentImage.sniffMediaType(bytes) ?: return null
            return Prepared(bytes, mediaType, ChatAttachmentImage.fileName(suggestedName, mediaType))
        }
        val jpeg = reencodeAsJpeg(bytes) ?: return null
        return Prepared(jpeg, "image/jpeg", ChatAttachmentImage.fileName(suggestedName, "image/jpeg"))
    }

    /** Decodes any platform-readable image and encodes an upright JPEG under the cap. */
    private fun reencodeAsJpeg(bytes: ByteArray): ByteArray? {
        var maxEdge = ChatAttachmentImage.INITIAL_MAX_EDGE
        while (maxEdge >= ChatAttachmentImage.MIN_MAX_EDGE) {
            val bitmap = decodeUpright(bytes, maxEdge) ?: return null
            val output = ByteArrayOutputStream()
            try {
                // JPEG has no alpha: flatten transparent pixels onto white
                // instead of letting them encode as black.
                val opaque = Bitmap.createBitmap(bitmap.width, bitmap.height, Bitmap.Config.ARGB_8888)
                Canvas(opaque).apply {
                    drawColor(Color.WHITE)
                    drawBitmap(bitmap, 0f, 0f, null)
                }
                opaque.compress(Bitmap.CompressFormat.JPEG, ChatAttachmentImage.JPEG_QUALITY, output)
                opaque.recycle()
            } finally {
                bitmap.recycle()
            }
            if (output.size() <= ChatAttachmentImage.MAX_BYTES) return output.toByteArray()
            maxEdge = maxEdge * 3 / 4
        }
        return null
    }

    /** A software bitmap whose longest edge is at most [maxEdge], EXIF-rotated. */
    private fun decodeUpright(bytes: ByteArray, maxEdge: Int): Bitmap? {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            // ImageDecoder reads HEIF and applies the EXIF orientation itself.
            return try {
                ImageDecoder.decodeBitmap(ImageDecoder.createSource(ByteBuffer.wrap(bytes))) { decoder, info, _ ->
                    decoder.allocator = ImageDecoder.ALLOCATOR_SOFTWARE
                    val longest = maxOf(info.size.width, info.size.height)
                    if (longest > maxEdge) {
                        val scale = maxEdge.toFloat() / longest
                        decoder.setTargetSize(
                            maxOf(1, (info.size.width * scale).toInt()),
                            maxOf(1, (info.size.height * scale).toInt()),
                        )
                    }
                }
            } catch (e: IOException) {
                Log.w(TAG, "could not decode chat attachment", e)
                null
            }
        }
        val bounds = BitmapFactory.Options().apply { inJustDecodeBounds = true }
        BitmapFactory.decodeByteArray(bytes, 0, bytes.size, bounds)
        if (bounds.outWidth <= 0 || bounds.outHeight <= 0) return null
        val options = BitmapFactory.Options().apply {
            inSampleSize = ChatAttachmentImage.sampleSizeFor(bounds.outWidth, bounds.outHeight, maxEdge)
        }
        val decoded = BitmapFactory.decodeByteArray(bytes, 0, bytes.size, options) ?: return null
        val longest = maxOf(decoded.width, decoded.height)
        val matrix = Matrix().apply {
            if (longest > maxEdge) postScale(maxEdge.toFloat() / longest, maxEdge.toFloat() / longest)
            postRotate(exifRotationDegrees(bytes))
        }
        if (matrix.isIdentity) return decoded
        val upright = Bitmap.createBitmap(decoded, 0, 0, decoded.width, decoded.height, matrix, true)
        if (upright !== decoded) decoded.recycle()
        return upright
    }

    private fun exifRotationDegrees(bytes: ByteArray): Float = try {
        when (
            ExifInterface(ByteArrayInputStream(bytes))
                .getAttributeInt(ExifInterface.TAG_ORIENTATION, ExifInterface.ORIENTATION_NORMAL)
        ) {
            ExifInterface.ORIENTATION_ROTATE_90 -> 90f
            ExifInterface.ORIENTATION_ROTATE_180 -> 180f
            ExifInterface.ORIENTATION_ROTATE_270 -> 270f
            else -> 0f
        }
    } catch (e: IOException) {
        0f
    }

    private fun readBounded(uri: Uri): ByteArray {
        val input = activity.contentResolver.openInputStream(uri)
            ?: throw IOException("content resolver returned no input stream")
        return input.use { stream ->
            val output = ByteArrayOutputStream()
            val buffer = ByteArray(DEFAULT_BUFFER_SIZE)
            var total = 0L
            while (true) {
                val count = stream.read(buffer)
                if (count < 0) break
                total += count
                if (total > ChatAttachmentImage.MAX_INPUT_BYTES) {
                    throw IOException("photo exceeds the 32 MiB mobile input limit")
                }
                output.write(buffer, 0, count)
            }
            output.toByteArray()
        }
    }

    private fun queryDisplayName(uri: Uri): String? = try {
        activity.contentResolver.query(uri, arrayOf(OpenableColumns.DISPLAY_NAME), null, null, null)
            ?.use { cursor ->
                if (!cursor.moveToFirst()) return@use null
                val index = cursor.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                if (index >= 0) cursor.getString(index) else null
            }
    } catch (e: Exception) {
        Log.w(TAG, "could not query chat attachment name", e)
        null
    }
}
