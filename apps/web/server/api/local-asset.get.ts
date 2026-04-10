import {
  createError,
  defineEventHandler,
  getQuery,
  getRequestHeader,
  setResponseHeaders,
} from 'h3';
import { readFile } from 'node:fs/promises';

import { resolveServableLocalImagePath } from '../utils/local-asset';

export default defineEventHandler(async (event) => {
  const { path } = getQuery(event) as { path?: string };
  const secFetchSite = getRequestHeader(event, 'sec-fetch-site');

  if (secFetchSite === 'cross-site') {
    throw createError({
      statusCode: 403,
      message: 'Cross-site local asset requests are blocked',
    });
  }

  if (!path?.trim()) {
    throw createError({
      statusCode: 400,
      message: 'Missing required query parameter: path',
    });
  }

  if (path.includes('\0') || !isAbsoluteLocalPath(path)) {
    throw createError({
      statusCode: 400,
      message: 'Only absolute local image paths are supported',
    });
  }

  const resolvedAsset = await resolveServableLocalImagePath(path);
  if (!resolvedAsset) {
    throw createError({
      statusCode: 404,
      message: 'Image file not found or unsupported',
    });
  }

  const content = await readFile(resolvedAsset.resolvedPath);
  setResponseHeaders(event, {
    'Content-Type': resolvedAsset.mimeType,
    'Cache-Control': 'no-cache',
    'Cross-Origin-Resource-Policy': 'same-origin',
    'X-Content-Type-Options': 'nosniff',
  });

  return content;
});

function isAbsoluteLocalPath(value: string): boolean {
  return /^[A-Za-z]:[\\/]/.test(value) || value.startsWith('\\\\') || value.startsWith('/');
}
