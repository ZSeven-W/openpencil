import { readFile, writeFile, readdir } from 'node:fs/promises';
import { resolve, join, extname, basename } from 'node:path';
import { openDocument, saveDocument, resolveDocPath } from '../document-manager';
import type { ThemePresetFile } from '@zseven-w/pen-types';
import type { VariableDefinition } from '@zseven-w/pen-types';

// ---------------------------------------------------------------------------
// save_theme_preset
// ---------------------------------------------------------------------------

export interface SaveThemePresetParams {
  filePath?: string;
  presetPath: string;
  name?: string;
}

export async function handleSaveThemePreset(
  params: SaveThemePresetParams,
): Promise<{ ok: boolean; path: string }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  const presetPath = resolve(params.presetPath);

  const preset: ThemePresetFile = {
    type: 'openpencil-theme-preset',
    version: '1.0.0',
    name: params.name ?? basename(presetPath, extname(presetPath)),
    themes: doc.themes ?? {},
    variables: doc.variables ?? {},
  };

  await writeFile(presetPath, JSON.stringify(preset, null, 2), 'utf-8');
  return { ok: true, path: presetPath };
}

// ---------------------------------------------------------------------------
// load_theme_preset
// ---------------------------------------------------------------------------

export interface LoadThemePresetParams {
  filePath?: string;
  presetPath: string;
}

export async function handleLoadThemePreset(
  params: LoadThemePresetParams,
): Promise<{ themes: Record<string, string[]>; variableCount: number }> {
  const filePath = resolveDocPath(params.filePath);
  const doc = await openDocument(filePath);
  const presetPath = resolve(params.presetPath);

  const raw = await readFile(presetPath, 'utf-8');
  const data = JSON.parse(raw) as ThemePresetFile;

  if (data.type !== 'openpencil-theme-preset') {
    throw new Error('Invalid theme preset file');
  }

  // Merge themes
  doc.themes = { ...doc.themes, ...data.themes };

  // Merge variables
  doc.variables = { ...doc.variables, ...data.variables } as Record<string, VariableDefinition>;

  await saveDocument(filePath, doc);
  return {
    themes: doc.themes,
    variableCount: Object.keys(doc.variables ?? {}).length,
  };
}

// ---------------------------------------------------------------------------
// list_theme_presets
// ---------------------------------------------------------------------------

export interface ListThemePresetsParams {
  directory: string;
}

export async function handleListThemePresets(
  params: ListThemePresetsParams,
): Promise<{ presets: { name: string; path: string }[] }> {
  const dir = resolve(params.directory);
  const entries = await readdir(dir);
  const presets: { name: string; path: string }[] = [];

  for (const entry of entries) {
    if (extname(entry) !== '.optheme') continue;
    const fullPath = join(dir, entry);
    try {
      const raw = await readFile(fullPath, 'utf-8');
      const data = JSON.parse(raw) as ThemePresetFile;
      if (data.type === 'openpencil-theme-preset') {
        presets.push({ name: data.name ?? basename(entry, '.optheme'), path: fullPath });
      }
    } catch {
      // skip invalid files
    }
  }

  return { presets };
}
