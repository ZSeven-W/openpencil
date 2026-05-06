import { openDocument, resolveDocPath } from '../document-manager';
import {
  parseDesignMd,
  generateDesignMd,
  extractDesignMdFromDocument,
} from '../utils/design-md-parser';
import type { DesignMdSpec } from '@zseven-w/pen-types';
import { setDesignMdForPrompt } from './design-prompt';

// In MCP 上下文（stdio 模式），没有 Zustand 存储。 We 为 design.md
// 规范保留模块级缓存。
let _mcpDesignMd: DesignMdSpec | undefined;

export interface GetDesignMdParams {
  filePath?: string;
}

export interface SetDesignMdParams {
  filePath?: string;
  /** Raw design.md 的降价内容 */
  markdown?: string;
  /** If true，从现有文档内容中自动提取 */
  autoExtract?: boolean;
}

export interface ExportDesignMdParams {
  filePath?: string;
}

/** Read design.md 规格。 */
export async function handleGetDesignMd(
  params: GetDesignMdParams,
): Promise<{ hasDesignMd: boolean; spec?: DesignMdSpec; markdown?: string }> {
  // Try 模块首先缓存
  if (_mcpDesignMd) {
    return {
      hasDesignMd: true,
      spec: _mcpDesignMd,
      markdown: generateDesignMd(_mcpDesignMd),
    };
  }

  // Try 从文档中自动提取
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  const spec = extractDesignMdFromDocument(doc);
  const hasContent = !!(
    spec.colorPalette?.length ||
    spec.typography?.fontFamily ||
    spec.visualTheme
  );

  if (hasContent) {
    _mcpDesignMd = spec;
    return { hasDesignMd: true, spec, markdown: generateDesignMd(spec) };
  }

  return { hasDesignMd: false };
}

/** Import design.md 内容。 */
export async function handleSetDesignMd(
  params: SetDesignMdParams,
): Promise<{ success: boolean; spec?: DesignMdSpec }> {
  let spec: DesignMdSpec;

  if (params.autoExtract) {
    const filePath = resolveDocPath(params.filePath);
    const doc = await openDocument(filePath);
    spec = extractDesignMdFromDocument(doc);
  } else if (params.markdown) {
    spec = parseDesignMd(params.markdown);
  } else {
    return { success: false };
  }

  _mcpDesignMd = spec;
  setDesignMdForPrompt(spec);

  return { success: true, spec };
}

/** Export design.md 作为降价文本。 */
export async function handleExportDesignMd(
  params: ExportDesignMdParams,
): Promise<{ markdown: string }> {
  if (_mcpDesignMd) {
    return { markdown: generateDesignMd(_mcpDesignMd) };
  }

  // Auto-从文档中提取
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  const spec = extractDesignMdFromDocument(doc);
  return { markdown: generateDesignMd(spec) };
}
