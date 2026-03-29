import { describe, it, expect } from 'vitest';

describe('i18n lazy locale loading', () => {
  it('should export loadLocale function', async () => {
    const { loadLocale } = await import('../index');
    expect(typeof loadLocale).toBe('function');
  });

  it('should load zh locale on demand', async () => {
    const i18nModule = await import('../index');
    const i18n = i18nModule.default;

    // Before loading, zh should not have resources
    expect(i18n.hasResourceBundle('zh', 'translation')).toBe(false);

    await i18nModule.loadLocale('zh');

    expect(i18n.hasResourceBundle('zh', 'translation')).toBe(true);
  });

  it('should no-op for en (already loaded statically)', async () => {
    const { loadLocale, default: i18n } = await import('../index');
    expect(i18n.hasResourceBundle('en', 'translation')).toBe(true);
    await loadLocale('en'); // should not throw
  });
});
