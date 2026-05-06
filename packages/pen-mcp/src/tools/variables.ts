import { openDocument, saveDocument, resolveDocPath } from '../document-manager';
import type { VariableDefinition } from '@zseven-w/pen-types';

export interface GetVariablesParams {
  filePath?: string;
}

export interface SetVariablesParams {
  filePath?: string;
  variables: Record<string, VariableDefinition>;
  replace?: boolean;
}

export async function handleGetVariables(
  params: GetVariablesParams,
): Promise<{ variables: Record<string, VariableDefinition>; themes: Record<string, string[]> }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  return {
    variables: doc.variables ?? {},
    themes: doc.themes ?? {},
  };
}

export async function handleSetVariables(
  params: SetVariablesParams,
): Promise<{ variables: Record<string, VariableDefinition> }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);

  if (params.replace) {
    doc.variables = params.variables;
  } else {
    doc.variables = { ...doc.variables, ...params.variables };
  }

  await saveDocument(filePath, doc);
  return { variables: doc.variables };
}

// ---------------------------------------------------------------------------
// set_themes
// ---------------------------------------------------------------------------

export interface SetThemesParams {
  filePath?: string;
  themes: Record<string, string[]>;
  replace?: boolean;
}

/**
 * Create，更新或替换
 *
 * 主题轴及其变体。 Data 模型：`doc.themes` 是 `Record<string,
 * string[]>`，其
 * 中 key = 主题轴名称（例如“Color Scheme”） value = 变体名称（例如
 *
 * [“Light”、“Dark”]）With `replace:
 * false`（默认），前提是轴合并到现有主题中 - 保留未提及的现有轴。 With
 * `replace:
 true`，现有主题已完全替换。
 */
export async function handleSetThemes(
  params: SetThemesParams,
): Promise<{ themes: Record<string, string[]> }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);

  if (params.replace) {
    doc.themes = params.themes;
  } else {
    doc.themes = { ...doc.themes, ...params.themes };
  }

  await saveDocument(filePath, doc);
  return { themes: doc.themes };
}
