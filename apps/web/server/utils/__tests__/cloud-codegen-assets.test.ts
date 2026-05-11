import { describe, expect, it, vi } from 'vitest';
import {
  addSignedUrlsToCodegenManifest,
  persistCodeGenerationAssets,
} from '../cloud-codegen-assets';

function makeSupabaseMock() {
  const upload = vi.fn();
  const createSignedUrl = vi.fn();
  const storageFrom = vi.fn(() => ({ upload, createSignedUrl }));
  const insert = vi.fn();
  const tableFrom = vi.fn(() => ({ insert }));
  return {
    supabase: {
      storage: { from: storageFrom },
      from: tableFrom,
    },
    upload,
    createSignedUrl,
    storageFrom,
    insert,
    tableFrom,
  };
}

describe('cloud codegen assets', () => {
  it('uploads generation assets and records file asset metadata', async () => {
    const storage = makeSupabaseMock();
    storage.upload.mockResolvedValue({ data: { path: 'ok' }, error: null });
    storage.insert.mockResolvedValue({ data: null, error: null });

    const manifest = await persistCodeGenerationAssets({
      supabase: storage.supabase as never,
      userId: 'user-1',
      fileId: 'file-1',
      generationId: 'gen-1',
      assets: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          bytesBase64: 'AQID',
          sourceNodeId: 'node-1',
          sourceNodeName: 'Card',
          sourceKind: 'image-fill',
        },
      ],
    });

    expect(storage.storageFrom).toHaveBeenCalledWith('openpencil-assets');
    expect(storage.upload).toHaveBeenCalledWith(
      'user-1/file-1/code-generations/gen-1/assets/card.png',
      new Uint8Array([1, 2, 3]),
      { contentType: 'image/png', upsert: false },
    );
    expect(storage.tableFrom).toHaveBeenCalledWith('file_assets');
    expect(storage.insert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: 'file-1',
        owner_id: 'user-1',
        kind: 'codegen_asset',
        path: 'user-1/file-1/code-generations/gen-1/assets/card.png',
        size_bytes: 3,
        metadata: expect.objectContaining({
          generationId: 'gen-1',
          assetId: 'asset-1',
          relativePath: './assets/card.png',
        }),
      }),
    );
    expect(manifest).toEqual([
      expect.objectContaining({
        id: 'asset-1',
        storagePath: 'user-1/file-1/code-generations/gen-1/assets/card.png',
        sizeBytes: 3,
      }),
    ]);
  });

  it('adds signed URLs to persisted manifest entries', async () => {
    const storage = makeSupabaseMock();
    storage.createSignedUrl.mockResolvedValue({
      data: { signedUrl: 'https://example.test/signed.png' },
      error: null,
    });

    const manifest = await addSignedUrlsToCodegenManifest(storage.supabase as never, [
      {
        id: 'asset-1',
        relativePath: './assets/card.png',
        zipPath: 'assets/card.png',
        mimeType: 'image/png',
        sizeBytes: 3,
        storagePath: 'user-1/file-1/code-generations/gen-1/assets/card.png',
        sourceKind: 'image-fill',
      },
      {
        id: 'asset-2',
        relativePath: './assets/old.png',
        zipPath: 'assets/old.png',
        mimeType: 'image/png',
        sizeBytes: 3,
        sourceKind: 'image-fill',
      },
    ]);

    expect(storage.createSignedUrl).toHaveBeenCalledWith(
      'user-1/file-1/code-generations/gen-1/assets/card.png',
      3600,
    );
    expect(manifest[0]?.signedUrl).toBe('https://example.test/signed.png');
    expect(manifest[1]?.signedUrl).toBeUndefined();
  });
});
