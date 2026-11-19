import { createHash } from 'node:crypto';
import { spawn, spawnSync } from 'node:child_process';
import {
  lstat,
  mkdtemp,
  open,
  readFile,
  readdir,
  rm,
  stat,
} from 'node:fs/promises';
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
import {
  validatedCollabBootstrapUrls,
  withoutCollabBootstrapBuildEnv,
} from './collab-bootstrap-config.mjs';
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

process.stdout.write(`verified ${selectedPlatforms.length} platform package metadata${requireRuntime ? ` and packaged runtime smoke${requireCollabBootstrap ? ' with production relay bootstrap' : ''}` : ' (metadata-only)'}\n`);

function assertRootMetadata() {
  if (rootManifest.name !== '@zseven-w/codex-openpencil') throw new Error('unexpected root package name');
  if (rootManifest.private === true) throw new Error('root package must not be private');
  if (rootManifest.publishConfig?.access !== 'public') throw new Error('root package must publish publicly');
  if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(rootManifest.version ?? '')) {
    throw new Error(`root package has an invalid release version: ${JSON.stringify(rootManifest.version)}`);
  }
  if (rootManifest.workspaces !== undefined) throw new Error('npm/* must not be declared through package.json workspaces');
  if (rootManifest.bundleDependencies !== undefined || rootManifest.bundledDependencies !== undefined) {
    throw new Error('root package must not bundle all platform dependencies');
  }
  if (rootManifest.dependencies !== undefined) {
    throw new Error('root package must remain runtime dependency-free outside optionalDependencies');
  }
  if (rootManifest.bin?.['codex-openpencil'] !== './bin/codex-openpencil.mjs') {
    throw new Error('root package must expose bin.codex-openpencil -> ./bin/codex-openpencil.mjs');
  }
  const expected = Object.fromEntries(platforms.map(platform => [platform.packageName, rootManifest.version]));
  assertRecord(rootManifest.optionalDependencies ?? {}, expected, 'root optionalDependencies');
  for (const requiredFile of ['.agents/plugins/marketplace.json', 'bin', 'plugins', 'platforms.json', 'THIRD_PARTY_NOTICES.md']) {
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
  if (manifest.repository?.directory !== `packages/op-codex/npm/${platform.id}`) {
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
    .filter(path => /^(?:vendor|npm|node_modules|target|tests|artifacts|\.git)(?:\/|$)/u.test(path));
  if (forbidden.length > 0) throw new Error(`root tarball leaked development/runtime files: ${forbidden.join(', ')}`);
  if (packed.bundled?.length > 0) throw new Error('root tarball unexpectedly bundled platform dependencies');
  for (const required of [
    'package.json',
    'platforms.json',
    'LICENSE',
    'NOTICE',
    'THIRD_PARTY_NOTICES.md',
    '.agents/plugins/marketplace.json',
    'bin/codex-openpencil.mjs',
    'plugins/openpencil/plugin.json',
    'plugins/openpencil/mcp.json',
    'plugins/openpencil/scripts/launch-mcp',
    'plugins/openpencil/scripts/launch-mcp.cmd',
    'plugins/openpencil/scripts/mcp-bridge.mjs',
    'plugins/openpencil/scripts/editor-runtime.mjs',
  ]) {
    if (!packed.files.some(file => file.path === required)) throw new Error(`root tarball is missing ${required}`);
  }
  if (process.platform !== 'win32') {
    for (const executable of ['bin/codex-openpencil.mjs', 'plugins/openpencil/scripts/launch-mcp']) {
      const entry = packed.files.find(file => file.path === executable);
      if (!entry || (entry.mode & 0o111) === 0) {
        throw new Error(`root tarball entry ${executable} must be executable`);
      }
    }
  }
}

async function verifyStagedRuntime(platform, bootstrap) {
  if (platform.os !== process.platform || platform.cpu !== process.arch) {
    throw new Error(`${platform.id}: runtime smoke must run on ${platform.os}/${platform.cpu}, current Node is ${process.platform}/${process.arch}`);
  }
  if (platform.libc === 'glibc' && process.report?.getReport?.().header?.glibcVersionRuntime === undefined) {
    throw new Error(`${platform.id}: runtime smoke requires a glibc Node runner`);
  }
  const packageRoot = platformPackageRoot(platform);
  const topLevel = (await readdir(packageRoot)).sort();
  assertList(
    topLevel,
    ['LICENSE', 'THIRD_PARTY_NOTICES.md', 'bin', runtimeManifestName, 'package.json'].sort(),
    `${platform.id}: package roots`,
  );
  const binaryPath = join(packageRoot, 'bin', platform.binaryName);
  const binary = await stat(binaryPath);
  if (!binary.isFile() || binary.size < 100_000) throw new Error(`${platform.id}: binary is missing or implausibly small`);
  if (platform.os !== 'win32' && (binary.mode & 0o777) !== 0o755) {
    throw new Error(`${platform.id}: binary mode must be exactly 0755`);
  }
  await verifyBinaryFormat(binaryPath, platform);
  if (bootstrap !== undefined) {
    const binaryBytes = await readFile(binaryPath);
    for (const [region, url] of Object.entries(bootstrap)) {
      if (!binaryBytes.includes(Buffer.from(url, 'utf8'))) {
        throw new Error(`${platform.id}: native daemon is missing its build-injected ${region} relay bootstrap`);
      }
    }
  }

  const webFiles = await walkFiles(join(packageRoot, 'bin', 'web-bundle'));
  const forbidden = webFiles.filter(path => path.endsWith('.d.ts') || path.endsWith('.opt.wasm'));
  if (forbidden.length > 0) throw new Error(`${platform.id}: redundant web artifacts were staged: ${forbidden.join(', ')}`);
  for (const path of runtimeKeyPaths(platform).slice(1)) {
    await assertRegularFile(join(packageRoot, ...path.split('/')), `${platform.id}: ${path}`);
  }

  const manifest = JSON.parse(await readFile(join(packageRoot, runtimeManifestName), 'utf8'));
  const expectedMetadata = {
    schemaVersion: 1,
    platform: platform.id,
    packageName: platform.packageName,
    openPencilVersion: openPencil.version,
    openPencilRevision: openPencil.revision,
  };
  for (const [key, value] of Object.entries(expectedMetadata)) {
    if (manifest[key] !== value) throw new Error(`${platform.id}: runtime manifest ${key} drifted`);
  }
  const payloadFiles = (await walkFiles(join(packageRoot, 'bin')))
    .map(path => relative(packageRoot, path).split(sep).join('/'));
  assertList(Object.keys(manifest.files ?? {}).sort(), payloadFiles.sort(), `${platform.id}: manifest files`);
  for (const path of runtimeKeyPaths(platform)) {
    if (manifest.files[path] === undefined) throw new Error(`${platform.id}: runtime manifest is missing ${path}`);
  }
  for (const [path, expectedHash] of Object.entries(manifest.files)) {
    if (!/^[0-9a-f]{64}$/u.test(expectedHash)) throw new Error(`${platform.id}: invalid SHA-256 for ${path}`);
    const actualHash = await sha256(join(packageRoot, ...path.split('/')));
    if (actualHash !== expectedHash) throw new Error(`${platform.id}: SHA-256 mismatch for ${path}`);
  }
}

async function verifyPlatformTarball(platform) {
  const packageRoot = platformPackageRoot(platform);
  const temporary = await mkdtemp(join(tmpdir(), `codex-openpencil-pack-${platform.id}-`));
  try {
    const packed = pack(`./npm/${platform.id}`, false, temporary);
    if (!/^sha512-/u.test(packed.integrity ?? '') || !/^[0-9a-f]{40}$/u.test(packed.shasum ?? '')) {
      throw new Error(`${platform.id}: npm pack did not report tarball integrity`);
    }
    const sourceFiles = await walkFiles(packageRoot);
    assertList(
      packed.files.map(file => file.path).sort(),
      sourceFiles.map(path => relative(packageRoot, path).split(sep).join('/')).sort(),
      `${platform.id}: packed runtime files`,
    );
    const binary = packed.files.find(file => file.path === `bin/${platform.binaryName}`);
    if (platform.os !== 'win32' && binary?.mode !== 0o755) {
      throw new Error(`${platform.id}: packed binary must retain mode 0755`);
    }
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
  const child = spawn(binary, [
    '--serve-web',
    '--managed',
    '--port',
    '0',
    '--allow-origin',
    'codex-openpencil://package-smoke',
  ], {
    cwd: packageRoot,
    env: withoutEditorDiscoveryEnv(withoutCollabBootstrapBuildEnv(process.env)),
    stdio: ['pipe', 'pipe', 'pipe'],
  });
  let stderr = '';
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', chunk => { stderr = (stderr + chunk).slice(-32_768); });
  let smokeFailure;
  try {
    const line = await firstLine(child, 10_000);
    const handshake = parseManagedHandshake(line, platform.id);
    if (handshake.version !== openPencil.version) {
      throw new Error(`${platform.id}: daemon reports an unexpected OpenPencil version`);
    }
    const base = `http://127.0.0.1:${handshake.port}`;
    for (const path of [
      '/',
      '/pkg/op_host_web.js',
      '/pkg/op_host_web_bg.wasm',
      '/pkg/assets/iconify-catalog-core.json',
      '/canvaskit/canvaskit.js',
      '/canvaskit/canvaskit.wasm',
    ]) {
      const response = await fetchWithTimeout(base + path);
      if (response.status !== 200) throw new Error(`${platform.id}: ${path} returned ${response.status}`);
      if ((await response.arrayBuffer()).byteLength === 0) throw new Error(`${platform.id}: ${path} returned an empty body`);
    }
    const health = await fetchWithTimeout(base + '/api/mcp/server');
    if (health.status !== 200) throw new Error(`${platform.id}: managed health returned ${health.status}`);
    const healthBody = await health.json();
    if (
      healthBody.running !== true
      || healthBody.server !== 'openpencil-mcp'
      || healthBody.mode !== 'web-canvas'
      || healthBody.serveMode !== 'managed'
      || healthBody.port !== handshake.port
      || healthBody.localIp !== '127.0.0.1'
    ) {
      throw new Error(`${platform.id}: managed health payload is invalid`);
    }
    const mcp = await fetchWithTimeout(base + '/mcp', {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}',
    });
    if (mcp.status !== 200) throw new Error(`${platform.id}: MCP initialize returned ${mcp.status}`);
    const mcpBody = await mcp.json();
    if (mcpBody?.result === undefined) throw new Error(`${platform.id}: MCP initialize did not return a result`);
  } catch (error) {
    smokeFailure = new Error(`${error.message}\n--- daemon stderr ---\n${stderr}`, { cause: error });
  }
  child.stdin.end();
  const exited = await waitForExit(child, 5_000);
  if (!exited) {
    const exitFailure = new Error(`${platform.id}: managed daemon did not exit within 5 seconds after stdin EOF`);
    child.kill('SIGKILL');
    await waitForExit(child, 2_000);
    smokeFailure = smokeFailure === undefined
      ? exitFailure
      : new AggregateError([smokeFailure, exitFailure], `${platform.id}: runtime smoke and managed shutdown both failed`);
  }
  if (smokeFailure !== undefined) throw smokeFailure;
}

function withoutEditorDiscoveryEnv(source) {
  const env = { ...source };
  for (const name of [
    'OPENPENCIL_WEB_BUNDLE_DIR',
    'OPENPENCIL_CANVASKIT_DIR',
    'OPENPENCIL_SOURCE_ROOT',
    'OPENPENCIL_EDITOR_RUNTIME_DIR',
  ]) delete env[name];
  return env;
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
    env: {
      ...withoutCollabBootstrapBuildEnv(process.env),
      npm_config_cache: join(tmpdir(), 'codex-openpencil-npm-cache'),
    },
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) throw new Error(`npm ${args.join(' ')} failed:\n${result.stderr || result.stdout}`);
  const reports = JSON.parse(result.stdout);
  if (!Array.isArray(reports) || reports.length !== 1) throw new Error(`npm pack returned an unexpected report for ${packagePath}`);
  return reports[0];
}

