import assert from 'node:assert/strict';
import { existsSync } from 'node:fs';
import { readFile, readdir, stat } from 'node:fs/promises';
import { resolve, relative, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../', import.meta.url));
const json = async (path) => JSON.parse(await readFile(path, 'utf8'));
const marketplace = await json(join(root, '.agents/plugins/marketplace.json'));
assert.equal(marketplace.plugins.length, 1);
const entry = marketplace.plugins[0];
assert.ok(entry.source.path.startsWith('./'));
const pluginRoot = resolve(root, entry.source.path);
assert.ok(!relative(root, pluginRoot).startsWith('..'));
const repoRoot = resolve(root, '../..');
const rootMarketplacePath = join(repoRoot, '.agents/plugins/marketplace.json');
if (existsSync(rootMarketplacePath)) {
  const rootMarketplace = await json(rootMarketplacePath);
  assert.equal(rootMarketplace.name, marketplace.name);
  assert.equal(rootMarketplace.plugins.length, 1);
  const rootEntry = rootMarketplace.plugins[0];
  assert.equal(rootEntry.name, entry.name);
  assert.ok(rootEntry.source.path.startsWith('./'));
  assert.equal(resolve(repoRoot, rootEntry.source.path), pluginRoot);
}
const manifest = await json(join(pluginRoot, 'plugin.json'));
const pkg = await json(join(root, 'package.json'));
const catalog = await json(join(pluginRoot, 'runtime-catalog.json'));
assert.equal(catalog.schemaVersion, 1);
if (catalog.repository !== undefined) {
  assert.match(catalog.repository, /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/);
}
assert.match(catalog.openPencilVersion, /^\d+\.\d+\.\d+(?:-[\w.-]+)?$/);
assert.match(catalog.openPencilRevision, /^[a-f0-9]{40}$/);
assert.equal(catalog.release, `runtime-v${catalog.openPencilVersion}-${catalog.openPencilRevision.slice(0, 7)}`);
const platforms = ['darwin-arm64', 'darwin-x64', 'linux-arm64', 'linux-x64', 'win32-arm64', 'win32-x64'];
assert.deepEqual(Object.keys(catalog.platforms).sort(), platforms, 'Publish checksums for all six runtime builds');
for (const platform of platforms) {
  const asset = catalog.platforms[platform];
  assert.equal(asset.platform, platform);
  assert.equal(asset.asset, `openpencil-runtime-${platform}.tar.gz`);
  assert.ok(Number.isSafeInteger(asset.bytes) && asset.bytes > 0 && asset.bytes <= 256 * 1024 * 1024);
  assert.match(asset.sha256, /^[a-f0-9]{64}$/);
}
assert.equal(manifest.name, entry.name);
assert.equal(manifest.version, pkg.version);
assert.equal(entry.policy.installation, 'AVAILABLE');
assert.equal(manifest.extensions['com.openai'].requires_local_executor, true);
for (const path of [manifest.extensions['com.openai'].onboardingSkill, './LICENSE']) {
  // LICENSE belongs to the plugin package too, not only the marketplace repo.
  assert.ok(path.startsWith('./'));
  const target = resolve(pluginRoot, path);
  assert.ok(!relative(pluginRoot, target).startsWith('..'));
  assert.ok((await stat(target)).isFile());
}
const presentation = manifest.extensions['com.openai'].interface;
for (const field of ['composerIcon', 'composerIconDark', 'logo', 'logoDark']) {
  const path = presentation[field];
  assert.ok(typeof path === 'string' && path.startsWith('./'), `Missing relative ${field}`);
  const target = resolve(pluginRoot, path);
  assert.ok(!relative(pluginRoot, target).startsWith('..'), `Escaping ${field}`);
  const bytes = await readFile(target);
  assert.ok(bytes.length >= 24 && bytes.length <= 5 * 1024 * 1024, `Invalid ${field} size`);
  assert.equal(bytes.subarray(0, 8).toString('hex'), '89504e470d0a1a0a', `Invalid ${field} PNG`);
  const width = bytes.readUInt32BE(16), height = bytes.readUInt32BE(20);
  assert.ok(width === height && width >= 48 && width <= 4096, `Invalid ${field} dimensions`);
}
const mcp = await json(join(pluginRoot, 'mcp.json'));
assert.equal(mcp.mcpServers.codex_openpencil.type, 'stdio');
assert.equal(mcp.mcpServers.codex_openpencil.command, './scripts/launch-mcp');
for (const path of ['scripts/launch-mcp', 'scripts/launch-mcp.cmd', 'scripts/mcp-bridge.mjs']) {
  assert.ok((await stat(join(pluginRoot, path))).isFile());
}
if (process.platform !== 'win32') assert.ok((await stat(join(pluginRoot, 'scripts/launch-mcp'))).mode & 0o111);
for (const asset of ['app.html', 'panel.html', 'panel.css', 'panel.js', 'selection.js', 'codegen.js']) {
  assert.ok((await stat(join(pluginRoot, 'ui', asset))).isFile(), `Missing canvas asset: ${asset}`);
}
async function scan(dir) {
  for (const entry of await readdir(dir, { withFileTypes: true })) {
    const path = join(dir, entry.name);
    if (entry.isDirectory()) await scan(path);
    else {
      // Packaged raster assets are validated above; text checks do not apply to them.
      if (entry.name.endsWith('.png')) continue;
      // Git may check text out with CRLF on Windows; validate content, not line endings.
      const body = (await readFile(path, 'utf8')).replace(/\r\n/g, '\n');
      assert.ok(!body.includes('/Users/fini/'), `Machine path in ${path}`);
      assert.ok(body.split('\n').length <= 800, `File exceeds 800 lines: ${path}`);
      if (entry.name === 'SKILL.md') {
        assert.match(body, /^---\nname: [a-z0-9-]+\ndescription: .+\n---/);
        const matches = [...body.matchAll(/\]\(([^)]+)\)/g)];
        for (const [, link] of matches) {
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
console.log('Plugin identity, runtime release catalog, MCP paths, skill references, portability and license: OK');
