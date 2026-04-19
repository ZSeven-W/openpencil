import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export type AddHeadingV0Level = 'display' | 'h1' | 'h2' | 'h3';

export interface AddHeadingV0Params {
  content: string;
  level?: AddHeadingV0Level;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Typographic heading with Pencil-demo-derived size / weight / lineHeight
 * presets. The `level` parameter does NOT change output structure (always
 * a single `text` node) — only typography — so it is an acceptable enum
 * under "应拆尽拆" (no structural branching).
 *
 * Presets (from memory `project_pencil_optimization.md` + demo data):
 *   display: 48 / 700 / 1.0  / letterSpacing -0.5  — hero headlines
 *   h1:      32 / 700 / 1.1                       — section titles
 *   h2:      24 / 600 / 1.2                       — sub-sections (DEFAULT)
 *   h3:      20 / 600 / 1.25                      — card / list headers
 *
 * Non-Claude models frequently forget lineHeight on heading text, which
 * makes multi-word headings visually stack too tight (1.5 default) or
 * produces leading whitespace quirks. Encoding the preset at the tool
 * boundary removes that failure mode.
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
const LEVEL_PRESETS: Record<
  AddHeadingV0Level,
  {
    fontSize: number;
    fontWeight: number;
    lineHeight: number;
    letterSpacing?: number;
  }
> = {
  display: { fontSize: 48, fontWeight: 700, lineHeight: 1.0, letterSpacing: -0.5 },
  h1: { fontSize: 32, fontWeight: 700, lineHeight: 1.1 },
  h2: { fontSize: 24, fontWeight: 600, lineHeight: 1.2 },
  h3: { fontSize: 20, fontWeight: 600, lineHeight: 1.25 },
};

export async function handleAddHeadingV0(
  params: AddHeadingV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const level = params.level ?? 'h2';
  const preset = LEVEL_PRESETS[level];
  const heading: Record<string, unknown> = {
    type: 'text',
    name: `Heading (${level})`,
    role: 'heading',
    content: params.content,
    fontSize: preset.fontSize,
    fontWeight: preset.fontWeight,
    lineHeight: preset.lineHeight,
  };
  if (preset.letterSpacing !== undefined) {
    heading.letterSpacing = preset.letterSpacing;
  }
  assignIdsRecursively(heading);
  return insertElementTree({ binding: 'h', tree: heading, ...params });
}
