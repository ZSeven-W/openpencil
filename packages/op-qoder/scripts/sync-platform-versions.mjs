import { readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { platforms, projectRoot } from './platforms.mjs';

const requested = process.argv[2]?.replace(/^(?:qoder-)?v/u, '');
const rootPath = join(projectRoot, 'package.json');
const rootManifest = JSON.parse(await readFile(rootPath, 'utf8'));
const version = requested ?? rootManifest.version;
if (typeof version !== 'string' || !/^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?$/u.test(version)) {
  throw new Error(`invalid release version ${JSON.stringify(version)}`);
}

rootManifest.version = version;
rootManifest.optionalDependencies = Object.fromEntries(
  platforms.map(platform => [platform.packageName, version]),
);
await writeFile(rootPath, JSON.stringify(rootManifest, null, 2) + '\n');

for (const platform of platforms) {
  const manifestPath = join(projectRoot, 'npm', platform.id, 'package.json');
  const manifest = JSON.parse(await readFile(manifestPath, 'utf8'));
  manifest.version = version;
  await writeFile(manifestPath, JSON.stringify(manifest, null, 2) + '\n');
}

for (const pluginRel of ['plugin.json', '.qoder-plugin/plugin.json']) {
  const pluginPath = join(projectRoot, 'plugins', 'openpencil', pluginRel);
  const pluginManifest = JSON.parse(await readFile(pluginPath, 'utf8'));
  pluginManifest.version = version;
  await writeFile(pluginPath, JSON.stringify(pluginManifest, null, 2) + '\n');
}

const controllerPath = join(projectRoot, 'plugins', 'openpencil', 'scripts', 'canvas-controller.mjs');
const controllerBefore = await readFile(controllerPath, 'utf8');
const controllerAfter = controllerBefore
  .replace(
    /export const CANVAS_URI = 'ui:\/\/openpencil\/canvas\/v[^']+\.html';/u,
    `export const CANVAS_URI = 'ui://openpencil/canvas/v${version}.html';`,
  )
  .replace(
    /serverInfo: \{ name: 'qoder-openpencil', version: '[^']+' \}/u,
    `serverInfo: { name: 'qoder-openpencil', version: '${version}' }`,
  );
if (
  !controllerAfter.includes(`'ui://openpencil/canvas/v${version}.html'`)
  || !controllerAfter.includes(`serverInfo: { name: 'qoder-openpencil', version: '${version}' }`)
) {
  throw new Error('failed to update version markers in plugins/openpencil/scripts/canvas-controller.mjs');
}
await writeFile(controllerPath, controllerAfter);

const appHtmlPath = join(projectRoot, 'plugins', 'openpencil', 'ui', 'app.html');
const appHtmlBefore = await readFile(appHtmlPath, 'utf8');
const appHtmlAfter = appHtmlBefore.replace(
  /appInfo: \{ name: 'OpenPencil Canvas', version: '[^']+' \}/u,
  `appInfo: { name: 'OpenPencil Canvas', version: '${version}' }`,
);
if (!appHtmlAfter.includes(`appInfo: { name: 'OpenPencil Canvas', version: '${version}' }`)) {
  throw new Error('failed to update appInfo.version in plugins/openpencil/ui/app.html');
}
await writeFile(appHtmlPath, appHtmlAfter);

process.stdout.write(`synchronized Qoder root, six runtime packages, and plugin metadata to ${version}\n`);
