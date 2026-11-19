import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import { mkdtemp, open, readFile, readdir, rm, stat } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, relative, sep } from 'node:path';
import {
  argumentValue,
  hasFlag,
  openPencil,
  platformById,
  platformPackageRoot,
  platforms,
  projectRoot,
  runtimeKeyPaths,
  runtimeManifestName,
} from './platforms.mjs';
import { validatedCollabBootstrapUrls, withoutCollabBootstrapBuildEnv } from './collab-bootstrap-config.mjs';
import { parseManagedHandshake } from './managed-handshake.mjs';

const selectedId = argumentValue('--platform');
const requireRuntime = hasFlag('--require-runtime');
const requireCollabBootstrap = hasFlag('--require-collab-bootstrap');
if (requireRuntime && selectedId === undefined) {
  throw new Error('--require-runtime requires --platform so the staged native daemon can run on its matching runner');
}
if (requireCollabBootstrap && !requireRuntime) {
  throw new Error('--require-collab-bootstrap requires --require-runtime');
}
const collabBootstrap = requireCollabBootstrap ? validatedCollabBootstrapUrls() : undefined;
const selectedPlatforms = selectedId === undefined ? platforms : [platformById(selectedId)];
const rootManifest = JSON.parse(await readFile(join(projectRoot, 'package.json'), 'utf8'));
const rootLicense = await readFile(join(projectRoot, 'LICENSE'));
const rootNotices = await readFile(join(projectRoot, 'THIRD_PARTY_NOTICES.md'));

assertRootMetadata();
for (const platform of selectedPlatforms) await verifyPlatformMetadata(platform);
await verifyRootTarball();

if (requireRuntime) {
  const platform = selectedPlatforms[0];
  await verifyStagedRuntime(platform, collabBootstrap);
  await verifyPlatformTarball(platform);
  await smokeRuntime(platform);
}

process.stdout.write(`verified ${selectedPlatforms.length} QwenWork platform package metadata${requireRuntime ? ' and packaged runtime smoke' : ' (metadata-only)'}\n`);

function assertRootMetadata() {
  if (rootManifest.name !== '@zseven-w/qwenwork-openpencil') throw new Error('unexpected root package name');
  if (rootManifest.private === true) throw new Error('root package must not be private');
  if (rootManifest.publishConfig?.access !== 'public') throw new Error('root package must publish publicly');
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(rootManifest.version ?? '')) {
    throw new Error(`root package has an invalid release version: ${JSON.stringify(rootManifest.version)}`);
  }
  if (rootManifest.dependencies !== undefined) {
    throw new Error('root package must remain runtime dependency-free outside optionalDependencies');
  }
  if (rootManifest.bin?.['qwenwork-openpencil'] !== './bin/qwenwork-openpencil.mjs') {
    throw new Error('root package must expose bin.qwenwork-openpencil -> ./bin/qwenwork-openpencil.mjs');
  }
  const expected = Object.fromEntries(platforms.map(platform => [platform.packageName, rootManifest.version]));
  assertRecord(rootManifest.optionalDependencies ?? {}, expected, 'root optionalDependencies');
  for (const requiredFile of ['bin', 'plugins', 'scripts', 'platforms.json', 'THIRD_PARTY_NOTICES.md']) {
    if (!rootManifest.files?.includes(requiredFile)) throw new Error(`root files must publish ${requiredFile}`);
  }
}

async function verifyPlatformMetadata(platform) {
  const packageRoot = platformPackageRoot(platform);
  const manifest = JSON.parse(await readFile(join(packageRoot, 'package.json'), 'utf8'));
  if (manifest.name !== platform.packageName) throw new Error(`${platform.id}: unexpected package name`);
  if (manifest.version !== rootManifest.version) throw new Error(`${platform.id}: version must match root package`);
  assertList(manifest.os, [platform.os], `${platform.id}: os selector`);
  assertList(manifest.cpu, [platform.cpu], `${platform.id}: cpu selector`);
  if (platform.libc === undefined) {
    if (manifest.libc !== undefined) throw new Error(`${platform.id}: non-Linux package must not declare libc`);
  } else {
    assertList(manifest.libc, [platform.libc], `${platform.id}: libc selector`);
  }
  if (manifest.bin !== undefined) throw new Error(`${platform.id}: runtime packages must not expose a global bin`);
  if (manifest.preferUnplugged !== true) throw new Error(`${platform.id}: preferUnplugged must be true`);
  if (manifest.publishConfig?.access !== 'public') throw new Error(`${platform.id}: scoped package must publish publicly`);
  if (manifest.engines?.node !== rootManifest.engines?.node) throw new Error(`${platform.id}: Node engine must match root package`);
  assertList(
    [...(manifest.files ?? [])].sort(),
    ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'bin', runtimeManifestName].sort(),
    `${platform.id}: files`,
  );
  if (manifest.dependencies !== undefined || manifest.optionalDependencies !== undefined) {
    throw new Error(`${platform.id}: atomic runtime package must not have package dependencies`);
  }
  if (manifest.repository?.directory !== `packages/op-qwenwork/npm/${platform.id}`) {
    throw new Error(`${platform.id}: repository directory drifted`);
  }
  if (!rootLicense.equals(await readFile(join(packageRoot, 'LICENSE')))) throw new Error(`${platform.id}: LICENSE differs from root`);
  if (!rootNotices.equals(await readFile(join(packageRoot, 'THIRD_PARTY_NOTICES.md')))) {
    throw new Error(`${platform.id}: THIRD_PARTY_NOTICES.md differs from root`);
  }
}

