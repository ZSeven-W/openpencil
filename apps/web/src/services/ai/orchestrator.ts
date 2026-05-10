/**
 * Orchestrator for parallel design generation.
 *
 * Flow:
 * 1. Fast "architect" API call decomposes the prompt into spatial sub-tasks
 * 2. Root frame is created on canvas
 * 3. Multiple sub-agents execute in parallel, each streaming JSONL
 * 4. Nodes are inserted to canvas in real-time with animation
 * 5. Post-generation screenshot validation (optional, requires API key)
 *
 * Falls back to single-call generation on any orchestrator failure.
 */

import type { PenNode, FrameNode } from '@/types/pen';
import type {
  AIDesignRequest,
  OrchestratorPlan,
  OrchestrationProgress,
  SubAgentResult,
} from './ai-types';
import type { DesignMdSpec } from '@/types/design-md';
import type { VariableDefinition } from '@/types/variables';
import { streamChat } from './ai-service';
import { resolveSkills } from '@zseven-w/pen-ai-skills';
import { styleGuideRegistry } from '@zseven-w/pen-ai-skills/_generated/style-guide-registry';
import { extractStyleGuideValues, selectStyleGuide } from '@zseven-w/pen-ai-skills/style-guide';
import {
  getOrchestratorTimeouts,
  prepareDesignPrompt,
  buildFallbackPlanFromPrompt,
  buildPlanningStyleGuideContext,
  buildCompactPlanningPrompt,
  getBuiltinPlanningTimeouts,
  DESIGN_MD_STYLE_GUIDE_NAME,
} from './orchestrator-prompt-optimizer';
import {
  adjustRootFrameHeightToContent,
  insertStreamingNode,
  resetGenerationRemapping,
  setGenerationContextHint,
  setGenerationCanvasWidth,
  getGenerationRemappedIds,
  getGenerationRootFrameId,
} from './design-generator';
import { setGenerationRootFrameId } from './design-canvas-ops';
import { useDocumentStore, DEFAULT_FRAME_ID, createEmptyDocument } from '@/stores/document-store';
import { useHistoryStore } from '@/stores/history-store';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';
import { resetAnimationState } from './design-animation';
import { VALIDATION_ENABLED } from './ai-runtime-config';
import { runPostGenerationValidation } from './design-validation';
import { scanAndFillImages } from './image-search-pipeline';
import { executeSubAgents } from './orchestrator-sub-agent';
import { isMobileFullScreen } from './orchestrator-plan-classify';
import { emitProgress, buildFinalStepTags } from './orchestrator-progress';
import { assignAgentIdentities } from './agent-identity';
import { addAgentFrame, clearAgentIndicators } from '@/canvas/agent-indicator';
import { createMobileStatusBar, inferStatusBarVariant } from './mobile-status-bar';
import { resolveModelProfile } from './model-profiles';
import { filterPlanningSkillsForPrompt, parseOrchestratorResponse } from './orchestrator-planning';
import { extractSidebarSurfaceColor } from './orchestrator-sidebar-color';

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

const STATUS_BAR_NAME_RE =
  /status\s*[-_]?\s*bar|状态栏|system\s*[-_]?\s*(bar|chrome)|phone\s*(status|chrome)|ios\s*(bar|status)/i;

/**
 * Removes AI-generated status-bar frames from root frames.
 * Re-reads from the store after each removal to avoid stale references.
 * Nodes with `role === 'status-bar'` (our pre-injected bar) are kept.
 */
function removeDuplicateStatusBars(rootNodes: FrameNode[]): void {
  const store = useDocumentStore.getState();
  for (const rn of rootNodes) {
    // Re-read after each removal pass to get fresh children
    let removed = true;
    while (removed) {
      removed = false;
      const root = store.getNodeById(rn.id);
      if (!root || root.type !== 'frame' || !root.children) break;
      for (const child of root.children) {
        if (isAIDuplicateStatusBar(child)) {
          store.removeNode(child.id);
          removed = true;
          break; // restart scan with fresh children
        }
        // One level deeper (e.g. inside a "Header" wrapper)
        if ('children' in child && Array.isArray(child.children)) {
          for (const gc of child.children) {
            if (isAIDuplicateStatusBar(gc)) {
              store.removeNode(gc.id);
              removed = true;
              break;
            }
          }
          if (removed) break;
        }
      }
    }
  }
}

/**
 * Type 0 component plans always have exactly 1 subtask, and the sub-agent
 * conventionally emits a "section root frame" inside the orchestrator's
 * pre-inserted page rootFrame. For multi-section pages that wrapper is the
 * section container; for a Type 0 single-component design it's a redundant
 * "Notification Card → Notification Card" double wrap that shows up in the
 * layers panel and inflates layout depth for no visual benefit.
 *
 * This pass runs only for component-shaped plans (narrow + auto-height).
 * If the orchestrator rootFrame ends up with exactly one child frame whose
 * id was assigned the sub-agent section-root suffix (`-root` / `-section`)
 * OR whose name copies the parent name, hoist that child's children up to
 * the rootFrame and delete the wrapper. Conservative on any other shape —
 * never touches multi-section pages.
 */
/**
 * Pure-function check (no store / no mutation) — returns true iff the
 * given root + plan satisfy the unwrap preconditions described above.
 * Extracted for unit testability; the integration path below feeds in
 * the live store-read root and applies the actual hoist + delete.
 */
export function shouldUnwrapSingleComponentSectionRoot(
  plan: OrchestratorPlan,
  root: PenNode,
): boolean {
  if (plan.subtasks.length !== 1) return false;
  if (plan.rootFrame.width > 480) return false;
  if (typeof plan.rootFrame.height === 'number' && plan.rootFrame.height >= 480) return false;
  if (!root || root.type !== 'frame' || !('children' in root) || !root.children) return false;
  if (root.children.length !== 1) return false;
  const wrapper = root.children[0];
  if (wrapper.type !== 'frame') return false;
  const wrapperChildren =
    'children' in wrapper && Array.isArray(wrapper.children) ? wrapper.children : [];
  if (wrapperChildren.length === 0) return false;
  const wrapperName = ('name' in wrapper ? (wrapper as { name?: string }).name : '') ?? '';
  const wrapperId = wrapper.id;
  const rootName = ('name' in root ? (root as { name?: string }).name : '') ?? '';
  return (
    wrapperId.endsWith('-root') ||
    wrapperId.endsWith('-section') ||
    (rootName !== '' && wrapperName === rootName)
  );
}

function unwrapSingleComponentSectionRoot(rootNodes: FrameNode[], plan: OrchestratorPlan): void {
  const store = useDocumentStore.getState();
  for (const rn of rootNodes) {
    const root = store.getNodeById(rn.id);
    if (!root) continue;
    if (!shouldUnwrapSingleComponentSectionRoot(plan, root)) continue;
    const wrapper = (root as { children: PenNode[] }).children[0];
    const wrapperChildren = (wrapper as { children: PenNode[] }).children;
    // Snapshot children ids in order before mutation, then move each one
    // up to the rootFrame and remove the wrapper. moveNode preserves
    // the relative order via the explicit index argument.
    const childIds = wrapperChildren.map((c) => c.id);
    childIds.forEach((cid, i) => store.moveNode(cid, root.id, i));
    store.removeNode(wrapper.id);
  }
}

