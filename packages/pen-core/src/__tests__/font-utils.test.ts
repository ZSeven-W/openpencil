import { describe, it, expect } from 'vitest';
import { cssFontFamily } from '../font-utils';

describe('cssFontFamily', () => {
  it('quotes multi-word font names', () => {
    expect(cssFontFamily('Noto Sans SC')).toBe('"Noto Sans SC"');
  });

  it('does not quote generic families', () => {
    expect(cssFontFamily('sans-serif')).toBe('sans-serif');
    expect(cssFontFamily('monospace')).toBe('monospace');
    expect(cssFontFamily('system-ui')).toBe('system-ui');
  });

  it('handles comma-separated lists', () => {
    expect(cssFontFamily('Inter, sans-serif')).toBe('"Inter", sans-serif');
  });

  it('preserves already-quoted names', () => {
    expect(cssFontFamily('"Noto Sans SC"')).toBe('"Noto Sans SC"');
  });

  it('handles -apple-system', () => {
    expect(cssFontFamily('-apple-system')).toBe('-apple-system');
  });
});
