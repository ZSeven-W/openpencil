import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { readFile, readdir } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { publishAndVerify } from './registry-convergence.mjs';
import { platforms } from './platforms.mjs';

if (process.argv.includes('--self-test')) runSelfTest();
else await main(resolve(process.argv[2] ?? 'artifacts'));

async function main(artifactRoot) {
  const reports = await readReports(artifactRoot);
  const expectedNames = ['@zseven-w/codex-openpencil', ...platforms.map(platform => platform.packageName)];
  if (reports.length !== expectedNames.length) {
    throw new Error(`expected exactly ${expectedNames.length} package reports, found ${reports.length}`);
  }
  const byName = new Map();
  for (const report of reports) {
    if (!expectedNames.includes(report.name)) throw new Error(`unexpected package report ${report.name}`);
    if (byName.has(report.name)) throw new Error(`duplicate package report ${report.name}`);
    await verifyReportIntegrity(report);
    byName.set(report.name, report);
  }
  const root = byName.get('@zseven-w/codex-openpencil');
  if (root === undefined) throw new Error('root package tarball is missing');
  const native = platforms.map(platform => {
    const report = byName.get(platform.packageName);
    if (report === undefined) throw new Error(`${platform.packageName} tarball is missing`);
    return report;
  });
  for (const report of [...native, root]) {
    if (report.version !== root.version) throw new Error(`${report.name} version differs from root ${root.version}`);
  }

  // Every release takes `latest`, prereleases included (see codex-release.yml).
  const tag = 'latest';
  const releaseReports = [...native, root];
  // Complete read-only preflight before the first registry mutation so an old,
  // partially published release cannot move a shared latest tag backward.
  const canRepairExistingTag = Boolean(process.env.NODE_AUTH_TOKEN?.trim());
  preflightRelease(releaseReports, tag, report => readReleaseState(report, tag), canRepairExistingTag);
  for (const report of releaseReports) await publishOrVerify(report, tag, canRepairExistingTag);
}

async function publishOrVerify(report, tag, canRepairExistingTag) {
  const specifier = `${report.name}@${report.version}`;
  const { existing, currentTag } = readReleaseState(report, tag);
  assertSafeReleaseState(report, tag, { existing, currentTag }, canRepairExistingTag);
  if (existing !== undefined) {
    if (existing !== report.integrity) {
      throw new Error(`${specifier} already exists with different integrity (${existing} != ${report.integrity})`);
    }
    const action = tagAction(currentTag, report.version, true);
    if (action === 'advance') {
      runNpm(['dist-tag', 'add', specifier, tag]);
      await waitForDistTag(report.name, report.version, tag);
      process.stdout.write(`${specifier} already published with matching integrity; advanced dist-tag ${tag}\n`);
    } else {
      process.stdout.write(
        `${specifier} already published with matching integrity; `
        + (action === 'skip-newer'
          ? `kept newer dist-tag ${tag}=${currentTag}\n`
          : `dist-tag ${tag} already matches\n`),
      );
    }
    return;
  }

  const action = tagAction(currentTag, report.version, false);
  if (action === 'refuse-missing-older' || action === 'refuse-inconsistent') {
    throw new Error(
      `${specifier} is not published while dist-tag ${tag} points to ${JSON.stringify(currentTag)}; `
      + 'refusing to publish because the shared tag would move backward or registry state is inconsistent',
    );
  }

  const tarball = join(report.directory, report.filename);
  await publishAndVerify({
    specifier, version: report.version, integrity: report.integrity, tag,
    publish: () => npmResult(['publish', tarball, '--access=public', `--tag=${tag}`, '--provenance']),
    readState: () => ({ integrity: registryIntegrity(specifier), tag: distTagVersion(report.name, tag) }),
  });
}

function readReleaseState(report, tag) {
  return {
    existing: registryIntegrity(`${report.name}@${report.version}`),
    currentTag: distTagVersion(report.name, tag),
  };
}

