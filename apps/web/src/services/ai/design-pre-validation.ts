/**
 * Pre-validation: pure code checks that don't require LLM.
 *
 * This file is now a thin wrapper around `@zseven-w/pen-ai-skills` diagnostics.
 * All detect logic lives in `packages/pen-ai-skills/src/diagnostics/detectors.ts`
 * as pure functions so debug tools can reuse them without side effects.
 *
 * See also:
 *   - packages/pen-ai-skills/src/diagnostics/detectors.ts (pure detect)
 *   - docs/superpowers/specs/2026-04-06-mcp-debug-tools-design.md (Phase 1.A)
 */

import { detectAllIssues, type Issue } from '@zseven-w/pen-ai-skills';
import { DEFAULT_FRAME_ID, useDocumentStore } from '@/stores/document-store';
import type { PenNode } from '@/types/pen';

/**
 * Run pre-validation detectors on the live document and apply suggested fixes.
 * Returns the number of fixes ACTUALLY applied (not the number detected).
 *
 * Detected issues that are skipped — `info` severity (detect-only) and
 * protected status-bar removals — are not counted, so callers can rely on
 * the return value as a faithful "did anything change" signal.
 *
 * Use {@link runPreValidationFixesDetailed} when the caller needs the
 * per-category breakdown for surfacing in the UI.
 */
export function runPreValidationFixes(): number {
  return runPreValidationFixesDetailed().total;
}

export interface PreValidationResult {
  total: number;
  /** Number of fixes APPLIED per detector category (not detected). */
  byCategory: Record<string, number>;
}

export function runPreValidationFixesDetailed(): PreValidationResult {
  const store = useDocumentStore.getState();
  const root = store.getNodeById(DEFAULT_FRAME_ID);
  if (!root) return { total: 0, byCategory: {} };

  const issues = detectAllIssues(root, store.document);
  return applyFixes(issues);
}

function applyFixes(issues: Issue[]): PreValidationResult {
  const store = useDocumentStore.getState();
  let applied = 0;
  const byCategory: Record<string, number> = {};
  for (const issue of issues) {
    // 'info' severity is detect-only — the detector emits the issue for
    // debug visibility but is not confident enough that an auto-fix would
    // be safe. Currently only sibling-inconsistency cross-role checks
    // (loose pass on cornerRadius) use this; rewriting them could damage
    // a structurally distinct sibling like a rounded chrome element.
    if (issue.severity === 'info') continue;

    if (issue.property === '__remove') {
      // Never remove pre-injected chrome (e.g. iPhone status bar)
      const target = store.getNodeById(issue.nodeId);
      if (target && 'role' in target && (target as { role?: string }).role === 'status-bar') {
        console.log(`[Pre-validation] ${issue.nodeId}: skipped removal (protected status-bar)`);
        continue;
      }
      store.removeNode(issue.nodeId);
      applied++;
      byCategory[issue.category] = (byCategory[issue.category] ?? 0) + 1;
      console.log(`[Pre-validation] ${issue.nodeId}: removed (${issue.reason})`);
    } else {
      store.updateNode(issue.nodeId, {
        [issue.property]: issue.suggestedValue,
      } as Partial<PenNode>);
      applied++;
      byCategory[issue.category] = (byCategory[issue.category] ?? 0) + 1;
      console.log(
        `[Pre-validation] ${issue.nodeId}: ${issue.property} → ${JSON.stringify(issue.suggestedValue)} (${issue.reason})`,
      );
    }
  }
  return { total: applied, byCategory };
}
