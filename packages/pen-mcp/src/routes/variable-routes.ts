import { handleGetVariables, handleSetVariables, handleSetThemes } from '../tools/variables';
import { handleGetDesignMd, handleSetDesignMd, handleExportDesignMd } from '../tools/design-md';
import {
  handleSaveThemePreset,
  handleLoadThemePreset,
  handleListThemePresets,
} from '../tools/theme-presets';

export const VARIABLE_TOOL_DEFINITIONS = [
  {
    name: 'get_variables',
    description: 'Get all design variables and themes defined in an .op file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
      },
      required: [],
    },
  },
  {
    name: 'set_variables',
    description:
      'Add or update design variables in an .op file. By default merges with existing variables.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        variables: { type: 'object', description: 'Variables to set (name → { type, value })' },
        replace: {
          type: 'boolean',
          description: 'Replace all variables instead of merging (default false)',
        },
      },
      required: ['variables'],
    },
  },
  {
    name: 'set_themes',
    description:
      'Create or update theme axes and their variants in an .op file. Each theme axis (e.g. "Color Scheme") has an array of variant names (e.g. ["Light", "Dark"]). Multiple independent axes are supported. By default merges with existing themes.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        themes: {
          type: 'object',
          description:
            'Theme axes to set (axis name → variant names array). Example: { "Color": ["Light", "Dark"], "Density": ["Compact", "Comfortable"] }',
        },
        replace: {
          type: 'boolean',
          description: 'Replace all themes instead of merging (default false)',
        },
      },
      required: ['themes'],
    },
  },
  {
    name: 'get_design_md',
    description:
      'Get the design.md (design system specification) from the document. Returns the parsed spec and raw markdown. If no design.md is loaded, returns hasDesignMd: false.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
      },
      required: [],
    },
  },
  {
    name: 'set_design_md',
    description:
      'Import a design.md (design system specification) into the document. Accepts raw markdown or autoExtract=true to generate from existing document content. The design.md guides AI design generation with consistent colors, typography, and component styles.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        markdown: { type: 'string', description: 'Raw markdown content of design.md file' },
        autoExtract: {
          type: 'boolean',
          description:
            'Auto-generate design.md from existing document variables and design content (default false)',
        },
      },
      required: [],
    },
  },
  {
    name: 'export_design_md',
    description:
      'Export the design.md as markdown text. If no design.md exists, auto-extracts from document content.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
      },
      required: [],
    },
  },
  {
    name: 'save_theme_preset',
    description:
      'Save the themes and variables from an .op document as a reusable .optheme preset file.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        presetPath: { type: 'string', description: 'Absolute path for the output .optheme file' },
        name: {
          type: 'string',
          description: 'Display name for the preset (defaults to file name)',
        },
      },
      required: ['presetPath'],
    },
  },
  {
    name: 'load_theme_preset',
    description:
      'Load a .optheme preset file and merge its themes and variables into an .op document.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        presetPath: { type: 'string', description: 'Absolute path to the .optheme file to load' },
      },
      required: ['presetPath'],
    },
  },
  {
    name: 'list_theme_presets',
    description: 'List all .optheme preset files in a directory.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        directory: { type: 'string', description: 'Absolute path to the directory to scan' },
      },
      required: ['directory'],
    },
  },
];

export const VARIABLE_TOOL_NAMES = new Set([
  'get_variables',
  'set_variables',
  'set_themes',
  'get_design_md',
  'set_design_md',
  'export_design_md',
  'save_theme_preset',
  'load_theme_preset',
  'list_theme_presets',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleVariableToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'get_variables':
      return JSON.stringify(await handleGetVariables(a), null, 2);
    case 'set_variables':
      return JSON.stringify(await handleSetVariables(a), null, 2);
    case 'set_themes':
      return JSON.stringify(await handleSetThemes(a), null, 2);
    case 'get_design_md':
      return JSON.stringify(await handleGetDesignMd(a), null, 2);
    case 'set_design_md':
      return JSON.stringify(await handleSetDesignMd(a), null, 2);
    case 'export_design_md':
      return JSON.stringify(await handleExportDesignMd(a), null, 2);
    case 'save_theme_preset':
      return JSON.stringify(await handleSaveThemePreset(a), null, 2);
    case 'load_theme_preset':
      return JSON.stringify(await handleLoadThemePreset(a), null, 2);
    case 'list_theme_presets':
      return JSON.stringify(await handleListThemePresets(a), null, 2);
    default:
      return '';
  }
}
