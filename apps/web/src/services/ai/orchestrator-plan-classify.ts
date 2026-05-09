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
  return plan.rootFrame.height >= 480;
}
