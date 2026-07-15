import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import test from 'node:test';

import * as syncVersion from './sync-version.mjs';

import {
  collectVersionDrift,
  inspectBunLockWorkspaceVersions,
  renderPackageManifest,
  renderSdkEntry,
} from './sync-version.mjs';

const fixtureManagedPaths = [
  'package.json',
  'op-web-sdk/package.json',
  'op-web-sdk-react/package.json',
  'op-web-sdk-vue/package.json',
  'op-web-sdk/src/index.ts',
  'op-web-sdk-react/src/index.ts',
  'op-web-sdk-vue/src/index.ts',
  'bun.lock',
];

function fixtureLockfile(version) {
  return `{
  "lockfileVersion": 1,
  "workspaces": {
    "": { "name": "@zseven-w/openpencil-packages" },
    "op-web-sdk": { "version": "${version}" },
    "op-web-sdk-react": { "version": "${version}" },
    "op-web-sdk-vue": { "version": "${version}" },
  },
}
`;
}

async function createRepositoryFixture(t, version = '1.0.0') {
  const repositoryRoot = await mkdtemp(join(tmpdir(), 'sync-version-'));
  const packagesRoot = join(repositoryRoot, 'packages');
  t.after(() => rm(repositoryRoot, { recursive: true, force: true }));

  const files = {
    'package.json': `${JSON.stringify({ name: 'fixture-root', version }, null, 2)}\n`,
    'op-web-sdk/package.json': `${JSON.stringify({ name: 'op-web-sdk', version }, null, 2)}\n`,
    'op-web-sdk-react/package.json': `${JSON.stringify({ name: 'op-web-sdk-react', version }, null, 2)}\n`,
    'op-web-sdk-vue/package.json': `${JSON.stringify({ name: 'op-web-sdk-vue', version }, null, 2)}\n`,
    'op-web-sdk/src/index.ts': `export const VERSION = '${version}';\n`,
    'op-web-sdk-react/src/index.ts': `export const VERSION = '${version}';\n`,
    'op-web-sdk-vue/src/index.ts': `export const VERSION = '${version}';\n`,
    'bun.lock': fixtureLockfile(version),
  };

  await Promise.all(
    Object.entries(files).map(async ([path, contents]) => {
      const absolutePath = join(packagesRoot, path);
      await mkdir(dirname(absolutePath), { recursive: true });
      await writeFile(absolutePath, contents);
    }),
  );

  return {
    repositoryRoot,
    packagesRoot,
    managedPaths: fixtureManagedPaths.map((path) => join(packagesRoot, path)),
  };
}

async function readManagedFiles(paths) {
  return Promise.all(paths.map((path) => readFile(path)));
}

function createCommandRunner(packagesRoot, version = '2.3.4', { regenerateLock = true } = {}) {
  const calls = [];
  return {
    calls,
    runCommand: async (command, arguments_) => {
      calls.push([command, ...arguments_]);
      if (command === 'sh') {
        return `${version}\n`;
      }
      if (arguments_[0] === '--version') {
        return '1.3.11\n';
      }
      if (arguments_[0] === 'install') {
        if (regenerateLock) {
          await writeFile(join(packagesRoot, 'bun.lock'), fixtureLockfile(version));
        }
        return '';
      }
      throw new Error(`Unexpected command: ${command} ${arguments_.join(' ')}`);
    },
  };
}

test('argument parsing selects write or check mode and rejects unknown arguments', () => {
  assert.equal(syncVersion.parseArguments([]), 'write');
  assert.equal(syncVersion.parseArguments(['--check']), 'check');
  assert.throws(() => syncVersion.parseArguments(['1.2.3']), /usage/i);
  assert.throws(() => syncVersion.parseArguments(['--check', '--extra']), /usage/i);
});

test('package manifest rendering changes only the top-level version and ends with a newline', () => {
  const input = `{
    "name": "example",
    "version": "1.0.0",
    "metadata": { "version": "unchanged" }
  }`;

  assert.equal(
    renderPackageManifest(input, '2.3.4'),
    `{
  "name": "example",
  "version": "2.3.4",
  "metadata": {
    "version": "unchanged"
  }
}
`,
  );
});

