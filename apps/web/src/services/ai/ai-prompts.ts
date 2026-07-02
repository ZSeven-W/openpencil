import { resolveSkills } from '@zseven-w/pen-ai-skills';
import type { DesignMdSpec } from '@/types/design-md';

// Safe code block delimiter
const BLOCK = '```';

// ---------------------------------------------------------------------------
// buildDesignMdStylePolicy — condensed design.md style policy for AI prompts
// ---------------------------------------------------------------------------

/** Build a condensed design.md style policy string for AI prompt injection. */
export function buildDesignMdStylePolicy(spec: DesignMdSpec): string {
  const parts: string[] = [];

  if (spec.visualTheme) {
    const theme =
      spec.visualTheme.length > 200 ? spec.visualTheme.substring(0, 200) + '...' : spec.visualTheme;
    parts.push(`VISUAL THEME: ${theme}`);
  }

  if (spec.colorPalette?.length) {
    const colors = spec.colorPalette
      .slice(0, 10)
      .map((c) => `${c.name} (${c.hex}) — ${c.role}`)
      .join('\n- ');
    parts.push(`COLOR PALETTE:\n- ${colors}`);

    // Call out surface/card/panel/sidebar colors explicitly. The page
    // background uses the primary-background color, but cards, panels, and
    // sidebars MUST use these surface colors — otherwise dashboards flatten
    // into a single-color wash and lose their layered look.
    const surfaces = spec.colorPalette.filter((c) =>
      /surface|card|panel|sidebar|tile|chip/i.test(c.role || ''),
    );
    if (surfaces.length > 0) {
      const surfaceLines = surfaces
        .slice(0, 6)
        .map((c) => `${c.name} (${c.hex}) — ${c.role}`)
        .join('\n- ');
      parts.push(
        `SURFACE COLORS (use ONLY as \`fill\` on visually distinct components placed on top of the page background — cards, sidebars, floating panels, chips, badges. DO NOT fill section root frames or generic wrapper frames with these; section containers must stay transparent and inherit the page background. NEVER use these as the page/rootFrame fill):\n- ${surfaceLines}`,
      );
    }
  }

  if (spec.typography?.fontFamily) {
    parts.push(`FONT: ${spec.typography.fontFamily}`);
  }
  if (spec.typography?.headings) {
    parts.push(`Headings: ${spec.typography.headings}`);
  }
  if (spec.typography?.body) {
    parts.push(`Body: ${spec.typography.body}`);
  }

  if (spec.componentStyles) {
    const styles =
      spec.componentStyles.length > 300
        ? spec.componentStyles.substring(0, 300) + '...'
        : spec.componentStyles;
    parts.push(`COMPONENT STYLES:\n${styles}`);
  }

  if (spec.layoutPrinciples) {
    const layout =
      spec.layoutPrinciples.length > 400
        ? spec.layoutPrinciples.substring(0, 400) + '...'
        : spec.layoutPrinciples;
    parts.push(`LAYOUT PRINCIPLES:\n${layout}`);
  }

  if (spec.generationNotes) {
    const notes =
      spec.generationNotes.length > 400
        ? spec.generationNotes.substring(0, 400) + '...'
        : spec.generationNotes;
    parts.push(`GENERATION NOTES:\n${notes}`);
  }

  return parts.join('\n\n');
}

// ---------------------------------------------------------------------------
// Core prompt templates (compact — design knowledge lives in pen-ai-skills)
// ---------------------------------------------------------------------------

const CHAT_CORE_PROMPT = `You are a design assistant for OpenPencil, a vector design tool that renders PenNode JSON on a canvas.

ABSOLUTE REQUIREMENT — When a user asks to create/generate/design/make ANY visual element or UI:
You MUST output a ${BLOCK}json code block containing a valid PenNode JSON array. This is NON-NEGOTIABLE.
Add a 1-2 sentence description AFTER the JSON block, not before.
NEVER describe what you "would" create — ALWAYS output the actual JSON immediately.
NEVER output HTML, CSS, or React code — ONLY PenNode JSON.
NEVER say "I will create..." — START DIRECTLY WITH <step>.
NEVER use "OpenPencil", "Pencil", or the tool name as brand/app name in designs. Use generic placeholders like "AppName", "Acme", or contextually relevant names.

You may include 1-2 brief <step> tags before the JSON (optional, keep them SHORT).
When a user asks non-design questions (explain, suggest colors, give advice), respond in text.`;