function assertSafeReleaseState(report, tag, state, canRepairExistingTag) {
  const specifier = `${report.name}@${report.version}`;
  if (state.existing !== undefined && state.existing !== report.integrity) {
    throw new Error(`${specifier} already exists with different integrity (${state.existing} != ${report.integrity})`);
  }
  const action = tagAction(state.currentTag, report.version, state.existing !== undefined);
  if (action === 'refuse-missing-older' || action === 'refuse-inconsistent') {
    throw new Error(
      `${specifier} is not published while dist-tag ${tag} points to ${JSON.stringify(state.currentTag)}; `
      + 'refusing the entire seven-package release before mutation because the shared tag would move backward or registry state is inconsistent',
    );
  }
  if (action === 'advance' && state.existing !== undefined && !canRepairExistingTag) {
    throw new Error(
      `${specifier} needs dist-tag ${tag} recovery before the release can continue: recovery requires NPM_TOKEN `
      + '(NODE_AUTH_TOKEN); trusted publishing covers new npm publish but not npm dist-tag add',
    );
  }
  return action;
}

function preflightRelease(reports, tag, stateFor, canRepairExistingTag) {
  const states = new Map();
  for (const report of reports) {
    const state = stateFor(report);
    const action = assertSafeReleaseState(report, tag, state, canRepairExistingTag);
    states.set(report.name, { ...state, action });
  }
  return states;
}

function distTagVersion(name, tag) {
  const result = npmResult(['view', name, 'dist-tags', '--json']);
  if (result.status !== 0) {
    const output = result.stderr + result.stdout;
    if (/E404|404 Not Found/u.test(output)) return undefined;
    throw new Error(`npm view ${name} dist-tags failed:\n${output}`);
  }
  const tags = JSON.parse(result.stdout);
  return tags?.[tag];
}

async function waitForDistTag(name, version, tag) {
  let current;
  for (let attempt = 1; attempt <= 30; attempt += 1) {
    current = distTagVersion(name, tag);
    if (current === version) return;
    process.stdout.write(`${name}: waiting for dist-tag ${tag}=${version}; current=${JSON.stringify(current)} (${attempt}/30)\n`);
    if (attempt < 30) await delay(10_000);
  }
  throw new Error(`${name}: dist-tag ${tag} did not converge to ${version}; current=${JSON.stringify(current)}`);
}

function tagAction(current, target, exists) {
  if (current === undefined) return exists ? 'advance' : 'publish-shared';
  const comparison = compareSemVer(current, target);
  if (comparison > 0) return exists ? 'skip-newer' : 'refuse-missing-older';
  if (comparison === 0) return exists ? 'skip-equal' : 'refuse-inconsistent';
  return exists ? 'advance' : 'publish-shared';
}

function compareSemVer(left, right) {
  const a = parseSemVer(left);
  const b = parseSemVer(right);
  for (const field of ['major', 'minor', 'patch']) {
    if (a[field] !== b[field]) return a[field] < b[field] ? -1 : 1;
  }
  if (a.prerelease.length === 0 || b.prerelease.length === 0) {
    if (a.prerelease.length === b.prerelease.length) return 0;
    return a.prerelease.length === 0 ? 1 : -1;
  }
  const count = Math.max(a.prerelease.length, b.prerelease.length);
  for (let index = 0; index < count; index += 1) {
    const leftId = a.prerelease[index];
    const rightId = b.prerelease[index];
    if (leftId === undefined || rightId === undefined) {
      if (leftId === rightId) return 0;
      return leftId === undefined ? -1 : 1;
    }
    if (leftId === rightId) continue;
    const leftNumeric = /^\d+$/u.test(leftId);
    const rightNumeric = /^\d+$/u.test(rightId);
    if (leftNumeric && rightNumeric) return BigInt(leftId) < BigInt(rightId) ? -1 : 1;
    if (leftNumeric !== rightNumeric) return leftNumeric ? -1 : 1;
    return leftId < rightId ? -1 : 1;
  }
  return 0;
}

function parseSemVer(value) {
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/u.exec(value);
  if (match === null) throw new Error(`registry returned invalid semantic version ${JSON.stringify(value)}`);
  const prerelease = match[4]?.split('.') ?? [];
  if (prerelease.some(identifier => /^\d+$/u.test(identifier) && identifier.length > 1 && identifier.startsWith('0'))) {
    throw new Error(`registry returned invalid semantic version ${JSON.stringify(value)}`);
  }
  return { major: BigInt(match[1]), minor: BigInt(match[2]), patch: BigInt(match[3]), prerelease };
}

