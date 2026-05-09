export type DesignType = 'mobile-screen' | 'desktop-screen' | 'landing-page' | 'component';

export interface DesignTypePreset {
  type: DesignType;
  width: number;
  /** Section total height (0 = auto based on section count) */
  height: number;
  /** Explicit rootFrame height (0 = auto) */
  rootHeight: number;
  defaultSections: string[];
}

/**
 * Component triggers — when the prompt names one atomic UI piece without a
 * surrounding screen ("X card", "X badge", "X chip", etc.). Mirrors the
 * Type 0 list in `pen-ai-skills/skills/phases/planning/design-type.md`.
 *
 * Match policy: word boundary + the noun must lead OR follow the qualifier
 * to keep noisy mid-sentence hits out (e.g. "shopping cart screen" should
 * not match "cart" as a component).
 */
const COMPONENT_TRIGGER_RE =
  /\b(?:[a-z一-鿿]+\s+)?(card|badge|chip|tile|tag|pill|toggle|switch|modal|dialog|tooltip|popover|sheet|widget|avatar|stepper|stat|metric)(?:\s+(?:design|component|widget))?\b/i;

/**
 * Minimal fallback design type detection.
 *
 * ONLY used when the orchestrator fails to parse the AI's JSON plan.
 * In normal operation, the AI classifies via decomposition.md.
 *
 * Keeps classification minimal — the AI's job is to reason about intent.
 * This fallback only needs to pick a reasonable width/height/section set.
 */
export function detectDesignType(prompt: string): DesignTypePreset {
  // Single-component prompts (Type 0 — see design-type.md). Checked BEFORE
  // mobile/dashboard so a "profile card" prompt doesn't fall through to
  // landing-page (1200px) when AI parsing fails — the user gets a
  // sensibly-sized 400px component instead.
  if (COMPONENT_TRIGGER_RE.test(prompt) && !/screen|page|app|网页|页面/i.test(prompt)) {
    return {
      type: 'component',
      width: 400,
      height: 0,
      rootHeight: 0,
      defaultSections: ['Component'],
    };
  }

  // Explicit mobile indicators (NOT "app" alone — too ambiguous)
  if (/mobile|手机|phone|移动端|ios|android/i.test(prompt)) {
    return {
      type: 'mobile-screen',
      width: 375,
      height: 812,
      rootHeight: 812,
      // Two safe buckets preserve a readable checklist while still keeping
      // weak-model fallback decomposition broad enough to avoid heavy
      // cross-section duplication.
      defaultSections: ['Top Summary', 'Main Content'],
    };
  }

  // Fixed-height desktop screens
  if (/dashboard|admin|管理|后台|控制台/i.test(prompt)) {
    return {
      type: 'desktop-screen',
      width: 1200,
      height: 800,
      rootHeight: 800,
      defaultSections: ['Header', 'Main Content', 'Actions'],
    };
  }

  // Default: scrollable desktop page (safest for landing, portfolio, pricing, etc.)
  return {
    type: 'landing-page',
    width: 1200,
    height: 0,
    rootHeight: 0,
    defaultSections: ['Header', 'Main Content', 'Supporting Content', 'Footer'],
  };
}