test('SDK entry rendering replaces exactly one public VERSION export', () => {
  const input = `export { mount } from './mount';
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `export { mount } from './mount';
export const VERSION = '2.3.4';
`,
  );
});

test('SDK entry rendering rejects a missing VERSION export', () => {
  assert.throws(
    () => renderSdkEntry(`export { mount } from './mount';\n`, '2.3.4'),
    /exactly one.*VERSION export/i,
  );
});

test('SDK entry rendering rejects multiple VERSION exports', () => {
  const input = `export const VERSION = '1.0.0';
export const VERSION = '1.0.1';
`;

  assert.throws(() => renderSdkEntry(input, '2.3.4'), /exactly one.*VERSION export/i);
});

test('SDK entry rendering does not count declaration text inside a comment', () => {
  const input = `// export const VERSION = 'documentation-only';
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `// export const VERSION = 'documentation-only';
export const VERSION = '2.3.4';
`,
  );
});

test('SDK entry rendering ignores declaration-shaped lines inside block comments', () => {
  const input = `/*
export const VERSION = 'documentation-only';
*/
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `/*
export const VERSION = 'documentation-only';
*/
export const VERSION = '2.3.4';
`,
  );
});

test('SDK entry rendering ignores declaration-shaped lines inside template literals', () => {
  const input = `const documentation = \`
export const VERSION = 'documentation-only';
\`;
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `const documentation = \`
export const VERSION = 'documentation-only';
\`;
export const VERSION = '2.3.4';
`,
  );
});

test('SDK entry rendering ignores declarations inside nested template literals', () => {
  const input = `const documentation = \`
\${\`nested
export const VERSION = 'documentation-only';
\`}
\`;
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `const documentation = \`
\${\`nested
export const VERSION = 'documentation-only';
\`}
\`;
export const VERSION = '2.3.4';
`,
  );
});

test('SDK entry rendering does not treat comment markers inside quoted strings as comments', () => {
  const input = `const blockMarker = '/*';
const lineMarker = "//";
export const VERSION = '1.0.0';
`;

  assert.equal(
    renderSdkEntry(input, '2.3.4'),
    `const blockMarker = '/*';
const lineMarker = "//";
export const VERSION = '2.3.4';
`,
  );
});

test('Bun lock inspection finds every versioned SDK workspace and ignores the root workspace', () => {
  const lockfile = `{
    "workspaces": {
      "": {
        "name": "@zseven-w/openpencil-packages",
      },
      "op-web-sdk": {
        "name": "@zseven-w/op-web-sdk",
        "version": "1.0.0",
        "devDependencies": { "fixture": "1.2.3" },
      },
      "op-web-sdk-react": {
        "name": "@zseven-w/op-web-sdk-react",
        "version": "1.0.1",
      },
      "op-web-sdk-vue": {
        "name": "@zseven-w/op-web-sdk-vue",
        "version": "1.0.2",
      },
    },
  }`;

  assert.deepEqual(inspectBunLockWorkspaceVersions(lockfile), {
    'op-web-sdk': '1.0.0',
    'op-web-sdk-react': '1.0.1',
    'op-web-sdk-vue': '1.0.2',
  });
});

test('Bun lock inspection ignores structural delimiters in block comments after primitive values', () => {
  const lockfile = `{
    "lockfileVersion": 1 /* comma, brace }, bracket ] */,
    "workspaces": {
      "op-web-sdk": { "version": "1.0.0" },
    },
  }`;

  assert.deepEqual(inspectBunLockWorkspaceVersions(lockfile), {
    'op-web-sdk': '1.0.0',
  });
});

