import { describe, expect, it, vi } from 'vitest';
import {
  createCodeGenerationRecord,
  deleteCodeGenerationRecord,
  promoteCodeGenerationRecord,
} from '../cloud-code-generation-records';

const generationRow = {
  id: 'gen-1',
  file_id: 'file-1',
  page_id: 'page-1',
  framework: 'uniapp',
  target_kind: 'selection',
  node_ids: ['node-1'],
  target_hash: 'hash-1',
  document_revision: 3,
  status: 'done',
  final_code: '---FILE: pages/index/index.vue---\n<template><view /></template>',
  entry_file: 'pages/index/index.vue',
  degraded: false,
  assets_manifest: [],
  model: 'model-a',
  provider: 'builtin',
  error: null,
  created_at: '2026-05-11T08:00:00.000Z',
  completed_at: '2026-05-11T08:00:01.000Z',
  promoted_at: null,
};

function makeOrderedSelectMock(data: unknown[]) {
  const order = vi.fn(async () => ({ data, error: null }));
  const eq = vi.fn(() => ({ order }));
  const select = vi.fn(() => ({ eq }));
  return { select, eq, order };
}

function makeGenerationLoadQuery(row: unknown) {
  const single = vi.fn(async () => ({ data: row, error: null }));
  const ownerEq = vi.fn(() => ({ single }));
  const idEq = vi.fn(() => ({ eq: ownerEq }));
  const select = vi.fn(() => ({ eq: idEq }));
  return { select, idEq, ownerEq, single };
}

function makeActivityInsertMock() {
  return vi.fn(async () => ({ data: null, error: null }));
}

describe('createCodeGenerationRecord', () => {
  it('persists generated files and returns them in detail payload', async () => {
    const generationSingle = vi.fn(async () => ({ data: generationRow, error: null }));
    const generationSelect = vi.fn(() => ({ single: generationSingle }));
    const generationInsert = vi.fn(() => ({ select: generationSelect }));
    const chunkQuery = makeOrderedSelectMock([]);
    const fileRows = [
      {
        id: 'file-row-1',
        generation_id: 'gen-1',
        path: 'pages/index/index.vue',
        language: 'vue',
        role: 'page',
        content: '<template><view /></template>',
        size_bytes: new TextEncoder().encode('<template><view /></template>').byteLength,
        order_index: 0,
        created_at: '2026-05-11T08:00:01.000Z',
      },
    ];
    const fileQuery = makeOrderedSelectMock(fileRows);
    const filesInsert = vi.fn(async () => ({ data: null, error: null }));
    const chunksInsert = vi.fn(async () => ({ data: null, error: null }));
    const activityInsert = makeActivityInsertMock();
    const from = vi.fn((table: string) => {
      if (table === 'code_generations') {
        return { insert: generationInsert };
      }
      if (table === 'code_generation_files') {
        return { insert: filesInsert, select: fileQuery.select };
      }
      if (table === 'code_generation_chunks') {
        return { insert: chunksInsert, select: chunkQuery.select };
      }
      if (table === 'activity_events') {
        return { insert: activityInsert };
      }
      throw new Error(`Unexpected table ${table}`);
    });

    const detail = await createCodeGenerationRecord({
      supabase: { from } as never,
      userId: 'user-1',
      fileId: 'file-1',
      pageId: 'page-1',
      framework: 'uniapp',
      targetKind: 'selection',
      nodeIds: ['node-1'],
      targetHash: 'hash-1',
      documentRevision: 3,
      status: 'done',
      finalCode: generationRow.final_code,
      entryFile: 'pages/index/index.vue',
      degraded: false,
      assets: [],
      files: [
        {
          path: 'pages/index/index.vue',
          language: 'vue',
          role: 'page',
          content: '<template><view /></template>',
          orderIndex: 0,
        },
      ],
      model: 'model-a',
      provider: 'builtin',
      chunks: [],
    });

    expect(generationInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        entry_file: 'pages/index/index.vue',
        final_code: generationRow.final_code,
        framework: 'uniapp',
      }),
    );
    expect(filesInsert).toHaveBeenCalledWith([
      expect.objectContaining({
        generation_id: 'gen-1',
        path: 'pages/index/index.vue',
        language: 'vue',
        role: 'page',
        size_bytes: new TextEncoder().encode('<template><view /></template>').byteLength,
        order_index: 0,
      }),
    ]);
    expect(detail.files).toEqual([
      expect.objectContaining({
        path: 'pages/index/index.vue',
        language: 'vue',
        role: 'page',
      }),
    ]);
    expect(detail.entryFile).toBe('pages/index/index.vue');
    expect(activityInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        owner_id: 'user-1',
        actor_id: 'user-1',
        file_id: 'file-1',
        generation_id: 'gen-1',
        type: 'codegen_created',
      }),
    );
  });
});

