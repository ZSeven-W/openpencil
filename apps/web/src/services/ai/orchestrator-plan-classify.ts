import type { OrchestratorPlan } from './ai-types';

/**
 * A plan represents a full mobile screen only when the root frame is narrow
 * AND tall. Narrow + auto-height (or small fixed height) is a Type 0 component
 * — a single card / badge / modal — and must not trigger phone-screen logic
 * (status bar pre-injection, mobile-app skill, "no phone mockup wrapper" prompt).
 *
 * See `pen-ai-skills/skills/phases/planning/design-type.md` for the full Type 0
 * specification. Both `orchestrator.ts` and `orchestrator-sub-agent.ts` MUST
 * use this helper — duplicating the threshold inline lets the two paths drift
 * (e.g. orchestrator skipping status bar injection while sub-agent still
 * loads the mobile-app skill, which is what triggered Codex review on
 * 2026-05-09).
 */
export function isMobileFullScreen(plan: OrchestratorPlan): boolean {
  if (plan.rootFrame.width > 480) return false;
  if (plan.rootFrame.height >= 480) return true;
  // Width-narrow + height-zero/auto: distinguish a single Type 0
  // component from a mobile page whose declared height got lost in
  // plan coercion. The LLM frequently emits a non-numeric height
  // ("fit_content" / "auto") which asNonNegativeNumber rejects → the
  // parser falls back to the landing-page preset's rootHeight=0 →
  // we'd misclassify as Type 0 and skip status-bar injection.
  //
  // 2026-05-10 user reported a DeepSeek "Bistro" mobile design that
  // arrived with a 375-wide root and missing iOS status bar; the live
  // tree had 6+ section frames so it was structurally a multi-section
  // mobile page, not a single card. A subtask-count discriminator
  // catches this without re-relaxing the height check for genuine
  // Type 0 components (badge / single card / modal — always 1 subtask).
  return plan.subtasks.length >= 2;
}
