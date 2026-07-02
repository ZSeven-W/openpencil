import { describe, it, expect } from 'vitest';
import type { PenNode } from '@zseven-w/pen-types';
import { computeLayoutPositions } from '../layout/engine.js';
import {
  assignIdsRecursively,
  buildBottomNav,
  buildCardRow,
  buildCheckbox,
  buildDivider,
  buildFormField,
  buildHeading,
  buildLink,
  buildListRow,
  buildStatGrid,
  buildSwitch,
  buildTextButton,
  buildTopNavBar,
  type ElementTree,
} from '../element-builders/index.js';

/**
 * Composition pipeline — builds complete screens from multiple
 * builders and asserts the assembled tree survives pen-core's
 * layout engine end-to-end. Tests "N builders cooperate in a real
 * layout" which is what the embedded orchestrator actually drives
 * when the AI emits multi-tag responses.
 *
 * Each case:
 *   1. Assemble builders into a screen frame (vertical stack)
 *   2. Stamp fresh ids across the whole tree
 *   3. Run computeLayoutPositions at every level
 *   4. Assert: no throws, expected role presence, total node count
 */

function countNodes(n: PenNode): number {
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  return 1 + children.reduce((s, c) => s + countNodes(c), 0);
}

function collectRoles(n: PenNode, out: string[] = []): string[] {
  const role = (n as PenNode & { role?: string }).role;
  if (role) out.push(role);
  const children = (n as PenNode & { children?: PenNode[] }).children ?? [];
  for (const c of children) collectRoles(c, out);
  return out;
}

/**
 * Walk layout recursively so every container runs
 * computeLayoutPositions, not just the top level.
 */
function layoutRecursive(parent: PenNode): void {
  const children = (parent as PenNode & { children?: PenNode[] }).children ?? [];
  if (children.length === 0) return;
  const positioned = computeLayoutPositions(parent, children);
  for (const c of positioned) {
    layoutRecursive(c);
  }
}

describe('composition — login screen', () => {
  it('assembles heading + 2 form fields + button + link + survives layout', () => {
    const screen: PenNode = {
      id: 'login-screen',
      type: 'frame',
      name: 'Login',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      layout: 'vertical',
      padding: [48, 24],
      gap: 20,
      children: [
        buildHeading({ content: 'Welcome back', level: 'h1' }),
        buildFormField({ label: 'Email', placeholder: 'you@example.com', leading_icon: 'mail' }),
        buildFormField({
          label: 'Password',
          leading_icon: 'lock',
          trailing_icon: 'eye',
          required: true,
        }),
        buildTextButton({ label: 'Sign in' }),
        buildLink({ label: 'Forgot password?' }),
      ] as unknown as PenNode[],
    } as unknown as PenNode;

    assignIdsRecursively(screen as unknown as ElementTree);
    expect(() => layoutRecursive(screen)).not.toThrow();

    const roles = collectRoles(screen);
    expect(roles).toContain('heading');
    expect(roles.filter((r) => r === 'form-field')).toHaveLength(2);
    expect(roles).toContain('form-input');
    expect(roles).toContain('button');
    expect(roles).toContain('link');
    expect(countNodes(screen)).toBeGreaterThan(10);
  });
});

describe('composition — dashboard screen', () => {
  it('top nav + stat grid + card row + bottom nav stacked + layout clean', () => {
    const screen: PenNode = {
      id: 'dash-screen',
      type: 'frame',
      name: 'Dashboard',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      layout: 'vertical',
      gap: 0,
      children: [
        buildTopNavBar({ title: 'Home', trailing_icon: 'bell' }),
        buildStatGrid({
          items: [
            { value: '8,432', label: 'Steps', icon: 'activity' },
            { value: '512', label: 'Kcal', icon: 'flame' },
            { value: '7h', label: 'Sleep', icon: 'moon' },
          ],
        }),
        buildCardRow({
          items: [
            { title: 'Hiit', subtitle: '30 min', icon: 'flame' },
            { title: 'Strength', subtitle: '45 min', icon: 'dumbbell' },
            { title: 'Yoga', subtitle: '25 min', icon: 'leaf' },
          ],
        }),
        buildBottomNav({
          items: [
            { title: 'Home', icon: 'home', active: true },
            { title: 'Search', icon: 'search' },
            { title: 'Profile', icon: 'user' },
          ],
        }),
      ] as unknown as PenNode[],
    } as unknown as PenNode;

    assignIdsRecursively(screen as unknown as ElementTree);
    expect(() => layoutRecursive(screen)).not.toThrow();

    const roles = collectRoles(screen);
    expect(roles).toContain('top-nav-bar');
    expect(roles).toContain('stat-grid');
    expect(roles.filter((r) => r === 'stat-cell')).toHaveLength(3);
    expect(roles).toContain('scroll-row-wrapper');
    expect(roles.filter((r) => r === 'card')).toHaveLength(3);
    expect(roles).toContain('bottom-tab-bar');
    expect(roles.filter((r) => r === 'nav-item-active' || r === 'nav-item')).toHaveLength(3);
  });
});

describe('composition — settings screen', () => {
  it('top nav + 5 list rows w/ interleaved dividers + trailing switch row + clean layout', () => {
    const screen: PenNode = {
      id: 'settings-screen',
      type: 'frame',
      name: 'Settings',
      x: 0,
      y: 0,
      width: 375,
      height: 812,
      layout: 'vertical',
      gap: 0,
      children: [
        buildTopNavBar({ title: 'Settings', leading_icon: 'chevron-left' }),
        buildListRow({
          title: 'Notifications',
          subtitle: 'Push, email, and in-app',
          leading_icon: 'bell',
          trailing_icon: 'chevron-right',
        }),
        buildDivider({}),
        buildListRow({
          title: 'Privacy',
          leading_icon: 'shield',
          trailing_icon: 'chevron-right',
        }),
        buildDivider({}),
        buildListRow({ title: 'Dark mode', leading_icon: 'moon' }),
        buildDivider({}),
        buildCheckbox({ label: 'Accept terms', checked: true }),
        buildSwitch({ active: true }),
      ] as unknown as PenNode[],
    } as unknown as PenNode;

    assignIdsRecursively(screen as unknown as ElementTree);
    expect(() => layoutRecursive(screen)).not.toThrow();

    const roles = collectRoles(screen);
    expect(roles).toContain('top-nav-bar');
    expect(roles.filter((r) => r === 'list-row')).toHaveLength(3);
    expect(roles.filter((r) => r === 'divider')).toHaveLength(3);
    expect(roles).toContain('checkbox-checked');
    expect(roles).toContain('switch');
  });
});
