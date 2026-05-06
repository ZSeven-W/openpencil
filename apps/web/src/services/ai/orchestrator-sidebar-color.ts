import type { OrchestratorPlan } from './ai-types';
import type { DesignMdSpec } from '@/types/design-md';

/**
 * Picks 预建仪表板侧
 *
 * 边栏框架的颜色。 Precedence：1. 目录样式指南中的“Sidebar
 * Surface”单元格（
 * 旧路径）。 2.desi
 * gn.md 调色板角色匹配侧边栏→面板→surface/card。 3. 未定义 — 调用者退回到 rootFrame
 *
 * 填充或中性默认值。 The design.md 路径是当用户提供自己的规范（故意跳过目录查找）时保持仪表板分层的路径。
 *
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
