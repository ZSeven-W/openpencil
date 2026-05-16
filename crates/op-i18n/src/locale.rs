/// UI locale — full set of 15 mirrored from
/// `apps/web/src/i18n/locales/`. ZhCn + EnUs ship with complete
/// chrome translation tables; the rest fall through to EnUs
/// (visually obvious that translation is pending).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    EnUs,
    ZhCn,
    ZhTw,
    Ja,
    Ko,
    Fr,
    Es,
    De,
    Pt,
    Ru,
    Hi,
    Tr,
    Th,
    Vi,
    Id,
}

impl Locale {
    /// All locales in TopBar dropdown order — matches TS app.
    pub const ALL: [Locale; 15] = [
        Locale::EnUs,
        Locale::ZhCn,
        Locale::ZhTw,
        Locale::Ja,
        Locale::Ko,
        Locale::Fr,
        Locale::Es,
        Locale::De,
        Locale::Pt,
        Locale::Ru,
        Locale::Hi,
        Locale::Tr,
        Locale::Th,
        Locale::Vi,
        Locale::Id,
    ];

    /// Cycle to the next locale (round-trips through `ALL`).
    pub fn next(self) -> Self {
        let i = Self::ALL.iter().position(|&l| l == self).unwrap_or(0);
        Self::ALL[(i + 1) % Self::ALL.len()]
    }

    /// Native-script display name (matches the TS dropdown).
    pub fn display_name(self) -> &'static str {
        match self {
            Locale::EnUs => "English",
            Locale::ZhCn => "简体中文",
            Locale::ZhTw => "繁體中文",
            Locale::Ja => "日本語",
            Locale::Ko => "한국어",
            Locale::Fr => "Français",
            Locale::Es => "Español",
            Locale::De => "Deutsch",
            Locale::Pt => "Português",
            Locale::Ru => "Русский",
            Locale::Hi => "हिन्दी",
            Locale::Tr => "Türkçe",
            Locale::Th => "ไทย",
            Locale::Vi => "Tiếng Việt",
            Locale::Id => "Bahasa Indonesia",
        }
    }
}
