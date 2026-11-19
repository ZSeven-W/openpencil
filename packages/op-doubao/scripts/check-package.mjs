import assert from 'node:assert/strict';
import { readFile, readdir, stat } from 'node:fs/promises';
import { resolve, relative, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parseEditorRuntimeCatalog } from '../plugins/openpencil/scripts/editor-runtime.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const json = async (path) => JSON.parse(await readFile(path, 'utf8'));

const pkg = await json(join(root, 'package.json'));
const catalog = parseEditorRuntimeCatalog(await json(join(root, 'platforms.json')));
assert.equal(catalog.platforms.length, 6, 'Publish runtime packages for all six platforms');
const expectedOptionals = Object.fromEntries(catalog.platforms.map(platform => [platform.packageName, pkg.version]));
assert.deepEqual(pkg.optionalDependencies, expectedOptionals);
for (const platform of catalog.platforms) {
  const platformPkg = await json(join(root, 'npm', platform.id, 'package.json'));
  assert.equal(platformPkg.name, platform.packageName);
  assert.equal(platformPkg.version, pkg.version);
}

const pluginRoot = resolve(root, 'plugins/openpencil');
const manifest = await json(join(pluginRoot, 'plugin.json'));
assert.equal(manifest.name, 'openpencil');
assert.equal(manifest.version, pkg.version);

for (const assetRel of ['./assets/icon.png', './assets/logo.png']) {
  const target = resolve(pluginRoot, assetRel);
  const bytes = await readFile(target);
  assert.ok(bytes.length >= 24 && bytes.length <= 5 * 1024 * 1024, `Invalid ${assetRel} size`);
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a', `Invalid ${assetRel} PNG`);
  const width = bytes.readUInt32BE(16);
  const height = bytes.readUInt32BE(20);
  assert.ok(width === height && width >= 48 && width <= 4096, `Invalid ${assetRel} dimensions`);
}

for (const mcpRel of ['.mcp.json', 'mcp.json']) {
  const mcp = await json(join(pluginRoot, mcpRel));
  assert.equal(mcp.mcpServers.openpencil.type, 'stdio');
  assert.equal(mcp.mcpServers.openpencil.command, './scripts/launch-mcp');
}

for (const path of ['LICENSE', 'NOTICE', 'CONNECTORS.md', 'scripts/launch-mcp', 'scripts/launch-mcp.cmd', 'scripts/mcp-bridge.mjs']) {
  assert.ok((await stat(join(pluginRoot, path))).isFile(), `Missing ${path}`);
}
if (process.platform !== 'win32') {
  assert.ok((await stat(join(pluginRoot, 'scripts/launch-mcp'))).mode & 0o111, 'launch-mcp must be executable');
  assert.ok((await stat(join(root, 'bin/doubao-openpencil.mjs'))).mode & 0o111, 'CLI bin must be executable');
}
for (const asset of ['app.html', 'panel.html', 'panel.css', 'panel.js', 'selection.js', 'codegen.js']) {
  assert.ok((await stat(join(pluginRoot, 'ui', asset))).isFile(), `Missing canvas asset: ${asset}`);
}

async function scan(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) {
      await scan(path);
    } else {
      if (entry.name.endsWith('.png')) continue;
      const body = (await readFile(path, 'utf8')).replace(/\r\n/g, '\n');
      assert.ok(!/\/Users\/[A-Za-z0-9_.-]+\//.test(body), `Machine path in ${path}`);
      assert.ok(body.split('\n').length <= 800, `File exceeds 800 lines: ${path}`);
      if (entry.name === 'SKILL.md') {
        assert.match(body, /^---\nname: [a-z0-9-]+\ndescription: .+\n---/);
        for (const [, link] of body.matchAll(/\]\(([^)]+)\)/g)) {
          if (/^https?:/.test(link)) continue;
          const target = resolve(dir, link);
          assert.ok(!relative(pluginRoot, target).startsWith('..'), `Escaping reference ${link}`);
          assert.ok((await stat(target)).isFile(), `Missing reference ${link}`);
        }
      }
    }
  }
}
await scan(pluginRoot);
console.log('Doubao connector identity, six-platform npm catalog, MCP paths, skill references, portability and license: OK');