async function verifyRootTarball() {
  const packed = pack('.', true);
  const forbidden = packed.files
    .map(file => file.path)
    .filter(path => /^(?:vendor|npm|node_modules|target|tests|artifacts|dist|\.git)(?:\/|$)/u.test(path));
  if (forbidden.length > 0) throw new Error(`root tarball leaked development/runtime files: ${forbidden.join(', ')}`);
  for (const required of [
    'package.json',
    'platforms.json',
    'LICENSE',
    'NOTICE',
    'THIRD_PARTY_NOTICES.md',
    'bin/qwenwork-openpencil.mjs',
    'scripts/pack-plugin.mjs',
    'scripts/host-config.mjs',
    'plugins/openpencil/.qwenwork-plugin/plugin.json',
    'plugins/openpencil/.qoder-plugin/plugin.json',
    'plugins/openpencil/.claude-plugin/plugin.json',
    'plugins/openpencil/.mcp.json',
    'plugins/openpencil/CONNECTORS.md',
    'plugins/openpencil/scripts/launch-mcp',
    'plugins/openpencil/scripts/launch-mcp.cmd',
    'plugins/openpencil/scripts/mcp-bridge.mjs',
  ]) {
    if (!packed.files.some(file => file.path === required)) throw new Error(`root tarball is missing ${required}`);
  }
  if (process.platform !== 'win32') {
    for (const executable of ['bin/qwenwork-openpencil.mjs', 'plugins/openpencil/scripts/launch-mcp']) {
      const entry = packed.files.find(file => file.path === executable);
      if (!entry || (entry.mode & 0o111) === 0) throw new Error(`root tarball entry ${executable} must be executable`);
    }
  }
}

async function verifyStagedRuntime(platform, bootstrap) {
  if (platform.os !== process.platform || platform.cpu !== process.arch) {
    throw new Error(`${platform.id}: runtime smoke must run on ${platform.os}/${platform.cpu}`);
  }
  const packageRoot = platformPackageRoot(platform);
  const binaryPath = join(packageRoot, 'bin', platform.binaryName);
  const binary = await stat(binaryPath);
  if (!binary.isFile() || binary.size < 100_000) throw new Error(`${platform.id}: binary is missing or implausibly small`);
  if (platform.os !== 'win32' && (binary.mode & 0o777) !== 0o755) throw new Error(`${platform.id}: binary mode must be 0755`);
  await verifyBinaryFormat(binaryPath, platform);
  if (bootstrap !== undefined) {
    const binaryBytes = await readFile(binaryPath);
    for (const [region, url] of Object.entries(bootstrap)) {
      if (!binaryBytes.includes(Buffer.from(url, 'utf8'))) {
        throw new Error(`${platform.id}: native daemon is missing its build-injected ${region} relay bootstrap`);
      }
    }
  }
  const manifest = JSON.parse(await readFile(join(packageRoot, runtimeManifestName), 'utf8'));
  const payloadFiles = (await walkFiles(join(packageRoot, 'bin'))).map(path => relative(packageRoot, path).split(sep).join('/'));
  assertList(Object.keys(manifest.files ?? {}).sort(), payloadFiles.sort(), `${platform.id}: manifest files`);
  for (const [path, expectedHash] of Object.entries(manifest.files)) {
    const actualHash = await sha256(join(packageRoot, ...path.split('/')));
    if (actualHash !== expectedHash) throw new Error(`${platform.id}: SHA-256 mismatch for ${path}`);
  }
}

