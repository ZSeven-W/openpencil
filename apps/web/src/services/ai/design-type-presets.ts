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
 * surrounding screen. Mirrors the Type 0 list in
 * `pen-ai-skills/skills/phases/planning/design-type.md`:
 *   - "X card" / "X 卡片" — profile / pricing / stat / event / user card
 *   - "X badge" / "X chip" / "X tag" / "X tile" / "X label" / "X row" / "X item"
 *   - "X button" / "X toggle" / "X switch" / "X selector"
 *   - "X modal" / "X dialog" / "X tooltip" / "X popover" / "X sheet"
 *   - "X widget" / "X panel" (when no surrounding screen)
 *   - Single visualisations: "a chart" / "a pie chart" / "a stat" / "a metric"
 *
 * The disqualifier regex below blocks the trigger when the prompt also
 * names a screen / page / app, so "shopping cart screen" or "navbar with
 * cart button" stays a non-component.
 */
// Split into Latin and CJK matchers because JS `\b` is ASCII-only and never
// fires between two CJK characters (so `\b卡片\b` would never match a prompt
// like "design a 卡片"). Each matcher runs independently and either is enough
// to classify as Type 0.
const COMPONENT_TRIGGER_LATIN_RE =
  /\b(card|badge|chip|tag|tile|pill|label|row|item|button|toggle|switch|selector|modal|dialog|tooltip|popover|sheet|widget|panel|avatar|stepper|stat|metric|chart)\b/i;
const COMPONENT_TRIGGER_CJK_RE = /(卡片|徽章|标签|按钮|开关|对话框|提示|气泡|图表)/;

// Disqualifiers fall into three buckets:
//   - Screen-level nouns ("screen", "page", "app", "home", "onboarding",
//     "flow", + zh-Hans equivalents) — the user named a whole screen, not
//     a single piece.
//   - Mobile-screen markers ("mobile", "phone", "ios", "android", "手机",
//     "移动端") — a mobile-skewed prompt should hit the mobile-screen
//     branch below, not the 400px-component branch.
//   - Workspace markers ("dashboard", "admin", "workspace", "console",
//     "管理", "后台", "控制台") — these anchor a desktop-screen, even when
//     the prompt also names a tile/panel/chart/metric inside it
//     ("admin dashboard with metric tiles" must not become a Type 0 tile).
const COMPONENT_DISQUALIFIER_RE =
  /\b(screen|page|app|home|onboarding|flow|mobile|phone|ios|android|dashboard|admin|workspace|console)\b|网页|页面|屏幕|手机|移动端|管理|后台|控制台|工作台|工作区/i;

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
  const matchesComponent =
    COMPONENT_TRIGGER_LATIN_RE.test(prompt) || COMPONENT_TRIGGER_CJK_RE.test(prompt);
  if (matchesComponent && !COMPONENT_DISQUALIFIER_RE.test(prompt)) {
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

  // Fixed-height desktop screens. Keep in sync with the workspace markers
  // in COMPONENT_DISQUALIFIER_RE — every keyword that disqualifies a
  // component fallback for "workspace context" MUST also be detected here,
  // otherwise the prompt skips component AND skips dashboard and falls
  // through to landing-page (1200×0) which is wrong for "design a workspace
  // with side panel" / "design a console with metrics" (Codex review #6).
  if (/dashboard|admin|workspace|console|管理|后台|控制台|工作台|工作区/i.test(prompt)) {
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
