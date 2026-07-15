import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const packagesRoot = resolve(scriptDirectory, '..');
const repositoryRoot = resolve(packagesRoot, '..');

const manifestPaths = [
  'package.json',
  'op-web-sdk/package.json',
  'op-web-sdk-react/package.json',
  'op-web-sdk-vue/package.json',
];

const sdkEntryPaths = [
  'op-web-sdk/src/index.ts',
  'op-web-sdk-react/src/index.ts',
  'op-web-sdk-vue/src/index.ts',
];

const sdkWorkspaceNames = ['op-web-sdk', 'op-web-sdk-react', 'op-web-sdk-vue'];

const versionExportPattern = /^([\t ]*)export const VERSION = '([^'\r\n]*)';([\t ]*)$/gm;

export function renderPackageManifest(source, version) {
  const manifest = JSON.parse(source);
  manifest.version = version;
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

function sdkVersionMatches(source) {
  return [...source.matchAll(versionExportPattern)];
}

export function renderSdkEntry(source, version) {
  const matches = sdkVersionMatches(source);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one public VERSION export, found ${matches.length}`);
  }

  return source.replace(versionExportPattern, (_declaration, prefix, _current, suffix) => {
    return `${prefix}export const VERSION = '${version}';${suffix}`;
  });
}

function skipTrivia(source, start) {
  let index = start;
  while (index < source.length) {
    if (/\s/.test(source[index])) {
      index += 1;
      continue;
    }
    if (source.startsWith('//', index)) {
      const lineEnd = source.indexOf('\n', index + 2);
      return lineEnd === -1 ? source.length : skipTrivia(source, lineEnd + 1);
    }
    if (source.startsWith('/*', index)) {
      const commentEnd = source.indexOf('*/', index + 2);
      if (commentEnd === -1) {
        throw new Error('Unterminated block comment in bun.lock');
      }
      index = commentEnd + 2;
      continue;
    }
    break;
  }
  return index;
}

function readJsoncString(source, start) {
  if (source[start] !== '"') {
    throw new Error(`Expected a quoted string at offset ${start}`);
  }

  let value = '';
  let index = start + 1;
  while (index < source.length) {
    const character = source[index];
    if (character === '"') {
      return { value, end: index + 1 };
    }
    if (character !== '\\') {
      value += character;
      index += 1;
      continue;
    }

    const escaped = source[index + 1];
    const simpleEscapes = {
      '"': '"',
      '\\': '\\',
      '/': '/',
      b: '\b',
      f: '\f',
      n: '\n',
      r: '\r',
      t: '\t',
    };
    if (escaped === 'u') {
      const hexadecimal = source.slice(index + 2, index + 6);
      if (!/^[0-9a-f]{4}$/i.test(hexadecimal)) {
        throw new Error(`Invalid Unicode escape at offset ${index}`);
      }
      value += String.fromCharCode(Number.parseInt(hexadecimal, 16));
      index += 6;
      continue;
    }
    if (!(escaped in simpleEscapes)) {
      throw new Error(`Invalid string escape at offset ${index}`);
    }
    value += simpleEscapes[escaped];
    index += 2;
  }

  throw new Error('Unterminated string in bun.lock');
}

function skipJsoncValue(source, start) {
  const valueStart = skipTrivia(source, start);
  if (source[valueStart] === '"') {
    return readJsoncString(source, valueStart).end;
  }

  const opening = source[valueStart];
  const closing = opening === '{' ? '}' : opening === '[' ? ']' : undefined;
  if (closing !== undefined) {
    let depth = 1;
    let index = valueStart + 1;
    while (index < source.length && depth > 0) {
      index = skipTrivia(source, index);
      if (source[index] === '"') {
        index = readJsoncString(source, index).end;
        continue;
      }
      if (source[index] === opening) {
        depth += 1;
      } else if (source[index] === closing) {
        depth -= 1;
      }
      index += 1;
    }
    if (depth !== 0) {
      throw new Error(`Unterminated ${opening} value in bun.lock`);
    }
    return index;
  }

  let index = valueStart;
  while (index < source.length) {
    const afterTrivia = skipTrivia(source, index);
    if (afterTrivia !== index) {
      index = afterTrivia;
      continue;
    }
    if (',}]'.includes(source[index])) {
      break;
    }
    index += 1;
  }
  return index;
}

function readJsoncObjectEntries(source, start) {
  const objectStart = skipTrivia(source, start);
  if (source[objectStart] !== '{') {
    throw new Error(`Expected an object at offset ${objectStart}`);
  }

  const entries = [];
  let index = objectStart + 1;
  while (index < source.length) {
    index = skipTrivia(source, index);
    if (source[index] === '}') {
      return entries;
    }

    const key = readJsoncString(source, index);
    index = skipTrivia(source, key.end);
    if (source[index] !== ':') {
      throw new Error(`Expected a colon at offset ${index}`);
    }

    const valueStart = skipTrivia(source, index + 1);
    const valueEnd = skipJsoncValue(source, valueStart);
    entries.push({ key: key.value, valueStart, valueEnd });
    index = skipTrivia(source, valueEnd);
    if (source[index] === ',') {
      index += 1;
    } else if (source[index] !== '}') {
      throw new Error(`Expected a comma or closing brace at offset ${index}`);
    }
  }

  throw new Error('Unterminated object in bun.lock');
}

export function inspectBunLockWorkspaceVersions(source) {
  const rootEntries = readJsoncObjectEntries(source, 0);
  const workspaces = rootEntries.find((entry) => entry.key === 'workspaces');
  if (workspaces === undefined) {
    throw new Error('bun.lock does not contain a top-level workspaces object');
  }

  const versions = {};
  for (const workspace of readJsoncObjectEntries(source, workspaces.valueStart)) {
    if (source[workspace.valueStart] !== '{') {
      continue;
    }
    const version = readJsoncObjectEntries(source, workspace.valueStart).find(
      (entry) => entry.key === 'version',
    );
    if (version === undefined || source[version.valueStart] !== '"') {
      continue;
    }
    versions[workspace.key] = readJsoncString(source, version.valueStart).value;
  }
  return versions;
}

export function collectVersionDrift(expectedVersion, consumers) {
  return consumers
    .filter(({ actualVersion }) => actualVersion !== expectedVersion)
    .map(({ path, actualVersion }) => ({
      path,
      expectedVersion,
      actualVersion,
    }));
}

function repositoryPath(packageRelativePath) {
  return `packages/${packageRelativePath}`;
}

async function readConsumers() {
  const manifestConsumers = await Promise.all(
    manifestPaths.map(async (path) => {
      const source = await readFile(resolve(packagesRoot, path), 'utf8');
      return {
        path: repositoryPath(path),
        actualVersion: JSON.parse(source).version,
      };
    }),
  );

  const sdkConsumers = await Promise.all(
    sdkEntryPaths.map(async (path) => {
      const source = await readFile(resolve(packagesRoot, path), 'utf8');
      const matches = sdkVersionMatches(source);
      return {
        path: repositoryPath(path),
        actualVersion: matches.length === 1 ? matches[0][2] : undefined,
      };
    }),
  );

  const lockfile = await readFile(resolve(packagesRoot, 'bun.lock'), 'utf8');
  const lockVersions = inspectBunLockWorkspaceVersions(lockfile);
  const lockConsumers = sdkWorkspaceNames.map((workspace) => ({
    path: `packages/bun.lock#workspaces.${workspace}`,
    actualVersion: lockVersions[workspace],
  }));

  return [...manifestConsumers, ...sdkConsumers, ...lockConsumers];
}

