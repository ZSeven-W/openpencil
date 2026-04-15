import type { OrchestratorPlan } from './ai-types';
import type { DesignMdSpec } from '@/types/design-md';

/**
 * Picks a color for the pre-built dashboard sidebar frame.
 *
 * Precedence:
 *   1. "Sidebar Surface" cell from a catalog style guide (legacy path).
 *   2. design.md palette role matching sidebar → panel → surface/card.
 *   3. undefined — caller falls back to rootFrame fill or a neutral default.
 *
 * The design.md path is what keeps dashboards layered when the user
 * supplies their own spec (which deliberately skips the catalog lookup).
 */
export function extractSidebarSurfaceColor(
  plan: OrchestratorPlan,
  designMd?: DesignMdSpec,
): string | undefined {
  const content = plan.selectedStyleGuideContent;
  if (content) {
    const tableMatch = content.match(/Sidebar Surface\s*\|\s*(#[0-9A-Fa-f]{6})/i);
    if (tableMatch) return tableMatch[1].toUpperCase();

    const inlineMatch = content.match(/Sidebar Surface[^#]*(#[0-9A-Fa-f]{6})/i);
    if (inlineMatch) return inlineMatch[1].toUpperCase();
  }

  const palette = designMd?.colorPalette;
  if (palette?.length) {
    const bySidebar = palette.find((c) => /sidebar/i.test(c.role || ''));
    if (bySidebar?.hex) return bySidebar.hex.toUpperCase();
    const byPanel = palette.find((c) => /panel/i.test(c.role || ''));
    if (byPanel?.hex) return byPanel.hex.toUpperCase();
    const bySurface = palette.find((c) => /surface|card/i.test(c.role || ''));
    if (bySurface?.hex) return bySurface.hex.toUpperCase();
  }

  return undefined;
}
