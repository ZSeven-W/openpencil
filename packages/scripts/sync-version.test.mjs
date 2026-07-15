import assert from 'node:assert/strict';
import test from 'node:test';

import {
  collectVersionDrift,
  inspectBunLockWorkspaceVersions,
  renderPackageManifest,
  renderSdkEntry,
} from './sync-version.mjs';

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
