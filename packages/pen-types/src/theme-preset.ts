import type { VariableDefinition } from './variables.js';

export interface ThemePreset {
  id: string;
  name: string;
  themes: Record<string, string[]>;
  variables: Record<string, VariableDefinition>;
  createdAt: number;
}

export interface ThemePresetFile {
  type: 'openpencil-theme-preset';
  version: '1.0.0';
  name: string;
  themes: Record<string, string[]>;
  variables: Record<string, VariableDefinition>;
}
