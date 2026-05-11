import type {
  CloudCodeGeneration,
  CloudCodeGenerationDetail,
  CloudFileRecord,
  CloudFileSummary,
  CloudFileVersion,
  CodegenAssetManifestEntry,
  CloudCodegenChunk,
} from '../../src/types/cloud';
import type { PenDocument } from '../../src/types/pen';
import type { Framework } from '@zseven-w/pen-types';

interface DesignFileSummaryRow {
  id: string;
  name: string;
  thumbnail_path: string | null;
  revision: number;
  created_at: string;
  updated_at: string;
}

interface DesignFileRow extends DesignFileSummaryRow {
  document: PenDocument;
}

interface VersionRow {
  id: string;
  file_id: string;
  revision: number;
  source: CloudFileVersion['source'];
  label: string | null;
  created_at: string;
}

interface CodeGenerationRow {
  id: string;
  file_id: string;
  page_id: string;
  framework: Framework;
  target_kind: 'page' | 'selection';
  node_ids: string[];
  target_hash: string;
  document_revision: number;
  status: 'done' | 'degraded' | 'failed';
  final_code: string | null;
  degraded: boolean;
  assets_manifest: CodegenAssetManifestEntry[];
  model: string | null;
  provider: string | null;
  error: string | null;
  created_at: string;
  completed_at: string | null;
}

interface CodeGenerationChunkRow {
  chunk_id: string;
  name: string;
  status: string;
  code: string | null;
  contract: unknown;
  error: string | null;
  order_index: number;
}


export function mapFileSummary(row: DesignFileSummaryRow): CloudFileSummary {
  return {
    id: row.id,
    name: row.name,
    thumbnailPath: row.thumbnail_path,
    revision: Number(row.revision),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function mapFileRecord(row: DesignFileRow): CloudFileRecord {
  return {
    ...mapFileSummary(row),
    document: row.document,
  };
}

export function mapFileVersion(row: VersionRow): CloudFileVersion {
  return {
    id: row.id,
    fileId: row.file_id,
    revision: Number(row.revision),
    source: row.source,
    label: row.label,
    createdAt: row.created_at,
  };
}

export function mapCodeGeneration(row: CodeGenerationRow): CloudCodeGeneration {
  return {
    id: row.id,
    fileId: row.file_id,
    pageId: row.page_id,
    framework: row.framework,
    targetKind: row.target_kind,
    nodeIds: row.node_ids ?? [],
    targetHash: row.target_hash,
    documentRevision: Number(row.document_revision),
    status: row.status,
    finalCode: row.final_code,
    degraded: row.degraded,
    assetsManifest: row.assets_manifest ?? [],
    model: row.model,
    provider: row.provider,
    error: row.error,
    createdAt: row.created_at,
    completedAt: row.completed_at,
  };
}

export function mapCodeGenerationChunk(row: CodeGenerationChunkRow): CloudCodegenChunk {
  return {
    chunkId: row.chunk_id,
    name: row.name,
    status: row.status,
    code: row.code ?? undefined,
    contract: row.contract ?? undefined,
    error: row.error ?? undefined,
    orderIndex: row.order_index,
  };
}

export function mapCodeGenerationDetail(input: {
  generation: CodeGenerationRow;
  chunks: CodeGenerationChunkRow[];
}): CloudCodeGenerationDetail {
  return {
    ...mapCodeGeneration(input.generation),
    chunks: input.chunks.map(mapCodeGenerationChunk),
  };
}
