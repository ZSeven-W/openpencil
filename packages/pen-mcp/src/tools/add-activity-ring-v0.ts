import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddActivityRingV0Params {
  size?: number;
  thickness?: number;
  center_text: string;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

// Hardcoded styling constants. Per spec D6 (Style Guide orthogonal), this
// tool emits structure only and ships colorless / with fixed typography.
// Callers override visual properties via a follow-up batch_design U-op —
// either directly or through a Style Guide injection pass.
const DEFAULT_RING_COLOR = '#000000';
const DEFAULT_TEXT_SIZE = 16;
const DEFAULT_TEXT_WEIGHT = 700;

/**
 * MVP element tool — activity ring (Apple-style progress ring with centered text).
 *
 * Solves the documented anti-pattern in
 * `packages/pen-ai-skills/skills/phases/generation/layout.md` §RING /
 * CIRCLE WITH CENTER CONTENT. LLMs frequently emit:
 *
 *   ellipse + sibling text        (WRONG — ellipse has no children,
 *                                  sibling stacks above/below instead
 *                                  of centering)
 *   layout=none + nested frame    (WRONG — absolute positioning doesn't
 *                                  render reliably under Skia)
 *
 * Correct pattern (emitted by this tool):
 *   frame(cornerRadius=size/2, stroke={thickness, ringColor}, fill=[],
 *         layout=horizontal, alignItems=center, justifyContent=center)
 *     └── text(center_text)
 *
 * Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §7
 */
export async function handleAddActivityRingV0(
  params: AddActivityRingV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const size = params.size ?? 80;
  const thickness = params.thickness ?? 8;
  const ring = buildRing(params, size, thickness);
  assignIdsRecursively(ring);
  return insertElementTree({ binding: 'ring', tree: ring, ...params });
}

function buildRing(
  params: AddActivityRingV0Params,
  size: number,
  thickness: number,
): Record<string, unknown> {
  return {
    type: 'frame',
    name: 'Activity Ring',
    role: 'activity-ring',
    width: size,
    height: size,
    cornerRadius: size / 2,
    fill: [],
    stroke: {
      thickness,
      fill: [{ type: 'solid', color: DEFAULT_RING_COLOR }],
    },
    layout: 'horizontal',
    alignItems: 'center',
    justifyContent: 'center',
    children: [
      {
        type: 'text',
        name: 'Center Text',
        role: 'heading',
        content: params.center_text,
        fontSize: DEFAULT_TEXT_SIZE,
        fontWeight: DEFAULT_TEXT_WEIGHT,
      },
    ],
  };
}