function isAIDuplicateStatusBar(node: PenNode): boolean {
  if (node.type !== 'frame') return false;
  if ('role' in node && (node as { role?: string }).role === 'status-bar') return false;
  const name = ('name' in node ? (node as { name?: string }).name : '') ?? '';
  return STATUS_BAR_NAME_RE.test(name);
}

function isSidebarSubtask(
  subtask: Pick<OrchestratorPlan['subtasks'][number], 'id' | 'label' | 'elements'>,
): boolean {
  const text = `${subtask.id} ${subtask.label} ${subtask.elements ?? ''}`.toLowerCase();
  return /(sidebar|side\s*bar|navigation|nav|menu)/.test(text) && !/(top\s*bar|header)/.test(text);
}

function isMainContentContainerSubtask(
  subtask: Pick<OrchestratorPlan['subtasks'][number], 'id' | 'label' | 'elements'>,
): boolean {
  const text = `${subtask.id} ${subtask.label} ${subtask.elements ?? ''}`.toLowerCase();
  return (
    /(main\s*content|content\s*area|main\s*area|content\s*column)/.test(text) &&
    !/(metric|chart|table|transaction|customer|analytics|revenue|growth|sidebar)/.test(text)
  );
}

function isDashboardLikePrompt(prompt: string, plan: OrchestratorPlan): boolean {
  const text = `${prompt}\n${plan.subtasks
    .map((st) => `${st.id} ${st.label} ${st.elements ?? ''}`)
    .join('\n')}`.toLowerCase();
  return /(dashboard|admin|analytics|fintech|workspace|data)/.test(text);
}

function shouldUseDashboardColumns(prompt: string, plan: OrchestratorPlan): boolean {
  if (plan.rootFrame.width <= 480) return false;

  const dashboardLike = isDashboardLikePrompt(prompt, plan);
  const hasSidebar = plan.subtasks.some((st) => isSidebarSubtask(st));
  const hasMainPanels = plan.subtasks.some((st) =>
    /(metric|chart|table|transaction|customer|revenue|growth|analytics|list)/.test(
      `${st.label} ${st.elements ?? ''}`.toLowerCase(),
    ),
  );

  return dashboardLike && hasSidebar && hasMainPanels;
}

function inferDashboardSectionHeight(
  subtask: Pick<OrchestratorPlan['subtasks'][number], 'id' | 'label' | 'elements'>,
): number {
  const text = `${subtask.id} ${subtask.label} ${subtask.elements ?? ''}`.toLowerCase();
  if (isSidebarSubtask(subtask)) return 760;
  if (/(top\s*header|top\s*bar|header)/.test(text)) return 96;
  if (/(metric|kpi)/.test(text)) return 160;
  if (/(chart|revenue)/.test(text)) return 320;
  if (/(transaction|activity|feed)/.test(text)) return 320;
  if (/(table|analytics|customer)/.test(text)) return 340;
  return 160;
}

function inferDashboardSectionWidth(
  subtask: Pick<OrchestratorPlan['subtasks'][number], 'id' | 'label' | 'elements'>,
  rootWidth: number,
): number {
  const sidebarWidth = 260;
  const mainWidth = Math.max(320, rootWidth - sidebarWidth);
  const text = `${subtask.id} ${subtask.label} ${subtask.elements ?? ''}`.toLowerCase();
  if (isSidebarSubtask(subtask)) return sidebarWidth;
  if (/(chart|revenue)/.test(text)) return Math.round(mainWidth * 0.62);
  if (/(transaction|activity|feed)/.test(text)) return Math.round(mainWidth * 0.38);
  return mainWidth;
}

function normalizeOrchestratorPlan(plan: OrchestratorPlan, prompt: string): void {
  if (!Array.isArray(plan.subtasks) || plan.subtasks.length === 0) return;

  const rootWidth =
    typeof plan.rootFrame.width === 'number' && plan.rootFrame.width > 0
      ? plan.rootFrame.width
      : 1200;
  const dashboardLike = isDashboardLikePrompt(prompt, plan);

  plan.rootFrame.width = rootWidth;
  if (plan.rootFrame.height == null || Number(plan.rootFrame.height) < 0) {
    plan.rootFrame.height = 0;
  }

  for (const st of plan.subtasks) {
    if (!st.region) {
      st.region = { width: rootWidth, height: 160 };
      continue;
    }

    if (dashboardLike) {
      const inferredWidth = inferDashboardSectionWidth(st, rootWidth);
      const inferredHeight = inferDashboardSectionHeight(st);
      st.region.width = inferredWidth;

      if (!(typeof st.region.height === 'number') || st.region.height <= 0) {
        st.region.height = inferredHeight;
      } else {
        const minHeight = Math.round(inferredHeight * 0.6);
        const maxHeight = Math.round(inferredHeight * 1.6);
        st.region.height = Math.max(minHeight, Math.min(st.region.height, maxHeight));
      }
      continue;
    }

    if (!(typeof st.region.width === 'number') || st.region.width <= 0) {
      st.region.width = rootWidth;
    }
    if (!(typeof st.region.height === 'number') || st.region.height <= 0) {
      st.region.height = 160;
    }
  }
}

function createDashboardColumnFrames(
  plan: OrchestratorPlan,
  rootId: string,
  designMd?: DesignMdSpec,
): {
  sidebar: FrameNode;
  main: FrameNode;
} {
  const sidebarFillColor =
    extractSidebarSurfaceColor(plan, designMd) ??
    (plan.rootFrame.fill as Array<{ color?: string }> | undefined)?.[0]?.color ??
    '#0F172A';
  const contentGap =
    typeof plan.rootFrame.gap === 'number' && plan.rootFrame.gap > 0 ? plan.rootFrame.gap : 20;

  return {
    sidebar: {
      id: `${rootId}-sidebar`,
      type: 'frame',
      name: 'Sidebar',
      width: 260,
      height: 'fit_content',
      layout: 'vertical',
      fill: [{ type: 'solid', color: sidebarFillColor }],
      children: [],
    },
    main: {
      id: `${rootId}-main`,
      type: 'frame',
      name: 'Main Content',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      gap: contentGap,
      children: [],
    },
  };
}

function normalizeDashboardMainSubtasks(plan: OrchestratorPlan): void {
  const mainSubtasks = plan.subtasks.filter((st) => !isSidebarSubtask(st));
  if (mainSubtasks.length < 2) return;

  const container = mainSubtasks.find((st) => isMainContentContainerSubtask(st));
  if (!container) return;

  const rawElements = container.elements ?? '';
  const cleanedElements = rawElements
    .replace(/[,;]?\s*metrics?\s*(cards?\s*)?(row|container)[^,.;\]]*/gi, '')
    .replace(/\s{2,}/g, ' ')
    .trim()
    .replace(/[;,]\s*$/, '');
  const topBarElements =
    cleanedElements.match(/top\s*bar[^.;\]]*/i)?.[0]?.trim() ??
    cleanedElements ??
    'Top bar with page title, date range selector, and export button';

  container.label = 'Top Bar';
  container.elements = topBarElements;
  container.region.height = Math.max(88, Math.min(container.region.height || 88, 120));
}

