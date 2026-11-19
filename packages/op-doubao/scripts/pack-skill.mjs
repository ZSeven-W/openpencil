import { deflateRawSync } from 'node:zlib';
import { lstat, mkdir, readdir, readFile, stat, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { SKILL_NAMES } from './host-config.mjs';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const defaultPluginRoot = join(packageRoot, 'plugins', 'openpencil');
const MAX_SKILL_ZIP_TOTAL_SIZE = 32 * 1024 * 1024;
const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let i = 0; i < 256; i++) {
    let c = i;
    for (let k = 0; k < 8; k++) c = (c & 1) ? (0xedb88320 ^ (c >>> 1)) : (c >>> 1);
    table[i] = c >>> 0;
  }
  return table;
})();

function crc32(buffer) {
  let crc = 0xffffffff;
  for (let i = 0; i < buffer.length; i++) {
    crc = CRC_TABLE[(crc ^ buffer[i]) & 0xff] ^ (crc >>> 8);
  }
  return (crc ^ 0xffffffff) >>> 0;
}

async function collectFiles(root) {
  const output = [];
  async function walk(dir) {
    const entries = await readdir(dir, { withFileTypes: true });
    entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
    for (const entry of entries) {
      if (entry.name === '.DS_Store' || entry.name === 'Thumbs.db' || entry.name === '__MACOSX') continue;
      const abs = join(dir, entry.name);
      const info = await lstat(abs);
      if (info.isSymbolicLink()) throw new Error(`Skill archive must not contain symbolic links: ${abs}`);
      if (info.isDirectory()) await walk(abs);
      else if (info.isFile()) output.push({ abs, rel: relative(root, abs).split(sep).join('/'), mode: info.mode });
    }
  }
  await walk(root);
  return output;
}

/**
 * Package a Doubao Desktop uploadable Skill ZIP (SKILL.md + references/)
 * with zero external dependencies for 技能·连接器·伙伴 -> 我的技能 -> 新建 -> 上传技能.
 */
