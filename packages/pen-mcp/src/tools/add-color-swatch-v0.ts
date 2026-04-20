import type { handleBatchDesign } from './batch-design';
import {
  assignIdsRecursively,
  ensureParentExists,
  insertElementTree,
} from './element-tool-helpers';

export interface AddColorSwatchV0Params {
  color: string;
  label?: string;
  size?: number;
  parent_id?: string;
  filePath?: string;
  pageId?: string;
}

/**
 * Design-system color swatch — one colored square (cornerRadius=12) with
 * an optional label underneath. Typical use: style-guide pages listing
 * theme tokens.
 *
 * `color` accepts a literal hex (`"#2563EB"`) or a `$variable` ref
 * (`"$color-primary"`). The resolver in pen-core turns `$refs` into the
 * active theme's color at render time, so this tool can be used to
 * visualize the live palette without pre-resolving.
 *
 * Square size defaults to 64px; callers can pick smaller (48) for dense
 * grids or larger (96+) for hero tokens.
 */
export async function handleAddColorSwatchV0(
  params: AddColorSwatchV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  await ensureParentExists(params);
  const size = Math.max(16, Math.floor(params.size ?? 64));
  const children: Record<string, unknown>[] = [
    {
      type: 'frame',
      name: 'Swatch Square',
      role: 'color-swatch-square',
      width: size,
      height: size,
      cornerRadius: 12,
      fill: [{ type: 'solid', color: params.color }],
    },
  ];
  if (params.label) {
    children.push({
      type: 'text',
      name: 'Label',
      role: 'color-swatch-label',
      content: params.label,
      fontSize: 12,
      fontWeight: 500,
    });
  }
  const swatch = {
    type: 'frame',
    name: 'Color Swatch',
    role: 'color-swatch',
    width: 'fit_content',
    height: 'fit_content',
    layout: 'vertical',
    alignItems: 'center',
    gap: 8,
    children,
  };
  assignIdsRecursively(swatch);
  return insertElementTree({ binding: 'color_swatch', tree: swatch, ...params });
}
