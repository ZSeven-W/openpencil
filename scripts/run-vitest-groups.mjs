#!/usr/bin/env node
import { spawnSync } from 'node:child_process';
import { existsSync, readdirSync, statSync } from 'node:fs';
import { dirname, join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const webRoot = join(repoRoot, 'apps', 'web');
const vitestCli = join(repoRoot, 'node_modules', 'vitest', 'vitest.mjs');
const nodeBin = process.env.VITEST_NODE_BIN ?? resolveNodeBin();
const maxBuffer = 64 * 1024 * 1024;

function resolveNodeBin() {
  const candidates = [
    process.env.NVM_BIN ? join(process.env.NVM_BIN, 'node') : '',
    '/opt/homebrew/bin/node',
    '/usr/local/bin/node',
    '/usr/bin/node',
  ].filter(Boolean);

  for (const candidate of candidates) {
    if (existsSync(candidate)) return candidate;
  }
  return 'node';
}

const ignoredDirs = new Set([
  '.git',
  '.turbo',
  'dist',
  'node_modules',
  'out',
  'coverage',
  'zig-out',
]);

function toPosix(path) {
  return path.split(sep).join('/');
}

function walkTestFiles(dir) {
  if (!existsSync(dir)) return [];
  const entries = readdirSync(dir).sort((a, b) => a.localeCompare(b));
  const files = [];
  for (const entry of entries) {
    if (ignoredDirs.has(entry)) continue;
    const fullPath = join(dir, entry);
    const stats = statSync(fullPath);
    if (stats.isDirectory()) {
      files.push(...walkTestFiles(fullPath));
      continue;
    }
    if (/\.test\.(ts|tsx)$/.test(entry)) files.push(fullPath);
  }
  return files;
}

function collectConfiguredTests() {
  const files = [
    ...walkTestFiles(join(webRoot, 'src')),
    ...walkTestFiles(join(webRoot, 'server')),
    ...walkTestFiles(join(repoRoot, 'apps', 'desktop')),
  ];

  const packagesRoot = join(repoRoot, 'packages');
  if (existsSync(packagesRoot)) {
    for (const packageName of readdirSync(packagesRoot).sort((a, b) => a.localeCompare(b))) {
      files.push(...walkTestFiles(join(packagesRoot, packageName, 'src')));
    }
  }

  return Array.from(new Set(files))
    .map((file) => toPosix(relative(webRoot, file)))
    .sort((a, b) => a.localeCompare(b));
}

function runVitest(files) {
  return spawnSync(
    nodeBin,
    [
      vitestCli,
      'run',
      ...files,
      '--passWithNoTests',
      '--pool=forks',
      '--fileParallelism=false',
    ],
    {
      cwd: webRoot,
      env: process.env,
      encoding: 'utf-8',
      maxBuffer,
    },
  );
}

function packageName(file) {
  const parts = file.split('/');
  return parts[3] ?? 'packages';
}

function desktopGroupName(file) {
  const parts = file.split('/');
  return parts[2] === '__tests__' ? 'desktop-root' : `desktop-${parts[2]}`;
}

function groupNameFor(file) {
  if (file.startsWith('../../packages/')) return `package-${packageName(file)}`;
  if (file.startsWith('../desktop/')) return desktopGroupName(file);
  if (file.startsWith('server/api/cloud/')) return 'server-api-cloud';
  if (file.startsWith('server/utils/')) return 'server-utils';
  if (file.startsWith('server/')) return 'server';
  if (file.startsWith('src/i18n/')) return 'src-i18n';
  if (file.startsWith('src/services/cloud/')) return 'src-services-cloud';
  if (file.startsWith('src/components/tasks/')) return 'src-components-tasks';
  if (file.startsWith('src/components/cloud/')) return 'src-components-cloud';
  if (file.startsWith('src/components/panels/git-panel/')) return 'src-components-git-panel';
  if (file.startsWith('src/components/panels/')) return 'src-components-panels';
  if (file.startsWith('src/components/')) return 'src-components';
  if (file.startsWith('src/canvas/')) return 'src-canvas';
  if (file.startsWith('src/services/ai/')) return 'src-services-ai';
  if (file.startsWith('src/services/')) return 'src-services';
  if (file.startsWith('src/stores/')) return 'src-stores';
  return file.split('/').slice(0, 2).join('-');
}

function buildGroups(files) {
  const groupMap = new Map();
  for (const file of files) {
    const name = groupNameFor(file);
    if (!groupMap.has(name)) groupMap.set(name, []);
    groupMap.get(name).push(file);
  }
  return Array.from(groupMap, ([name, groupFiles]) => ({
    name,
    files: groupFiles.sort((a, b) => a.localeCompare(b)),
  })).sort((a, b) => a.name.localeCompare(b.name));
}

const testFiles = collectConfiguredTests();
const groups = buildGroups(testFiles);
const failures = [];

console.log(`[vitest] running ${testFiles.length} test files in ${groups.length} isolated groups`);

for (const group of groups) {
  process.stdout.write(`[vitest] ${group.name} (${group.files.length} files) ... `);
  const result = runVitest(group.files);
  if (result.status === 0) {
    process.stdout.write('ok\n');
    continue;
  }

  process.stdout.write('failed, isolating files\n');
  for (const file of group.files) {
    process.stdout.write(`[vitest]   ${file} ... `);
    const isolated = runVitest([file]);
    if (isolated.status === 0) {
      process.stdout.write('ok\n');
      continue;
    }
    process.stdout.write('failed\n');
    failures.push(file);
    if (isolated.stdout) process.stdout.write(isolated.stdout);
    if (isolated.stderr) process.stderr.write(isolated.stderr);
  }
}

if (failures.length > 0) {
  console.error('\n[vitest] failed test files:');
  for (const file of failures) {
    console.error(` - ${file}`);
  }
  process.exit(1);
}

console.log('\n[vitest] all configured test files passed');