async function renderVersionedFiles(version) {
  const manifests = await Promise.all(
    manifestPaths.map(async (path) => {
      const absolutePath = resolve(packagesRoot, path);
      const source = await readFile(absolutePath, 'utf8');
      return {
        absolutePath,
        source,
        output: renderPackageManifest(source, version),
      };
    }),
  );

  const sdkEntries = await Promise.all(
    sdkEntryPaths.map(async (path) => {
      const absolutePath = resolve(packagesRoot, path);
      const source = await readFile(absolutePath, 'utf8');
      try {
        return {
          absolutePath,
          source,
          output: renderSdkEntry(source, version),
        };
      } catch (error) {
        throw new Error(`${repositoryPath(path)}: ${error.message}`, {
          cause: error,
        });
      }
    }),
  );

  return [...manifests, ...sdkEntries];
}

function canonicalVersion() {
  const output = execFileSync('sh', ['scripts/workspace-version.sh', 'Cargo.toml'], {
    cwd: repositoryRoot,
    encoding: 'utf8',
    stdio: ['ignore', 'pipe', 'inherit'],
  });
  const version = output.trim();
  if (version.length === 0 || /\s/.test(version)) {
    throw new Error('scripts/workspace-version.sh returned an invalid version');
  }
  return version;
}

function reportDrift(drift) {
  for (const { path, expectedVersion, actualVersion } of drift) {
    const actual = actualVersion ?? 'missing or invalid';
    console.error(
      `${path}: expected version ${expectedVersion}, actual ${actual}; run "bun run sync-version"`,
    );
  }
}

export async function main(arguments_) {
  const check = arguments_.length === 1 && arguments_[0] === '--check';
  if (arguments_.length > 0 && !check) {
    throw new Error('Usage: node scripts/sync-version.mjs [--check]');
  }

  const version = canonicalVersion();
  if (!check) {
    const rendered = await renderVersionedFiles(version);
    await Promise.all(
      rendered
        .filter(({ source, output }) => source !== output)
        .map(({ absolutePath, output }) => writeFile(absolutePath, output)),
    );
    execFileSync('bun', ['install', '--lockfile-only'], {
      cwd: packagesRoot,
      stdio: 'inherit',
    });
  }

  const drift = collectVersionDrift(version, await readConsumers());
  if (drift.length > 0) {
    reportDrift(drift);
    process.exitCode = 1;
    return;
  }

  const action = check ? 'Verified' : 'Synchronized';
  console.log(`${action} package and SDK versions at ${version}.`);
}

const isMainModule =
  process.argv[1] !== undefined && pathToFileURL(resolve(process.argv[1])).href === import.meta.url;

if (isMainModule) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