function groupDashboardMainRows(plan: OrchestratorPlan): {
  rows: Array<OrchestratorPlan['subtasks']>;
  fullWidth: number;
  rowGap: number;
} {
  const mainSubtasks = plan.subtasks.filter((st) => !isSidebarSubtask(st));
  const fullWidth = Math.max(
    plan.rootFrame.width - 260,
    ...mainSubtasks.map((st) => (st.region.width > 0 ? st.region.width : 0)),
  );
  const rowGap =
    typeof plan.rootFrame.gap === 'number' && plan.rootFrame.gap > 0 ? plan.rootFrame.gap : 24;

  if (mainSubtasks.length === 0) {
    return { rows: [], fullWidth, rowGap };
  }

  const rows: Array<OrchestratorPlan['subtasks']> = [];
  let currentRow: OrchestratorPlan['subtasks'] = [];
  let currentWidth = 0;

  const flushRow = () => {
    if (currentRow.length > 0) {
      rows.push(currentRow);
      currentRow = [];
      currentWidth = 0;
    }
  };

  for (const subtask of mainSubtasks) {
    const width = subtask.region.width > 0 ? subtask.region.width : fullWidth;
    const isStandalone = width >= fullWidth * 0.82 || isMainContentContainerSubtask(subtask);

    if (isStandalone) {
      flushRow();
      rows.push([subtask]);
      continue;
    }

    const nextWidth = currentRow.length === 0 ? width : currentWidth + rowGap + width;
    if (currentRow.length > 0 && nextWidth > fullWidth * 1.05) {
      flushRow();
    }

    currentRow.push(subtask);
    currentWidth = currentRow.length === 1 ? width : currentWidth + rowGap + width;

    if (currentWidth >= fullWidth * 0.92) {
      flushRow();
    }
  }

  flushRow();
  return { rows, fullWidth, rowGap };
}

function assignDashboardMainParents(
  plan: OrchestratorPlan,
  mainParentId: string,
): Array<{ node: FrameNode; parentId: string }> {
  const rowFrames: Array<{ node: FrameNode; parentId: string }> = [];
  const { rows, fullWidth, rowGap } = groupDashboardMainRows(plan);
  if (rows.length === 0) return rowFrames;

  let rowIndex = 1;
  for (const row of rows) {
    if (row.length === 1) {
      const subtask = row[0];
      const slotId = `${mainParentId}-row-${rowIndex}-${subtask.id}-slot`;
      rowFrames.push({
        parentId: mainParentId,
        node: {
          id: slotId,
          type: 'frame',
          name: `${subtask.label} Slot`,
          width: 'fill_container',
          height: 'fit_content',
          layout: 'vertical',
          children: [],
        },
      });
      subtask.parentFrameId = slotId;
      rowIndex++;
      continue;
    }

    const rowId = `${mainParentId}-row-${rowIndex}`;
    const slotGap = rowGap * (row.length - 1);
    const availableWidth = Math.max(320, fullWidth - slotGap);
    const widthSum = row.reduce((sum, st) => sum + Math.max(1, st.region.width || fullWidth), 0);

    rowFrames.push({
      parentId: mainParentId,
      node: {
        id: rowId,
        type: 'frame',
        name: `Dashboard Row ${rowIndex}`,
        width: 'fill_container',
        height: 'fit_content',
        layout: 'horizontal',
        gap: rowGap,
        children: [],
      },
    });

    let assignedWidth = 0;
    row.forEach((subtask, idx) => {
      const isLast = idx === row.length - 1;
      const proportionalWidth = Math.max(
        220,
        Math.round((availableWidth * Math.max(1, subtask.region.width || fullWidth)) / widthSum),
      );
      const slotWidth = isLast
        ? 'fill_container'
        : Math.min(
            availableWidth - assignedWidth - 220 * (row.length - idx - 1),
            proportionalWidth,
          );
      const slotId = `${rowId}-${subtask.id}-slot`;
      rowFrames.push({
        parentId: rowId,
        node: {
          id: slotId,
          type: 'frame',
          name: `${subtask.label} Slot`,
          width: slotWidth,
          height: 'fit_content',
          layout: 'vertical',
          children: [],
        },
      });
      if (typeof slotWidth === 'number') assignedWidth += slotWidth;
      subtask.parentFrameId = slotId;
    });

    rowIndex++;
  }

  return rowFrames;
}

function getDashboardPlaceholderHeight(plan: OrchestratorPlan): number {
  const { rows, rowGap } = groupDashboardMainRows(plan);
  const sidebarHeight = plan.subtasks
    .filter((st) => isSidebarSubtask(st))
    .reduce((sum, st) => sum + Math.max(0, st.region.height || 0), 0);

  const visibleRows = rows.slice(0, rows.length >= 3 ? 3 : 2);
  const foldHeight =
    visibleRows.reduce(
      (sum, row) =>
        sum +
        Math.max(
          0,
          ...row.map((st) => (typeof st.region.height === 'number' ? st.region.height : 0)),
        ),
      0,
    ) +
    Math.max(0, visibleRows.length - 1) * rowGap;

  const mainFoldHint = visibleRows.length > 0 ? foldHeight : 560;
  const sidebarHint = sidebarHeight > 0 ? Math.min(sidebarHeight, 600) : 0;
  return Math.max(560, Math.min(Math.max(mainFoldHint, sidebarHint), 680));
}

function reorderDashboardMainChildren(plan: OrchestratorPlan, mainParentId: string): void {
  const store = useDocumentStore.getState();
  const mainNode = store.getNodeById(mainParentId);
  if (!mainNode || !('children' in mainNode) || !Array.isArray(mainNode.children)) return;

  const desiredOrder: string[] = [];
  const seen = new Set<string>();

  const pushOnce = (id: string | null | undefined) => {
    if (!id || seen.has(id)) return;
    seen.add(id);
    desiredOrder.push(id);
  };

  for (const subtask of plan.subtasks) {
    if (isSidebarSubtask(subtask)) continue;

    if (subtask.parentFrameId && subtask.parentFrameId !== mainParentId) {
      const parentOfSlot = store.getParentOf(subtask.parentFrameId);
      if (parentOfSlot?.id === mainParentId) {
        pushOnce(subtask.parentFrameId);
        continue;
      }

      const rowId = parentOfSlot?.id;
      if (rowId && store.getParentOf(rowId)?.id === mainParentId) {
        pushOnce(rowId);
        continue;
      }
    }

    pushOnce(subtask.generatedRootId);
  }

  desiredOrder.forEach((id, index) => {
    store.moveNode(id, mainParentId, index);
  });
}

/**
 * Plan-derived v1 token names that this function manages. Keep in sync with
 * the mappings below — these are the keys we (re-)write on every plan run.
 * User-owned variables outside this set (custom names, themed values) are
 * never touched.
 */
