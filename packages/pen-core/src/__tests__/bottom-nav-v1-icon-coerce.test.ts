import { describe, it, expect, beforeEach } from 'vitest';
import {
  clearCoerceWarnings,
  getCoerceWarnings,
  coerceNavTabIcon,
} from '../element-builders/coerce-params.js';
import { buildBottomNavV1 } from '../element-builders/bottom-nav-v1.js';
import type { ElementTree } from '../element-builders/helpers.js';

describe('coerceNavTabIcon', () => {
  beforeEach(() => clearCoerceWarnings());

  it('swaps shopping-bag → shopping-cart for a Cart tab + emits warning', () => {
    const result = coerceNavTabIcon('Cart', 'shopping-bag', 'test');
    expect(result).toBe('shopping-cart');
    const warnings = getCoerceWarnings();
    expect(warnings).toHaveLength(1);
    expect(warnings[0].param).toBe('icon');
    expect(warnings[0].fallback).toBe('shopping-cart');
  });

  it('passes shopping-cart through without warning when already canonical', () => {
    const result = coerceNavTabIcon('Cart', 'shopping-cart', 'test');
    expect(result).toBe('shopping-cart');
    expect(getCoerceWarnings()).toHaveLength(0);
  });

  it('passes a custom icon through (e.g. package for delivery-app cart)', () => {
    // package IS in the KNOWN_WRONG_ALTS for shopping-cart, so it gets
    // swapped. Pick a name that is NOT in the wrong-alts list.
    const result = coerceNavTabIcon('Cart', 'rocket', 'test');
    expect(result).toBe('rocket');
    expect(getCoerceWarnings()).toHaveLength(0);
  });

  it('swaps profile / account / avatar → user for a Profile tab', () => {
    expect(coerceNavTabIcon('Profile', 'profile', 'test')).toBe('user');
    expect(coerceNavTabIcon('Profile', 'account', 'test')).toBe('user');
    expect(coerceNavTabIcon('Profile', 'avatar', 'test')).toBe('user');
  });

  it('swaps notification → bell for a Notifications tab', () => {
    expect(coerceNavTabIcon('Notifications', 'notification', 'test')).toBe('bell');
  });

  it('handles Chinese title 购物车 → shopping-cart', () => {
    const result = coerceNavTabIcon('购物车', 'shopping-bag', 'test');
    expect(result).toBe('shopping-cart');
  });

  it('handles case-insensitive title matching', () => {
    expect(coerceNavTabIcon('cart', 'shopping-bag', 'test')).toBe('shopping-cart');
    expect(coerceNavTabIcon('CART', 'shopping-bag', 'test')).toBe('shopping-cart');
    expect(coerceNavTabIcon('  Cart  ', 'shopping-bag', 'test')).toBe('shopping-cart');
  });

  it('returns icon untouched for an unknown title (e.g. "Custom")', () => {
    expect(coerceNavTabIcon('Custom', 'rocket', 'test')).toBe('rocket');
    expect(getCoerceWarnings()).toHaveLength(0);
  });
});

describe('buildBottomNavV1 — icon canonicalisation through the builder', () => {
  beforeEach(() => clearCoerceWarnings());

  it('rewrites Cart tab icon shopping-bag → shopping-cart in the emitted tree', () => {
    const tree = buildBottomNavV1({
      items: [
        { title: 'Home', icon: 'home' },
        { title: 'Cart', icon: 'shopping-bag' },
        { title: 'Profile', icon: 'user' },
      ],
    });
    // Tab 1 (Cart) → child 0 = icon_font, iconFontName should be canonical
    const tabs = (tree as { children: ElementTree[] }).children;
    const cartTab = tabs[1] as { children: ElementTree[] };
    const cartIcon = cartTab.children[0] as { iconFontName?: string };
    expect(cartIcon.iconFontName).toBe('shopping-cart');
    // Warning was emitted by the builder
    expect(getCoerceWarnings().length).toBeGreaterThanOrEqual(1);
  });

  it('does not change explicitly-correct icons', () => {
    const tree = buildBottomNavV1({
      items: [
        { title: 'Home', icon: 'house' },
        { title: 'Cart', icon: 'shopping-cart' },
      ],
    });
    const tabs = (tree as { children: ElementTree[] }).children;
    const homeIcon = (tabs[0] as { children: ElementTree[] }).children[0] as {
      iconFontName?: string;
    };
    const cartIcon = (tabs[1] as { children: ElementTree[] }).children[0] as {
      iconFontName?: string;
    };
    expect(homeIcon.iconFontName).toBe('house');
    expect(cartIcon.iconFontName).toBe('shopping-cart');
    expect(getCoerceWarnings()).toHaveLength(0);
  });
});
