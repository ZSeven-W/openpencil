package tech.zseven.openpencil

import android.content.Context

/**
 * The engine's 15 UI locales (mirrors `op_i18n::Locale::ALL` order and
 * codes; the native-login contract pins the codes against the engine
 * header side). Display names match the engine's own picker.
 */
internal object EngineLanguage {
    val all: List<Pair<String, String>> = listOf(
        "en-US" to "English",
        "zh-CN" to "简体中文",
        "zh-TW" to "繁體中文",
        "ja" to "日本語",
        "ko" to "한국어",
        "fr" to "Français",
        "es" to "Español",
        "de" to "Deutsch",
        "pt" to "Português",
        "ru" to "Русский",
        "hi" to "हिन्दी",
        "tr" to "Türkçe",
        "th" to "ไทย",
        "vi" to "Tiếng Việt",
        "id" to "Bahasa Indonesia",
    )

    private const val PREFS = "engine-locale"
    private const val KEY = "tag"

    fun storedPreference(context: Context): String? =
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getString(KEY, null)

    fun savePreference(context: Context, tag: String) {
        context.getSharedPreferences(PREFS, Context.MODE_PRIVATE)
            .edit().putString(KEY, tag).apply()
    }
}