const PLAN_DERIVED_VARIABLE_NAMES = [
  'color-bg-deep',
  'color-surface',
  'color-text-primary',
  'color-text-body',
  'color-text-muted',
  'color-accent',
  'color-border',
] as const;

/**
 * Apply a `name → VariableDefinition | undefined` patch to `doc.variables`
 * via a single direct setState — bypasses `store.setVariable` /
 * `store.removeVariable` to AVOID their side effect of walking the node
 * tree (`replaceVariableRefsInTree`) and rewriting any node that uses the
 * affected ref.
 *
 * Why bypass: this orchestrator-level patch is meant to swap the "ambient"
 * palette tokens between briefs / on rollback. It must NOT mutate user
 * nodes that legitimately reference these tokens — those nodes should
 * just resolve through the new palette at render time. Going through
 * `removeVariable` would null-out their refs and silently break colors
 * in pre-existing user content.
 *
 * Pass `undefined` for a name to delete the key.
 */
function patchDocVariables(patch: Record<string, VariableDefinition | undefined>): void {
  useDocumentStore.setState((s) => {
    const current = s.document.variables ?? {};
    const next: Record<string, VariableDefinition> = { ...current };
    for (const [name, value] of Object.entries(patch)) {
      if (value === undefined) {
        delete next[name];
      } else {
        next[name] = value;
      }
    }
    return {
      document: {
        ...s.document,
        variables: Object.keys(next).length > 0 ? next : undefined,
      },
      isDirty: true,
    };
  });
}

/**
 * Undo the work of `seedDocVariablesFromStyleGuide` by restoring exactly
 * the 7 plan-derived names to their pre-seed state. Variables outside the
 * plan-derived set are never touched. Uses `patchDocVariables` so existing
 * node refs are not rewritten.
 */
function rollbackPlanDerivedVariables(
  snapshot: Record<string, VariableDefinition | undefined>,
): void {
  const patch: Record<string, VariableDefinition | undefined> = {};
  for (const name of PLAN_DERIVED_VARIABLE_NAMES) {
    patch[name] = snapshot[name];
  }
  patchDocVariables(patch);
}

/**
 * Seed `doc.variables` with the plan's style-guide palette (mapped to v1
 * semantic token names) so sub-agent JSONL output containing `$color-*` refs
 * resolves to the user's chosen palette at render time, not the
 * DEFAULT_PALETTE_FALLBACK blue.
 *
 * Without this, the sub-agent prompt teaches the model to emit
 * `$color-accent` (see `buildSubAgentStyleGuideInstruction` in
 * `orchestrator-sub-agent-compact.ts`) but the renderer would fall back to
 * the default `#2563EB` blue — completely overriding any warm/orange/etc
 * style guide picked by the planner.
 *
 * Always overwrites the plan-derived keys: every brief gets a fresh plan, and
 * the prompt's ref + (hex) instruction must match the resolved palette.
 * design.md flows bypass this function entirely (sub-agent uses designMd
 * policy instead of styleGuide injection). User-set variables outside the
 * plan-derived key set are left untouched.
 *
 * Stale-key guard: also clears any plan-derived key the new palette doesn't
 * carry (e.g. ai-generated `plan.styleGuide` has no `textMuted`, but a prior
 * brief may have seeded one from a catalog guide). Without this, the prior
 * brief's textMuted lingers and refs in this brief resolve to the wrong color.
 */
export function seedDocVariablesFromStyleGuide(plan: OrchestratorPlan): void {
  const palette: Record<string, string> = {};

  // Priority: catalog content (designed palette, high confidence) >
  // AI-generated styleGuide (model often invents an off-domain accent —
  // empirically MiniMax/GLM gravitate to indigo `#6366F1` regardless of
  // the warm-food/wellness/etc. catalog snippet they were shown). When the
  // planner picks a catalog name, trust the catalog's curated palette;
  // only fall back to the AI palette when no catalog content was attached.
  if (plan.selectedStyleGuideContent) {
    const v = extractStyleGuideValues(plan.selectedStyleGuideContent).colors;
    if (v.background) palette['color-bg-deep'] = v.background;
    if (v.surface) palette['color-surface'] = v.surface;
    if (v.textPrimary) palette['color-text-primary'] = v.textPrimary;
    if (v.textSecondary) palette['color-text-body'] = v.textSecondary;
    if (v.textMuted) palette['color-text-muted'] = v.textMuted;
    if (v.accent) palette['color-accent'] = v.accent;
    if (v.border) palette['color-border'] = v.border;
  } else if (plan.styleGuide?.palette) {
    const p = plan.styleGuide.palette;
    palette['color-bg-deep'] = p.background;
    palette['color-surface'] = p.surface;
    palette['color-text-primary'] = p.text;
    palette['color-text-body'] = p.secondary;
    palette['color-accent'] = p.accent;
    palette['color-border'] = p.border;
  }

  if (Object.keys(palette).length === 0) return;

  const patch: Record<string, VariableDefinition | undefined> = {};
  for (const name of PLAN_DERIVED_VARIABLE_NAMES) {
    const next = palette[name];
    patch[name] = next !== undefined ? { type: 'color', value: next } : undefined;
  }
  patchDocVariables(patch);
}

import { applyAppendContextToPlan } from './orchestrator-append';
export { applyAppendContextToPlan } from './orchestrator-append';
export type { AppendPlanResult } from './orchestrator-append';