describe('promoteCodeGenerationRecord', () => {
  it('marks the target generation as promoted and clears sibling promotions', async () => {
    const record = { ...generationRow, promoted_at: null };
    const loadQuery = makeGenerationLoadQuery(record);
    const clearResult = vi.fn(async () => ({ data: null, error: null }));
    const clearNeq = vi.fn(() => clearResult());
    const clearTargetHashEq = vi.fn(() => ({ neq: clearNeq }));
    const clearTargetKindEq = vi.fn(() => ({ eq: clearTargetHashEq }));
    const clearFrameworkEq = vi.fn(() => ({ eq: clearTargetKindEq }));
    const clearPageEq = vi.fn(() => ({ eq: clearFrameworkEq }));
    const clearFileEq = vi.fn(() => ({ eq: clearPageEq }));
    const clearOwnerEq = vi.fn(() => ({ eq: clearFileEq }));
    const promoteSingle = vi.fn(async () => ({
      data: { ...record, promoted_at: '2026-05-11T09:00:00.000Z' },
      error: null,
    }));
    const promoteSelect = vi.fn(() => ({ single: promoteSingle }));
    const promoteOwnerEq = vi.fn(() => ({ select: promoteSelect }));
    const promoteIdEq = vi.fn(() => ({ eq: promoteOwnerEq }));
    const promoteUpdate = vi
      .fn()
      .mockImplementationOnce(() => ({ eq: clearOwnerEq }))
      .mockImplementationOnce(() => ({ eq: promoteIdEq }));
    const activityInsert = makeActivityInsertMock();
    const from = vi.fn((table: string) => {
      if (table === 'code_generations') {
        return {
          select: loadQuery.select,
          update: promoteUpdate,
        };
      }
      if (table === 'activity_events') {
        return { insert: activityInsert };
      }
      throw new Error(`Unexpected table ${table}`);
    });

    const promoted = await promoteCodeGenerationRecord({
      supabase: { from } as never,
      userId: 'user-1',
      generationId: 'gen-1',
    });

    expect(promoted.promotedAt).toBe('2026-05-11T09:00:00.000Z');
    expect(promoteUpdate).toHaveBeenNthCalledWith(1, { promoted_at: null });
    expect(clearOwnerEq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(clearFileEq).toHaveBeenCalledWith('file_id', 'file-1');
    expect(clearPageEq).toHaveBeenCalledWith('page_id', 'page-1');
    expect(clearFrameworkEq).toHaveBeenCalledWith('framework', 'uniapp');
    expect(clearTargetKindEq).toHaveBeenCalledWith('target_kind', 'selection');
    expect(clearTargetHashEq).toHaveBeenCalledWith('target_hash', 'hash-1');
    expect(clearNeq).toHaveBeenCalledWith('id', 'gen-1');
    expect(clearResult).toHaveBeenCalled();
    expect(promoteUpdate).toHaveBeenCalledWith({ promoted_at: expect.any(String) });
    expect(promoteIdEq).toHaveBeenCalledWith('id', 'gen-1');
    expect(promoteOwnerEq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(promoteSelect).toHaveBeenCalled();
    expect(activityInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: 'file-1',
        generation_id: 'gen-1',
        type: 'codegen_promoted',
      }),
    );
  });
});

describe('deleteCodeGenerationRecord', () => {
  it('deletes the generation and removes codegen assets', async () => {
    const generationWithAsset = {
      ...generationRow,
      assets_manifest: [
        {
          id: 'asset-1',
          relativePath: './assets/card.png',
          zipPath: 'assets/card.png',
          mimeType: 'image/png',
          sizeBytes: 3,
          storagePath: 'user-1/file-1/code-generations/gen-1/assets/card.png',
          sourceNodeId: 'node-1',
          sourceNodeName: 'Card',
          sourceKind: 'image-fill',
        },
      ],
    };
    const loadQuery = makeGenerationLoadQuery(generationWithAsset);
    const generationDeleteResult = vi.fn(async () => ({ data: null, error: null }));
    const generationDeleteOwnerEq = vi.fn(() => generationDeleteResult());
    const generationDeleteIdEq = vi.fn(() => ({ eq: generationDeleteOwnerEq }));
    const generationDelete = vi.fn(() => ({ eq: generationDeleteIdEq }));
    const assetDeleteResult = vi.fn(async () => ({ data: null, error: null }));
    const assetDeleteIn = vi.fn(() => assetDeleteResult());
    const assetDeleteKindEq = vi.fn(() => ({ in: assetDeleteIn }));
    const assetDeleteOwnerEq = vi.fn(() => ({ eq: assetDeleteKindEq }));
    const assetDeleteFileEq = vi.fn(() => ({ eq: assetDeleteOwnerEq }));
    const assetDelete = vi.fn(() => ({ eq: assetDeleteFileEq }));
    const remove = vi.fn(async () => ({ data: null, error: null }));
    const activityInsert = makeActivityInsertMock();
    const from = vi.fn((table: string) => {
      if (table === 'code_generations') {
        return { select: loadQuery.select, delete: generationDelete };
      }
      if (table === 'file_assets') {
        return { delete: assetDelete };
      }
      if (table === 'activity_events') {
        return { insert: activityInsert };
      }
      throw new Error(`Unexpected table ${table}`);
    });

    await deleteCodeGenerationRecord({
      supabase: {
        from,
        storage: { from: vi.fn(() => ({ remove })) },
      } as never,
      userId: 'user-1',
      generationId: 'gen-1',
    });

    expect(remove).toHaveBeenCalledWith(['user-1/file-1/code-generations/gen-1/assets/card.png']);
    expect(generationDelete).toHaveBeenCalled();
    expect(generationDeleteIdEq).toHaveBeenCalledWith('id', 'gen-1');
    expect(generationDeleteOwnerEq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(assetDeleteFileEq).toHaveBeenCalledWith('file_id', 'file-1');
    expect(assetDeleteOwnerEq).toHaveBeenCalledWith('owner_id', 'user-1');
    expect(assetDeleteKindEq).toHaveBeenCalledWith('kind', 'codegen_asset');
    expect(assetDeleteIn).toHaveBeenCalledWith('path', [
      'user-1/file-1/code-generations/gen-1/assets/card.png',
    ]);
    expect(assetDeleteResult).toHaveBeenCalled();
    expect(activityInsert).toHaveBeenCalledWith(
      expect.objectContaining({
        file_id: 'file-1',
        generation_id: 'gen-1',
        type: 'codegen_deleted',
      }),
    );
  });
});
