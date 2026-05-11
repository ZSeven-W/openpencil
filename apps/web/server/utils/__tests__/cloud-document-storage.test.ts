import { describe, expect, it, vi } from 'vitest';
import type { PenDocument } from '../../../src/types/pen';
import {
  CLOUD_DOCUMENT_REF_MARKER,
  isCloudDocumentRef,
  prepareCloudDocumentForStorage,
  resolveCloudDocumentFromStorage,
} from '../cloud-document-storage';

function makeDocument(content: string): PenDocument {
  return {
    version: '1.0.0',
    children: [{ id: 'node-1', type: 'text', name: content, content }],
  } as PenDocument;
}

function makeSupabaseMock() {
  const upload = vi.fn();
  const download = vi.fn();
  const from = vi.fn(() => ({ upload, download }));
  return {
    supabase: { storage: { from } },
    upload,
    download,
    from,
  };
}

describe('cloud document storage', () => {
  it('keeps small documents inline', async () => {
    const storage = makeSupabaseMock();
    const document = makeDocument('small');

    const prepared = await prepareCloudDocumentForStorage({
      supabase: storage.supabase as never,
      userId: 'user-1',
      fileId: 'file-1',
      revision: 2,
      document,
      inlineMaxBytes: 2048,
    });

    expect(prepared.storedDocument).toBe(document);
    expect(prepared.sizeBytes).toBeGreaterThan(0);
    expect(prepared.isExternal).toBe(false);
    expect(storage.upload).not.toHaveBeenCalled();
  });

  it('stores large documents in private storage and returns a JSONB-safe reference', async () => {
    const storage = makeSupabaseMock();
    storage.upload.mockResolvedValue({ data: { path: 'ok' }, error: null });
    const document = makeDocument('x'.repeat(128));

    const prepared = await prepareCloudDocumentForStorage({
      supabase: storage.supabase as never,
      userId: 'user-1',
      fileId: 'file-1',
      revision: 3,
      document,
      inlineMaxBytes: 64,
      attemptId: 'attempt-1',
    });

    expect(prepared.isExternal).toBe(true);
    expect(prepared.storedDocument).toEqual({
      [CLOUD_DOCUMENT_REF_MARKER]: 1,
      bucket: 'openpencil-assets',
      path: 'user-1/file-1/documents/rev-3-attempt-1.json',
      sizeBytes: prepared.sizeBytes,
      contentType: 'application/json',
    });
    expect(storage.from).toHaveBeenCalledWith('openpencil-assets');
    expect(storage.upload).toHaveBeenCalledWith(
      'user-1/file-1/documents/rev-3-attempt-1.json',
      expect.any(String),
      { contentType: 'application/json', upsert: false },
    );
  });

  it('uses a unique object path for each large-document save attempt', async () => {
    const storage = makeSupabaseMock();
    storage.upload.mockResolvedValue({ data: { path: 'ok' }, error: null });
    const document = makeDocument('x'.repeat(128));

    const first = await prepareCloudDocumentForStorage({
      supabase: storage.supabase as never,
      userId: 'user-1',
      fileId: 'file-1',
      revision: 3,
      document,
      inlineMaxBytes: 64,
      attemptId: 'attempt-1',
    });
    const second = await prepareCloudDocumentForStorage({
      supabase: storage.supabase as never,
      userId: 'user-1',
      fileId: 'file-1',
      revision: 3,
      document,
      inlineMaxBytes: 64,
      attemptId: 'attempt-2',
    });

    expect(isCloudDocumentRef(first.storedDocument)).toBe(true);
    expect(isCloudDocumentRef(second.storedDocument)).toBe(true);
    expect((first.storedDocument as { path: string }).path).not.toBe(
      (second.storedDocument as { path: string }).path,
    );
  });

  it('resolves stored document references before returning files to clients', async () => {
    const storage = makeSupabaseMock();
    const document = makeDocument('restored');
    storage.download.mockResolvedValue({
      data: new Blob([JSON.stringify(document)], { type: 'application/json' }),
      error: null,
    });

    const resolved = await resolveCloudDocumentFromStorage(storage.supabase as never, {
      [CLOUD_DOCUMENT_REF_MARKER]: 1,
      bucket: 'openpencil-assets',
      path: 'user-1/file-1/documents/rev-4.json',
      sizeBytes: 123,
      contentType: 'application/json',
    });

    expect(resolved).toEqual(document);
    expect(storage.from).toHaveBeenCalledWith('openpencil-assets');
    expect(storage.download).toHaveBeenCalledWith('user-1/file-1/documents/rev-4.json');
  });

  it('recognizes only complete document reference objects', () => {
    expect(
      isCloudDocumentRef({
        [CLOUD_DOCUMENT_REF_MARKER]: 1,
        bucket: 'openpencil-assets',
        path: 'user-1/file-1/documents/rev-5.json',
        sizeBytes: 123,
      }),
    ).toBe(true);
    expect(isCloudDocumentRef({ [CLOUD_DOCUMENT_REF_MARKER]: 1 })).toBe(false);
    expect(isCloudDocumentRef(makeDocument('inline'))).toBe(false);
  });
});
