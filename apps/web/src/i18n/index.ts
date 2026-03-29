import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';
import { appStorage } from '@/utils/app-storage';

// Only English is bundled statically as the fallback language.
// All other locales are loaded on demand via loadLocale().
import en from '@/i18n/locales/en';

export const SUPPORTED_LANGS = [
  'en',
  'zh',
  'zh-TW',
  'ja',
  'ko',
  'fr',
  'es',
  'de',
  'pt',
  'ru',
  'hi',
  'tr',
  'th',
  'vi',
  'id',
];

if (!i18n.isInitialized) {
  i18n.use(initReactI18next).init({
    lng: 'en',
    resources: { en: { translation: en } },
    supportedLngs: SUPPORTED_LANGS,
    fallbackLng: 'en',
    interpolation: { escapeValue: false },
  });
}

/** Lazy-load a locale's translations and switch to it. No-op for 'en'. */
export async function loadLocale(lang: string): Promise<void> {
  if (lang === 'en') return;
  if (!SUPPORTED_LANGS.includes(lang)) return;
  if (!i18n.hasResourceBundle(lang, 'translation')) {
    // File names are kebab-case (zh-TW → zh-tw.ts)
    const fileName = lang.toLowerCase();
    const mod = await import(`@/i18n/locales/${fileName}.ts`);
    i18n.addResourceBundle(lang, 'translation', mod.default, true, true);
  }
  i18n.changeLanguage(lang);
}

// Persist language changes
i18n.on('languageChanged', (lng) => {
  appStorage.setItem('openpencil-language', lng);
});

/** Detect user language from persisted storage or navigator, after hydration. */
export async function detectLanguagePostHydration(): Promise<void> {
  const stored = appStorage.getItem('openpencil-language');
  if (stored && SUPPORTED_LANGS.includes(stored)) {
    await loadLocale(stored);
    return;
  }
  const nav = typeof navigator !== 'undefined' ? navigator.language : 'en';
  if (SUPPORTED_LANGS.includes(nav)) {
    await loadLocale(nav);
  } else {
    const base = nav.split('-')[0];
    if (SUPPORTED_LANGS.includes(base)) {
      await loadLocale(base);
    }
  }
}

// Expose i18n.t on window so Electron main process can query it
if (typeof window !== 'undefined') {
  (window as unknown as Record<string, unknown>).__i18nT = (key: string) => i18n.t(key);
}

export default i18n;
