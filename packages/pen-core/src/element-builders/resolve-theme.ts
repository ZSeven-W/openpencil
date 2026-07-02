import {
  getSemanticPaletteHex,
  SEMANTIC_PALETTE_THEME_LIGHT,
  SEMANTIC_PALETTE_THEME_DARK,
} from '../variables/semantic-palette.js';

/**
 * Theme mode used by all v1 element builders.
 *
 * - `'light'` (default): emit hex literals matching v0 light-theme
 *   hardcoded values — byte-parity with v0 (Plan 14 migration contract).
 * - `'dark'`:  emit dark-theme hex literals. Numeric values (typography /
 *   spacing / radius) are theme-agnostic and identical to light mode.
 * - `'system'`: emit `$color-*` / `$type-*` / `$spacing-*` / `$radius-*`
 *   refs for all fields. Requires `applySemanticPalette(doc)` to have
 *   been called first; resolver-side fallback (DEFAULT_PALETTE_FALLBACK)
 *   ensures un-seeded docs still render with correct default values.
 */
export type V1Theme = 'light' | 'dark' | 'system';

/**
 * Fully resolved theme values returned by `resolveTheme()`.
 *
 * In 'light'/'dark' mode: all fields are concrete hex strings or numbers.
 * In 'system' mode: color fields are `$color-*` ref strings; numeric
 * fields (typography / spacing / radius) are `$type-*` / `$spacing-*` /
 * `$radius-*` ref strings.
 */
export interface ThemeResolution {
  colors: {
    /** Primary surface — card, modal, tooltip background */
    surface: string;
    /** Secondary surface — chip, input, hover background */
    surface2: string;
    /** Tertiary surface — pressed, deeper-nested background */
    surface3: string;
    /** Page background, skeleton hosts */
    bgDeep: string;
    /** Dividers, input strokes, card outlines */
    border: string;
    /** Dashed chart-placeholder border */
    borderStrong: string;
    /** Headlines, active page numbers */
    textPrimary: string;
    /** Body paragraphs, nav labels */
    textBody: string;
    /** Secondary text, timestamps, placeholders */
    textMuted: string;
    /** Tertiary text, disabled states */
    textSubtle: string;
    /** Primary brand, active pill, focus ring */
    accent: string;
    /** Delete actions, error state, trending-down */
    destructive: string;
    /** Trending-up, "Online" status */
    success: string;
  };
  alertColors: {
    infoBg: string;
    infoText: string;
    successBg: string;
    successText: string;
    warningBg: string;
    warningText: string;
    dangerBg: string;
    dangerText: string;
  };
  chartColors: {
    chart1: string;
    chart2: string;
    chart3: string;
    chart4: string;
    chart5: string;
    chart6: string;
  };
  typography: {
    displaySize: string | number;
    displayWeight: string | number;
    displayLineHeight: string | number;
    displayLetterSpacing: string | number;
    h1Size: string | number;
    h1Weight: string | number;
    h1LineHeight: string | number;
    h2Size: string | number;
    h2Weight: string | number;
    h2LineHeight: string | number;
    h3Size: string | number;
    h3Weight: string | number;
    h3LineHeight: string | number;
    bodySize: string | number;
    bodyWeight: string | number;
    bodyLineHeight: string | number;
    captionSize: string | number;
    captionWeight: string | number;
    captionLineHeight: string | number;
    uppercaseLetterSpacing: string | number;
  };
  spacing: {
    s1: string | number;
    s2: string | number;
    s3: string | number;
    s4: string | number;
    s5: string | number;
  };
  radius: {
    sm: string | number;
    md: string | number;
    lg: string | number;
  };
}

/**
 * Resolve a V1Theme to concrete color/typography/spacing/radius values.
 *
 * In 'system' mode ALL fields (colors, alert colors, chart colors,
 * typography, spacing, radius) return their `$token-name` ref strings.
 * Downstream renderer resolves them via `resolveVariableRef()` which
 * falls back to DEFAULT_PALETTE_FALLBACK for un-seeded documents —
 * ensuring 'system' + un-seeded == 'light' at render time.
 *
 * In 'light'/'dark' mode numeric values (typography / spacing / radius)
 * are theme-agnostic (same in both) while color values differ.
 */