export async function executeOrchestration(
  request: AIDesignRequest,
  callbacks?: {
    onApplyPartial?: (count: number) => void;
    onTextUpdate?: (text: string) => void;
    animated?: boolean;
  },
  abortSignal?: AbortSignal,
): Promise<{ nodes: PenNode[]; rawResponse: string }> {
  setGenerationContextHint(request.prompt);
  const animated = callbacks?.animated ?? false;
  const preparedPrompt = prepareDesignPrompt(request.prompt);

  const renderPlanningStatus = (message: string) => {
    callbacks?.onTextUpdate?.(`<step title="Planning layout" status="streaming">${message}</step>`);
  };

  try {
    // -- Phase 1: Planning (streaming) --
    renderPlanningStatus('Analyzing design structure...');

    // Always attempt AI planning first — even for builtin providers.
    // callOrchestrator already falls back to buildFallbackPlanFromPrompt
    // when the model returns unparseable responses, so we don't need to
    // skip the call preemptively.  Builtin providers get a tighter timeout
    // (30s no-text / 60s hard) so planning fails fast when the provider
    // can't handle the long system prompt, while still giving slow-but-capable
    // models enough time once they start streaming.
    const isBuiltin = (request.provider as string) === 'builtin';
    let plan: OrchestratorPlan;
    try {
      plan = await callOrchestrator(
        preparedPrompt.orchestratorPrompt,
        preparedPrompt.originalLength,
        request.model,
        request.provider,
        abortSignal,
        isBuiltin,
        request.context?.designMd,
      );
    } catch (err) {
      // User abort — propagate so the outer catch cleans up without mutating canvas
      if (abortSignal?.aborted) throw err;
      // Network error, timeout, or provider failure — use heuristic plan
      plan = buildFallbackPlanFromPrompt(
        preparedPrompt.orchestratorPrompt,
        request.context?.designMd,
      );
    }

    if (shouldUseDashboardColumns(request.prompt, plan)) {
      normalizeDashboardMainSubtasks(plan);
    }

    const appendResult = applyAppendContextToPlan(plan, request.context?.appendContext);

    // Remove status-bar subtasks on mobile — the bar is pre-injected.
    // In append mode the existing page already carries the status bar, so the
    // planner-emitted one is stripped by applyAppendContextToPlan above.
    const isMobileScreen = isMobileFullScreen(plan);
    if (isMobileScreen && !appendResult.skipStatusBar) {
      plan.subtasks = plan.subtasks.filter(
        (st) => !STATUS_BAR_NAME_RE.test(`${st.id} ${st.label}`),
      );
    }

    // Assign ID prefixes
    for (const st of plan.subtasks) {
      st.idPrefix = st.id;
      st.parentFrameId = plan.rootFrame.id;
    }

    // Set canvas width hint for accurate text height estimation
    setGenerationCanvasWidth(plan.rootFrame.width);

    // Set context hint once with all subtask labels to avoid race conditions
    // during concurrent sub-agent execution
    setGenerationContextHint(request.prompt + ' ' + plan.subtasks.map((st) => st.label).join(' '));

    // Show planning done + all subtask steps as pending
    emitProgress(
      plan,
      {
        phase: 'generating',
        subtasks: plan.subtasks.map((st) => ({
          id: st.id,
          label: st.label,
          status: 'pending' as const,
          nodeCount: 0,
        })),
        totalNodes: 0,
      },
      callbacks,
    );

    // -- Phase 2: Setup canvas --
    resetGenerationRemapping();
    const concurrency = request.concurrency ?? 1;

    // Group subtasks by screen for concurrent mode.
    // Only use concurrent path when there are MULTIPLE distinct screens.
    // Single-page designs always use the sequential path (proven, simpler).
    const screenGroups: { screen: string; indices: number[] }[] = [];
    if (concurrency > 1) {
      const hasAnyScreen = plan.subtasks.some((st) => st.screen);
      if (hasAnyScreen) {
        const screenMap = new Map<string, number>();
        const firstScreen = plan.subtasks.find((st) => st.screen)?.screen ?? 'page';
        for (let i = 0; i < plan.subtasks.length; i++) {
          const screen = plan.subtasks[i].screen ?? firstScreen;
          if (screenMap.has(screen)) {
            screenGroups[screenMap.get(screen)!].indices.push(i);
          } else {
            screenMap.set(screen, screenGroups.length);
            screenGroups.push({ screen, indices: [i] });
          }
        }
      }
    }

    // Effective concurrency: only parallel when there are multiple screen groups.
    // Append mode is forced sequential — the concurrent branch creates multiple
    // root frames which conflicts with reusing an existing content-root.
    const effectiveConcurrency = appendResult.skipRootInsertion
      ? 1
      : screenGroups.length > 1
        ? concurrency
        : 1;

    // Assign agent identities — one per screen group (concurrent) or per subtask (sequential)
    const subtaskIdentity = new Map<number, { color: string; name: string }>();
    if (effectiveConcurrency > 1) {
      const agentIdentities = assignAgentIdentities(screenGroups.length);
      for (let g = 0; g < screenGroups.length; g++) {
        if (agentIdentities[g]) {
          for (const idx of screenGroups[g].indices) {
            subtaskIdentity.set(idx, agentIdentities[g]);
          }
        }
      }
    } else {
      // Sequential mode: single agent handles all subtasks
      const [identity] = assignAgentIdentities(1);
      if (identity) {
        for (let i = 0; i < plan.subtasks.length; i++) {
          subtaskIdentity.set(i, identity);
        }
      }
    }

    if (animated) {
      resetAnimationState();
      useHistoryStore.getState().startBatch(useDocumentStore.getState().document);
    }

    // Reuse the L742 classification — `isMobileScreen` was computed BEFORE
    // the status-bar subtask got stripped from `plan.subtasks`. Re-running
    // `isMobileFullScreen(plan)` here would see the post-strip count, and
    // a plan that originally had [status-bar, content] (2 items, fits the
    // narrow + multi-subtask fallback added 2026-05-10 for height-loss
    // cases) would drop to 1 item and misclassify as Type 0 → no status
    // bar injection. Codex stop-hook 2026-05-10 caught this; the in-place
    // strip + re-classify pattern is the fragile bit.
    const isMobile = isMobileScreen;
    const useDashboardColumns = shouldUseDashboardColumns(request.prompt, plan);
    const defaultFill: FrameNode['fill'] = (plan.rootFrame.fill as FrameNode['fill']) ?? [
      { type: 'solid', color: plan.styleGuide?.palette?.background ?? '#FFFFFF' },
    ];

    // Track all root frame nodes for result collection
    const rootNodes: FrameNode[] = [];
    let dashboardColumnIds: { sidebarId: string; mainId: string } | null = null;

    if (effectiveConcurrency > 1) {
      // Concurrent mode: create one root frame per screen group.
      // Subtasks sharing the same screen insert into the same root frame.
      //
      // IMPORTANT: insertStreamingNode(node, null) has heavy side effects —
      // it may replace the default empty frame (remapping the node ID to
      // DEFAULT_FRAME_ID) and mutates generationRootFrameId. We only call it
      // for the first frame to handle the empty-canvas case. Subsequent frames
      // are inserted with addNode directly to avoid ID remapping and state
      // corruption.
      const { addNode } = useDocumentStore.getState();
      const remappedIds = getGenerationRemappedIds();
      const gap = 100;
      let nextX = 0;

      for (let g = 0; g < screenGroups.length; g++) {
        const group = screenGroups[g];
        const firstSt = plan.subtasks[group.indices[0]];
        const originalId = `${plan.rootFrame.id}-${group.screen}`;

        // Height: sum of all subtask regions in this group (mobile uses fixed viewport)
        const totalRegionHeight = group.indices.reduce(
          (sum, i) => sum + plan.subtasks[i].region.height,
          0,
        );
        const frameHeight = isMobile
          ? plan.rootFrame.height || 812
          : Math.max(320, totalRegionHeight);

        // Frame name: use screen name if available, else first subtask's short name
        const frameName = firstSt.screen
          ? firstSt.screen
          : firstSt.label.replace(/\s*[（(].+$/, '').trim() || firstSt.label;

        const rootNode: FrameNode = {
          id: originalId,
          type: 'frame',
          name: frameName,
          x: nextX,
          y: 0,
          width: plan.rootFrame.width,
          height: frameHeight,
          layout: plan.rootFrame.layout ?? 'vertical',
          gap: isMobile ? plan.rootFrame.gap || 16 : (plan.rootFrame.gap ?? 16),
          ...(plan.rootFrame.padding != null ? { padding: plan.rootFrame.padding } : {}),
          fill: defaultFill,
          children: [],
        };

        if (g === 0) {
          // First frame: use insertStreamingNode to handle empty canvas replacement
          insertStreamingNode(rootNode, null);
          const actualId = remappedIds.get(originalId) ?? originalId;
          for (const idx of group.indices) {
            plan.subtasks[idx].parentFrameId = actualId;
          }
          rootNode.id = actualId;
        } else {
          addNode(null, rootNode);
          for (const idx of group.indices) {
            plan.subtasks[idx].parentFrameId = originalId;
          }
        }

        rootNodes.push(rootNode);

        // Inject fixed status bar for all mobile screens (iOS-style chrome
        // used as universal mockup — intentional, matches industry convention)
        if (isMobile) {
          const bgColor = (defaultFill as Array<{ color?: string }>)?.[0]?.color;
          const statusBar = createMobileStatusBar(inferStatusBarVariant(bgColor));
          insertStreamingNode(statusBar, rootNode.id);
        }

        // Register agent badge on the root frame immediately
        const identity = subtaskIdentity.get(group.indices[0]);
        if (identity) {
          addAgentFrame(rootNode.id, identity.color, identity.name);
        }

        nextX += plan.rootFrame.width + gap;
      }
    } else {
      // Sequential mode: single root frame containing all sections.
      // In append mode we reuse the caller-provided content-root instead
      // of creating a new root + status bar + dashboard columns.
      if (appendResult.skipRootInsertion) {
        const actualRootId = plan.rootFrame.id;
        setGenerationRootFrameId(actualRootId);
        for (const st of plan.subtasks) {
          st.parentFrameId = actualRootId;
        }
        // No root frame insert, no status bar, no agent badge, no dashboard
        // columns — the existing page already has all of that.
      } else {
        const totalPlannedHeight = plan.subtasks.reduce((sum, st) => sum + st.region.height, 0);
        const initialHeight = isMobile
          ? plan.rootFrame.height || 812
          : useDashboardColumns
            ? getDashboardPlaceholderHeight(plan)
            : Math.max(320, totalPlannedHeight);
        const rootNode: FrameNode = {
          id: plan.rootFrame.id,
          type: 'frame',
          name: plan.rootFrame.name,
          x: 0,
          y: 0,
          width: plan.rootFrame.width,
          height: useDashboardColumns ? `fit_content(${initialHeight})` : initialHeight,
          layout: useDashboardColumns ? 'horizontal' : (plan.rootFrame.layout ?? 'vertical'),
          gap: useDashboardColumns
            ? 0
            : isMobile
              ? plan.rootFrame.gap || 16
              : (plan.rootFrame.gap ?? 16),
          ...(plan.rootFrame.padding != null ? { padding: plan.rootFrame.padding } : {}),
          fill: defaultFill,
          children: [],
        };
        insertStreamingNode(rootNode, null);
        // insertStreamingNode may remap ID (e.g. replacing empty frame)
        const actualRootId = getGenerationRootFrameId();
        rootNode.id = actualRootId;
        rootNodes.push(rootNode);

        // Inject fixed iPhone status bar for iOS mobile screens
        if (isMobile) {
          const bgColor = (defaultFill as Array<{ color?: string }>)?.[0]?.color;
          const statusBar = createMobileStatusBar(inferStatusBarVariant(bgColor));
          insertStreamingNode(statusBar, actualRootId);
        }

        // Register agent badge on the actual root frame
        const firstIdentity = subtaskIdentity.get(0);
        if (firstIdentity) {
          addAgentFrame(actualRootId, firstIdentity.color, firstIdentity.name);
        }

        if (useDashboardColumns) {
          const dashboardColumns = createDashboardColumnFrames(
            plan,
            actualRootId,
            request.context?.designMd,
          );
          insertStreamingNode(dashboardColumns.sidebar, actualRootId);
          insertStreamingNode(dashboardColumns.main, actualRootId);
          dashboardColumnIds = {
            sidebarId: dashboardColumns.sidebar.id,
            mainId: dashboardColumns.main.id,
          };
          for (const st of plan.subtasks) {
            st.parentFrameId = isSidebarSubtask(st) ? dashboardColumns.sidebar.id : null;
          }
          const mainRowFrames = assignDashboardMainParents(plan, dashboardColumns.main.id);
          for (const frame of mainRowFrames) {
            insertStreamingNode(frame.node, frame.parentId);
          }
          for (const st of plan.subtasks) {
            if (isSidebarSubtask(st) && st.parentFrameId == null) {
              st.parentFrameId = dashboardColumns.sidebar.id;
            }
            if (!isSidebarSubtask(st) && st.parentFrameId == null) {
              st.parentFrameId = dashboardColumns.main.id;
            }
          }
        } else {
          for (const st of plan.subtasks) {
            st.parentFrameId = actualRootId;
          }
        }
      }
    }

    if (typeof window !== 'undefined') {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => zoomToFitContent());
      });
    }

    // Snapshot descendant count per root frame AFTER Phase 2 scaffold setup
    // (status bar, dashboard columns, row frames) so we can distinguish
    // scaffold-only frames from frames that received real sub-agent content.
    const scaffoldCounts = new Map<string, number>();
    {
      const store = useDocumentStore.getState();
      const countDescendants = (node: PenNode): number => {
        let n = 0;
        if ('children' in node && Array.isArray(node.children)) {
          n = node.children.length;
          for (const c of node.children) n += countDescendants(c as PenNode);
        }
        return n;
      };
      for (const rn of rootNodes) {
        const live = store.getNodeById(rn.id);
        scaffoldCounts.set(rn.id, live ? countDescendants(live) : 0);
      }
    }

    // -- Phase 3: Parallel sub-agent execution --
    const progress: OrchestrationProgress = {
      phase: 'generating',
      subtasks: plan.subtasks.map((st, i) => {
        const identity = subtaskIdentity.get(i);
        return {
          id: st.id,
          label: st.label,
          status: 'pending' as const,
          nodeCount: 0,
          ...(identity ? { agentColor: identity.color, agentName: identity.name } : {}),
        };
      }),
      totalNodes: 0,
    };

    // Snapshot plan-derived variables BEFORE seed so we can roll back if
    // generation fails entirely (no surviving content). Without this, a
    // failed/aborted brief permanently pollutes the user's doc.variables
    // with the plan's palette even though no nodes were applied.
    const variablesBeforeSeed: Record<string, VariableDefinition | undefined> = {};
    {
      const existing = useDocumentStore.getState().document.variables ?? {};
      for (const name of PLAN_DERIVED_VARIABLE_NAMES) {
        variablesBeforeSeed[name] = existing[name];
      }
    }

    seedDocVariablesFromStyleGuide(plan);

    let results: SubAgentResult[];
    try {
      results = await executeSubAgents(
        plan,
        request,
        preparedPrompt,
        progress,
        effectiveConcurrency,
        callbacks,
        abortSignal,
      );
      if (dashboardColumnIds) {
        reorderDashboardMainChildren(plan, dashboardColumnIds.mainId);
      }
      // Height adjustment for animated mode is deferred to after Phase 4b
      // (duplicate status-bar removal) so it sees the cleaned node tree.
    } catch (e) {
      // Remove root frames that have no sub-agent content — only scaffold
      // nodes (status bar, dashboard columns) from Phase 2 setup.
      const store = useDocumentStore.getState();
      const countDesc = (node: PenNode): number => {
        let n = 0;
        if ('children' in node && Array.isArray(node.children)) {
          n = node.children.length;
          for (const c of node.children) n += countDesc(c as PenNode);
        }
        return n;
      };
      let anyContentSurvived = false;
      for (const rn of rootNodes) {
        const live = store.getNodeById(rn.id);
        if (!live) continue;
        const nowCount = countDesc(live);
        const beforeCount = scaffoldCounts.get(rn.id) ?? 0;
        if (nowCount <= beforeCount) {
          if (rn.id === DEFAULT_FRAME_ID) {
            // Full replace: remove then re-add so no stale properties
            // (layout, gap, fill, children) survive from the failed run.
            const emptyDoc = createEmptyDocument();
            const defaultFrame = emptyDoc.pages?.[0]?.children.find(
              (n) => n.id === DEFAULT_FRAME_ID,
            );
            if (defaultFrame) {
              try {
                store.removeNode(DEFAULT_FRAME_ID);
              } catch {
                /* ok */
              }
              store.addNode(null, defaultFrame as PenNode);
            }
          } else {
            try {
              store.removeNode(rn.id);
            } catch {
              /* already gone */
            }
          }
        } else {
          anyContentSurvived = true;
        }
      }
      // Roll back the plan-derived variables only when no content survived.
      // With partial content the user can still see a salvageable design and
      // its colors should match the seeded palette; rolling back would flip
      // every $color-* ref to the default-palette blue.
      if (!anyContentSurvived) {
        rollbackPlanDerivedVariables(variablesBeforeSeed);
      }
      // On streaming failure, still close the batch before re-throwing
      if (animated) {
        useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
      }
      throw e;
    }

    // Everything below until endBatch must also be guarded so the undo
    // batch is closed even if Phase 4/4b/height-adjustment throws.
    // Declared outside try so they're accessible after the finally block
    const aborted = abortSignal?.aborted ?? false;
    let allNodes: PenNode[] = [];

    try {
      // -- Phase 4: Collect results --

      if (!aborted) {
        for (const entry of progress.subtasks) {
          if (entry.status !== 'error') {
            entry.status = 'done';
          }
        }
        progress.phase = 'done';
      } else {
        for (const entry of progress.subtasks) {
          if (entry.status === 'streaming') {
            entry.status = 'pending';
          }
        }
        progress.phase = 'done';
      }
      emitProgress(plan, progress, callbacks);

      allNodes = [...rootNodes];
      for (const r of results) {
        allNodes.push(...r.nodes);
      }

      const generatedNodeCount = allNodes.length - rootNodes.length;
      if (generatedNodeCount === 0) {
        // No sub-agent content was applied — roll back the plan-derived
        // variables so the user's doc isn't left with a polluted palette
        // that has no design backing it. Covers two cases:
        //   1. !aborted: about to throw — rollback before throw so failure
        //      and empty paths share the same post-state semantics as the
        //      catch-block rollback above.
        //   2. aborted: user cancelled before any content landed; without
        //      rollback the seeded palette persists silently across briefs.
        rollbackPlanDerivedVariables(variablesBeforeSeed);
        if (!aborted) {
          throw new Error('Orchestration produced no nodes beyond root frame');
        }
      }

      // -- Phase 4b: Remove duplicate status bars on mobile --
      // Must run BEFORE height adjustment so removed nodes don't inflate the frame.
      if (isMobile) {
        removeDuplicateStatusBars(rootNodes);
      }

      // -- Phase 4c: Unwrap single-component section root (Type 0) --
      // Runs only for component-shape plans (narrow + auto-height) and
      // is a no-op for everything else, so mutually exclusive with the
      // mobile dedup above. Must also run BEFORE height adjustment.
      unwrapSingleComponentSectionRoot(rootNodes, plan);

      // Height adjustment runs after duplicate removal for both animated and
      // non-animated paths so the frame size reflects the cleaned node tree.
      if (dashboardColumnIds) {
        adjustRootFrameHeightToContent(dashboardColumnIds.sidebarId);
        adjustRootFrameHeightToContent(dashboardColumnIds.mainId);
      }
      if (effectiveConcurrency > 1) {
        for (const rn of rootNodes) {
          adjustRootFrameHeightToContent(rn.id);
        }
      } else {
        adjustRootFrameHeightToContent();
      }
      // Sync heights back to rootNode objects for result
      for (const rn of rootNodes) {
        const adjusted = useDocumentStore.getState().getNodeById(rn.id);
        if (adjusted && adjusted.type === 'frame') {
          rn.height = adjusted.height;
        }
      }
    } finally {
      // Close the undo batch AFTER cleanup + height adjustment so the entire
      // generation (including status-bar dedup) is a single undo operation.
      if (animated) {
        useHistoryStore.getState().endBatch(useDocumentStore.getState().document);
      }
    }

    // -- Phase 5: Visual validation (skip if user stopped or disabled) --
    if (!aborted && VALIDATION_ENABLED) {
      const validationEntry: OrchestrationProgress['subtasks'][number] = {
        id: '_validation',
        label: 'Validating design',
        status: 'pending',
        nodeCount: 0,
      };
      progress.subtasks.push(validationEntry);
      // Also add to plan.subtasks so buildFinalStepTags includes it
      plan.subtasks.push({
        id: '_validation',
        label: 'Validating design',
        region: { width: 0, height: 0 },
        idPrefix: '_validation',
        parentFrameId: null,
      });
      emitProgress(plan, progress, callbacks);

      try {
        const validationResult = await runPostGenerationValidation({
          onStatusUpdate: (status, message) => {
            validationEntry.status =
              status === 'streaming'
                ? 'streaming'
                : status === 'done'
                  ? 'done'
                  : status === 'error'
                    ? 'error'
                    : 'pending';
            validationEntry.thinking = message;
            emitProgress(plan, progress, callbacks);
          },
          model: request.model,
          provider: request.provider,
        });
        if (validationResult.applied > 0) {
          validationEntry.nodeCount = validationResult.applied;
        }
        validationEntry.status = 'done';
      } catch {
        validationEntry.status = 'done';
        validationEntry.thinking = 'Skipped';
      }
      emitProgress(plan, progress, callbacks);
    }

    // Auto-fill image nodes with search results (fire-and-forget)
    const rootId = getGenerationRootFrameId();
    if (rootId) scanAndFillImages(rootId).catch(() => {});

    // Build final rawResponse that includes step tags so the chat message
    // shows the complete pipeline progress after streaming ends
    const finalStepTags = buildFinalStepTags(plan, progress);

    return { nodes: allNodes, rawResponse: finalStepTags };
  } finally {
    clearAgentIndicators();
    setGenerationContextHint('');
    setGenerationCanvasWidth(1200); // Reset to default
  }
}

