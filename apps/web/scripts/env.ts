import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const PROJECT_ROOT = resolve(import.meta.dirname, '../../..');

export const DEFAULT_ENV_FILES = ['.env', '.env.local', 'apps/web/.env', 'apps/web/.env.local'];

export interface LoadedEnvFile {
  path: string;
  relativePath: string;
  applied: string[];
  skipped: string[];
}

export interface LoadEnvResult {
  searched: string[];
  loaded: LoadedEnvFile[];
}

export interface LoadEnvFilesOptions {
  rootDir?: string;
  files?: readonly string[];
  env?: NodeJS.ProcessEnv;
  override?: boolean;
}

function stripInlineComment(value: string): string {
  let quote: '"' | "'" | null = null;
  for (let index = 0; index < value.length; index += 1) {
    const char = value[index];
    if ((char === '"' || char === "'") && value[index - 1] !== '\\') {
      quote = quote === char ? null : quote ?? char;
      continue;
    }
    if (char === '#' && quote === null && (index === 0 || /\s/.test(value[index - 1] ?? ''))) {
      return value.slice(0, index).trimEnd();
    }
  }
  return value.trimEnd();
}

function unquote(value: string): string {
  const trimmed = stripInlineComment(value.trim());
  const first = trimmed[0];
  const last = trimmed[trimmed.length - 1];
  if ((first === '"' || first === "'") && last === first) {
    const inner = trimmed.slice(1, -1);
    if (first === '"') {
      return inner.replace(/\\n/g, '\n').replace(/\\r/g, '\r').replace(/\\t/g, '\t');
    }
    return inner;
  }
  return trimmed;
}

export function parseDotenv(content: string): Record<string, string> {
  const values: Record<string, string> = {};
  for (const rawLine of content.split(/\r?\n/)) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const match = /^(?:export\s+)?([A-Za-z_][A-Za-z0-9_]*)\s*=\s*(.*)$/.exec(line);
    if (!match) continue;
    values[match[1]] = unquote(match[2] ?? '');
  }
  return values;
}

export function loadEnvFiles(options: LoadEnvFilesOptions = {}): LoadEnvResult {
  const rootDir = options.rootDir ?? PROJECT_ROOT;
  const files = options.files ?? DEFAULT_ENV_FILES;
  const env = options.env ?? process.env;
  const initialKeys = new Set(Object.keys(env));
  const loaded: LoadedEnvFile[] = [];
  const searched: string[] = [];

  for (const relativePath of files) {
    const path = resolve(rootDir, relativePath);
    searched.push(path);
    if (!existsSync(path)) continue;

    const parsed = parseDotenv(readFileSync(path, 'utf8'));
    const applied: string[] = [];
    const skipped: string[] = [];
    for (const [key, value] of Object.entries(parsed)) {
      if (!options.override && initialKeys.has(key)) {
        skipped.push(key);
        continue;
      }
      env[key] = value;
      applied.push(key);
    }
    loaded.push({ path, relativePath, applied, skipped });
  }

  return { searched, loaded };
}
