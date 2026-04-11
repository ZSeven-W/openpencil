/**
 * Electron development workflow orchestrator.
 *
 * 1. Start Vite dev server (bun run dev)
 * 2. Wait for it to be ready on port 3000
 * 3. Compile electron/ with esbuild
 * 4. Launch Electron pointing at the dev server
 */

import { spawn, execSync, type ChildProcess } from 'node:child_process';
import { build } from 'esbuild';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { compileSkills } from '../../packages/pen-ai-skills/vite-plugin-skills';
import {
  getDevServerConflictMessage,
  getElectronBinaryPath,
  getElectronSpawnEnv,
} from './dev-utils';

const DESKTOP_DIR = import.meta.dirname;
const ROOT = join(DESKTOP_DIR, '..', '..');
const WEB_DIR = join(ROOT, 'apps', 'web');
const VITE_DEV_PORT = 3000;
const GENERATED_SKILL_REGISTRY = join(
  ROOT,
  'packages',
  'pen-ai-skills',
  'src',
  '_generated',
  'skill-registry.ts',
);

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

async function waitForViteServer(
  baseUrl: string,
  vite: ChildProcess,
  port: number,
  timeoutMs = 30_000,
): Promise<void> {
  const start = Date.now();
  let viteExit: { code: number | null; signal: NodeJS.Signals | null } | null = null;
  const handleExit = (code: number | null, signal: NodeJS.Signals | null) => {
    viteExit = { code, signal };
  };

  vite.once('exit', handleExit);
  while (Date.now() - start < timeoutMs) {
    let baseReachable = false;
    let viteClientReachable = false;
    let viteClientStatus: number | null = null;

    try {
      const res = await fetch(baseUrl, {
        signal: AbortSignal.timeout(500),
      });
      baseReachable = res.ok || res.status < 500;
    } catch {
      // server not ready yet
    }

    try {
      const res = await fetch(`${baseUrl}/@vite/client`, {
        signal: AbortSignal.timeout(500),
      });
      viteClientStatus = res.status;
      viteClientReachable = res.ok;
      if (viteClientReachable) {
        vite.off('exit', handleExit);
        return;
      }
    } catch {
      // Vite client not ready yet.
    }

    const conflict = getDevServerConflictMessage(
      {
        baseReachable,
        viteClientReachable,
        viteClientStatus,
      },
      port,
    );
    if (conflict) {
      vite.off('exit', handleExit);
      throw new Error(conflict);
    }

    if (viteExit) {
      vite.off('exit', handleExit);
      const detail = viteExit.signal
        ? `signal ${viteExit.signal}`
        : `exit code ${viteExit.code ?? 'unknown'}`;
      throw new Error(`Vite dev server exited before becoming ready (${detail}).`);
    }

    await new Promise((r) => setTimeout(r, 500));
  }

  vite.off('exit', handleExit);
  throw new Error(`Timeout waiting for Vite dev server on ${baseUrl}`);
}

async function compileElectron(): Promise<void> {
  const common: Parameters<typeof build>[0] = {
    platform: 'node',
    bundle: true,
    sourcemap: true,
    external: ['electron'],
    target: 'node20',
    outdir: join(ROOT, 'out', 'desktop'),
    outExtension: { '.js': '.cjs' },
    format: 'cjs' as const,
  };

  await Promise.all([
    build({
      ...common,
      entryPoints: [join(DESKTOP_DIR, 'main.ts')],
    }),
    build({
      ...common,
      entryPoints: [join(DESKTOP_DIR, 'preload.ts')],
    }),
  ]);

  console.log('[electron-dev] Electron files compiled');
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

async function main(): Promise<void> {
  // 1. Start Vite dev server
  console.log('[electron-dev] Starting Vite dev server...');
  // Launch Vite directly on Windows. Spawning through `bun run dev` can tear
  // down the inner `vite.exe` process after startup, leaving Electron with a
  // ready log but no live dev server to connect to.
  const vite = spawn('bun', ['--bun', 'vite', 'dev', '--port', String(VITE_DEV_PORT)], {
    cwd: WEB_DIR,
    stdio: 'inherit',
    env: { ...process.env },
  });

  const stopVite = () => {
    if (process.platform === 'win32' && vite.pid) {
      try {
        execSync(`taskkill /pid ${vite.pid} /T /F`, { stdio: 'ignore' });
      } catch {
        /* ignore */
      }
      return;
    }

    vite.kill();
  };

  // Ensure cleanup on exit
  const cleanup = () => {
    stopVite();
    process.exit();
  };
  process.on('SIGINT', cleanup);
  process.on('SIGTERM', cleanup);

  // 2. Wait for Vite to be ready
  console.log(`[electron-dev] Waiting for Vite on port ${VITE_DEV_PORT}...`);
  try {
    await waitForViteServer(`http://localhost:${VITE_DEV_PORT}`, vite, VITE_DEV_PORT);
  } catch (error) {
    stopVite();
    throw error;
  }
  console.log('[electron-dev] Vite is ready');

  // 3. Compile MCP server + Electron files
  try {
    compileSkills(join(ROOT, 'packages', 'pen-ai-skills'));
  } catch (err) {
    if (!existsSync(GENERATED_SKILL_REGISTRY)) {
      throw err;
    }
    console.warn('[electron-dev] Skill registry refresh failed, using existing generated registry');
    console.warn(err);
  }
  console.log('[electron-dev] Compiling MCP server...');
  await build({
    platform: 'node',
    bundle: true,
    sourcemap: true,
    target: 'node20',
    format: 'cjs',
    entryPoints: [join(ROOT, 'packages', 'pen-mcp', 'src', 'server.ts')],
    outfile: join(ROOT, 'out', 'mcp-server.cjs'),
    alias: {
      '@zseven-w/pen-types': join(ROOT, 'packages', 'pen-types', 'src'),
      '@zseven-w/pen-core': join(ROOT, 'packages', 'pen-core', 'src'),
      '@zseven-w/pen-figma': join(ROOT, 'packages', 'pen-figma', 'src'),
      '@zseven-w/pen-renderer': join(ROOT, 'packages', 'pen-renderer', 'src'),
      '@zseven-w/pen-sdk': join(ROOT, 'packages', 'pen-sdk', 'src'),
      '@zseven-w/pen-ai-skills': join(ROOT, 'packages', 'pen-ai-skills', 'src'),
      '@zseven-w/pen-mcp': join(ROOT, 'packages', 'pen-mcp', 'src'),
      '@zseven-w/pen-engine': join(ROOT, 'packages', 'pen-engine', 'src'),
      '@zseven-w/pen-react': join(ROOT, 'packages', 'pen-react', 'src'),
    },
    define: { 'import.meta.env': '{}' },
    external: ['canvas', 'paper'],
  });
  console.log('[electron-dev] MCP server compiled');

  await compileElectron();

  // 4. Launch Electron
  console.log('[electron-dev] Starting Electron...');
  const electronBin = getElectronBinaryPath(ROOT);
  const electron = spawn(electronBin, [ROOT], {
    cwd: ROOT,
    stdio: 'inherit',
    env: getElectronSpawnEnv(process.env),
  }) as ChildProcess;

  electron.on('exit', () => {
    stopVite();
    process.exit();
  });
}

main().catch((err) => {
  console.error(err);
  process.exit(1);
});
