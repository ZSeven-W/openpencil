import type { ThemePresetFile } from '@/types/theme-preset';
import type { VariableDefinition } from '@/types/variables';
import { supportsFileSystemAccess } from './file-operations';

function buildPresetFile(
  name: string,
  themes: Record<string, string[]>,
  variables: Record<string, VariableDefinition>,
): ThemePresetFile {
  return {
    type: 'openpencil-theme-preset',
    version: '1.0.0',
    name,
    themes,
    variables,
  };
}

function validatePresetFile(data: unknown): data is ThemePresetFile {
  if (!data || typeof data !== 'object') return false;
  const d = data as Record<string, unknown>;
  return (
    d.type === 'openpencil-theme-preset' &&
    typeof d.name === 'string' &&
    typeof d.themes === 'object' &&
    d.themes !== null &&
    typeof d.variables === 'object' &&
    d.variables !== null
  );
}

/** Export current themes+variables as a .optheme file. */
export async function exportThemePreset(
  name: string,
  themes: Record<string, string[]>,
  variables: Record<string, VariableDefinition>,
): Promise<void> {
  const preset = buildPresetFile(name, themes, variables);
  const json = JSON.stringify(preset, null, 2);
  const fileName = `${name.replace(/[^a-zA-Z0-9_-]/g, '_')}.optheme`;

  if (supportsFileSystemAccess()) {
    try {
      const handle: FileSystemFileHandle = await (
        window as unknown as {
          showSaveFilePicker: (opts: unknown) => Promise<FileSystemFileHandle>;
        }
      ).showSaveFilePicker({
        suggestedName: fileName,
        types: [
          {
            description: 'OpenPencil Theme Preset',
            accept: { 'application/json': ['.optheme'] },
          },
        ],
      });
      const writable = await handle.createWritable();
      await writable.write(json);
      await writable.close();
      return;
    } catch {
      // User cancelled
      return;
    }
  }

  // Fallback: browser download
  const blob = new Blob([json], { type: 'application/json' });
  const url = URL.createObjectURL(blob);
  const a = document.createElement('a');
  a.href = url;
  a.download = fileName;
  a.click();
  URL.revokeObjectURL(url);
}

/** Import a .optheme file and return parsed preset data. */
export async function importThemePreset(): Promise<{
  name: string;
  themes: Record<string, string[]>;
  variables: Record<string, VariableDefinition>;
} | null> {
  if (supportsFileSystemAccess()) {
    try {
      const [handle]: FileSystemFileHandle[] = await (
        window as unknown as {
          showOpenFilePicker: (opts: unknown) => Promise<FileSystemFileHandle[]>;
        }
      ).showOpenFilePicker({
        types: [
          {
            description: 'OpenPencil Theme Preset',
            accept: { 'application/json': ['.optheme'] },
          },
        ],
      });
      const file = await handle.getFile();
      const text = await file.text();
      const data = JSON.parse(text);
      if (!validatePresetFile(data)) return null;
      return { name: data.name, themes: data.themes, variables: data.variables };
    } catch {
      return null;
    }
  }

  // Fallback: <input type="file">
  return new Promise((resolve) => {
    const input = document.createElement('input');
    input.type = 'file';
    input.accept = '.optheme';
    input.onchange = async () => {
      const file = input.files?.[0];
      if (!file) {
        resolve(null);
        return;
      }
      try {
        const text = await file.text();
        const data = JSON.parse(text);
        if (!validatePresetFile(data)) {
          resolve(null);
          return;
        }
        resolve({ name: data.name, themes: data.themes, variables: data.variables });
      } catch {
        resolve(null);
      }
    };
    input.oncancel = () => resolve(null);
    input.click();
  });
}
