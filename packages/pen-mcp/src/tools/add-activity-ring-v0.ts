import { handleBatchDesign } from './batch-design';
import { generateId } from '../utils/id';

export interface AddActivityRingV0Params {
  size?: number;
  thickness?: number;
  ring_color?: string;
  center_text: string;
  text_size?: number;
  text_weight?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

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
  const size = params.size ?? 80;
  const thickness = params.thickness ?? 8;
  const ringColor = params.ring_color ?? '#000000';
  const textSize = params.text_size ?? 16;
  const textWeight = params.text_weight ?? 700;
  const ring = buildRing(params, size, thickness, ringColor, textSize, textWeight);
  assignIdsRecursively(ring);
  const parentRef = params.parent_id ? `"${params.parent_id}"` : 'null';
  const dsl = `ring=I(${parentRef}, ${JSON.stringify(ring)})`;
  return handleBatchDesign({
    operations: dsl,
    filePath: params.filePath,
    pageId: params.pageId,
    postProcess: false,
  });
}

function buildRing(
  params: AddActivityRingV0Params,
  size: number,
  thickness: number,
  ringColor: string,
  textSize: number,
  textWeight: number,
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
      fill: [{ type: 'solid', color: ringColor }],
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
        fontSize: textSize,
        fontWeight: textWeight,
      },
    ],
  };
}

function assignIdsRecursively(node: Record<string, unknown>): void {
  if (typeof node.id !== 'string') node.id = generateId();
  const children = node.children;
  if (Array.isArray(children)) {
    for (const child of children) {
      if (child && typeof child === 'object') {
        assignIdsRecursively(child as Record<string, unknown>);
      }
    }
  }
}
