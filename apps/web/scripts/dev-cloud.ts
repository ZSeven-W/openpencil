import { spawn, type ChildProcess } from 'node:child_process';
import { resolve } from 'node:path';
import { loadEnvFiles } from './env';
import { buildDevCloudWebEnv } from './dev-cloud-config';

const PROJECT_ROOT = resolve(import.meta.dirname, '../../..');
const BUN_BINARY = process.env.OPENPENCIL_BUN_BINARY ?? 'bun';

const loadedEnv = loadEnvFiles();

function hasAnyEnv(...keys: string[]) {
  return keys.some((key) => Boolean(process.env[key]));
}

function validateCloudEnv() {
  const missing: string[] = [];
  if (!hasAnyEnv('SUPABASE_URL', 'VITE_SUPABASE_URL')) {
    missing.push('SUPABASE_URL or VITE_SUPABASE_URL');
  }
  if (!hasAnyEnv('SUPABASE_ANON_KEY', 'VITE_SUPABASE_ANON_KEY')) {
    missing.push('SUPABASE_ANON_KEY or VITE_SUPABASE_ANON_KEY');
  }
  if (!hasAnyEnv('SUPABASE_SERVICE_ROLE_KEY')) {
    missing.push('SUPABASE_SERVICE_ROLE_KEY');
  }
  if (missing.length === 0) return;

  const loaded = loadedEnv.loaded.map((file) => file.relativePath).join(', ') || 'none';
  console.error('[dev:cloud] Missing required cloud env values:');
  for (const key of missing) console.error(`  - ${key}`);
  console.error(
    '[dev:cloud] Put server-only values in .env.local or apps/web/.env.local. Do not prefix SUPABASE_SERVICE_ROLE_KEY with VITE_.',
  );
  console.error(`[dev:cloud] Loaded env files: ${loaded}.`);
  process.exit(1);
}

validateCloudEnv();

const children = new Set<ChildProcess>();
let shuttingDown = false;

function stopChildren(signal: NodeJS.Signals = 'SIGTERM') {
  shuttingDown = true;
  for (const child of children) {
    if (!child.killed) child.kill(signal);
  }
}

function spawnChild(label: string, args: string[], env: NodeJS.ProcessEnv = process.env) {
  const child = spawn(BUN_BINARY, args, {
    cwd: PROJECT_ROOT,
    stdio: 'inherit',
    env: { ...env },
  });
  children.add(child);
  child.on('exit', (code, signal) => {
    children.delete(child);
    if (shuttingDown) return;
    console.error(
      `[dev:cloud] ${label} exited${signal ? ` with ${signal}` : ` with code ${code ?? 0}`}.`,
    );
    stopChildren();
    process.exit(code ?? (signal ? 1 : 0));
  });
  return child;
}

console.log('[dev:cloud] Starting web dev server and codegen worker.');
console.log('[dev:cloud] The web process disables the embedded Nitro worker; standalone worker is authoritative.');

spawnChild('web', ['run', 'apps/web/dev.ts'], {
  ...buildDevCloudWebEnv(process.env),
});
spawnChild('codegen worker', ['--bun', 'apps/web/scripts/codegen-worker.ts']);

process.on('SIGINT', () => stopChildren('SIGINT'));
process.on('SIGTERM', () => stopChildren('SIGTERM'));
