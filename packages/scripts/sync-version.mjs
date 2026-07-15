import { execFileSync } from 'node:child_process';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const defaultPackagesRoot = resolve(scriptDirectory, '..');
const defaultRepositoryRoot = resolve(defaultPackagesRoot, '..');

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
const managedPaths = [...manifestPaths, ...sdkEntryPaths, 'bun.lock'];

const versionExportPattern = /^([\t ]*)export const VERSION = '([^'\r\n]*)';([\t ]*)$/gm;

export function renderPackageManifest(source, version) {
  const manifest = JSON.parse(source);
  manifest.version = version;
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

function maskSdkCommentsAndTemplates(source) {
  const masked = source.split('');
  const regexPrefixKeywords = new Set([
    'await',
    'case',
    'delete',
    'else',
    'in',
    'instanceof',
    'new',
    'of',
    'return',
    'throw',
    'typeof',
    'void',
    'yield',
  ]);

  const mask = (index) => {
    if (source[index] !== '\n' && source[index] !== '\r') {
      masked[index] = ' ';
    }
  };

  function isEscaped(index) {
    let backslashes = 0;
    for (let cursor = index - 1; cursor >= 0 && source[cursor] === '\\'; cursor -= 1) {
      backslashes += 1;
    }
    return backslashes % 2 === 1;
  }

  function skipQuoted(start, quote) {
    let index = start + 1;
    while (index < source.length) {
      if (source[index] === '\\') {
        index += 2;
      } else if (source[index] === quote) {
        return index + 1;
      } else {
        index += 1;
      }
    }
    return index;
  }

  function maskQuoted(start, quote) {
    let index = start;
    while (index < source.length) {
      const character = source[index];
      mask(index);
      if (index > start && character === quote) {
        return index + 1;
      }
      if (character === '\\') {
        mask(index + 1);
        index += 2;
      } else {
        index += 1;
      }
    }
    return index;
  }

  function maskLineComment(start) {
    let index = start;
    while (index < source.length && source[index] !== '\n' && source[index] !== '\r') {
      mask(index);
      index += 1;
    }
    return index;
  }

  function maskBlockComment(start) {
    let index = start;
    while (index < source.length) {
      mask(index);
      if (source[index] === '*' && source[index + 1] === '/') {
        mask(index + 1);
        return index + 2;
      }
      index += 1;
    }
    return index;
  }

  function scanRegexLiteral(start, maskContents) {
    let inCharacterClass = false;
    let index = start + 1;
    if (maskContents) {
      mask(start);
    }
    while (index < source.length) {
      const character = source[index];
      if (character === '\n' || character === '\r') {
        return index;
      }
      if (maskContents) {
        mask(index);
      }
      if (character === '\\') {
        if (maskContents) {
          mask(index + 1);
        }
        index += 2;
      } else if (character === '[') {
        inCharacterClass = true;
        index += 1;
      } else if (character === ']') {
        inCharacterClass = false;
        index += 1;
      } else if (character === '/' && !inCharacterClass) {
        index += 1;
        while (/[a-z]/i.test(source[index] ?? '')) {
          if (maskContents) {
            mask(index);
          }
          index += 1;
        }
        return index;
      } else {
        index += 1;
      }
    }
    return index;
  }

  function scanIdentifier(start, maskContents) {
    let index = start;
    while (/[\w$]/.test(source[index] ?? '')) {
      if (maskContents) {
        mask(index);
      }
      index += 1;
    }
    return {
      end: index,
      canStartRegex: regexPrefixKeywords.has(source.slice(start, index)),
    };
  }

  function maskTemplateExpression(start) {
    let depth = 1;
    let index = start;
    let canStartRegex = true;
    while (index < source.length) {
      const character = source[index];
      const next = source[index + 1];
      if (character === "'" || character === '"') {
        index = maskQuoted(index, character);
        canStartRegex = false;
        continue;
      }
      if (character === '`') {
        index = maskTemplateLiteral(index);
        canStartRegex = false;
        continue;
      }
      if (character === '/' && next === '/' && !isEscaped(index)) {
        index = maskLineComment(index);
        continue;
      }
      if (character === '/' && next === '*' && !isEscaped(index)) {
        index = maskBlockComment(index);
        continue;
      }
      if (character === '/' && canStartRegex) {
        index = scanRegexLiteral(index, true);
        canStartRegex = false;
        continue;
      }
      if (/[A-Za-z_$]/.test(character)) {
        const identifier = scanIdentifier(index, true);
        index = identifier.end;
        canStartRegex = identifier.canStartRegex;
        continue;
      }

      mask(index);
      if (character === '{') {
        depth += 1;
        canStartRegex = true;
      } else if (character === '}') {
        depth -= 1;
        index += 1;
        if (depth === 0) {
          return index;
        }
        canStartRegex = false;
        continue;
      } else if (/\d/.test(character) || '.)]'.includes(character)) {
        canStartRegex = false;
      } else if (!/\s/.test(character)) {
        canStartRegex = true;
      }
      index += 1;
    }
    return index;
  }

  function maskTemplateLiteral(start) {
    mask(start);
    let index = start + 1;
    while (index < source.length) {
      const character = source[index];
      const next = source[index + 1];
      if (character === '\\') {
        mask(index);
        mask(index + 1);
        index += 2;
      } else if (character === '`') {
        mask(index);
        return index + 1;
      } else if (character === '$' && next === '{') {
        mask(index);
        mask(index + 1);
        index = maskTemplateExpression(index + 2);
      } else {
        mask(index);
        index += 1;
      }
    }
    return index;
  }

  let canStartRegex = true;
  for (let index = 0; index < source.length; ) {
    const character = source[index];
    const next = source[index + 1];
    if (/\s/.test(character)) {
      index += 1;
      continue;
    }
    if (character === "'" || character === '"') {
      index = skipQuoted(index, character);
      canStartRegex = false;
      continue;
    }
    if (character === '`') {
      index = maskTemplateLiteral(index);
      canStartRegex = false;
      continue;
    }
    if (character === '/' && next === '/' && !isEscaped(index)) {
      index = maskLineComment(index);
      continue;
    }
    if (character === '/' && next === '*' && !isEscaped(index)) {
      index = maskBlockComment(index);
      continue;
    }
    if (character === '/' && canStartRegex) {
      index = scanRegexLiteral(index, false);
      canStartRegex = false;
      continue;
    }
    if (/[A-Za-z_$]/.test(character)) {
      const identifier = scanIdentifier(index, false);
      index = identifier.end;
      canStartRegex = identifier.canStartRegex;
      continue;
    }
    canStartRegex = !(/\d/.test(character) || '.)]}'.includes(character));
    index += 1;
  }

  return masked.join('');
}

function sdkVersionMatches(source) {
  return [...maskSdkCommentsAndTemplates(source).matchAll(versionExportPattern)];
}

export function renderSdkEntry(source, version) {
  const matches = sdkVersionMatches(source);
  if (matches.length !== 1) {
    throw new Error(`Expected exactly one public VERSION export, found ${matches.length}`);
  }

  const [declaration, prefix, _current, suffix] = matches[0];
  const start = matches[0].index;
  return `${source.slice(0, start)}${prefix}export const VERSION = '${version}';${suffix}${source.slice(start + declaration.length)}`;
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

async function readConsumers(packagesRoot) {
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

async function renderVersionedFiles(version, packagesRoot) {
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

function defaultRunCommand(command, arguments_, { cwd, captureOutput = false }) {
  return execFileSync(command, arguments_, {
    cwd,
    encoding: 'utf8',
    stdio: captureOutput ? ['ignore', 'pipe', 'inherit'] : 'inherit',
  });
}

async function canonicalVersion(repositoryRoot, runCommand) {
  const output = await runCommand('sh', ['scripts/workspace-version.sh', 'Cargo.toml'], {
    cwd: repositoryRoot,
    captureOutput: true,
  });
  const version = String(output).trim();
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

function errorMessage(error) {
  return error instanceof Error ? error.message : String(error);
}

async function snapshotManagedFiles(packagesRoot) {
  return Promise.all(
    managedPaths.map(async (path) => ({
      absolutePath: resolve(packagesRoot, path),
      contents: await readFile(resolve(packagesRoot, path)),
    })),
  );
}

async function writeManagedFiles(files, writeManagedFile) {
  const results = await Promise.allSettled(
    files.map(({ absolutePath, contents }) => writeManagedFile(absolutePath, contents)),
  );
  const failure = results.find((result) => result.status === 'rejected');
  if (failure !== undefined) {
    throw failure.reason;
  }
}

async function restoreManagedFiles(snapshot, writeManagedFile) {
  await writeManagedFiles(snapshot, writeManagedFile);
}

function postWriteValidationError(drift) {
  const consumers = drift
    .map(({ path, expectedVersion, actualVersion }) => {
      return `${path} (expected ${expectedVersion}, actual ${actualVersion ?? 'missing or invalid'})`;
    })
    .join('; ');
  return new Error(`Post-write validation failed: ${consumers}`);
}

export function parseArguments(arguments_) {
  if (arguments_.length === 0) {
    return 'write';
  }
  if (arguments_.length === 1 && arguments_[0] === '--check') {
    return 'check';
  }
  throw new Error('Usage: node scripts/sync-version.mjs [--check]');
}

export async function synchronizeVersions({
  mode,
  repositoryRoot = defaultRepositoryRoot,
  packagesRoot = defaultPackagesRoot,
  runCommand = defaultRunCommand,
  writeManagedFile = writeFile,
}) {
  if (mode !== 'write' && mode !== 'check') {
    throw new Error(`Unknown synchronization mode: ${mode}`);
  }

  const version = await canonicalVersion(repositoryRoot, runCommand);
  if (mode === 'check') {
    const drift = collectVersionDrift(version, await readConsumers(packagesRoot));
    return { version, drift };
  }

  const rendered = await renderVersionedFiles(version, packagesRoot);
  const snapshot = await snapshotManagedFiles(packagesRoot);
  try {
    await runCommand('bun', ['--version'], {
      cwd: packagesRoot,
      captureOutput: true,
    });
  } catch (error) {
    throw new Error(`Bun preflight failed: ${errorMessage(error)}. Install Bun and retry.`, {
      cause: error,
    });
  }

  try {
    await writeManagedFiles(
      rendered
        .filter(({ source, output }) => source !== output)
        .map(({ absolutePath, output }) => ({ absolutePath, contents: output })),
      writeManagedFile,
    );
    try {
      await runCommand('bun', ['install', '--lockfile-only'], {
        cwd: packagesRoot,
        captureOutput: false,
      });
    } catch (error) {
      throw new Error(`Bun lockfile regeneration failed: ${errorMessage(error)}`, {
        cause: error,
      });
    }

    const drift = collectVersionDrift(version, await readConsumers(packagesRoot));
    if (drift.length > 0) {
      throw postWriteValidationError(drift);
    }
    return { version, drift };
  } catch (error) {
    try {
      await restoreManagedFiles(snapshot, writeManagedFile);
    } catch (rollbackError) {
      throw new Error(
        `Version synchronization failed (${errorMessage(error)}) and rollback failed (${errorMessage(rollbackError)}). Restore the managed package files from version control.`,
        { cause: rollbackError },
      );
    }
    throw error;
  }
}

export async function main(arguments_) {
  const mode = parseArguments(arguments_);
  const { version, drift } = await synchronizeVersions({ mode });
  if (drift.length > 0) {
    reportDrift(drift);
    process.exitCode = 1;
    return;
  }

  const action = mode === 'check' ? 'Verified' : 'Synchronized';
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
