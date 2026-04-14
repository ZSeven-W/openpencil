import { spawn } from 'node:child_process';
import { join } from 'node:path';

const VITE_CLI = join(import.meta.dirname, '..', '..', 'node_modules', 'vite', 'bin', 'vite.js');

const child = spawn('node', [VITE_CLI, 'dev', '--port', '3000'], {
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