test('Bun lock inspection ignores structural delimiters in line comments after primitive values', () => {
  const lockfile = `{
    "lockfileVersion": 1 // comma, brace }, bracket ]
    ,
    "workspaces": {
      "op-web-sdk": { "version": "1.0.0" },
    },
  }`;

  assert.deepEqual(inspectBunLockWorkspaceVersions(lockfile), {
    'op-web-sdk': '1.0.0',
  });
});

test('Bun lock inspection still rejects unterminated block comments after primitive values', () => {
  const lockfile = `{
    "lockfileVersion": 1 /* unterminated }, ],
    "workspaces": {},
  }`;

  assert.throws(() => inspectBunLockWorkspaceVersions(lockfile), /unterminated block comment/i);
});

test('drift collection reports each stale path with expected and actual versions', () => {
  assert.deepEqual(
    collectVersionDrift('2.3.4', [
      { path: 'packages/package.json', actualVersion: '2.3.3' },
      { path: 'packages/op-web-sdk/package.json', actualVersion: '2.3.4' },
      { path: 'packages/bun.lock#op-web-sdk', actualVersion: undefined },
    ]),
    [
      {
        path: 'packages/package.json',
        expectedVersion: '2.3.4',
        actualVersion: '2.3.3',
      },
      {
        path: 'packages/bun.lock#op-web-sdk',
        expectedVersion: '2.3.4',
        actualVersion: undefined,
      },
    ],
  );
});

test('an already synchronized in-memory repository reports no drift', () => {
  const version = '2.3.4';
  const manifest = renderPackageManifest('{"name":"fixture","version":"1.0.0"}', version);
  const sdkEntry = renderSdkEntry("export const VERSION = '1.0.0';\n", version);
  const lockVersions = inspectBunLockWorkspaceVersions(`{
    "workspaces": {
      "op-web-sdk": { "version": "${version}" },
      "op-web-sdk-react": { "version": "${version}" },
      "op-web-sdk-vue": { "version": "${version}" },
    },
  }`);

  assert.deepEqual(
    collectVersionDrift(version, [
      { path: 'packages/package.json', actualVersion: JSON.parse(manifest).version },
      {
        path: 'packages/op-web-sdk/src/index.ts',
        actualVersion: sdkEntry.match(/VERSION = '([^']+)'/)?.[1],
      },
      ...Object.entries(lockVersions).map(([workspace, actualVersion]) => ({
        path: `packages/bun.lock#${workspace}`,
        actualVersion,
      })),
    ]),
    [],
  );
});

test('check mode reports stale paths without changing managed files', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const before = await readManagedFiles(fixture.managedPaths);
  const runner = createCommandRunner(fixture.packagesRoot);

  const result = await syncVersion.synchronizeVersions({
    mode: 'check',
    ...fixture,
    runCommand: runner.runCommand,
  });

  assert.equal(result.version, '2.3.4');
  assert.deepEqual(
    result.drift.map(({ path }) => path),
    [
      'packages/package.json',
      'packages/op-web-sdk/package.json',
      'packages/op-web-sdk-react/package.json',
      'packages/op-web-sdk-vue/package.json',
      'packages/op-web-sdk/src/index.ts',
      'packages/op-web-sdk-react/src/index.ts',
      'packages/op-web-sdk-vue/src/index.ts',
      'packages/bun.lock#workspaces.op-web-sdk',
      'packages/bun.lock#workspaces.op-web-sdk-react',
      'packages/bun.lock#workspaces.op-web-sdk-vue',
    ],
  );
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), before);
  assert.deepEqual(runner.calls, [['sh', 'scripts/workspace-version.sh', 'Cargo.toml']]);
});

