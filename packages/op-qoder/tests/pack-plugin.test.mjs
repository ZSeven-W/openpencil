import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { inflateRawSync } from 'node:zlib';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { packQoderPlugin } from '../scripts/pack-plugin.mjs';

function readZipCentralDirectory(buffer) {
  const eocdOffset = buffer.length - 22;
  assert.equal(buffer.readUInt32LE(eocdOffset), 0x06054b50, 'Missing ZIP EOCD signature');
  const totalEntries = buffer.readUInt16LE(eocdOffset + 10);
  const centralSize = buffer.readUInt32LE(eocdOffset + 12);
  const centralOffset = buffer.readUInt32LE(eocdOffset + 16);
  assert.equal(centralOffset + centralSize, eocdOffset);

  const entries = [];
  let cursor = centralOffset;
  for (let i = 0; i < totalEntries; i++) {
    assert.equal(buffer.readUInt32LE(cursor), 0x02014b50, 'Invalid central directory header');
    const versionMadeBy = buffer.readUInt16LE(cursor + 4);
    const method = buffer.readUInt16LE(cursor + 10);
    const compressedSize = buffer.readUInt32LE(cursor + 20);
    const uncompressedSize = buffer.readUInt32LE(cursor + 24);
    const nameLen = buffer.readUInt16LE(cursor + 28);
    const extraLen = buffer.readUInt16LE(cursor + 30);
    const commentLen = buffer.readUInt16LE(cursor + 32);
    const externalAttr = buffer.readUInt32LE(cursor + 38);
    const localOffset = buffer.readUInt32LE(cursor + 42);
    const name = buffer.subarray(cursor + 46, cursor + 46 + nameLen).toString('utf8');

    const localNameLen = buffer.readUInt16LE(localOffset + 26);
    const localExtraLen = buffer.readUInt16LE(localOffset + 28);
    const dataStart = localOffset + 30 + localNameLen + localExtraLen;
    const raw = buffer.subarray(dataStart, dataStart + compressedSize);
    const data = method === 8 ? inflateRawSync(raw) : raw;
    assert.equal(data.length, uncompressedSize, `Uncompressed length mismatch for ${name}`);

    entries.push({
      name,
      creatorOs: versionMadeBy >> 8,
      posixMode: (externalAttr >>> 16) & 0xffff,
      data,
    });
    cursor += 46 + nameLen + extraLen + commentLen;
  }
  return entries;
}

test('packQoderPlugin places .qoder-plugin/plugin.json directly at ZIP root with POSIX launch-mcp mode', async (t) => {
  const temp = await mkdtemp(join(tmpdir(), 'op-qoder-zip-'));
  t.after(() => rm(temp, { recursive: true, force: true }));
  const outputPath = join(temp, 'openpencil-qoder-plugin.zip');

  const packed = await packQoderPlugin({ outputPath });
  assert.equal(packed.ok, true);
  assert.equal(packed.pluginName, 'openpencil');
  assert.ok(packed.fileCount >= 20);

  const bytes = await readFile(outputPath);
  const entries = readZipCentralDirectory(bytes);
  const byName = new Map(entries.map(entry => [entry.name, entry]));

  // Qoder archive contract: .qoder-plugin/plugin.json and .mcp.json directly at ZIP root.
  for (const required of [
    '.qoder-plugin/plugin.json',
    '.mcp.json',
    'CONNECTORS.md',
    'scripts/launch-mcp',
    'scripts/launch-mcp.cmd',
    'scripts/mcp-bridge.mjs',
    'skills/openpencil-design/SKILL.md',
    'skills/openpencil-setup/SKILL.md',
  ]) {
    assert.ok(byName.has(required), `ZIP missing required root entry ${required}`);
  }

  const manifest = JSON.parse(byName.get('.qoder-plugin/plugin.json').data.toString('utf8'));
  assert.equal(manifest.name, 'openpencil');
  const mcp = JSON.parse(byName.get('.mcp.json').data.toString('utf8'));
  assert.equal(mcp.mcpServers.openpencil.command, '${QODER_PLUGIN_ROOT}/scripts/launch-mcp');

  const launcher = byName.get('scripts/launch-mcp');
  assert.equal(launcher.creatorOs, 3, 'Expected Unix creator byte');
  assert.equal(launcher.posixMode, 0o100755, 'Expected POSIX 0755 executable bits on scripts/launch-mcp');

  if (process.platform !== 'win32') {
    const check = spawnSync('unzip', ['-t', outputPath], { encoding: 'utf8' });
    if (check.error?.code !== 'ENOENT') {
      assert.equal(check.status, 0, `unzip -t failed:\n${check.stderr || check.stdout}`);
    }
  }
});
