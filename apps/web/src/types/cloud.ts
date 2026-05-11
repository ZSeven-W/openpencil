import type { Framework } from '@zseven-w/pen-types';
import type { PenDocument } from '@/types/pen';

export type CloudSaveState = 'idle' | 'saving' | 'saved' | 'conflict' | 'error';
export type CloudVersionSource =
  | 'manual_save'
  | 'autosave'
  | 'import'
  | 'code_generation'
  | 'restore';

export interface CloudFileSummary {
  id: string;
  name: string;
  thumbnailPath: string | null;
  revision: number;
  createdAt: string;
  updatedAt: string;
}

export interface CloudFileRecord extends CloudFileSummary {
  document: PenDocument;
}

export interface CloudFileVersion {
  id: string;
  fileId: string;
  revision: number;
  source: CloudVersionSource;
  label: string | null;
  createdAt: string;
}

export interface CloudSaveConflict {
  code: 'revision_conflict';
  serverRevision: number;
}

export type CodegenTargetKind = 'page' | 'selection';

export interface CodegenTarget {
  pageId: string;
  targetKind: CodegenTargetKind;
  nodeIds: string[];
  targetHash: string;
}

export interface CodegenAssetManifestEntry {
  id: string;
  relativePath: string;
  zipPath: string;
  mimeType: string;
  sizeBytes: number;
  storagePath?: string;
  signedUrl?: string;
  sourceNodeId?: string;
  sourceNodeName?: string;
  sourceKind: string;
}

export interface SaveCodegenAssetInput {
  id: string;
  relativePath: string;
  zipPath: string;
  mimeType: string;
  bytesBase64: string;
  sourceNodeId: string;
  sourceNodeName?: string;
  sourceKind: 'image-node' | 'image-fill';
}

export interface CloudCodegenChunk {
  chunkId: string;
  name: string;
  status: string;
  code?: string;
  contract?: unknown;
  error?: string;
  orderIndex?: number;
}

export interface SaveCodeGenerationInput extends CodegenTarget {
  fileId: string;
  framework: Framework;
  documentRevision: number;
  status: 'done' | 'degraded' | 'failed';
  finalCode?: string;
  degraded: boolean;
  assets: SaveCodegenAssetInput[];
  model?: string;
  provider?: string;
  error?: string;
  chunks: CloudCodegenChunk[];
}

export interface CloudCodeGeneration extends CodegenTarget {
  id: string;
  fileId: string;
  framework: Framework;
  documentRevision: number;
  status: 'done' | 'degraded' | 'failed';
  finalCode: string | null;
  degraded: boolean;
  assetsManifest: CodegenAssetManifestEntry[];
  model: string | null;
  provider: string | null;
  error: string | null;
  createdAt: string;
  completedAt: string | null;
}

export interface CloudCodeGenerationDetail extends CloudCodeGeneration {
  chunks: CloudCodegenChunk[];
}
