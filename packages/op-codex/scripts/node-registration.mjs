import { mkdir, writeFile, rename, rm } from 'node:fs/promises';
import { isAbsolute, join } from 'node:path';
import { randomUUID } from 'node:crypto';
import { homedir } from 'node:os';

// A fixed private location remains discoverable when the host strips custom env.
export async function registerNode(state = join(homedir(), '.codex-openpencil'), binary = process.execPath) {
  if (!isAbsolute(binary) || /[\r\n\0]/.test(binary)) throw new Error('Invalid Node executable path.');
  await mkdir(state, { recursive: true, mode: 0o700 });
  const temporary = join(state, `.node-${randomUUID()}`);
  try {
    await writeFile(temporary, `${binary}\n`, { flag: 'wx', mode: 0o600 });
    await rename(temporary, join(state, 'node-path'));
  } finally { await rm(temporary, { force: true }); }
}
