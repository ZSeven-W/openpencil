package tech.zseven.openpencil

import android.content.Context
import android.content.res.Configuration
import android.graphics.Color
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.text.InputType
import android.util.TypedValue
import android.view.Gravity
import android.view.ViewGroup
import android.widget.EditText
import android.widget.ImageButton
import android.widget.LinearLayout
import android.widget.TextView

/**
 * Shared visual language for the native SSO screens, matching the ZSeven
 * web sign-in design: labeled boxed inputs, a blue→purple gradient primary
 * button, hairline-bordered provider icon cards, and blue text links.
 */
internal object AuthUi {
    const val ACCENT = 0xFF2E6BFF.toInt()
    private const val GRADIENT_END = 0xFF7A5CFF.toInt()
    const val DANGER = 0xFFD32F2F.toInt()

    private val MONOCHROME_PROVIDERS = setOf("apple", "github", "douyin")

    fun isNight(context: Context): Boolean =
        (context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK) ==
            Configuration.UI_MODE_NIGHT_YES

    fun textColor(context: Context): Int = if (isNight(context)) Color.WHITE else Color.BLACK

    fun secondaryColor(context: Context): Int =
        if (isNight(context)) 0xFF9E9E9E.toInt() else 0xFF616161.toInt()

    fun backgroundColor(context: Context): Int =
        if (isNight(context)) 0xFF101010.toInt() else Color.WHITE

    fun borderColor(context: Context): Int =
        if (isNight(context)) 0x2EFFFFFF else 0x1F000000

    fun dp(context: Context, value: Int): Int =
        (value * context.resources.displayMetrics.density).toInt()

    fun fieldLabel(context: Context, text: String): TextView {
        val label = TextView(context)
        label.text = text
        label.setTextColor(textColor(context))
        label.setTypeface(null, Typeface.BOLD)
        label.textSize = 14f
        label.setPadding(0, dp(context, 10), 0, dp(context, 6))
        return label
    }

    /** Rounded hairline-bordered container wrapping [field]. */
    fun boxedField(
        context: Context,
        field: EditText,
        hint: String,
        password: Boolean = false,
        iconRes: Int = 0,
    ): LinearLayout {
        val box = LinearLayout(context)
        box.orientation = LinearLayout.HORIZONTAL
        box.gravity = Gravity.CENTER_VERTICAL
        box.background = roundedBorder(context)
        box.setPadding(dp(context, 14), 0, dp(context, 6), 0)
        box.minimumHeight = dp(context, 52)

        if (iconRes != 0) {
            val icon = android.widget.ImageView(context)
            icon.setImageResource(iconRes)
            icon.setColorFilter(secondaryColor(context))
            val iconParams = LinearLayout.LayoutParams(dp(context, 18), dp(context, 18))
            iconParams.marginEnd = dp(context, 10)
            box.addView(icon, iconParams)
        }

        field.hint = hint
        field.background = null
        field.setTextColor(textColor(context))
        field.setHintTextColor(secondaryColor(context))
        field.textSize = 16f
        if (password) {
            field.inputType =
                InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
        }
        val params = LinearLayout.LayoutParams(0, ViewGroup.LayoutParams.WRAP_CONTENT, 1f)
        box.addView(field, params)

        if (password) {
            val eye = android.widget.ImageView(context)
            eye.setImageResource(R.drawable.ic_eye_off)
            eye.setColorFilter(secondaryColor(context))
            eye.setPadding(dp(context, 10), dp(context, 10), dp(context, 10), dp(context, 10))
            eye.setOnClickListener {
                val visible = field.inputType and InputType.TYPE_TEXT_VARIATION_PASSWORD == 0
                field.inputType = if (visible) {
                    InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_PASSWORD
                } else {
                    InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD
                }
                field.setSelection(field.text?.length ?: 0)
                eye.setImageResource(if (visible) R.drawable.ic_eye_off else R.drawable.ic_eye)
            }
            box.addView(eye)
        }
        return box
    }

    fun roundedBorder(context: Context): GradientDrawable {
        val drawable = GradientDrawable()
        drawable.cornerRadius = dp(context, 12).toFloat()
        drawable.setStroke(dp(context, 1), borderColor(context))
        drawable.setColor(if (isNight(context)) 0x14FFFFFF else 0x08000000)
        return drawable
    }

    /** Full-width primary button with the brand gradient. */
    fun primaryButton(context: Context, text: String, onClick: () -> Unit): TextView {
        val button = TextView(context)
        button.text = text
        button.setTextColor(Color.WHITE)
        button.setTypeface(null, Typeface.BOLD)
        button.setTextSize(TypedValue.COMPLEX_UNIT_SP, 17f)
        button.gravity = Gravity.CENTER
        val gradient = GradientDrawable(
            GradientDrawable.Orientation.LEFT_RIGHT,
            intArrayOf(ACCENT, GRADIENT_END),
        )
        gradient.cornerRadius = dp(context, 12).toFloat()
        button.background = gradient
        button.minimumHeight = dp(context, 52)
        button.setOnClickListener { onClick() }
        return button
    }

    /** Small blue text link. */
    fun link(context: Context, text: String, onClick: () -> Unit): TextView {
        val link = TextView(context)
        link.text = text
        link.setTextColor(ACCENT)
        link.textSize = 14f
        link.setPadding(
            dp(context, 8),
            dp(context, 8),
            dp(context, 8),
            dp(context, 8),
        )
        link.setOnClickListener { onClick() }
        return link
    }

    /** "── 或使用以下方式继续 ──" divider row. */
    fun divider(context: Context, text: String): LinearLayout {
        val row = LinearLayout(context)
        row.orientation = LinearLayout.HORIZONTAL
        row.gravity = Gravity.CENTER_VERTICAL
        fun line() = TextView(context).apply {
            setBackgroundColor(borderColor(context))
        }
        val label = TextView(context)
        label.text = text
        label.setTextColor(secondaryColor(context))
        label.textSize = 13f
        label.setPadding(dp(context, 12), 0, dp(context, 12), 0)
        val lineParams = LinearLayout.LayoutParams(0, dp(context, 1), 1f)
        row.addView(line(), lineParams)
        row.addView(label)
        row.addView(line(), lineParams)
        return row
    }

    /** Rounded bordered card holding one provider brand icon. */
    fun providerCard(context: Context, providerId: String, onClick: () -> Unit): ImageButton {
        val night = isNight(context)
        val card = ImageButton(context)
        val bg = GradientDrawable()
        bg.cornerRadius = dp(context, 14).toFloat()
        bg.setStroke(dp(context, 1), borderColor(context))
        bg.setColor(if (night) 0x14FFFFFF else Color.WHITE)
        card.background = bg
        card.layoutParams = LinearLayout.LayoutParams(dp(context, 56), dp(context, 56))
        val plain = context.resources.getIdentifier(
            "provider_$providerId",
            "drawable",
            context.packageName,
        )
        if (plain != 0) {
            card.setImageResource(plain)
            card.scaleType = android.widget.ImageView.ScaleType.FIT_CENTER
            val pad = dp(context, 13)
            card.setPadding(pad, pad, pad, pad)
            // Monochrome brand marks follow the theme text color; colored
            // marks keep their brand colors as-is.
            if (providerId in MONOCHROME_PROVIDERS) {
                card.setColorFilter(textColor(context))
            }
        }
        card.setOnClickListener { onClick() }
        return card
    }
}
