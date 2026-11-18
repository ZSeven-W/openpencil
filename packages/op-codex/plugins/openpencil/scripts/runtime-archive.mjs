import { gunzipSync, gzipSync } from 'node:zlib';
import { mkdir, writeFile, lstat } from 'node:fs/promises';
import { dirname, join } from 'node:path';

const BLOCK = 512;
const MAX_BYTES = 768 * 1024 * 1024;
const string = (header, start, length) => header.subarray(start, start + length).toString('utf8').replace(/\0.*$/s, '');
function safeName(name) {
  if (!name || name.includes('\\') || name.includes(':') || name.startsWith('/') ||
      name.split('/').some(part => !part || part === '.' || part === '..')) {
    throw new Error('Unsafe runtime archive path.');
  }
  return name;
}

// Our releases use a deliberately small USTAR subset: regular files only.
// Reject links, extensions, duplicate entries and traversal before writing anything.
export function unpackArchive(compressed) {
  const tar = gunzipSync(compressed, { maxOutputLength: MAX_BYTES });
  const files = new Map();
  let offset = 0;
  while (offset + BLOCK <= tar.length) {
    const h = tar.subarray(offset, offset + BLOCK);
    if (h.every(byte => byte === 0)) {
      if (tar.subarray(offset).some(byte => byte !== 0)) throw new Error('Invalid runtime archive trailer.');
      return files;
    }
    const checksum = [...h].reduce((sum, byte, i) => sum + (i >= 148 && i < 156 ? 32 : byte), 0);
    if (parseInt(string(h, 148, 8).trim(), 8) !== checksum || string(h, 257, 6) !== 'ustar') {
      throw new Error('Invalid runtime archive header.');
    }
    if (h[156] !== 48 && h[156] !== 0) throw new Error('Runtime archive must contain regular files only.');
    const prefix = string(h, 345, 155);
    const name = safeName((prefix ? prefix + '/' : '') + string(h, 0, 100));
    const sizeText = string(h, 124, 12).trim();
    if (!/^[0-7]+$/.test(sizeText)) throw new Error('Invalid runtime archive size.');
    const size = parseInt(sizeText, 8);
    const start = offset + BLOCK;
    if (size > MAX_BYTES || start + size > tar.length || files.has(name) || files.size >= 20_000) {
      throw new Error('Invalid runtime archive entry.');
    }
    files.set(name, tar.subarray(start, start + size));
    offset = start + Math.ceil(size / BLOCK) * BLOCK;
  }
  throw new Error('Truncated runtime archive.');
}

export async function extractArchive(compressed, destination) {
  const files = unpackArchive(compressed);
  if (!(await lstat(destination)).isDirectory()) throw new Error('Expected a fresh runtime staging directory.');
  for (const [name, bytes] of files) {
    const path = join(destination, ...name.split('/'));
    await mkdir(dirname(path), { recursive: true, mode: 0o700 });
    await writeFile(path, bytes, { flag: 'wx', mode: name.startsWith('bin/op-host-web-server') ? 0o755 : 0o644 });
  }
  return [...files.keys()];
}

export function packArchive(files) {
  const chunks = [];
  const field = (header, offset, length, value) => {
    if (Buffer.byteLength(value) > length) throw new Error('Runtime archive field exceeds USTAR limit.');
    header.write(value, offset, length, 'utf8');
  };
  for (const [name, bytes] of files) {
    safeName(name);
    const header = Buffer.alloc(BLOCK);
    let leaf = name, prefix = '';
    if (Buffer.byteLength(name) > 100) {
      const slash = name.lastIndexOf('/');
      prefix = name.slice(0, slash); leaf = name.slice(slash + 1);
    }
    field(header, 0, 100, leaf); field(header, 345, 155, prefix);
    field(header, 100, 8, (name.startsWith('bin/op-host-web-server') ? '0000755' : '0000644') + '\0');
    for (const offset of [108, 116]) field(header, offset, 8, '0000000\0');
    field(header, 124, 12, bytes.length.toString(8).padStart(11, '0') + '\0');
    field(header, 136, 12, '00000000000\0');
    header.fill(32, 148, 156); header[156] = 48;
    field(header, 257, 6, 'ustar\0'); field(header, 263, 2, '00');
    field(header, 148, 8, [...header].reduce((a, b) => a + b, 0).toString(8).padStart(6, '0') + '\0 ');
    chunks.push(header, bytes, Buffer.alloc((BLOCK - bytes.length % BLOCK) % BLOCK));
  }
  chunks.push(Buffer.alloc(BLOCK * 2));
  return gzipSync(Buffer.concat(chunks), { level: 9 });
}
