import type {
  CloudCodeGeneration,
  CloudCodeGenerationDetail,
  CloudFileShare,
  CloudFileRecord,
  CloudFileSummary,
  CloudFileVersion,
  CloudFolder,
  CloudProject,
  CodegenAssetManifestEntry,
  CloudCodegenChunk,
  CloudCodegenFile,
  CloudActivityEvent,
} from '../../src/types/cloud';
import type { PenDocument } from '../../src/types/pen';
import type { Framework } from '@zseven-w/pen-types';

interface DesignFileSummaryRow {
  id: string;
  project_id?: string | null;
  folder_id?: string | null;
  name: string;
  thumbnail_path: string | null;
  revision: number;
  metadata?: Record<string, unknown> | null;
  starred?: boolean | null;
  last_opened_at?: string | null;
  deleted_at?: string | null;
  created_at: string;
  updated_at: string;
  share_role?: CloudFileShare['role'] | null;
  shared_by_email?: string | null;
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
  actor_id?: string | null;
  size_bytes?: number | null;
  created_at: string;
}

interface ActivityEventRow {
  id: string;
  file_id: string | null;
  generation_id: string | null;
  actor_id: string | null;
  owner_id: string;
  type: CloudActivityEvent['type'];
  metadata?: Record<string, unknown> | null;
  created_at: string;
}

interface ProjectRow {
  id: string;
  name: string;
  description: string | null;
  icon: string | null;
  color: string | null;
  created_at: string;
  updated_at: string;
}

interface FolderRow {
  id: string;
  project_id: string;
  parent_id: string | null;
  name: string;
  sort_order: number;
  created_at: string;
  updated_at: string;
}

interface ShareRow {
  id: string;
  file_id: string;
  owner_id: string;
  shared_with_user_id: string | null;
  shared_with_email: string | null;
  role: CloudFileShare['role'];
  created_at: string;
  updated_at: string;
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
  entry_file?: string | null;
  degraded: boolean;
  assets_manifest: CodegenAssetManifestEntry[];
  model: string | null;
  provider: string | null;
  error: string | null;
  created_at: string;
  completed_at: string | null;
  promoted_at?: string | null;
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

interface CodeGenerationFileRow {
  id: string;
  generation_id: string;
  path: string;
  language: string;
  role: CloudCodegenFile['role'];
  content: string;
  size_bytes: number;
  order_index: number;
  created_at: string;
}

export function mapFileSummary(row: DesignFileSummaryRow): CloudFileSummary {
  return {
    id: row.id,
    projectId: row.project_id ?? null,
    folderId: row.folder_id ?? null,
    name: row.name,
    thumbnailPath: row.thumbnail_path,
    revision: Number(row.revision),
    metadata: row.metadata ?? {},
    starred: Boolean(row.starred),
    lastOpenedAt: row.last_opened_at ?? null,
    deletedAt: row.deleted_at ?? null,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    shareRole: row.share_role ?? undefined,
    sharedByEmail: row.shared_by_email ?? null,
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
    actorId: row.actor_id ?? null,
    sizeBytes: Number(row.size_bytes ?? 0),
    createdAt: row.created_at,
  };
}

export function mapActivityEvent(row: ActivityEventRow): CloudActivityEvent {
  return {
    id: row.id,
    fileId: row.file_id,
    generationId: row.generation_id,
    actorId: row.actor_id,
    ownerId: row.owner_id,
    type: row.type,
    metadata: row.metadata ?? {},
    createdAt: row.created_at,
  };
}

export function mapProject(row: ProjectRow): CloudProject {
  return {
    id: row.id,
    name: row.name,
    description: row.description,
    icon: row.icon,
    color: row.color,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function mapFolder(row: FolderRow): CloudFolder {
  return {
    id: row.id,
    projectId: row.project_id,
    parentId: row.parent_id,
    name: row.name,
    sortOrder: row.sort_order,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function mapFileShare(row: ShareRow): CloudFileShare {
  return {
    id: row.id,
    fileId: row.file_id,
    ownerId: row.owner_id,
    sharedWithUserId: row.shared_with_user_id,
    sharedWithEmail: row.shared_with_email,
    role: row.role,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
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
    entryFile: row.entry_file ?? null,
    degraded: row.degraded,
    assetsManifest: row.assets_manifest ?? [],
    model: row.model,
    provider: row.provider,
    error: row.error,
    createdAt: row.created_at,
    completedAt: row.completed_at,
    promotedAt: row.promoted_at ?? null,
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

export function mapCodeGenerationFile(row: CodeGenerationFileRow): CloudCodegenFile {
  return {
    id: row.id,
    generationId: row.generation_id,
    path: row.path,
    language: row.language,
    role: row.role,
    content: row.content,
    sizeBytes: Number(row.size_bytes),
    orderIndex: row.order_index,
    createdAt: row.created_at,
  };
}

export function mapCodeGenerationDetail(input: {
  generation: CodeGenerationRow;
  files?: CodeGenerationFileRow[];
  chunks: CodeGenerationChunkRow[];
}): CloudCodeGenerationDetail {
  return {
    ...mapCodeGeneration(input.generation),
    files: (input.files ?? []).map(mapCodeGenerationFile),
    chunks: input.chunks.map(mapCodeGenerationChunk),
  };
}