test('write mode synchronizes every consumer and is idempotent', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const runner = createCommandRunner(fixture.packagesRoot);

  const first = await syncVersion.synchronizeVersions({
    mode: 'write',
    ...fixture,
    runCommand: runner.runCommand,
  });
  const afterFirst = await readManagedFiles(fixture.managedPaths);
  const second = await syncVersion.synchronizeVersions({
    mode: 'write',
    ...fixture,
    runCommand: runner.runCommand,
  });

  assert.deepEqual(first.drift, []);
  assert.deepEqual(second.drift, []);
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), afterFirst);
  await Promise.all(
    fixture.managedPaths.slice(0, 4).map(async (path) => {
      assert.equal(JSON.parse(await readFile(path, 'utf8')).version, '2.3.4');
    }),
  );
  await Promise.all(
    fixture.managedPaths.slice(4, 7).map(async (path) => {
      assert.match(await readFile(path, 'utf8'), /VERSION = '2\.3\.4'/);
    }),
  );
  assert.deepEqual(
    inspectBunLockWorkspaceVersions(await readFile(fixture.managedPaths[7], 'utf8')),
    {
      'op-web-sdk': '2.3.4',
      'op-web-sdk-react': '2.3.4',
      'op-web-sdk-vue': '2.3.4',
    },
  );
  assert.equal(runner.calls.filter(([, argument]) => argument === 'install').length, 2);
});

test('write mode performs a Bun preflight before making any changes', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const before = await readManagedFiles(fixture.managedPaths);
  const calls = [];
  const runCommand = async (command, arguments_) => {
    calls.push([command, ...arguments_]);
    if (command === 'sh') {
      return '2.3.4\n';
    }
    if (arguments_[0] === '--version') {
      throw new Error('bun unavailable');
    }
    throw new Error('Bun install should not run after a failed preflight');
  };

  await assert.rejects(
    syncVersion.synchronizeVersions({
      mode: 'write',
      ...fixture,
      runCommand,
    }),
    /bun.*preflight.*unavailable/i,
  );
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), before);
  assert.deepEqual(calls, [
    ['sh', 'scripts/workspace-version.sh', 'Cargo.toml'],
    ['bun', '--version'],
  ]);
});

test('write mode restores every managed file when Bun lock regeneration fails', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const before = await readManagedFiles(fixture.managedPaths);
  const runCommand = async (command, arguments_) => {
    if (command === 'sh') {
      return '2.3.4\n';
    }
    if (arguments_[0] === '--version') {
      return '1.3.11\n';
    }
    await writeFile(join(fixture.packagesRoot, 'bun.lock'), 'partially regenerated\n');
    throw new Error('install exploded');
  };

  await assert.rejects(
    syncVersion.synchronizeVersions({
      mode: 'write',
      ...fixture,
      runCommand,
    }),
    /lockfile regeneration failed.*install exploded/i,
  );
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), before);
});

test('write mode restores every managed file when post-write validation fails', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const before = await readManagedFiles(fixture.managedPaths);
  const runner = createCommandRunner(fixture.packagesRoot, '2.3.4', {
    regenerateLock: false,
  });

  await assert.rejects(
    syncVersion.synchronizeVersions({
      mode: 'write',
      ...fixture,
      runCommand: runner.runCommand,
    }),
    /post-write validation failed.*bun\.lock/i,
  );
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), before);
});

test('write mode waits for sibling writes to settle before rollback', async (t) => {
  const fixture = await createRepositoryFixture(t);
  const before = await readManagedFiles(fixture.managedPaths);
  const runner = createCommandRunner(fixture.packagesRoot);
  const attempts = new Map();
  const writeManagedFile = async (path, contents) => {
    const attempt = attempts.get(path) ?? 0;
    attempts.set(path, attempt + 1);
    if (attempt === 0 && path === fixture.managedPaths[0]) {
      throw new Error('simulated write failure');
    }
    if (attempt === 0 && path === fixture.managedPaths[1]) {
      await new Promise((resolvePromise) => setTimeout(resolvePromise, 30));
    }
    await writeFile(path, contents);
  };

  await assert.rejects(
    syncVersion.synchronizeVersions({
      mode: 'write',
      ...fixture,
      runCommand: runner.runCommand,
      writeManagedFile,
    }),
    /simulated write failure/i,
  );
  await new Promise((resolvePromise) => setTimeout(resolvePromise, 40));
  assert.deepEqual(await readManagedFiles(fixture.managedPaths), before);
});
