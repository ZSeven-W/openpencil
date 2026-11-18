import { spawn } from 'node:child_process';
import { randomUUID } from 'node:crypto';
import { mkdir, mkdtemp, readFile, writeFile, rename, lstat, rm } from 'node:fs/promises';
import { basename, dirname, isAbsolute, join } from 'node:path';
import { stateRoot, sha256 } from './editor-runtime.mjs';
import { ensureRuntime } from './runtime-install.mjs';
import { post } from './transport.mjs';
import { snapshotMatches, diskDocument } from './document-format.mjs';

export function validateDocument(bytes) {
  const document = JSON.parse(bytes.toString());
  if (!document || typeof document !== 'object' || Array.isArray(document) ||
      (!Array.isArray(document.children) && !Array.isArray(document.pages))) {
    throw new Error('Expected an OpenPencil document with children or pages.');
  }
  return document;
}

export async function sourceBytes(path) {
  if (!isAbsolute(path) || !path.toLowerCase().endsWith('.op')) throw new Error('Use an absolute .op file path.');
  const stat = await lstat(path);
  if (!stat.isFile() || stat.isSymbolicLink() || stat.size > 32 * 1024 * 1024) {
    throw new Error('The .op source must be a regular file smaller than 32 MiB.');
  }
  const bytes = await readFile(path);
  validateDocument(bytes);
  return bytes;
}

export async function atomicSave(path, bytes, baseline) {
  if (Buffer.byteLength(bytes) > 32 * 1024 * 1024) throw new Error('Saved document exceeds 32 MiB.');
  validateDocument(bytes);
  if (baseline === null) {
    // Save As never silently replaces a different existing document.
    await writeFile(path, bytes, { flag: 'wx', mode: 0o600 });
    return sha256(bytes);
  }
  if (sha256(await sourceBytes(path)) !== baseline) throw new Error('Source changed outside this canvas. Save was stopped; keep the recovery draft.');
  const temp = join(dirname(path), `.${basename(path)}.codex-${randomUUID()}.tmp`);
  try {
    await writeFile(temp, bytes, { flag: 'wx', mode: 0o600 });
    // Check again after staging, before replacing the source.
    if (sha256(await sourceBytes(path)) !== baseline) throw new Error('Source changed while saving; save was stopped.');
    await rename(temp, path);
    return sha256(bytes);
  } finally { await rm(temp, { force: true }); }
}

function handshake(child) {
  return new Promise((resolve, reject) => {
    let buffer = '';
    const timer = setTimeout(() => finish(new Error('Rust editor startup timed out.')), 20_000);
    const onExit = code => finish(new Error(process.platform === 'win32' && [0xc0000135, 0xc000007b].includes(code >>> 0)
      ? 'Rust editor needs the matching Microsoft Visual C++ v14 Redistributable. Install it from Microsoft, then run setup again.'
      : 'Rust editor exited before its handshake.'));
    const onData = (bytes) => {
      buffer += bytes.toString('utf8');
      if (buffer.length > 16_384) return finish(new Error('Rust editor handshake exceeded its limit.'));
      const newline = buffer.indexOf('\n');
      if (newline < 0) return;
      try {
        const value = JSON.parse(buffer.slice(0, newline));
        if (!value.ok || !Number.isInteger(value.port) || value.port < 1 || value.port > 65535 ||
            typeof value.token !== 'string' || value.token.length < 8) throw new Error();
        finish(null, value);
      } catch { finish(new Error('Invalid Rust editor startup handshake.')); }
    };
    const finish = (error, value) => {
      clearTimeout(timer); child.stdout.off('data', onData); child.off('exit', onExit); child.off('error', finish);
      if (error) reject(error); else resolve(value);
    };
    child.stdout.on('data', onData); child.once('exit', onExit); child.once('error', finish);
  });
}

