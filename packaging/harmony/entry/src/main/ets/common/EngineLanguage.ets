import { preferences } from '@kit.ArkData';
import { common } from '@kit.AbilityKit';
import { hilog } from '@kit.PerformanceAnalysisKit';

const DOMAIN = 0x0001;
const TAG = 'OpenPencilPlayer';
const PREFS = 'engine-locale';
const KEY = 'tag';

/** One row of the engine's language picker. */
export class EngineLocale {
  readonly code: string;
  readonly name: string;

  constructor(code: string, name: string) {
    this.code = code;
    this.name = name;
  }
}

/**
 * The engine's 15 UI locales (mirrors `op_i18n::Locale::ALL` order and codes,
 * identical to `EngineLanguage.kt` on Android and `EngineLanguage.swift` on
 * iOS). Display names match the engine's own picker.
 */
export class EngineLanguage {
  static readonly all: EngineLocale[] = [
    new EngineLocale('en-US', 'English'),
    new EngineLocale('zh-CN', '简体中文'),
    new EngineLocale('zh-TW', '繁體中文'),
    new EngineLocale('ja', '日本語'),
    new EngineLocale('ko', '한국어'),
    new EngineLocale('fr', 'Français'),
    new EngineLocale('es', 'Español'),
    new EngineLocale('de', 'Deutsch'),
    new EngineLocale('pt', 'Português'),
    new EngineLocale('ru', 'Русский'),
    new EngineLocale('hi', 'हिन्दी'),
    new EngineLocale('tr', 'Türkçe'),
    new EngineLocale('th', 'ไทย'),
    new EngineLocale('vi', 'Tiếng Việt'),
    new EngineLocale('id', 'Bahasa Indonesia'),
  ];

  /** Reads the persisted tag, or null when the user never picked one. */
  static async storedPreference(context: common.Context): Promise<string | null> {
    try {
      const store: preferences.Preferences = await preferences.getPreferences(context, PREFS);
      const value: preferences.ValueType = await store.get(KEY, '');
      const tag: string = typeof value === 'string' ? value : '';
      return tag.length > 0 ? tag : null;
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not read the stored locale: %{public}s', JSON.stringify(error));
      return null;
    }
  }

  /** Persists the tag so the next launch re-applies it after engine create. */
  static async savePreference(context: common.Context, tag: string): Promise<void> {
    try {
      const store: preferences.Preferences = await preferences.getPreferences(context, PREFS);
      await store.put(KEY, tag);
      await store.flush();
    } catch (error) {
      hilog.warn(DOMAIN, TAG, 'could not store the locale: %{public}s', JSON.stringify(error));
    }
  }
}
