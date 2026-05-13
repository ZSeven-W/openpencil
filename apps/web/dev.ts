import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import { loadEnvFiles } from './scripts/env';

const VITE_CLI = join(import.meta.dirname, '..', '..', 'node_modules', 'vite', 'bin', 'vite.js');

loadEnvFiles();

const DEV_PORT = process.env.PORT ?? process.env.VITE_DEV_PORT ?? '3003';

function resolveNodeBinary() {
  if (process.env.OPENPENCIL_NODE_BINARY) return process.env.OPENPENCIL_NODE_BINARY;

  const nvmNode = process.env.NVM_BIN ? join(process.env.NVM_BIN, 'node') : null;
  if (nvmNode && existsSync(nvmNode)) return nvmNode;

  return 'node';
}

const child = spawn(resolveNodeBinary(), [VITE_CLI, 'dev', '--port', DEV_PORT, '--strictPort'], {
  cwd: import.meta.dirname,
  stdio: 'inherit',
  env: { ...process.env },
});

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 0);
});