const GENERATOR_CORE_PROMPT = `You are a PenNode JSON streaming engine. Convert design descriptions into flat PenNode JSON, one element at a time.

OUTPUT FORMAT — ELEMENT-BY-ELEMENT STREAMING:
Each element is rendered to the canvas the INSTANT it finishes generating. Output flat JSON objects inside a single ${BLOCK}json block.

STEP 1 — PLAN: List ALL planned sections as <step> tags BEFORE the json block.
STEP 2 — BUILD: ${BLOCK}json block with flat JSON objects, ONE PER LINE. Every node MUST have "_parent" field.

CRITICAL:
- DO NOT use nested "children" arrays — each node is a FLAT JSON object with "_parent".
- ONE JSON object per line. Output parent before children (depth-first).
- Root frame: "_parent": null, x:0, y:0.
- Start with <step> tags, then immediately the json block. NO preamble.
- After the json block, add a 1-sentence summary.
Design like a professional: hierarchy, contrast, whitespace, consistent palette.`;

// ---------------------------------------------------------------------------
// Prompt builders (progressive skill loading via pen-ai-skills)
// ---------------------------------------------------------------------------

/**
 * Build a chat system prompt with only the skills needed for the user's message.
 * Uses pen-ai-skills resolver for progressive skill loading.
 */
export function buildChatSystemPrompt(
  userMessage: string,
  options?: {
    hasDesignMd?: boolean;
    hasVariables?: boolean;
    designMd?: DesignMdSpec;
  },
): string {
  const genCtx = resolveSkills('generation', userMessage, {
    flags: {
      hasDesignMd: !!options?.hasDesignMd,
      hasVariables: !!options?.hasVariables,
    },
    dynamicContent: options?.designMd
      ? { designMdContent: buildDesignMdStylePolicy(options.designMd) }
      : undefined,
  });
  const knowledge = genCtx.skills.map((s) => s.content).join('\n\n');
  return `${CHAT_CORE_PROMPT}\n\n${knowledge}`;
}

/**
 * Build a generator system prompt with only the skills needed.
 * Uses pen-ai-skills resolver for progressive skill loading.
 */
export function buildGeneratorSystemPrompt(
  userMessage: string,
  options?: {
    hasDesignMd?: boolean;
    hasVariables?: boolean;
    designMd?: DesignMdSpec;
  },
): string {
  const genCtx = resolveSkills('generation', userMessage, {
    flags: {
      hasDesignMd: !!options?.hasDesignMd,
      hasVariables: !!options?.hasVariables,
    },
    dynamicContent: options?.designMd
      ? { designMdContent: buildDesignMdStylePolicy(options.designMd) }
      : undefined,
  });
  const knowledge = genCtx.skills.map((s) => s.content).join('\n\n');
  return `${GENERATOR_CORE_PROMPT}\n\n${knowledge}`;
}

/**
 * Build a modifier system prompt using maintenance-phase skills.
 * Uses pen-ai-skills resolver for progressive skill loading.
 */
export function buildModifierSystemPrompt(
  userMessage: string,
  options?: {
    hasDesignMd?: boolean;
    hasVariables?: boolean;
    designMd?: DesignMdSpec;
  },
): string {
  const maintenanceCtx = resolveSkills('maintenance', userMessage, {
    flags: {
      hasVariables: !!options?.hasVariables,
      hasDesignMd: !!options?.hasDesignMd,
    },
  });
  let prompt = maintenanceCtx.skills.map((s) => s.content).join('\n\n');
  // Append design-md context if present (design-md skill is generation-phase only)
  if (options?.designMd) {
    prompt += '\n\n' + buildDesignMdStylePolicy(options.designMd);
  }
  return prompt;
}
