import { describe, it, expect } from 'vitest';
import { parseDesignMd, generateDesignMd, designMdColorsToVariables } from '../design-md-parser';

describe('parseDesignMd', () => {
  it('should parse a basic design.md with color palette', () => {
    const md = `# My App\n\n## Color Palette\n- primary: #3B82F6 (main brand)\n- secondary: #10B981 (accents)\n\n## Typography\nFont family: Inter\nHeadings: 700 weight\nBody: 400 weight\nScale: 1.25\n`;
    const result = parseDesignMd(md);
    expect(result.projectName).toBe('My App');
    expect(result.colorPalette).toHaveLength(2);
    expect(result.colorPalette![0]).toMatchObject({ name: 'primary', hex: '#3B82F6' });
    expect(result.typography?.fontFamily).toBe('Inter');
  });

  it('should return empty spec for empty input', () => {
    const result = parseDesignMd('');
    expect(result.projectName).toBeUndefined();
    expect(result.colorPalette).toEqual([]);
  });

  it('should handle fuzzy section headers', () => {
    const md = `# Test\n\n## Visual Theme & Atmosphere\nMinimal, clean\n\n## Colour Roles\n- bg: #FFFFFF (background)\n`;
    const result = parseDesignMd(md);
    expect(result.visualTheme).toContain('Minimal');
    expect(result.colorPalette).toHaveLength(1);
  });
});

describe('generateDesignMd', () => {
  it('should roundtrip parse → generate → parse', () => {
    const md = `# Roundtrip\n\n## Color Palette\n- primary: #FF0000 (brand)\n- text: #333333 (body text)\n\n## Typography\nFont family: Roboto\nHeadings: 700 weight\nBody: 400 weight\nScale: 1.2\n`;
    const parsed = parseDesignMd(md);
    const generated = generateDesignMd(parsed);
    const reparsed = parseDesignMd(generated);
    expect(reparsed.colorPalette).toHaveLength(parsed.colorPalette!.length);
    expect(reparsed.typography?.fontFamily).toBe(parsed.typography?.fontFamily);
  });
});

describe('designMdColorsToVariables', () => {
  it('should convert colors to variable definitions', () => {
    const colors = [
      { name: 'primary', hex: '#3B82F6', role: 'main brand' },
      { name: 'bg', hex: '#FFFFFF', role: 'background' },
    ];
    const vars = designMdColorsToVariables(colors);
    expect(vars['primary']).toMatchObject({ type: 'color', value: '#3B82F6' });
    expect(vars['bg']).toMatchObject({ type: 'color', value: '#FFFFFF' });
  });
});