export function resolveTheme(theme: V1Theme): ThemeResolution {
  if (theme === 'system') {
    return {
      colors: {
        surface: '$color-surface',
        surface2: '$color-surface-2',
        surface3: '$color-surface-3',
        bgDeep: '$color-bg-deep',
        border: '$color-border',
        borderStrong: '$color-border-strong',
        textPrimary: '$color-text-primary',
        textBody: '$color-text-body',
        textMuted: '$color-text-muted',
        textSubtle: '$color-text-subtle',
        accent: '$color-accent',
        destructive: '$color-destructive',
        success: '$color-success',
      },
      alertColors: {
        infoBg: '$color-info-bg',
        infoText: '$color-info-text',
        successBg: '$color-success-bg',
        successText: '$color-success-text',
        warningBg: '$color-warning-bg',
        warningText: '$color-warning-text',
        dangerBg: '$color-danger-bg',
        dangerText: '$color-danger-text',
      },
      chartColors: {
        chart1: '$color-chart-1',
        chart2: '$color-chart-2',
        chart3: '$color-chart-3',
        chart4: '$color-chart-4',
        chart5: '$color-chart-5',
        chart6: '$color-chart-6',
      },
      typography: {
        displaySize: '$type-display-size',
        displayWeight: '$type-display-weight',
        displayLineHeight: '$type-display-line-height',
        displayLetterSpacing: '$type-display-letter-spacing',
        h1Size: '$type-h1-size',
        h1Weight: '$type-h1-weight',
        h1LineHeight: '$type-h1-line-height',
        h2Size: '$type-h2-size',
        h2Weight: '$type-h2-weight',
        h2LineHeight: '$type-h2-line-height',
        h3Size: '$type-h3-size',
        h3Weight: '$type-h3-weight',
        h3LineHeight: '$type-h3-line-height',
        bodySize: '$type-body-size',
        bodyWeight: '$type-body-weight',
        bodyLineHeight: '$type-body-line-height',
        captionSize: '$type-caption-size',
        captionWeight: '$type-caption-weight',
        captionLineHeight: '$type-caption-line-height',
        uppercaseLetterSpacing: '$type-uppercase-label-letter-spacing',
      },
      spacing: {
        s1: '$spacing-1',
        s2: '$spacing-2',
        s3: '$spacing-3',
        s4: '$spacing-4',
        s5: '$spacing-5',
      },
      radius: {
        sm: '$radius-sm',
        md: '$radius-md',
        lg: '$radius-lg',
      },
    };
  }

  const mode = theme === 'dark' ? SEMANTIC_PALETTE_THEME_DARK : SEMANTIC_PALETTE_THEME_LIGHT;
  const hex = getSemanticPaletteHex(mode);

  return {
    colors: {
      surface: hex['color-surface'],
      surface2: hex['color-surface-2'],
      surface3: hex['color-surface-3'],
      bgDeep: hex['color-bg-deep'],
      border: hex['color-border'],
      borderStrong: hex['color-border-strong'],
      textPrimary: hex['color-text-primary'],
      textBody: hex['color-text-body'],
      textMuted: hex['color-text-muted'],
      textSubtle: hex['color-text-subtle'],
      accent: hex['color-accent'],
      destructive: hex['color-destructive'],
      success: hex['color-success'],
    },
    alertColors: {
      infoBg: hex['color-info-bg'],
      infoText: hex['color-info-text'],
      successBg: hex['color-success-bg'],
      successText: hex['color-success-text'],
      warningBg: hex['color-warning-bg'],
      warningText: hex['color-warning-text'],
      dangerBg: hex['color-danger-bg'],
      dangerText: hex['color-danger-text'],
    },
    chartColors: {
      // Chart colors are single-value (no theme axis) — same in light/dark
      chart1: hex['color-chart-1'],
      chart2: hex['color-chart-2'],
      chart3: hex['color-chart-3'],
      chart4: hex['color-chart-4'],
      chart5: hex['color-chart-5'],
      chart6: hex['color-chart-6'],
    },
    typography: {
      // Numeric tokens: theme-agnostic (same value in light and dark)
      displaySize: 64,
      displayWeight: 700,
      displayLineHeight: 1.0,
      displayLetterSpacing: -0.5,
      h1Size: 24,
      h1Weight: 600,
      h1LineHeight: 1.2,
      h2Size: 20,
      h2Weight: 600,
      h2LineHeight: 1.25,
      h3Size: 16,
      h3Weight: 600,
      h3LineHeight: 1.3,
      bodySize: 14,
      bodyWeight: 400,
      bodyLineHeight: 1.5,
      captionSize: 12,
      captionWeight: 400,
      captionLineHeight: 1.4,
      uppercaseLetterSpacing: 1.5,
    },
    spacing: {
      s1: 4,
      s2: 8,
      s3: 12,
      s4: 16,
      s5: 24,
    },
    radius: {
      sm: 4,
      md: 8,
      lg: 12,
    },
  };
}