export class EditorSession {
  static async start({ filePath, allowOrigin, runtimeRoot }) {
    const runtime = await ensureRuntime(runtimeRoot);
    await mkdir(join(stateRoot(), 'sessions'), { recursive: true, mode: 0o700 });
    const dir = await mkdtemp(join(stateRoot(), 'sessions', 'canvas-'));
    const draft = join(dir, 'recovery.op');
    const initial = filePath ? await sourceBytes(filePath) : Buffer.from('{"version":"0.8.5","children":[]}\n');
    await writeFile(draft, initial, { mode: 0o600 });
    const config = join(stateRoot(), 'settings', `${runtime.version}-${sha256(runtime.root).slice(0,12)}`);
    await mkdir(config, { recursive: true, mode: 0o700 });
    const child = spawn(runtime.binary, ['--serve-web', '--managed', '--port', '0', '--file', draft,
      '--allow-origin', allowOrigin, '--config-root', config], {
      stdio: ['pipe', 'pipe', 'pipe'], env: { ...process.env,
        OPENPENCIL_WEB_BUNDLE_DIR: runtime.webBundle, OPENPENCIL_CANVASKIT_DIR: runtime.canvasKit },
    });
    child.stderr.on('data', () => {}); // Never relay runtime credentials or private document diagnostics.
    try {
      const info = await handshake(child);
      if (String(info.version) !== runtime.version) throw new Error('Rust editor and bundle versions disagree.');
      const session = new EditorSession();
      Object.assign(session, { child, draft, dir, source: filePath || null, baseline: filePath ? sha256(initial) : null,
        port: info.port, token: info.token, version: runtime.version, closed: false });
      child.once('exit', () => { session.closed = true; });
      await session.document();
      return session;
    } catch (error) { child.kill(); throw error; }
  }

  assertLive() { if (this.closed) throw new Error('Canvas runtime ended. Reopen the recovery draft in a new canvas.'); }

  async document() {
    this.assertLive();
    const response = await fetch(`http://127.0.0.1:${this.port}/api/mcp/document`, { signal: AbortSignal.timeout(5_000) });
    if (!response.ok) throw new Error(`Canvas document read failed (${response.status}).`);
    const chunks = []; let size = 0;
    for await (const chunk of response.body) {
      size += chunk.length;
      if (size > 32 * 1024 * 1024) throw new Error('Canvas document exceeds 32 MiB.');
      chunks.push(chunk);
    }
    const value = JSON.parse(Buffer.concat(chunks).toString('utf8'));
    validateDocument(JSON.stringify(value.document));
    if (!Number.isSafeInteger(value.version)) throw new Error('Canvas document version is invalid.');
    return value;
  }

  async forward(message) { this.assertLive(); return post(this.port, message); }

  async selectionDocument(pageIndex) {
    this.assertLive();
    const response = await fetch(`http://127.0.0.1:${this.port}/api/mcp/version`, { signal: AbortSignal.timeout(5_000) });
    if (!response.ok) throw new Error('Canvas version read failed.');
    const value = await response.json();
    if (!Number.isSafeInteger(value.version)) throw new Error('Canvas version is invalid.');
    // Selection polling must not repeatedly transfer embedded document images.
    if (this.selectionSnapshot?.version !== value.version || this.selectionSnapshot.activePageIndex !== pageIndex) {
      this.selectionSnapshot = await this.document();
    }
    return this.selectionSnapshot;
  }

  async checkpoint() {
    const value = await this.document();
    const bytes = JSON.stringify(diskDocument(value)) + '\n';
    const temp = join(this.dir, 'checkpoint.tmp');
    await writeFile(temp, bytes, { mode: 0o600 });
    await rename(temp, this.draft);
    return { recoveryPath: this.draft, version: value.version };
  }

  async save({ filePath = this.source, docJson } = {}) {
    if (!filePath || !isAbsolute(filePath) || !filePath.toLowerCase().endsWith('.op')) throw new Error('Choose an absolute .op path to save this canvas.');
    const value = await this.document();
    // The editor flushes its sync gate before requesting Save. Refuse a stale UI snapshot.
    const snapshot = docJson === undefined ? diskDocument(value) : validateDocument(docJson);
    if (docJson !== undefined && !snapshotMatches(snapshot, value)) {
      throw new Error('The canvas changed while saving. Refresh and save again; no file was overwritten.');
    }
    const bytes = Buffer.from(JSON.stringify(snapshot, null, 2) + '\n');
    const hash = await atomicSave(filePath, bytes, filePath === this.source ? this.baseline : null);
    this.source = filePath; this.baseline = hash;
    await this.checkpoint();
    this.lastSave = { filePath, sha256: hash, version: value.version };
    return this.lastSave;
  }

  async close() {
    if (this.closed) return;
    try { await this.checkpoint(); } finally {
      this.closed = true;
      this.child.stdin.end(); // Release only this daemon's supervisor lease.
      this.child.kill();
    }
  }
}