async function verifyPlatformTarball(platform) {
  const temporary = await mkdtemp(join(tmpdir(), `qwenwork-openpencil-pack-${platform.id}-`));
  try {
    const packed = pack(`./npm/${platform.id}`, false, temporary);
    for (const required of [runtimeManifestName, ...runtimeKeyPaths(platform), 'LICENSE', 'THIRD_PARTY_NOTICES.md']) {
      if (!packed.files.some(file => file.path === required)) throw new Error(`${platform.id}: packed tarball is missing ${required}`);
    }
    const tarball = await stat(join(temporary, packed.filename));
    if (!tarball.isFile() || tarball.size < 100_000) throw new Error(`${platform.id}: npm did not create a plausible tgz`);
  } finally {
    await rm(temporary, { recursive: true, force: true });
  }
}

async function smokeRuntime(platform) {
  const packageRoot = platformPackageRoot(platform);
  const binary = join(packageRoot, 'bin', platform.binaryName);
  const child = spawn(binary, ['--serve-web', '--managed', '--port', '0', '--allow-origin', 'qwenwork-openpencil://package-smoke'], {
    cwd: packageRoot,
    env: withoutCollabBootstrapBuildEnv(process.env),
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  try {
    const line = await firstLine(child, 10_000);
    const handshake = parseManagedHandshake(line, platform.id);
    if (handshake.version !== openPencil.version) throw new Error(`${platform.id}: unexpected OpenPencil version`);
    const health = await fetch(`http://127.0.0.1:${handshake.port}/api/mcp/server`, { signal: AbortSignal.timeout(10_000) });
    if (!health.ok) throw new Error(`${platform.id}: managed health returned ${health.status}`);
  } finally {
    child.stdin.end();
    child.kill();
  }
}

function pack(packagePath, dryRun, destination) {
  const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  const args = ['pack', '--json', '--ignore-scripts'];
  if (dryRun) args.push('--dry-run');
  if (destination !== undefined) args.push('--pack-destination', destination);
  args.push(packagePath);
  const result = spawnSync(npm, args, {
    cwd: projectRoot,
    encoding: 'utf8',
    env: { ...withoutCollabBootstrapBuildEnv(process.env), npm_config_cache: join(tmpdir(), 'qwenwork-openpencil-npm-cache') },
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) throw new Error(`npm ${args.join(' ')} failed:\n${result.stderr || result.stdout}`);
  return JSON.parse(result.stdout)[0];
}

async function verifyBinaryFormat(path, platform) {
  const handle = await open(path, 'r');
  const header = Buffer.alloc(512);
  try { await handle.read(header, 0, header.length, 0); } finally { await handle.close(); }
  if (platform.os === 'darwin') {
    if (!header.subarray(0, 4).equals(Buffer.from([0xcf, 0xfa, 0xed, 0xfe]))) throw new Error(`${platform.id}: expected 64-bit Mach-O`);
  } else if (platform.os === 'linux') {
    if (!header.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46]))) throw new Error(`${platform.id}: expected ELF64`);
  } else if (header.toString('ascii', 0, 2) !== 'MZ') {
    throw new Error(`${platform.id}: expected PE/COFF`);
  }
}

async function firstLine(child, timeoutMs) {
  return await new Promise((resolvePromise, rejectPromise) => {
    let buffer = '';
    const timer = setTimeout(() => rejectPromise(new Error('managed daemon handshake timed out')), timeoutMs);
    const finish = (cb, val) => {
      clearTimeout(timer);
      child.stdout.off('data', onData);
      child.off('error', onError);
      child.off('exit', onExit);
      cb(val);
    };
    const onData = chunk => {
      buffer += chunk.toString();
      const nl = buffer.indexOf('\n');
      if (nl !== -1) finish(resolvePromise, buffer.slice(0, nl).trim());
    };
    const onError = error => finish(rejectPromise, error);
    const onExit = code => finish(rejectPromise, new Error(`managed daemon exited ${code} before handshake`));
    child.stdout.on('data', onData);
    child.once('error', onError);
    child.once('exit', onExit);
  });
}

async function walkFiles(root) {
  const output = [];
  const entries = await readdir(root, { withFileTypes: true });
  entries.sort((a, b) => a.name.localeCompare(b.name, 'en'));
  for (const entry of entries) {
    const path = join(root, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`package payload must not contain symbolic links: ${path}`);
    if (entry.isDirectory()) output.push(...await walkFiles(path));
    else if (entry.isFile()) output.push(path);
  }
  return output;
}

function assertList(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

function assertRecord(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}
