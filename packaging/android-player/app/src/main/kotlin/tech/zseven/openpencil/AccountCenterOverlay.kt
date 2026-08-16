package tech.zseven.openpencil

import android.content.ActivityNotFoundException
import android.content.Intent
import android.graphics.Color
import android.graphics.Typeface
import android.net.Uri
import android.util.TypedValue
import android.view.Gravity
import android.view.View
import android.view.ViewGroup
import android.widget.FrameLayout
import android.widget.LinearLayout
import android.widget.ScrollView
import android.widget.TextView
import androidx.activity.ComponentActivity
import androidx.appcompat.app.AlertDialog
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.json.JSONObject

/**
 * Signed-in profile snapshot decoded from the engine's JSON account
 * snapshot (`op_editor_account_snapshot`).
 */
internal data class AccountSnapshot(
    val signedIn: Boolean,
    val displayName: String?,
    val username: String?,
    val primaryEmail: String?,
) {
    companion object {
        fun parse(json: String): AccountSnapshot? = try {
            val payload = JSONObject(json)
            AccountSnapshot(
                signedIn = payload.optBoolean("signed_in", false),
                displayName = payload.optString("display_name").takeIf { it.isNotEmpty() },
                username = payload.optString("username").takeIf { it.isNotEmpty() },
                primaryEmail = payload.optString("primary_email").takeIf { it.isNotEmpty() },
            )
        } catch (_: Exception) {
            null
        }
    }
}

/**
 * Full-window, platform-native account center. Profile data comes from the
 * engine snapshot (the private auth runtime never exposes its device token
 * to the shell), region selection is a next-launch preference, and deeper
 * management opens the regional web account page in the system browser.
 */
internal class AccountCenterOverlay(
    private val activity: ComponentActivity,
    private val root: FrameLayout,
    private val regionStore: SsoRegionStore,
    private val onSignOut: () -> Unit,
    private val onVisibilityChanged: (visible: Boolean) -> Unit = {},
) {
    private var container: FrameLayout? = null

    val isVisible: Boolean
        get() = container != null

    fun show(snapshot: AccountSnapshot) {
        dismiss()
        val view = buildContent(snapshot)
        container = view
        root.addView(
            view,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        onVisibilityChanged(true)
    }

    fun handleBack() {
        dismiss()
    }

    fun destroy() {
        dismiss()
    }

    private fun dismiss() {
        val view = container ?: return
        container = null
        root.removeView(view)
        onVisibilityChanged(false)
    }

    private fun buildContent(snapshot: AccountSnapshot): FrameLayout {
        val overlay = FrameLayout(activity)
        overlay.isClickable = true
        val night = isNightMode()
        overlay.setBackgroundColor(if (night) Color.BLACK else Color.WHITE)
        val textColor = if (night) Color.WHITE else Color.BLACK
        val secondary = if (night) 0xFF9E9E9E.toInt() else 0xFF616161.toInt()

        val scroll = ScrollView(activity)
        val column = LinearLayout(activity)
        column.orientation = LinearLayout.VERTICAL
        column.setPadding(dp(24), dp(16), dp(24), dp(24))

        val closeRow = TextView(activity)
        closeRow.text = activity.getString(R.string.native_login_close)
        closeRow.setTextColor(secondary)
        closeRow.textSize = 15f
        closeRow.setPadding(0, dp(8), 0, dp(8))
        closeRow.setOnClickListener { dismiss() }

        val name = TextView(activity)
        name.text = snapshot.displayName ?: snapshot.username
            ?: activity.getString(R.string.account_center_signed_out)
        name.setTextColor(textColor)
        name.setTypeface(null, Typeface.BOLD)
        name.setTextSize(TypedValue.COMPLEX_UNIT_SP, 24f)
        name.gravity = Gravity.CENTER_HORIZONTAL
        name.setPadding(0, dp(20), 0, dp(2))

        val detail = TextView(activity)
        detail.text = snapshot.primaryEmail ?: snapshot.username.orEmpty()
        detail.setTextColor(secondary)
        detail.textSize = 14f
        detail.gravity = Gravity.CENTER_HORIZONTAL
        detail.setPadding(0, 0, 0, dp(24))

        column.addView(closeRow)
        column.addView(name)
        column.addView(detail)
        column.addView(makeRow(
            activity.getString(
                R.string.account_center_region_value,
                regionName(regionStore.resolved()),
            ),
            textColor,
        ) { presentRegionPicker(it) })
        column.addView(makeRow(
            activity.getString(R.string.account_center_manage),
            textColor,
        ) { openAccountPage() })
        column.addView(makeRow(
            activity.getString(R.string.account_center_sign_out),
            0xFFD32F2F.toInt(),
        ) { confirmSignOut() })

        scroll.addView(
            column,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.WRAP_CONTENT,
            ),
        )
        overlay.addView(
            scroll,
            FrameLayout.LayoutParams(
                ViewGroup.LayoutParams.MATCH_PARENT,
                ViewGroup.LayoutParams.MATCH_PARENT,
            ),
        )
        ViewCompat.setOnApplyWindowInsetsListener(overlay) { view, insets ->
            val bars = insets.getInsets(WindowInsetsCompat.Type.systemBars())
            view.setPadding(bars.left, bars.top, bars.right, bars.bottom)
            insets
        }
        ViewCompat.requestApplyInsets(overlay)
        return overlay
    }

    private fun makeRow(text: String, color: Int, onClick: (TextView) -> Unit): TextView {
        val row = TextView(activity)
        row.text = text
        row.setTextColor(color)
        row.textSize = 16f
        row.setPadding(0, dp(14), 0, dp(14))
        row.setOnClickListener { onClick(row) }
        return row
    }

    private fun regionName(region: SsoRegion): String = activity.getString(
        if (region == SsoRegion.CHINA) R.string.sso_region_china else R.string.sso_region_global,
    )

    private fun presentRegionPicker(row: TextView) {
        val regions = SsoRegion.entries
        AlertDialog.Builder(activity)
            .setTitle(R.string.sso_region_picker_title)
            .setItems(regions.map(::regionName).toTypedArray()) { _, index ->
                regionStore.saveUserOverride(regions[index])
                row.text = activity.getString(
                    R.string.account_center_region_value,
                    regionName(regions[index]),
                )
            }
            .setMessage(R.string.sso_region_restart_note)
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun openAccountPage() {
        val url = "${regionStore.resolved().origin}/account"
        try {
            activity.startActivity(Intent(Intent.ACTION_VIEW, Uri.parse(url)))
        } catch (_: ActivityNotFoundException) {
            // No browser installed: the row simply does nothing.
        }
    }

    private fun confirmSignOut() {
        AlertDialog.Builder(activity)
            .setTitle(R.string.account_center_sign_out_confirm)
            .setPositiveButton(R.string.account_center_sign_out) { _, _ ->
                onSignOut()
                dismiss()
            }
            .setNegativeButton(android.R.string.cancel, null)
            .show()
    }

    private fun isNightMode(): Boolean =
        (activity.resources.configuration.uiMode and
            android.content.res.Configuration.UI_MODE_NIGHT_MASK) ==
            android.content.res.Configuration.UI_MODE_NIGHT_YES

    private fun dp(value: Int): Int =
        (value * activity.resources.displayMetrics.density).toInt()
}
