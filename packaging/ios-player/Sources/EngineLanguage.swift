import UIKit

/// The engine's 15 UI locales (mirrors `op_i18n::Locale::ALL` order and
/// codes; the lifecycle contract pins the codes against the engine header
/// side). Native display names match the engine's own picker.
enum EngineLanguage {
    static let all: [(code: String, name: String)] = [
        ("en-US", "English"),
        ("zh-CN", "简体中文"),
        ("zh-TW", "繁體中文"),
        ("ja", "日本語"),
        ("ko", "한국어"),
        ("fr", "Français"),
        ("es", "Español"),
        ("de", "Deutsch"),
        ("pt", "Português"),
        ("ru", "Русский"),
        ("hi", "हिन्दी"),
        ("tr", "Türkçe"),
        ("th", "ไทย"),
        ("vi", "Tiếng Việt"),
        ("id", "Bahasa Indonesia"),
    ]

    private static let preferenceKey = "engine.locale"

    static var storedPreference: String? {
        UserDefaults.standard.string(forKey: preferenceKey)
    }

    static func savePreference(_ code: String) {
        UserDefaults.standard.set(code, forKey: preferenceKey)
    }
}
