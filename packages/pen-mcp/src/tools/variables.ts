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
 * Create, update, or replace theme axes and their variants.
 *
 * Data model: `doc.themes` is `Record<string, string[]>` where
 *   key = theme axis name (e.g. "Color Scheme")
 *   value = variant names (e.g. ["Light", "Dark"])
 *
 * With `replace: false` (default), provided axes are merged into existing
 * themes — existing axes not mentioned are preserved.
 * With `replace: true`, existing themes are fully replaced.
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