export async function packDoubaoSkill({
  pluginRoot = defaultPluginRoot,
  skillName = 'openpencil-design',
  flat = false,
  outputPath,
} = {}) {
  if (!SKILL_NAMES.includes(skillName)) {
    throw new Error(`Unknown skill ${JSON.stringify(skillName)}. Expected one of: ${SKILL_NAMES.join(', ')}`);
  }
  const skillDir = join(resolve(pluginRoot), 'skills', skillName);
  const skillMdPath = join(skillDir, 'SKILL.md');
  const skillMd = (await readFile(skillMdPath, 'utf8')).replace(/\r\n/g, '\n');
  if (!/^---\nname: [a-z0-9-]+\ndescription: .+\n---/u.test(skillMd)) {
    throw new Error(`${skillMdPath} must begin with valid YAML frontmatter (name + description).`);
  }
  for (const [, link] of skillMd.matchAll(/\]\(([^)]+)\)/gu)) {
    if (/^https?:/.test(link)) continue;
    const target = resolve(skillDir, link);
    if (!relative(skillDir, target).startsWith('..')) {
      if (!(await stat(target)).isFile()) throw new Error(`Missing skill reference: ${link}`);
    }
  }

  const absOutputPath = resolve(outputPath || join(packageRoot, 'dist', `${skillName}-doubao-skill.zip`));
  const files = await collectFiles(skillDir);
  const prefix = flat ? '' : skillName;

  const localChunks = [];
  const centralChunks = [];
  let offset = 0;
  let totalUncompressed = 0;

  for (const file of files) {
    const data = await readFile(file.abs);
    totalUncompressed += data.length;
    if (totalUncompressed > MAX_SKILL_ZIP_TOTAL_SIZE) {
      throw new Error('Skill archive exceeds 32 MiB size limit.');
    }
    const zipName = prefix ? `${prefix}/${file.rel}` : file.rel;
    const nameBytes = Buffer.from(zipName, 'utf8');
    const compressed = deflateRawSync(data, { level: 9 });
    const useDeflate = compressed.length < data.length;
    const payload = useDeflate ? compressed : data;
    const method = useDeflate ? 8 : 0;
    const checksum = crc32(data);
    const posixMode = 0o100000 | (file.mode & 0o777 || 0o644);

    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0);
    localHeader.writeUInt16LE(20, 4);
    localHeader.writeUInt16LE(0x0800, 6);
    localHeader.writeUInt16LE(method, 8);
    localHeader.writeUInt16LE(0, 10);
    localHeader.writeUInt16LE(0x5421, 12);
    localHeader.writeUInt32LE(checksum, 14);
    localHeader.writeUInt32LE(payload.length, 18);
    localHeader.writeUInt32LE(data.length, 22);
    localHeader.writeUInt16LE(nameBytes.length, 26);
    localHeader.writeUInt16LE(0, 28);
    localChunks.push(localHeader, nameBytes, payload);

    const centralHeader = Buffer.alloc(46);
    centralHeader.writeUInt32LE(0x02014b50, 0);
    centralHeader.writeUInt16LE((3 << 8) | 20, 4);
    centralHeader.writeUInt16LE(20, 6);
    centralHeader.writeUInt16LE(0x0800, 8);
    centralHeader.writeUInt16LE(method, 10);
    centralHeader.writeUInt16LE(0, 12);
    centralHeader.writeUInt16LE(0x5421, 14);
    centralHeader.writeUInt32LE(checksum, 16);
    centralHeader.writeUInt32LE(payload.length, 20);
    centralHeader.writeUInt32LE(data.length, 24);
    centralHeader.writeUInt16LE(nameBytes.length, 28);
    centralHeader.writeUInt16LE(0, 30);
    centralHeader.writeUInt16LE(0, 32);
    centralHeader.writeUInt16LE(0, 34);
    centralHeader.writeUInt16LE(0, 36);
    centralHeader.writeUInt32LE((posixMode << 16) >>> 0, 38);
    centralHeader.writeUInt32LE(offset, 42);
    centralChunks.push(centralHeader, nameBytes);

    offset += localHeader.length + nameBytes.length + payload.length;
  }

  const centralBuffer = Buffer.concat(centralChunks);
  const eocd = Buffer.alloc(22);
  eocd.writeUInt32LE(0x06054b50, 0);
  eocd.writeUInt16LE(0, 4);
  eocd.writeUInt16LE(0, 6);
  eocd.writeUInt16LE(files.length, 8);
  eocd.writeUInt16LE(files.length, 10);
  eocd.writeUInt32LE(centralBuffer.length, 12);
  eocd.writeUInt32LE(offset, 16);
  eocd.writeUInt16LE(0, 20);

  const archive = Buffer.concat([...localChunks, centralBuffer, eocd]);
  await mkdir(dirname(absOutputPath), { recursive: true });
  await writeFile(absOutputPath, archive);
  return {
    ok: true,
    outputPath: absOutputPath,
    skillName,
    flat,
    fileCount: files.length,
    bytes: archive.length,
  };
}

export async function runPackSkill(args = process.argv.slice(2)) {
  let skillName = 'openpencil-design';
  let outputPath;
  let flat = false;
  for (let i = 0; i < args.length; i++) {
    const arg = args[i];
    if ((arg === '--output' || arg === '-o') && args[i + 1]) outputPath = args[++i];
    else if (arg === '--skill' && args[i + 1]) skillName = args[++i];
    else if (arg === '--flat') flat = true;
    else throw new Error('Usage: doubao-openpencil pack-skill [--skill openpencil-design|openpencil-setup] [--flat] [--output /path/to/skill.zip]');
  }
  try {
    const result = await packDoubaoSkill({ skillName, flat, outputPath });
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(`Failed to pack Doubao skill ZIP: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runPackSkill(process.argv.slice(2));
}
