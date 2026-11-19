import { spawnSync } from 'node:child_process';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { openPencil, openPencilRoot, platforms, projectRoot } from './platforms.mjs';

const requested = process.argv[2]?.replace(/^(?:qwenwork-)?v/u, '');
if (requested === undefined) throw new Error('usage: node scripts/check-release-version.mjs <version>');
if (!/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(requested)) {
  throw new Error(`invalid release version ${JSON.stringify(requested)}`);
}

const root = JSON.parse(await readFile(join(projectRoot, 'package.json'), 'utf8'));
if (root.version !== requested) throw new Error(`tag version ${requested} does not match package.json ${root.version}`);
const expectedOptionals = Object.fromEntries(platforms.map(platform => [platform.packageName, requested]));
if (JSON.stringify(root.optionalDependencies) !== JSON.stringify(expectedOptionals)) {
  throw new Error('root optionalDependencies are not the exact synchronized six-platform set');
}
for (const platform of platforms) {
  const manifest = JSON.parse(await readFile(join(projectRoot, 'npm', platform.id, 'package.json'), 'utf8'));
  if (manifest.version !== requested) throw new Error(`${platform.packageName} is not synchronized to ${requested}`);
}
for (const pluginRel of [
  'plugin.json',
  '.qwenwork-plugin/plugin.json',
  '.qoder-plugin/plugin.json',
  '.claude-plugin/plugin.json',
]) {
  const pluginManifest = JSON.parse(await readFile(join(projectRoot, 'plugins', 'openpencil', pluginRel), 'utf8'));
  if (pluginManifest.version !== requested) {
    throw new Error(`plugins/openpencil/${pluginRel} is ${pluginManifest.version}, expected ${requested}`);
  }
}

const controllerText = await readFile(join(projectRoot, 'plugins', 'openpencil', 'scripts', 'canvas-controller.mjs'), 'utf8');
if (!controllerText.includes(`'ui://openpencil/canvas/v${requested}.html'`)) {
  throw new Error(`canvas-controller.mjs CANVAS_URI does not match ${requested}`);
}
if (!controllerText.includes(`serverInfo: { name: 'qwenwork-openpencil', version: '${requested}' }`)) {
  throw new Error(`canvas-controller.mjs serverInfo.version does not match ${requested}`);
}
const appHtmlText = await readFile(join(projectRoot, 'plugins', 'openpencil', 'ui', 'app.html'), 'utf8');
if (!appHtmlText.includes(`appInfo: { name: 'OpenPencil Canvas', version: '${requested}' }`)) {
  throw new Error(`plugins/openpencil/ui/app.html appInfo.version does not match ${requested}`);
}

const cargo = await readFile(join(openPencilRoot, 'Cargo.toml'), 'utf8');
const cargoVersion = /^\[workspace\.package\][\s\S]*?^version = "([^"]+)"/mu.exec(cargo)?.[1];
if (cargoVersion !== openPencil.version) {
  throw new Error(`platforms.json pins OpenPencil ${openPencil.version}, Cargo.toml reports ${cargoVersion ?? 'nothing'}`);
}
git(['-C', openPencilRoot, 'merge-base', '--is-ancestor', openPencil.revision, 'HEAD']);
process.stdout.write(requested + '\n');

function git(args) {
  const result = spawnSync('git', args, { cwd: projectRoot, encoding: 'utf8' });
  if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed:\n${result.stderr || result.stdout}`);
  return result.stdout;
}