// ---------------------------------------------------------------------------
// Orchestrator call — fast decomposition
// ---------------------------------------------------------------------------

async function callOrchestrator(
  prompt: string,
  timeoutHintLength: number,
  model?: string,
  provider?: AIDesignRequest['provider'],
  abortSignal?: AbortSignal,
  fastTimeout?: boolean,
  designMd?: DesignMdSpec,
): Promise<OrchestratorPlan> {
  const modelProfile = resolveModelProfile(model);
  const attemptModes =
    modelProfile.tier === 'full'
      ? (['rich'] as const)
      : modelProfile.tier === 'basic'
        ? (['rich', 'minimal', 'compact'] as const)
        : (['rich', 'minimal'] as const);

  // Builtin providers get a tighter timeout so planning fails fast when
  // the provider can't handle the long system prompt, while giving
  // slow-but-capable models enough runway once they start streaming.
  const timeouts = fastTimeout
    ? getBuiltinPlanningTimeouts(model)
    : getOrchestratorTimeouts(timeoutHintLength, model);
  let lastPlanningFailure: {
    reason: 'stream_error' | 'parse_error';
    mode: string;
    detail?: string;
    preview?: string;
  } | null = null;

  for (const [attemptIdx, mode] of attemptModes.entries()) {
    let rawResponse = '';
    let planningSystemPrompt: string;
    let planningUserPrompt = prompt;
    let planningGuideContext: ReturnType<typeof buildPlanningStyleGuideContext> | null = null;
    let forcedStyleGuideName: string | undefined;

    if (mode === 'compact') {
      const compact = buildCompactPlanningPrompt(prompt, model, designMd);
      planningSystemPrompt = compact.systemPrompt;
      planningUserPrompt = compact.userPrompt;
      forcedStyleGuideName = compact.selectedStyleGuideName;
      console.info('[Orchestrator] planning compact retry', {
        model: model ?? 'default',
        tier: modelProfile.tier,
        mode,
        hasDesignMd: !!designMd,
        selectedStyleGuideName: forcedStyleGuideName,
        systemChars: planningSystemPrompt.length,
      });
    } else {
      planningGuideContext = buildPlanningStyleGuideContext(prompt, model, mode, designMd);
      console.info('[Orchestrator] planning shortlist', {
        model: model ?? 'default',
        tier: modelProfile.tier,
        mode,
        hasDesignMd: !!designMd,
        metadataCount: planningGuideContext.metadataCount,
        snippetCount: planningGuideContext.snippetCount,
        topGuides: planningGuideContext.topGuideNames,
        snippetGuides: planningGuideContext.snippetGuideNames,
      });
      const planningCtx = resolveSkills('planning', prompt, {
        dynamicContent: { availableStyleGuides: planningGuideContext.availableStyleGuides },
      });
      const filteredSkills = filterPlanningSkillsForPrompt(planningCtx.skills, prompt);
      planningSystemPrompt =
        filteredSkills.map((s) => s.content).join('\n\n') +
        (mode === 'rich'
          ? '\n\n---\nCRITICAL OUTPUT FORMAT ENFORCEMENT:\nYou MUST output ONLY a single JSON object. Start your response with { and end with }.\nDo NOT output any text, explanation, analysis, markdown, or tool calls before or after the JSON.\nDo NOT "explore" or "think out loud". Do NOT use <tool_call> or function calls.\nAny pre-design analysis (concept extraction, superfan simulation, etc.) must happen internally — include results as JSON fields, never as prose.\nViolating this format will cause a system error.'
          : '\n\nOUTPUT ONLY ONE JSON OBJECT. No prose. No markdown. No tool calls.');
    }

    try {
      for await (const chunk of streamChat(
        planningSystemPrompt,
        [{ role: 'user', content: planningUserPrompt }],
        model,
        timeouts,
        provider,
        abortSignal,
      )) {
        if (chunk.type === 'text') {
          rawResponse += chunk.content;
        } else if (chunk.type === 'thinking') {
          continue;
        } else if (chunk.type === 'error') {
          throw new Error(chunk.content);
        }
      }
    } catch (err) {
      if (abortSignal?.aborted) throw err;
      const detail = err instanceof Error ? err.message : String(err);
      lastPlanningFailure = { reason: 'stream_error', mode, detail };
      if (attemptIdx < attemptModes.length - 1) {
        console.warn('[Orchestrator] Planning attempt failed. Retrying with lighter context.', {
          mode,
          detail,
        });
        continue;
      }
      console.warn('[Orchestrator] Planning failed on final attempt.', {
        mode,
        detail,
      });
      break;
    }

    if (abortSignal?.aborted) {
      throw new Error('Aborted');
    }

    const parsed = parseOrchestratorResponse(rawResponse, prompt, designMd);
    if (parsed) {
      if (parsed.repaired) {
        console.info('[Orchestrator] repaired near-miss planning JSON', {
          mode,
          preview: rawResponse.trim().slice(0, 150),
        });
      }

      const plan = parsed.plan;
      if (!plan.styleGuideName && forcedStyleGuideName) {
        plan.styleGuideName = forcedStyleGuideName;
      }
      normalizeOrchestratorPlan(plan, prompt);

      // design.md takes precedence over the pre-built catalog. If the user
      // has a design.md, we intentionally DO NOT resolve plan.styleGuideName
      // through styleGuideRegistry — leaving selectedStyleGuideContent
      // undefined prevents a catalog guide from bleeding into sub-agent
      // prompts and overriding design.md.
      if (plan.styleGuideName && !designMd && plan.styleGuideName !== DESIGN_MD_STYLE_GUIDE_NAME) {
        const rootWidth = typeof plan.rootFrame.width === 'number' ? plan.rootFrame.width : 1440;
        const platform = rootWidth <= 500 ? 'mobile' : 'webapp';

        let selected = selectStyleGuide(styleGuideRegistry, {
          name: plan.styleGuideName,
          platform,
        });
        if (!selected) {
          selected = selectStyleGuide(styleGuideRegistry, {
            tags: [plan.styleGuideName.toLowerCase()],
            platform,
          });
        }
        if (selected) {
          plan.selectedStyleGuideContent = selected.content;
        }
      }

      return plan;
    }

    lastPlanningFailure = {
      reason: 'parse_error',
      mode,
      preview: rawResponse.trim().slice(0, 150),
    };
    if (attemptIdx < attemptModes.length - 1) {
      console.warn(
        `[Orchestrator] Planning attempt ${attemptIdx + 1} returned no parseable JSON. Retrying with lighter context.`,
        {
          mode,
          preview: rawResponse.trim().slice(0, 150),
          metadataCount: planningGuideContext?.metadataCount ?? 0,
          snippetCount: planningGuideContext?.snippetCount ?? 0,
        },
      );
      continue;
    }

    console.warn(
      '[Orchestrator] Could not parse model response, using fallback plan. Preview:',
      rawResponse.trim().slice(0, 150),
    );
  }

  console.warn('[Orchestrator] Using fallback plan after planner failed.', {
    model: model ?? 'default',
    tier: modelProfile.tier,
    reason: lastPlanningFailure?.reason ?? 'unknown',
    mode: lastPlanningFailure?.mode ?? 'unknown',
    detail: lastPlanningFailure?.detail,
    preview: lastPlanningFailure?.preview,
  });
  const fallback = buildFallbackPlanFromPrompt(prompt, designMd);
  normalizeOrchestratorPlan(fallback, prompt);
  return fallback;
}