function runSelfTest() {
  const comparisons = [
    ['0.3.13-rc.2', '0.3.13-rc.10', -1],
    ['0.3.13-rc.10', '0.3.13', -1],
    ['1.0.0-alpha', '1.0.0-alpha.1', -1],
    ['1.0.0-alpha.1', '1.0.0-alpha.beta', -1],
    ['1.2.3', '1.2.3', 0],
    ['2.0.0', '1.99.99', 1],
  ];
  for (const [left, right, expected] of comparisons) {
    assert.equal(Math.sign(compareSemVer(left, right)), expected, `${left} versus ${right}`);
    assert.equal(Math.sign(compareSemVer(right, left)), expected === 0 ? 0 : -expected, `${right} versus ${left}`);
  }
  assert.equal(tagAction(undefined, '0.3.14-rc.3', true), 'advance');
  assert.equal(tagAction('0.3.14-rc.2', '0.3.14-rc.3', true), 'advance');
  assert.equal(tagAction('0.3.14-rc.3', '0.3.14-rc.3', true), 'skip-equal');
  assert.equal(tagAction('0.3.14-rc.4', '0.3.14-rc.3', true), 'skip-newer');
  assert.equal(tagAction(undefined, '0.3.14-rc.3', false), 'publish-shared');
  assert.equal(tagAction('0.3.14-rc.2', '0.3.14-rc.3', false), 'publish-shared');
  assert.equal(tagAction('0.3.14-rc.3', '0.3.14-rc.3', false), 'refuse-inconsistent');
  assert.equal(tagAction('0.3.14-rc.4', '0.3.14-rc.3', false), 'refuse-missing-older');

  const fixtureReports = [
    { name: 'native-first', version: '0.3.14-rc.3', integrity: 'sha512-first' },
    { name: 'root-last', version: '0.3.14-rc.3', integrity: 'sha512-root' },
  ];
  let mutations = 0;
  assert.throws(() => {
    preflightRelease(fixtureReports, 'next', report => (
      report.name === 'native-first'
        ? { existing: undefined, currentTag: '0.3.14-rc.2' }
        : { existing: undefined, currentTag: '0.3.14-rc.4' }
    ), false);
    for (const _report of fixtureReports) mutations += 1;
  }, /refusing the entire seven-package release before mutation/);
  assert.equal(mutations, 0, 'a later preflight refusal must prevent every publish mutation');

  assert.throws(
    () => preflightRelease(
      fixtureReports,
      'next',
      report => ({ existing: report.integrity, currentTag: '0.3.14-rc.2' }),
      false,
    ),
    /recovery requires NPM_TOKEN.*trusted publishing covers new npm publish but not npm dist-tag add/,
  );
  const repairPlan = preflightRelease(
    fixtureReports,
    'next',
    report => ({ existing: report.integrity, currentTag: '0.3.14-rc.2' }),
    true,
  );
  assert.equal(repairPlan.get('root-last')?.action, 'advance');
  process.stdout.write('publish release SemVer/dist-tag fixtures passed\n');
}

function registryIntegrity(specifier) {
  const result = npmResult(['view', specifier, 'dist.integrity', '--json']);
  if (result.status === 0) {
    const value = JSON.parse(result.stdout);
    return typeof value === 'string' ? value : undefined;
  }
  const output = result.stderr + result.stdout;
  if (/E404|404 Not Found/u.test(output)) return undefined;
  throw new Error(`npm view ${specifier} failed:\n${output}`);
}

function runNpm(args) {
  const result = npmResult(args);
  if (result.status !== 0) throw new Error(`npm ${args.join(' ')} failed:\n${result.stderr || result.stdout}`);
}

function npmResult(args) {
  const npm = process.platform === 'win32' ? 'npm.cmd' : 'npm';
  return spawnSync(npm, args, {
    encoding: 'utf8',
    env: { ...process.env, npm_config_cache: join(tmpdir(), 'codex-openpencil-npm-cache') },
    shell: process.platform === 'win32',
  });
}

async function verifyReportIntegrity(report) {
  const tarball = await readFile(join(report.directory, report.filename));
  const integrity = `sha512-${createHash('sha512').update(tarball).digest('base64')}`;
  const shasum = createHash('sha1').update(tarball).digest('hex');
  if (report.integrity !== integrity || report.shasum !== shasum) {
    throw new Error(`${report.name}@${report.version}: report integrity does not match tarball`);
  }
}

async function readReports(directory) {
  const output = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) output.push(...await readReports(path));
    else if (entry.name.endsWith('.tgz.json')) {
      output.push({ ...JSON.parse(await readFile(path, 'utf8')), directory });
    }
  }
  return output;
}
