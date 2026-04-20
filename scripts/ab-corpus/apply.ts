/**
 * Concrete ApplyFn implementation that routes element-tool calls and
 * batch_design DSL through the real pen-mcp handlers. Lives in
 * scripts/ (not a package) to keep the pen-ai-skills ↔ pen-mcp
 * dependency unidirectional — the scorer in packages/pen-ai-skills
 * declares the ApplyFn contract, this file implements it.
 */

import { mkdtempSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import type { PenDocument } from '@zseven-w/pen-types';
import type { ApplyFn, ApplyResult, ParsedOutput } from '@zseven-w/pen-ai-skills';
import {
  ELEMENT_TOOL_NAMES,
  handleBatchDesign,
  handleElementToolCall,
  invalidateCache,
} from '@zseven-w/pen-mcp';

const EMPTY = JSON.stringify({ version: '1.0.0', children: [] });

/**
 * Apply a parsed output to a fresh, disposable .op file. The scorer
 * reads the resulting PenDocument back and does not need the file to
 * persist — we clean up per call.
 *
 * On success: `ok: true` + `doc: PenDocument`.
 * On failure (handler throws OR writes no root): `ok: false` +
 *   `error: <message>` + `doc: null`. Throws are caught so a single
 *   bad run never halts the whole corpus sweep.
 */
export const applyToFreshDoc: ApplyFn = async (parsed: ParsedOutput): Promise<ApplyResult> => {
  if (parsed.kind === 'garbage') {
    return { ok: false, error: `unparseable: ${parsed.reason}`, doc: null };
  }
  const tmpDir = mkdtempSync(join(tmpdir(), 'ab-corpus-apply-'));
  const fp = join(tmpDir, 'doc.op');
  writeFileSync(fp, EMPTY, 'utf-8');
  try {
    if (parsed.kind === 'tool_call') {
      if (!ELEMENT_TOOL_NAMES.has(parsed.name)) {
        return {
          ok: false,
          error: `unknown element tool "${parsed.name}"`,
          doc: null,
        };
      }
      await handleElementToolCall(parsed.name, { ...parsed.arguments, filePath: fp });
    } else {
      const result = await handleBatchDesign({
        operations: parsed.dsl,
        filePath: fp,
        postProcess: false,
      });
      if (result.errors && result.errors.length > 0) {
        const summary = result.errors.map((e) => `${e.line.slice(0, 60)}: ${e.error}`).join('; ');
        return { ok: false, error: `batch_design errors: ${summary}`, doc: null };
      }
    }
    const doc = JSON.parse(readFileSync(fp, 'utf-8')) as PenDocument;
    const topLevel = (doc.children ?? doc.pages?.[0]?.children ?? []) as unknown[];
    if (topLevel.length === 0) {
      return { ok: false, error: 'apply produced no root nodes', doc: null };
    }
    return { ok: true, error: '', doc };
  } catch (err) {
    return {
      ok: false,
      error: err instanceof Error ? err.message : String(err),
      doc: null,
    };
  } finally {
    invalidateCache(fp);
    try {
      rmSync(tmpDir, { recursive: true, force: true });
    } catch {
      // Best-effort cleanup; leaving a tmp file behind is harmless.
    }
  }
};
