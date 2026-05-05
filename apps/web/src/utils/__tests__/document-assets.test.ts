import { describe, expect, it } from 'vitest';

import {
  isLocalAssetPath,
  toLocalAssetBridgeUrl,
  resolveDocumentAssetPath,
  resolveRuntimeAssetSource,
  toStoredAssetPath,
} from '../document-assets';

describe('document asset paths', () => {
  it('resolves relative asset paths against the document location', () => {
    expect(
      resolveDocumentAssetPath(
        'C:/Users/fangx/Desktop/project/layout.pen',
        './assets/hero image.png',
      ),
    ).toBe('C:/Users/fangx/Desktop/project/assets/hero image.png');
  });

  it('converts local asset paths into runtime file urls', () => {
    expect(
      resolveRuntimeAssetSource(
        './assets/hero image.png',
        'C:/Users/fangx/Desktop/project/layout.pen',
      ),
    ).toEqual({
      sourcePath: 'C:/Users/fangx/Desktop/project/assets/hero image.png',
      runtimeUrl: 'file:///C:/Users/fangx/Desktop/project/assets/hero%20image.png',
      isLocal: true,
      unresolved: false,
    });

    expect(resolveRuntimeAssetSource('D:\\shared\\mountain.png', null)).toEqual({
      sourcePath: 'D:/shared/mountain.png',
      runtimeUrl: 'file:///D:/shared/mountain.png',
      isLocal: true,
      unresolved: false,
    });
  });

  it('keeps remote and inline assets unchanged at runtime', () => {
    expect(resolveRuntimeAssetSource('https://example.com/hero.png', null)).toEqual({
      sourcePath: null,
      runtimeUrl: 'https://example.com/hero.png',
      isLocal: false,
      unresolved: false,
    });

    expect(resolveRuntimeAssetSource('data:image/png;base64,abc', null)).toEqual({
      sourcePath: null,
      runtimeUrl: 'data:image/png;base64,abc',
      isLocal: false,
      unresolved: false,
    });
  });

  it('marks relative assets unresolved when there is no document path', () => {
    expect(resolveRuntimeAssetSource('./assets/hero.png', null)).toEqual({
      sourcePath: null,
      runtimeUrl: null,
      isLocal: true,
      unresolved: true,
    });
  });

  it('does not bridge unsupported local asset paths that cannot be served as images', () => {
    expect(
      resolveRuntimeAssetSource(
        'C:/Users/fangx/Desktop/project/assets/page-18.pdf',
        'C:/Users/fangx/Desktop/project/layout.pen',
      ),
    ).toEqual({
      sourcePath: 'C:/Users/fangx/Desktop/project/assets/page-18.pdf',
      runtimeUrl: null,
      isLocal: true,
      unresolved: false,
    });
  });

  it('keeps extensionless local asset paths bridgeable so the server can infer image type', () => {
    expect(
      resolveRuntimeAssetSource('./assets/hero', 'C:/Users/fangx/Desktop/project/layout.pen'),
    ).toEqual({
      sourcePath: 'C:/Users/fangx/Desktop/project/assets/hero',
      runtimeUrl: 'file:///C:/Users/fangx/Desktop/project/assets/hero',
      isLocal: true,
      unresolved: false,
    });
  });

  it('stores imported assets as relative paths when the document path allows it', () => {
    expect(
      toStoredAssetPath(
        'C:/Users/fangx/Desktop/project/assets/hero.png',
        'C:/Users/fangx/Desktop/project/layout.pen',
      ),
    ).toBe('assets/hero.png');

    expect(
      toStoredAssetPath('D:/shared/hero.png', 'C:/Users/fangx/Desktop/project/layout.pen'),
    ).toBe('D:/shared/hero.png');
  });

  it('detects local asset paths without misclassifying web urls', () => {
    expect(isLocalAssetPath('./assets/hero.png')).toBe(true);
    expect(isLocalAssetPath('C:/assets/hero.png')).toBe(true);
    expect(isLocalAssetPath('file:///C:/assets/hero.png')).toBe(true);
    expect(isLocalAssetPath('https://example.com/hero.png')).toBe(false);
    expect(isLocalAssetPath('data:image/png;base64,abc')).toBe(false);
  });

  it('bridges local assets through the app origin when running over http', () => {
    const originalWindow = globalThis.window;
    Object.defineProperty(globalThis, 'window', {
      configurable: true,
      value: {
        location: {
          origin: 'http://localhost:3000',
          protocol: 'http:',
        },
      },
    });

    try {
      expect(toLocalAssetBridgeUrl('C:/Users/fangx/Desktop/project/assets/hero image.png')).toBe(
        'http://localhost:3000/api/local-asset?path=C%3A%2FUsers%2Ffangx%2FDesktop%2Fproject%2Fassets%2Fhero%20image.png',
      );
    } finally {
      if (originalWindow === undefined) {
        delete (globalThis as { window?: typeof globalThis.window }).window;
      } else {
        Object.defineProperty(globalThis, 'window', {
          configurable: true,
          value: originalWindow,
        });
      }
    }
  });
});
