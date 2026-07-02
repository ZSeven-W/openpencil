import { describe, it, expect } from 'vitest';
import { resolveTheme } from '../element-builders/resolve-theme.js';

describe('resolveTheme helper', () => {
  describe('light mode', () => {
    it('returns hex literals matching semantic palette light values', () => {
      const r = resolveTheme('light');
      expect(r.colors.surface).toBe('#FFFFFF');
      expect(r.colors.surface2).toBe('#F1F5F9');
      expect(r.colors.surface3).toBe('#F3F4F6');
      expect(r.colors.bgDeep).toBe('#F8FAFC');
      expect(r.colors.border).toBe('#E2E8F0');
      expect(r.colors.borderStrong).toBe('#CBD5E1');
      expect(r.colors.textPrimary).toBe('#0F172A');
      expect(r.colors.textBody).toBe('#334155');
      expect(r.colors.textMuted).toBe('#64748B');
      expect(r.colors.textSubtle).toBe('#94A3B8');
      expect(r.colors.accent).toBe('#2563EB');
      expect(r.colors.destructive).toBe('#EF4444');
      expect(r.colors.success).toBe('#10B981');
    });

    it('returns alert colors matching light palette', () => {
      const r = resolveTheme('light');
      expect(r.alertColors.infoBg).toBe('#DBEAFE');
      expect(r.alertColors.infoText).toBe('#1E40AF');
      expect(r.alertColors.successBg).toBe('#DCFCE7');
      expect(r.alertColors.successText).toBe('#166534');
      expect(r.alertColors.warningBg).toBe('#FEF3C7');
      expect(r.alertColors.warningText).toBe('#92400E');
      expect(r.alertColors.dangerBg).toBe('#FEE2E2');
      expect(r.alertColors.dangerText).toBe('#991B1B');
    });

    it('returns chart colors (single-value, same in all modes)', () => {
      const r = resolveTheme('light');
      expect(r.chartColors.chart1).toBe('#3B82F6');
      expect(r.chartColors.chart2).toBe('#8B5CF6');
      expect(r.chartColors.chart3).toBe('#EC4899');
      expect(r.chartColors.chart4).toBe('#14B8A6');
      expect(r.chartColors.chart5).toBe('#F59E0B');
      expect(r.chartColors.chart6).toBe('#F97316');
    });

    it('returns numeric typography values', () => {
      const r = resolveTheme('light');
      expect(r.typography.displaySize).toBe(64);
      expect(r.typography.displayWeight).toBe(700);
      expect(r.typography.displayLineHeight).toBe(1.0);
      expect(r.typography.displayLetterSpacing).toBe(-0.5);
      expect(r.typography.h1Size).toBe(24);
      expect(r.typography.h1Weight).toBe(600);
      expect(r.typography.h1LineHeight).toBe(1.2);
      expect(r.typography.h2Size).toBe(20);
      expect(r.typography.h2Weight).toBe(600);
      expect(r.typography.h2LineHeight).toBe(1.25);
      expect(r.typography.h3Size).toBe(16);
      expect(r.typography.h3Weight).toBe(600);
      expect(r.typography.h3LineHeight).toBe(1.3);
      expect(r.typography.bodySize).toBe(14);
      expect(r.typography.bodyWeight).toBe(400);
      expect(r.typography.bodyLineHeight).toBe(1.5);
      expect(r.typography.captionSize).toBe(12);
      expect(r.typography.captionWeight).toBe(400);
      expect(r.typography.captionLineHeight).toBe(1.4);
      expect(r.typography.uppercaseLetterSpacing).toBe(1.5);
    });

    it('returns numeric spacing values', () => {
      const r = resolveTheme('light');
      expect(r.spacing.s1).toBe(4);
      expect(r.spacing.s2).toBe(8);
      expect(r.spacing.s3).toBe(12);
      expect(r.spacing.s4).toBe(16);
      expect(r.spacing.s5).toBe(24);
    });

    it('returns numeric radius values', () => {
      const r = resolveTheme('light');
      expect(r.radius.sm).toBe(4);
      expect(r.radius.md).toBe(8);
      expect(r.radius.lg).toBe(12);
    });
  });

  describe('dark mode', () => {
    it('returns dark hex for color fields', () => {
      const r = resolveTheme('dark');
      expect(r.colors.textPrimary).toBe('#F1F5F9');
      expect(r.colors.surface).toBe('#1E293B');
      expect(r.colors.surface2).toBe('#334155');
      expect(r.colors.bgDeep).toBe('#0F172A');
      expect(r.colors.border).toBe('#334155');
      expect(r.colors.accent).toBe('#60A5FA');
      expect(r.colors.textMuted).toBe('#94A3B8');
    });

    it('returns dark alert colors', () => {
      const r = resolveTheme('dark');
      expect(r.alertColors.infoBg).toBe('#1E3A8A');
      expect(r.alertColors.infoText).toBe('#BFDBFE');
      expect(r.alertColors.dangerBg).toBe('#7F1D1D');
      expect(r.alertColors.dangerText).toBe('#FECACA');
    });

    it('returns same chart colors as light (single-value)', () => {
      const light = resolveTheme('light');
      const dark = resolveTheme('dark');
      expect(dark.chartColors).toEqual(light.chartColors);
    });

    it('returns same numeric typography as light (theme-agnostic)', () => {
      const light = resolveTheme('light');
      const dark = resolveTheme('dark');
      expect(dark.typography).toEqual(light.typography);
    });

    it('returns same numeric spacing as light', () => {
      const light = resolveTheme('light');
      const dark = resolveTheme('dark');
      expect(dark.spacing).toEqual(light.spacing);
    });

    it('returns same numeric radius as light', () => {
      const light = resolveTheme('light');
      const dark = resolveTheme('dark');
      expect(dark.radius).toEqual(light.radius);
    });
  });

  describe('system mode', () => {
    it('returns $color-* refs for all color fields', () => {
      const r = resolveTheme('system');
      expect(r.colors.surface).toBe('$color-surface');
      expect(r.colors.surface2).toBe('$color-surface-2');
      expect(r.colors.surface3).toBe('$color-surface-3');
      expect(r.colors.bgDeep).toBe('$color-bg-deep');
      expect(r.colors.border).toBe('$color-border');
      expect(r.colors.borderStrong).toBe('$color-border-strong');
      expect(r.colors.textPrimary).toBe('$color-text-primary');
      expect(r.colors.textBody).toBe('$color-text-body');
      expect(r.colors.textMuted).toBe('$color-text-muted');
      expect(r.colors.textSubtle).toBe('$color-text-subtle');
      expect(r.colors.accent).toBe('$color-accent');
      expect(r.colors.destructive).toBe('$color-destructive');
      expect(r.colors.success).toBe('$color-success');
    });

    it('returns $color-* refs for alert colors', () => {
      const r = resolveTheme('system');
      expect(r.alertColors.infoBg).toBe('$color-info-bg');
      expect(r.alertColors.infoText).toBe('$color-info-text');
      expect(r.alertColors.successBg).toBe('$color-success-bg');
      expect(r.alertColors.successText).toBe('$color-success-text');
      expect(r.alertColors.warningBg).toBe('$color-warning-bg');
      expect(r.alertColors.warningText).toBe('$color-warning-text');
      expect(r.alertColors.dangerBg).toBe('$color-danger-bg');
      expect(r.alertColors.dangerText).toBe('$color-danger-text');
    });

    it('returns $color-chart-* refs for chart colors', () => {
      const r = resolveTheme('system');
      expect(r.chartColors.chart1).toBe('$color-chart-1');
      expect(r.chartColors.chart2).toBe('$color-chart-2');
      expect(r.chartColors.chart3).toBe('$color-chart-3');
      expect(r.chartColors.chart4).toBe('$color-chart-4');
      expect(r.chartColors.chart5).toBe('$color-chart-5');
      expect(r.chartColors.chart6).toBe('$color-chart-6');
    });

    it('returns $type-* refs for all typography fields', () => {
      const r = resolveTheme('system');
      expect(r.typography.displaySize).toBe('$type-display-size');
      expect(r.typography.displayWeight).toBe('$type-display-weight');
      expect(r.typography.displayLineHeight).toBe('$type-display-line-height');
      expect(r.typography.displayLetterSpacing).toBe('$type-display-letter-spacing');
      expect(r.typography.h1Size).toBe('$type-h1-size');
      expect(r.typography.h1Weight).toBe('$type-h1-weight');
      expect(r.typography.h1LineHeight).toBe('$type-h1-line-height');
      expect(r.typography.h2Size).toBe('$type-h2-size');
      expect(r.typography.h2Weight).toBe('$type-h2-weight');
      expect(r.typography.h2LineHeight).toBe('$type-h2-line-height');
      expect(r.typography.h3Size).toBe('$type-h3-size');
      expect(r.typography.h3Weight).toBe('$type-h3-weight');
      expect(r.typography.h3LineHeight).toBe('$type-h3-line-height');
      expect(r.typography.bodySize).toBe('$type-body-size');
      expect(r.typography.bodyWeight).toBe('$type-body-weight');
      expect(r.typography.bodyLineHeight).toBe('$type-body-line-height');
      expect(r.typography.captionSize).toBe('$type-caption-size');
      expect(r.typography.captionWeight).toBe('$type-caption-weight');
      expect(r.typography.captionLineHeight).toBe('$type-caption-line-height');
      expect(r.typography.uppercaseLetterSpacing).toBe('$type-uppercase-label-letter-spacing');
    });

    it('returns $spacing-* refs for all spacing fields', () => {
      const r = resolveTheme('system');
      expect(r.spacing.s1).toBe('$spacing-1');
      expect(r.spacing.s2).toBe('$spacing-2');
      expect(r.spacing.s3).toBe('$spacing-3');
      expect(r.spacing.s4).toBe('$spacing-4');
      expect(r.spacing.s5).toBe('$spacing-5');
    });

    it('returns $radius-* refs for all radius fields', () => {
      const r = resolveTheme('system');
      expect(r.radius.sm).toBe('$radius-sm');
      expect(r.radius.md).toBe('$radius-md');
      expect(r.radius.lg).toBe('$radius-lg');
    });
  });
});