async function verifyBinaryFormat(path, platform) {
  const handle = await open(path, 'r');
  const header = Buffer.alloc(512);
  try {
    await handle.read(header, 0, header.length, 0);
  } finally {
    await handle.close();
  }
  if (platform.os === 'darwin') {
    if (!header.subarray(0, 4).equals(Buffer.from([0xcf, 0xfa, 0xed, 0xfe]))) {
      throw new Error(`${platform.id}: binary is not a little-endian 64-bit Mach-O`);
    }
    const cpu = header.readUInt32LE(4);
    const expected = platform.cpu === 'arm64' ? 0x0100000c : 0x01000007;
    if (cpu !== expected) throw new Error(`${platform.id}: Mach-O CPU type ${cpu.toString(16)} is incorrect`);
  } else if (platform.os === 'linux') {
    if (!header.subarray(0, 4).equals(Buffer.from([0x7f, 0x45, 0x4c, 0x46])) || header[4] !== 2 || header[5] !== 1) {
      throw new Error(`${platform.id}: binary is not a little-endian ELF64`);
    }
    const machine = header.readUInt16LE(18);
    const expected = platform.cpu === 'arm64' ? 183 : 62;
    if (machine !== expected) throw new Error(`${platform.id}: ELF machine ${machine} is incorrect`);
  } else {
    if (header.toString('ascii', 0, 2) !== 'MZ') throw new Error(`${platform.id}: binary is not PE/COFF`);
    const peOffset = header.readUInt32LE(0x3c);
    if (peOffset + 6 > header.length) throw new Error(`${platform.id}: PE header is outside validation window`);
    if (header.toString('ascii', peOffset, peOffset + 4) !== 'PE\0\0') throw new Error(`${platform.id}: invalid PE signature`);
    const machine = header.readUInt16LE(peOffset + 4);
    const expected = platform.cpu === 'arm64' ? 0xaa64 : 0x8664;
    if (machine !== expected) throw new Error(`${platform.id}: PE machine ${machine.toString(16)} is incorrect`);
  }
}

