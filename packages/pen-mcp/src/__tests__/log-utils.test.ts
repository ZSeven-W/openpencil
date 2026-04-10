import { describe, it, expect, beforeEach, afterEach } from 'vitest';
import { writeFile, unlink, mkdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { SENSITIVE_LOG_PATTERN, readDebugTail, readLogTail } from '../utils/log-utils';

const TMP_DIR = join(tmpdir(), 'pen-mcp-log-utils-tests');

beforeEach(async () => {
  await mkdir(TMP_DIR, { recursive: true });
});

afterEach(async () => {
  try {
    await unlink(join(TMP_DIR, 'test.log'));
  } catch {}
});

describe('SENSITIVE_LOG_PATTERN', () => {
  it('matches ANTHROPIC_API_KEY assignment', () => {
    expect(SENSITIVE_LOG_PATTERN.test('ANTHROPIC_API_KEY=sk-ant-xxx')).toBe(true);
  });
  it('matches Authorization Bearer', () => {
    expect(SENSITIVE_LOG_PATTERN.test('Authorization: Bearer eyJ0...')).toBe(true);
  });
  it('matches api_key / api-key / apikey case-insensitive', () => {
    expect(SENSITIVE_LOG_PATTERN.test('api_key: secret')).toBe(true);
    expect(SENSITIVE_LOG_PATTERN.test('API-KEY=xxx')).toBe(true);
    expect(SENSITIVE_LOG_PATTERN.test('ApiKey: xxx')).toBe(true);
  });
  it('does not match unrelated lines', () => {
    expect(SENSITIVE_LOG_PATTERN.test('[INFO] request completed')).toBe(false);
  });
});

describe('readDebugTail', () => {
  it('returns undefined for missing path', async () => {
    expect(await readDebugTail(undefined)).toBeUndefined();
  });

  it('strips sensitive lines from the tail', async () => {
    const path = join(TMP_DIR, 'test.log');
    await writeFile(path, ['[INFO] ok', 'ANTHROPIC_API_KEY=sk-bad', '[INFO] done', ''].join('\n'));
    const tail = await readDebugTail(path, 10);
    expect(tail).toBeDefined();
    expect(tail!).toEqual(['[INFO] ok', '[INFO] done']);
  });
});

describe('readLogTail', () => {
  it('reads last N lines', async () => {
    const path = join(TMP_DIR, 'test.log');
    const lines = Array.from({ length: 20 }, (_, i) => `line-${i}`);
    await writeFile(path, lines.join('\n') + '\n');
    const out = await readLogTail({ path, tailLines: 5 });
    expect(out).toEqual(['line-15', 'line-16', 'line-17', 'line-18', 'line-19']);
  });

  it('filters by grep regex', async () => {
    const path = join(TMP_DIR, 'test.log');
    await writeFile(path, ['[INFO] a', '[WARN] b', '[INFO] c', '[ERROR] d', ''].join('\n'));
    const out = await readLogTail({ path, grep: '\\[INFO\\]' });
    expect(out).toEqual(['[INFO] a', '[INFO] c']);
  });

  it('redacts sensitive lines before returning', async () => {
    const path = join(TMP_DIR, 'test.log');
    await writeFile(
      path,
      ['[INFO] ok', 'Authorization: Bearer secret', '[INFO] done', ''].join('\n'),
    );
    const out = await readLogTail({ path });
    expect(out).toEqual(['[INFO] ok', '[INFO] done']);
  });
});
