import { join } from 'node:path';

export function getElectronBinaryPath(
  root: string,
  platform: NodeJS.Platform = process.platform,
): string {
  if (platform === 'win32') {
    return toForwardSlashes(join(root, 'node_modules', 'electron', 'dist', 'electron.exe'));
  }

  return toForwardSlashes(join(root, 'node_modules', '.bin', 'electron'));
}

function toForwardSlashes(path: string): string {
  return path.replace(/\\/g, '/');
}

export function getElectronSpawnEnv(env: NodeJS.ProcessEnv = process.env): NodeJS.ProcessEnv {
  const next = { ...env };
  delete next.ELECTRON_RUN_AS_NODE;
  return next;
}

interface DevServerProbeResult {
  baseReachable: boolean;
  viteClientReachable: boolean;
  viteClientStatus: number | null;
}

export function getDevServerConflictMessage(
  probe: DevServerProbeResult,
  port: number,
): string | null {
  if (probe.viteClientReachable) return null;
  if (probe.baseReachable && probe.viteClientStatus === 404) {
    return [
      `Port ${port} is responding, but it is not serving the Vite dev client.`,
      'A stale production server is likely still running on that port',
      '(for example `bun run ./out/web/server/index.mjs`).',
      'Stop it and retry the Electron dev launcher.',
    ].join(' ');
  }

  return null;
}
