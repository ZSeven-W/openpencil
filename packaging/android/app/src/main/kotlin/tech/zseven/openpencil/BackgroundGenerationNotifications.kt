package tech.zseven.openpencil

import android.Manifest
import android.app.Notification
import android.app.NotificationChannel
import android.app.NotificationManager
import android.app.PendingIntent
import android.content.Context
import android.content.Intent
import android.content.pm.PackageManager
import android.os.Build
import androidx.core.app.NotificationCompat
import androidx.core.app.NotificationManagerCompat
import androidx.core.content.ContextCompat

internal object BackgroundGenerationNotifications {
    const val ONGOING_ID = 71_001
    private const val COMPLETED_ID = 71_002
    private const val PAUSED_ID = 71_003
    private const val ONGOING_CHANNEL = "openpencil_generation"
    private const val RESULT_CHANNEL = "openpencil_generation_result"

    fun createChannels(context: Context) {
        val manager = context.getSystemService(NotificationManager::class.java)
        manager.createNotificationChannel(
            NotificationChannel(
                ONGOING_CHANNEL,
                context.getString(R.string.background_generation_channel),
                NotificationManager.IMPORTANCE_LOW,
            ).apply {
                description = context.getString(R.string.background_generation_channel_description)
                setShowBadge(false)
            },
        )
        manager.createNotificationChannel(
            NotificationChannel(
                RESULT_CHANNEL,
                context.getString(R.string.background_generation_result_channel),
                NotificationManager.IMPORTANCE_DEFAULT,
            ),
        )
    }

    fun ongoing(context: Context): Notification =
        NotificationCompat.Builder(context, ONGOING_CHANNEL)
            .setSmallIcon(R.drawable.ic_openpencil_notification)
            .setContentTitle(context.getString(R.string.background_generation_running_title))
            .setContentText(context.getString(R.string.background_generation_running_body))
            .setContentIntent(returnToEditorIntent(context))
            .setCategory(NotificationCompat.CATEGORY_PROGRESS)
            .setPriority(NotificationCompat.PRIORITY_LOW)
            .setProgress(0, 0, true)
            .setOnlyAlertOnce(true)
            .setOngoing(true)
            .build()

    fun showCompleted(context: Context) {
        notifyResult(
            context,
            COMPLETED_ID,
            R.string.background_generation_complete_title,
            R.string.background_generation_complete_body,
        )
    }

    fun showPaused(context: Context) {
        notifyResult(
            context,
            PAUSED_ID,
            R.string.background_generation_paused_title,
            R.string.background_generation_paused_body,
        )
    }

    /** Returning to the editor supersedes an earlier background-pause alert. */
    fun dismissPaused(context: Context) {
        NotificationManagerCompat.from(context).cancel(PAUSED_ID)
    }

    private fun notifyResult(context: Context, id: Int, title: Int, body: Int) {
        createChannels(context)
        // POST_NOTIFICATIONS is not required to run an FGS. When it is denied,
        // Android still exposes the active service in Task Manager, but normal
        // completion notifications must be skipped.
        if (
            Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU &&
            ContextCompat.checkSelfPermission(context, Manifest.permission.POST_NOTIFICATIONS) !=
            PackageManager.PERMISSION_GRANTED
        ) {
            return
        }
        val notification = NotificationCompat.Builder(context, RESULT_CHANNEL)
            .setSmallIcon(R.drawable.ic_openpencil_notification)
            .setContentTitle(context.getString(title))
            .setContentText(context.getString(body))
            .setContentIntent(returnToEditorIntent(context))
            .setCategory(NotificationCompat.CATEGORY_STATUS)
            .setPriority(NotificationCompat.PRIORITY_DEFAULT)
            .setAutoCancel(true)
            .build()
        NotificationManagerCompat.from(context).notify(id, notification)
    }

    private fun returnToEditorIntent(context: Context): PendingIntent {
        val intent = Intent(context, MainActivity::class.java).apply {
            flags = Intent.FLAG_ACTIVITY_CLEAR_TOP or Intent.FLAG_ACTIVITY_SINGLE_TOP
        }
        return PendingIntent.getActivity(
            context,
            0,
            intent,
            PendingIntent.FLAG_UPDATE_CURRENT or PendingIntent.FLAG_IMMUTABLE,
        )
    }
}