async function firstLine(child, timeoutMs) {
  return await new Promise((resolvePromise, rejectPromise) => {
    let buffer = '';
    const timer = setTimeout(() => rejectPromise(new Error('managed daemon handshake timed out')), timeoutMs);
    const finish = (callback, value) => {
      clearTimeout(timer);
      child.stdout.off('data', onData);
      child.off('error', onError);
      child.off('exit', onExit);
      callback(value);
    };
    const onData = chunk => {
      buffer += chunk.toString();
      const newline = buffer.indexOf('\n');
      if (newline !== -1) finish(resolvePromise, buffer.slice(0, newline).trim());
    };
    const onError = error => finish(rejectPromise, error);
    const onExit = code => finish(rejectPromise, new Error(`managed daemon exited ${code} before handshake`));
    child.stdout.on('data', onData);
    child.once('error', onError);
    child.once('exit', onExit);
  });
}

async function fetchWithTimeout(url, init = {}) {
  return await fetch(url, { ...init, signal: AbortSignal.timeout(10_000) });
}

async function waitForExit(child, timeoutMs) {
  if (child.exitCode !== null || child.signalCode !== null) return true;
  return await new Promise(resolvePromise => {
    const timer = setTimeout(() => {
      child.off('exit', onExit);
      resolvePromise(false);
    }, timeoutMs);
    const onExit = () => {
      clearTimeout(timer);
      resolvePromise(true);
    };
    child.once('exit', onExit);
  });
}

async function walkFiles(root) {
  const output = [];
  const entries = await readdir(root, { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
  for (const entry of entries) {
    const path = join(root, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`package payload must not contain symbolic links: ${path}`);
    if (entry.isDirectory()) output.push(...await walkFiles(path));
    else if (entry.isFile()) output.push(path);
    else throw new Error(`unsupported package payload entry: ${path}`);
  }
  return output;
}

async function assertRegularFile(path, label) {
  const info = await lstat(path);
  if (!info.isFile() || info.isSymbolicLink()) throw new Error(`${label} must be a regular file`);
}

function assertList(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

function assertRecord(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(`${label}: expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`);
  }
}

async function sha256(path) {
  return createHash('sha256').update(await readFile(path)).digest('hex');
}
