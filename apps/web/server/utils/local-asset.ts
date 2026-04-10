import { readFile, stat } from 'node:fs/promises';
import { extname, resolve } from 'node:path';

export const IMAGE_MIME_TYPES: Record<string, string> = {
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.jpeg': 'image/jpeg',
  '.gif': 'image/gif',
  '.webp': 'image/webp',
  '.bmp': 'image/bmp',
  '.svg': 'image/svg+xml',
  '.avif': 'image/avif',
};

const EXTENSIONLESS_IMAGE_CANDIDATES = Object.keys(IMAGE_MIME_TYPES);

export async function resolveServableLocalImagePath(path: string): Promise<{
  resolvedPath: string;
  mimeType: string;
} | null> {
  const resolvedPath = resolve(path);
  const extension = extname(resolvedPath).toLowerCase();

  if (extension) {
    const mimeType = IMAGE_MIME_TYPES[extension];
    if (!mimeType) return null;

    if (!(await isFile(resolvedPath))) return null;
    return { resolvedPath, mimeType };
  }

  if (await isFile(resolvedPath)) {
    const mimeType = await inferMimeTypeFromFile(resolvedPath);
    if (!mimeType) return null;
    return { resolvedPath, mimeType };
  }

  for (const candidateExt of EXTENSIONLESS_IMAGE_CANDIDATES) {
    const candidatePath = `${resolvedPath}${candidateExt}`;
    if (await isFile(candidatePath)) {
      return {
        resolvedPath: candidatePath,
        mimeType: IMAGE_MIME_TYPES[candidateExt],
      };
    }
  }

  return null;
}

async function isFile(path: string): Promise<boolean> {
  try {
    const fileStat = await stat(path);
    return fileStat.isFile();
  } catch {
    return false;
  }
}

async function inferMimeTypeFromFile(path: string): Promise<string | null> {
  const content = await readFile(path);
  return inferMimeTypeFromBuffer(content);
}

function inferMimeTypeFromBuffer(content: Buffer): string | null {
  if (
    content.length >= 8 &&
    content[0] === 0x89 &&
    content[1] === 0x50 &&
    content[2] === 0x4e &&
    content[3] === 0x47 &&
    content[4] === 0x0d &&
    content[5] === 0x0a &&
    content[6] === 0x1a &&
    content[7] === 0x0a
  ) {
    return 'image/png';
  }

  if (content.length >= 3 && content[0] === 0xff && content[1] === 0xd8 && content[2] === 0xff) {
    return 'image/jpeg';
  }

  if (content.length >= 6) {
    const header = content.subarray(0, 6).toString('ascii');
    if (header === 'GIF87a' || header === 'GIF89a') {
      return 'image/gif';
    }
  }

  if (
    content.length >= 12 &&
    content.subarray(0, 4).toString('ascii') === 'RIFF' &&
    content.subarray(8, 12).toString('ascii') === 'WEBP'
  ) {
    return 'image/webp';
  }

  if (content.length >= 2 && content[0] === 0x42 && content[1] === 0x4d) {
    return 'image/bmp';
  }

  if (
    content.length >= 12 &&
    content.subarray(4, 8).toString('ascii') === 'ftyp' &&
    /^avif|avis$/i.test(content.subarray(8, 12).toString('ascii'))
  ) {
    return 'image/avif';
  }

  const textProbe = content.subarray(0, 512).toString('utf8').trimStart();
  if (
    textProbe.startsWith('<svg') ||
    (textProbe.startsWith('<?xml') && textProbe.includes('<svg'))
  ) {
    return 'image/svg+xml';
  }

  return null;
}
