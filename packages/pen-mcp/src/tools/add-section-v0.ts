import { handleBatchDesign } from './batch-design';

export interface AddSectionV0Params {
  title: string;
  layout: 'horizontal' | 'vertical';
  filePath?: string;
  pageId?: string;
}

/**
 * D0 parity spike tool — not for production use.
 *
 * Sugar over handleBatchDesign with two fixed parameters (title, layout).
 * By design it does NOT extract any shared helpers; it reuses the existing
 * batch_design entry point verbatim to validate the N-tool "additive-only"
 * hypothesis: adding this tool must not change `ListTools` output for
 * existing tools, nor change `batch_design` output on any corpus input.
 *
 * See spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0
 */
export async function handleAddSectionV0(
  params: AddSectionV0Params,
): Promise<Awaited<ReturnType<typeof handleBatchDesign>>> {
  const nodeData = {
    type: 'frame',
    name: params.title,
    layout: params.layout,
    width: 1200,
    height: 0,
    children: [] as unknown[],
  };
  const dsl = `sec=I(null, ${JSON.stringify(nodeData)})`;
  return handleBatchDesign({
    operations: dsl,
    filePath: params.filePath,
    pageId: params.pageId,
    postProcess: false,
  });
}
