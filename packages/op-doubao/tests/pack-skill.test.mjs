import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { inflateRawSync } from 'node:zlib';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { packDoubaoSkill } from '../scripts/pack-skill.mjs';

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
    const method = buffer.readUInt16LE(cursor + 10);
    const compressedSize = buffer.readUInt32LE(cursor + 20);
    const uncompressedSize = buffer.readUInt32LE(cursor + 24);
    const nameLen = buffer.readUInt16LE(cursor + 28);
    const extraLen = buffer.readUInt16LE(cursor + 30);
    const commentLen = buffer.readUInt16LE(cursor + 32);
    const localOffset = buffer.readUInt32LE(cursor + 42);
    const name = buffer.subarray(cursor + 46, cursor + 46 + nameLen).toString('utf8');

    const localNameLen = buffer.readUInt16LE(localOffset + 26);
    const localExtraLen = buffer.readUInt16LE(localOffset + 28);
    const dataStart = localOffset + 30 + localNameLen + localExtraLen;
    const raw = buffer.subarray(dataStart, dataStart + compressedSize);
    const data = method === 8 ? inflateRawSync(raw) : raw;
    assert.equal(data.length, uncompressedSize, `Uncompressed length mismatch for ${name}`);

    entries.push({ name, data });
    cursor += 46 + nameLen + extraLen + commentLen;
  }
  return entries;
}

test('packDoubaoSkill packages folder-wrapped and flat uploadable Skill ZIPs cleanly', async (t) => {
  const temp = await mkdtemp(join(tmpdir(), 'op-doubao-skill-zip-'));
  t.after(() => rm(temp, { recursive: true, force: true }));

  const wrappedPath = join(temp, 'openpencil-design-doubao-skill.zip');
  const wrapped = await packDoubaoSkill({ outputPath: wrappedPath });
  assert.equal(wrapped.ok, true);
  assert.equal(wrapped.skillName, 'openpencil-design');
  assert.equal(wrapped.flat, false);
  assert.equal(wrapped.fileCount, 4);

  const wrappedEntries = readZipCentralDirectory(await readFile(wrappedPath));
  const wrappedNames = wrappedEntries.map(entry => entry.name);
  assert.ok(wrappedNames.includes('openpencil-design/SKILL.md'));
  assert.ok(wrappedNames.includes('openpencil-design/references/node-schema.md'));
  assert.ok(wrappedNames.includes('openpencil-design/references/images.md'));
  assert.ok(wrappedNames.includes('openpencil-design/references/codegen.md'));

  const flatPath = join(temp, 'openpencil-design-flat.zip');
  const flat = await packDoubaoSkill({ flat: true, outputPath: flatPath });
  assert.equal(flat.flat, true);
  const flatEntries = readZipCentralDirectory(await readFile(flatPath));
  const flatNames = flatEntries.map(entry => entry.name);
  assert.ok(flatNames.includes('SKILL.md'));
  assert.ok(flatNames.includes('references/node-schema.md'));

  if (process.platform !== 'win32') {
    for (const zipFile of [wrappedPath, flatPath]) {
      const check = spawnSync('unzip', ['-t', zipFile], { encoding: 'utf8' });
      if (check.error?.code !== 'ENOENT') {
        assert.equal(check.status, 0, `unzip -t failed for ${zipFile}:\n${check.stderr || check.stdout}`);
      }
    }
  }

  await assert.rejects(() => packDoubaoSkill({ skillName: 'unknown-skill', outputPath: join(temp, 'bad.zip') }), /Unknown skill/);
});
