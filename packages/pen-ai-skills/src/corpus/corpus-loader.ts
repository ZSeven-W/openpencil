import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { load as loadYaml } from 'js-yaml';
import type { CorpusPrompt } from './types';

const VALID_CATEGORIES = new Set(['mobile', 'dashboard', 'landing']);
const VALID_DIFFICULTIES = new Set(['obvious', 'optional']);

/**
 * Load every *.yaml file in the corpus directory. Validates the schema
 * per entry and throws on malformed files so a typo doesn't silently
 * drop a prompt from the run. Sort order is stable (filename ascending)
 * so downstream score aggregation is deterministic.
 *
 * Pass the DIRECTORY containing the yaml files — not the repo root.
 */
export function loadCorpus(dir: string): CorpusPrompt[] {
  let entries: string[];
  try {
    entries = readdirSync(dir);
  } catch (err) {
    const msg = err instanceof Error ? err.message : String(err);
    throw new Error(`loadCorpus: cannot read directory ${dir}: ${msg}`);
  }
  const files = entries.filter((f) => f.endsWith('.yaml')).sort();
  const prompts: CorpusPrompt[] = [];
  for (const file of files) {
    const fullPath = join(dir, file);
    const st = statSync(fullPath);
    if (!st.isFile()) continue;
    const raw = readFileSync(fullPath, 'utf-8');
    const parsed = loadYaml(raw) as unknown;
    const prompt = validateCorpusPrompt(parsed, file);
    prompts.push(prompt);
  }
  const ids = new Set<string>();
  for (const p of prompts) {
    if (ids.has(p.id)) {
      throw new Error(`loadCorpus: duplicate prompt id "${p.id}"`);
    }
    ids.add(p.id);
  }
  return prompts;
}

function validateCorpusPrompt(raw: unknown, file: string): CorpusPrompt {
  if (!raw || typeof raw !== 'object') {
    throw new Error(`loadCorpus: ${file} is not a YAML object at the top level`);
  }
  const o = raw as Record<string, unknown>;
  const id = requireString(o.id, file, 'id');
  const category = requireString(o.category, file, 'category');
  if (!VALID_CATEGORIES.has(category)) {
    throw new Error(`loadCorpus: ${file}: invalid category "${category}"`);
  }
  const difficulty = requireString(o.difficulty, file, 'difficulty');
  if (!VALID_DIFFICULTIES.has(difficulty)) {
    throw new Error(`loadCorpus: ${file}: invalid difficulty "${difficulty}"`);
  }
  const prompt = requireString(o.prompt, file, 'prompt');
  const expected = validateExpected(o.expected, file);
  const toolHint = o.expected_tool_if_any;
  if (toolHint !== undefined && typeof toolHint !== 'string') {
    throw new Error(`loadCorpus: ${file}: expected_tool_if_any must be a string when present`);
  }
  if (difficulty === 'obvious' && !toolHint) {
    throw new Error(
      `loadCorpus: ${file}: difficulty=obvious requires expected_tool_if_any (used to score M5 routing)`,
    );
  }
  return {
    id,
    category: category as CorpusPrompt['category'],
    difficulty: difficulty as CorpusPrompt['difficulty'],
    prompt,
    expected,
    expected_tool_if_any: toolHint,
  };
}

function validateExpected(raw: unknown, file: string): CorpusPrompt['expected'] {
  if (raw === undefined || raw === null) return {};
  if (typeof raw !== 'object') {
    throw new Error(`loadCorpus: ${file}: expected must be an object or omitted`);
  }
  const o = raw as Record<string, unknown>;
  const out: CorpusPrompt['expected'] = {};
  if (o.must_contain_roles !== undefined) {
    if (!Array.isArray(o.must_contain_roles)) {
      throw new Error(`loadCorpus: ${file}: must_contain_roles must be an array`);
    }
    for (const r of o.must_contain_roles) {
      if (typeof r !== 'string') {
        throw new Error(`loadCorpus: ${file}: must_contain_roles entries must be strings`);
      }
    }
    out.must_contain_roles = o.must_contain_roles as string[];
  }
  if (o.min_roles !== undefined) {
    if (!o.min_roles || typeof o.min_roles !== 'object' || Array.isArray(o.min_roles)) {
      throw new Error(`loadCorpus: ${file}: min_roles must be an object of role→number`);
    }
    const m = o.min_roles as Record<string, unknown>;
    const validated: Record<string, number> = {};
    for (const [role, count] of Object.entries(m)) {
      if (typeof count !== 'number' || !Number.isFinite(count) || count < 0) {
        throw new Error(`loadCorpus: ${file}: min_roles.${role} must be a non-negative number`);
      }
      validated[role] = count;
    }
    out.min_roles = validated;
  }
  return out;
}

function requireString(v: unknown, file: string, key: string): string {
  if (typeof v !== 'string' || v.length === 0) {
    throw new Error(`loadCorpus: ${file}: field "${key}" must be a non-empty string`);
  }
  return v;
}
