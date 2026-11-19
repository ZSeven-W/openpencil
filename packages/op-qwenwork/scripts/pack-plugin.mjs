import { deflateRawSync } from 'node:zlib';
import { lstat, mkdir, readdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const defaultPluginRoot = join(packageRoot, 'plugins', 'openpencil');
const MAX_PLUGIN_ZIP_TOTAL_SIZE = 256 * 1024 * 1024;
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
      if (entry.name === '.DS_Store' || entry.name === 'Thumbs.db') continue;
      const abs = join(dir, entry.name);
      const info = await lstat(abs);
      if (info.isSymbolicLink()) throw new Error(`Plugin archive must not contain symbolic links: ${abs}`);
      if (info.isDirectory()) await walk(abs);
      else if (info.isFile()) output.push({ abs, rel: relative(root, abs).split(sep).join('/'), mode: info.mode });
    }
  }
  await walk(root);
  return output;
}

/**
 * Build a standard ZIP 2.0 archive directly from plugins/openpencil/ with zero
 * external dependencies, preserving POSIX executable bits on scripts/launch-mcp
 * and matching QwenWorkCN's plugin ZIP upload contract.
 */
export async function packQwenWorkPlugin({
  pluginRoot = defaultPluginRoot,
  outputPath = join(packageRoot, 'dist', 'openpencil-qwenwork-plugin.zip'),
  rootPrefix = 'openpencil',
} = {}) {
  const absPluginRoot = resolve(pluginRoot);
  const absOutputPath = resolve(outputPath);
  const files = await collectFiles(absPluginRoot);
  const manifestEntry = files.find(file => file.rel === '.qwenwork-plugin/plugin.json');
  if (!manifestEntry) {
    throw new Error('Plugin root is missing .qwenwork-plugin/plugin.json.');
  }
  const manifest = JSON.parse(await readFile(manifestEntry.abs, 'utf8'));
  if (typeof manifest.name !== 'string' || !manifest.name.trim()) {
    throw new Error('.qwenwork-plugin/plugin.json must contain a non-empty name field.');
  }

  const localChunks = [];
  const centralChunks = [];
  let offset = 0;
  let totalUncompressed = 0;

  for (const file of files) {
    const data = await readFile(file.abs);
    totalUncompressed += data.length;
    if (totalUncompressed > MAX_PLUGIN_ZIP_TOTAL_SIZE) {
      throw new Error('Plugin archive exceeds QwenWork’s 256 MiB total size limit.');
    }
    const zipName = rootPrefix ? `${rootPrefix}/${file.rel}` : file.rel;
    const nameBytes = Buffer.from(zipName, 'utf8');
    const compressed = deflateRawSync(data, { level: 9 });
    const useDeflate = compressed.length < data.length;
    const payload = useDeflate ? compressed : data;
    const method = useDeflate ? 8 : 0;
    const checksum = crc32(data);
    const posixMode = file.rel === 'scripts/launch-mcp' ? 0o100755 : (0o100000 | (file.mode & 0o777 || 0o644));

    const localHeader = Buffer.alloc(30);
    localHeader.writeUInt32LE(0x04034b50, 0);
    localHeader.writeUInt16LE(20, 4);
    localHeader.writeUInt16LE(0x0800, 6); // UTF-8 filename flag
    localHeader.writeUInt16LE(method, 8);
    localHeader.writeUInt16LE(0, 10);
    localHeader.writeUInt16LE(0x5421, 12); // Deterministic DOS date (2022-01-01)
    localHeader.writeUInt32LE(checksum, 14);
    localHeader.writeUInt32LE(payload.length, 18);
    localHeader.writeUInt32LE(data.length, 22);
    localHeader.writeUInt16LE(nameBytes.length, 26);
    localHeader.writeUInt16LE(0, 28);
    localChunks.push(localHeader, nameBytes, payload);

    const centralHeader = Buffer.alloc(46);
    centralHeader.writeUInt32LE(0x02014b50, 0);
    centralHeader.writeUInt16LE((3 << 8) | 20, 4); // Unix creator, ZIP 2.0
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
    pluginName: manifest.name,
    version: manifest.version,
    fileCount: files.length,
    bytes: archive.length,
  };
}

export async function runPackPlugin(args = process.argv.slice(2)) {
  let outputPath;
  for (let i = 0; i < args.length; i++) {
    if ((args[i] === '--output' || args[i] === '-o') && args[i + 1]) outputPath = args[++i];
    else throw new Error('Usage: qwenwork-openpencil pack-plugin [--output /path/to/openpencil-qwenwork-plugin.zip]');
  }
  try {
    const result = await packQwenWorkPlugin(outputPath ? { outputPath } : {});
    console.log(JSON.stringify(result, null, 2));
  } catch (error) {
    console.error(`Failed to pack QwenWork plugin ZIP: ${error.message}`);
    process.exitCode = 1;
  }
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  await runPackPlugin(process.argv.slice(2));
}
