import { describe, it, expect } from 'vitest';
import {
  mapOpenverseResult,
  mapWikimediaPages,
  simplifySearchQuery,
} from '../../../../server/api/ai/image-search';

// ---------------------------------------------------------------------------
// mapOpenverseResult
// ---------------------------------------------------------------------------

describe('mapOpenverseResult', () => {
  it('maps an Openverse result to ImageSearchResult correctly', () => {
    const raw = {
      id: 'abc-123',
      url: 'https://openverse.org/images/abc-123/photo.jpg',
      thumbnail: 'https://openverse.org/thumbs/abc-123/thumb.jpg',
      width: 1920,
      height: 1080,
      license: 'CC BY',
      license_version: '2.0',
      attribution: 'Photo by Artist (CC BY 2.0)',
    };

    const result = mapOpenverseResult(raw);

    expect(result.id).toBe('abc-123');
    expect(result.url).toBe(raw.url);
    expect(result.thumbUrl).toBe(raw.thumbnail);
    expect(result.thumbUrl).toContain('openverse.org');
    expect(result.width).toBe(1920);
    expect(result.height).toBe(1080);
    expect(result.source).toBe('openverse');
    expect(result.license).toBe('CC BY 2.0');
    expect(result.attribution).toBe('Photo by Artist (CC BY 2.0)');
  });

  it('combines license and license_version with a space', () => {
    const raw = {
      id: 'xyz',
      url: 'https://example.com/img.jpg',
      thumbnail: 'https://example.com/thumb.jpg',
      width: 800,
      height: 600,
      license: 'CC0',
      license_version: '1.0',
      attribution: '',
    };

    const result = mapOpenverseResult(raw);
    expect(result.license).toBe('CC0 1.0');
  });

  it('trims license when license_version is empty string', () => {
    const raw = {
      id: 'xyz',
      url: 'https://example.com/img.jpg',
      thumbnail: 'https://example.com/thumb.jpg',
      width: 800,
      height: 600,
      license: 'PDM',
      license_version: '',
      attribution: '',
    };

    const result = mapOpenverseResult(raw);
    expect(result.license).toBe('PDM');
  });
});

// ---------------------------------------------------------------------------
// mapWikimediaPages
// ---------------------------------------------------------------------------

describe('mapWikimediaPages', () => {
  it('maps Wikimedia pages to ImageSearchResult correctly', () => {
    const pages = {
      '12345': {
        pageid: 12345,
        title: 'File:Example.jpg',
        imageinfo: [
          {
            url: 'https://upload.wikimedia.org/wikipedia/commons/a/ab/Example.jpg',
            thumburl:
              'https://upload.wikimedia.org/wikipedia/commons/thumb/a/ab/Example.jpg/800px-Example.jpg',
            width: 1600,
            height: 1200,
            mime: 'image/jpeg',
            extmetadata: {
              LicenseShortName: { value: 'CC BY-SA 4.0' },
            },
          },
        ],
      },
    };

    const results = mapWikimediaPages(pages);

    expect(results).toHaveLength(1);
    const r = results[0];
    expect(r.id).toBe('12345');
    expect(r.url).toContain('wikimedia.org');
    expect(r.thumbUrl).toContain('800px');
    expect(r.width).toBe(1600);
    expect(r.height).toBe(1200);
    expect(r.source).toBe('wikimedia');
    expect(r.license).toBe('CC BY-SA 4.0');
  });

  it('handles pages with no imageinfo gracefully (returns empty)', () => {
    const pages = {
      '99999': {
        pageid: 99999,
        title: 'File:NoInfo.jpg',
        // imageinfo intentionally absent
      },
    };

    const results = mapWikimediaPages(pages);
    expect(results).toHaveLength(0);
  });

  it('handles pages with empty imageinfo array (returns empty)', () => {
    const pages = {
      '88888': {
        pageid: 88888,
        title: 'File:EmptyInfo.jpg',
        imageinfo: [],
      },
    };

    const results = mapWikimediaPages(pages);
    expect(results).toHaveLength(0);
  });

  it('maps multiple pages and skips those with no imageinfo', () => {
    const pages = {
      '1': {
        pageid: 1,
        title: 'File:A.jpg',
        imageinfo: [
          {
            url: 'https://upload.wikimedia.org/a.jpg',
            thumburl: 'https://upload.wikimedia.org/thumb/a.jpg',
            width: 400,
            height: 300,
            mime: 'image/jpeg',
            extmetadata: { LicenseShortName: { value: 'CC0' } },
          },
        ],
      },
      '2': {
        pageid: 2,
        title: 'File:B.jpg',
        // no imageinfo
      },
      '3': {
        pageid: 3,
        title: 'File:C.jpg',
        imageinfo: [
          {
            url: 'https://upload.wikimedia.org/c.jpg',
            thumburl: 'https://upload.wikimedia.org/thumb/c.jpg',
            width: 600,
            height: 400,
            mime: 'image/png',
            extmetadata: { LicenseShortName: { value: 'CC BY 4.0' } },
          },
        ],
      },
    };

    const results = mapWikimediaPages(pages);
    expect(results).toHaveLength(2);
    expect(results.map((r) => r.source)).toEqual(['wikimedia', 'wikimedia']);
    const licenses = results.map((r) => r.license).sort();
    expect(licenses).toContain('CC0');
    expect(licenses).toContain('CC BY 4.0');
  });
});

// ---------------------------------------------------------------------------
// simplifySearchQuery
// ---------------------------------------------------------------------------

describe('simplifySearchQuery', () => {
  it('extracts keywords from verbose AI prompt', () => {
    const result = simplifySearchQuery('delicious burger with fries and fresh vegetables');
    expect(result).toBe('delicious burger fries fresh');
  });

  it('removes stop words', () => {
    const result = simplifySearchQuery('a beautiful photo of the sunset on the beach');
    expect(result).toBe('beautiful photo sunset beach');
  });

  it('limits to 4 keywords', () => {
    const result = simplifySearchQuery(
      'modern office workspace natural lighting wooden desk plants',
    );
    const words = result.split(' ');
    expect(words.length).toBeLessThanOrEqual(4);
  });

  it('handles short queries unchanged', () => {
    const result = simplifySearchQuery('burger');
    expect(result).toBe('burger');
  });

  it('falls back to truncated input when all words are stop words', () => {
    const result = simplifySearchQuery('a the an');
    expect(result.length).toBeGreaterThan(0);
  });
});

// (keep original last test)
describe('mapWikimediaPages (continued)', () => {
  it('falls back to empty string license when extmetadata is missing (original)', () => {
    const pages = {
      '55555': {
        pageid: 55555,
        title: 'File:NoLicense.jpg',
        imageinfo: [
          {
            url: 'https://upload.wikimedia.org/nolicense.jpg',
            thumburl: 'https://upload.wikimedia.org/thumb/nolicense.jpg',
            width: 200,
            height: 150,
            mime: 'image/jpeg',
            // no extmetadata
          },
        ],
      },
    };

    const results = mapWikimediaPages(pages);
    expect(results).toHaveLength(1);
    expect(results[0].license).toBe('');
  });
});
