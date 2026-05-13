import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { afterEach, describe, expect, it } from 'vitest';
import { loadEnvFiles, parseDotenv } from '../../../scripts/env';
import { buildDevCloudWebEnv } from '../../../scripts/dev-cloud-config';

const tempRoots: string[] = [];

afterEach(() => {
  for (const root of tempRoots.splice(0)) {
    rmSync(root, { recursive: true, force: true });
  }
});

function makeRoot() {
  const root = mkdtempSync(join(tmpdir(), 'openpencil-env-'));
  tempRoots.push(root);
  mkdirSync(join(root, 'apps', 'web'), { recursive: true });
  return root;
}

describe('script env loading', () => {
  it('parses quoted values, comments, and export prefixes', () => {
    expect(
      parseDotenv(`
        # comment
        export SUPABASE_URL="https://example.supabase.co"
        SUPABASE_SERVICE_ROLE_KEY='service # value'
        INLINE=value # trailing comment
        HASH=abc#123
      `),
    ).toEqual({
      SUPABASE_URL: 'https://example.supabase.co',
      SUPABASE_SERVICE_ROLE_KEY: 'service # value',
      INLINE: 'value',
      HASH: 'abc#123',
    });
  });

  it('loads root and web env files without overriding existing shell values', () => {
    const root = makeRoot();
    writeFileSync(join(root, '.env'), 'SUPABASE_URL=https://root.supabase.co\nWORKER_FLAG=root\n');
    writeFileSync(
      join(root, 'apps', 'web', '.env'),
      'SUPABASE_SERVICE_ROLE_KEY=web-service\nWORKER_FLAG=web\n',
    );
    writeFileSync(
      join(root, 'apps', 'web', '.env.local'),
      'SUPABASE_SERVICE_ROLE_KEY=local-service\n',
    );
    const env: NodeJS.ProcessEnv = {
      SUPABASE_URL: 'https://shell.supabase.co',
    };

    const result = loadEnvFiles({ rootDir: root, env });

    expect(env.SUPABASE_URL).toBe('https://shell.supabase.co');
    expect(env.SUPABASE_SERVICE_ROLE_KEY).toBe('local-service');
    expect(env.WORKER_FLAG).toBe('web');
    expect(result.loaded.map((item) => item.relativePath)).toEqual([
      '.env',
      'apps/web/.env',
      'apps/web/.env.local',
    ]);
    expect(result.loaded[0]?.skipped).toContain('SUPABASE_URL');
  });

  it('builds a web dev env that disables embedded Nitro codegen workers', () => {
    expect(buildDevCloudWebEnv({ NODE_ENV: 'development' })).toMatchObject({
      NODE_ENV: 'development',
      OPENPENCIL_CODEGEN_WORKER: 'disabled',
      OPENPENCIL_DEV_CLOUD: '1',
      VITE_OPENPENCIL_CODEGEN_WORKER: 'disabled',
    });
  });
});
