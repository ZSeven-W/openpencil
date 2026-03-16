import i18n from 'i18next'
import { initReactI18next } from 'react-i18next'
import { appStorage } from '@/utils/app-storage'

import en from '@/i18n/locales/en'
import zh from '@/i18n/locales/zh'
import zhTW from '@/i18n/locales/zh-tw'
import ja from '@/i18n/locales/ja'
import ko from '@/i18n/locales/ko'
import fr from '@/i18n/locales/fr'
import es from '@/i18n/locales/es'
import de from '@/i18n/locales/de'
import pt from '@/i18n/locales/pt'
import ru from '@/i18n/locales/ru'
import hi from '@/i18n/locales/hi'
import tr from '@/i18n/locales/tr'
import th from '@/i18n/locales/th'
import vi from '@/i18n/locales/vi'
import id from '@/i18n/locales/id'

export const SUPPORTED_LANGS = ['en', 'zh', 'zh-TW', 'ja', 'ko', 'fr', 'es', 'de', 'pt', 'ru', 'hi', 'tr', 'th', 'vi', 'id']

const resources = {
  en: { translation: en },
  zh: { translation: zh },
  'zh-TW': { translation: zhTW },
  ja: { translation: ja },
  ko: { translation: ko },
  fr: { translation: fr },
  es: { translation: es },
  de: { translation: de },
  pt: { translation: pt },
  ru: { translation: ru },
  hi: { translation: hi },
  tr: { translation: tr },
  th: { translation: th },
  vi: { translation: vi },
  id: { translation: id },
}

if (!i18n.isInitialized) {
  i18n
    .use(initReactI18next)
    .init({
      lng: 'en',
      resources,
      supportedLngs: SUPPORTED_LANGS,
      fallbackLng: 'en',
      interpolation: {
        escapeValue: false,
      },
    })
}

// Persist language changes
i18n.on('languageChanged', (lng) => {
  appStorage.setItem('openpencil-language', lng)
})

/** Detect user language from persisted storage or navigator, after hydration. */
export function detectLanguagePostHydration() {
  const stored = appStorage.getItem('openpencil-language')
  if (stored && SUPPORTED_LANGS.includes(stored)) {
    i18n.changeLanguage(stored)
    return
  }
  const nav = typeof navigator !== 'undefined' ? navigator.language : 'en'
  if (SUPPORTED_LANGS.includes(nav)) {
    i18n.changeLanguage(nav)
  } else {
    const base = nav.split('-')[0]
    if (SUPPORTED_LANGS.includes(base)) {
      i18n.changeLanguage(base)
    }
  }
}

export default i18n
