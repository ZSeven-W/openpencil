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
  CloudCodegenJob,
  CloudCodegenJobEvent,
  CloudCodegenJobStep,
  CodegenProviderConfig,
  CodegenProviderConfigAudit,
  TaskNotification,
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
  metadata?: Record<string, unknown> | null;
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

interface CodegenJobRow {
  id: string;
  file_id: string;
  owner_id: string;
  generation_id: string | null;
  job_kind?: CloudCodegenJob['jobKind'] | null;
  status: CloudCodegenJob['status'];
  framework: Framework;
  page_id: string;
  target_kind: CloudCodegenJob['targetKind'];
  node_ids: string[];
  target_hash: string;
  document_revision: number;
  provider: string | null;
  model: string | null;
  priority: number;
  progress: number;
  attempts: number;
  max_attempts: number;
  locked_by: string | null;
  locked_until: string | null;
  last_heartbeat_at?: string | null;
  next_run_at?: string | null;
  dead_lettered_at?: string | null;
  last_error?: string | null;
  failure_type?: CloudCodegenJob['failureType'] | null;
  input_snapshot?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
  error: string | null;
  created_at: string;
  updated_at: string;
  started_at: string | null;
  completed_at: string | null;
  canceled_at: string | null;
}

interface CodegenJobStepRow {
  id: string;
  job_id: string;
  agent_role: CloudCodegenJobStep['agentRole'];
  attempt?: number | null;
  status: CloudCodegenJobStep['status'];
  progress: number;
  input?: Record<string, unknown> | null;
  output?: Record<string, unknown> | null;
  error: string | null;
  started_at: string | null;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}

interface CodegenJobEventRow {
  id: string;
  job_id: string;
  type: string;
  message: string | null;
  metadata?: Record<string, unknown> | null;
  created_at: string;
}

interface TaskNotificationRow {
  id: string;
  owner_id: string;
  job_id: string | null;
  file_id: string | null;
  generation_id: string | null;
  kind: TaskNotification['kind'];
  title: string;
  message: string | null;
  read_at: string | null;
  created_at: string;
}

interface CodegenProviderConfigRow {
  provider: string;
  enabled?: boolean | null;
  max_per_minute?: number | null;
  circuit_threshold?: number | null;
  circuit_open_ms?: number | null;
  updated_by?: string | null;
  created_at: string;
  updated_at: string;
}

interface CodegenProviderConfigAuditRow {
  id: string;
  provider: string;
  actor_id?: string | null;
  actor_email?: string | null;
  before_config?: Record<string, unknown> | null;
  after_config?: Record<string, unknown> | null;
  reason?: string | null;
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
    metadata: row.metadata ?? {},
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

export function mapCodegenJobStep(row: CodegenJobStepRow): CloudCodegenJobStep {
  return {
    id: row.id,
    jobId: row.job_id,
    agentRole: row.agent_role,
    attempt: Number(row.attempt ?? 1),
    status: row.status,
    progress: Number(row.progress),
    input: row.input ?? {},
    output: row.output ?? {},
    error: row.error,
    startedAt: row.started_at,
    completedAt: row.completed_at,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function mapCodegenJobEvent(row: CodegenJobEventRow): CloudCodegenJobEvent {
  return {
    id: row.id,
    jobId: row.job_id,
    type: row.type,
    message: row.message,
    metadata: row.metadata ?? {},
    createdAt: row.created_at,
  };
}

export function mapCodegenJob(
  row: CodegenJobRow,
  extras: {
    steps?: CodegenJobStepRow[];
    events?: CodegenJobEventRow[];
  } = {},
): CloudCodegenJob {
  return {
    id: row.id,
    fileId: row.file_id,
    ownerId: row.owner_id,
    generationId: row.generation_id,
    jobKind: row.job_kind ?? 'full_generation',
    status: row.status,
    framework: row.framework,
    pageId: row.page_id,
    targetKind: row.target_kind,
    nodeIds: row.node_ids ?? [],
    targetHash: row.target_hash,
    documentRevision: Number(row.document_revision),
    provider: row.provider,
    model: row.model,
    priority: Number(row.priority),
    progress: Number(row.progress),
    attempts: Number(row.attempts),
    maxAttempts: Number(row.max_attempts),
    lockedBy: row.locked_by,
    lockedUntil: row.locked_until,
    lastHeartbeatAt: row.last_heartbeat_at ?? null,
    nextRunAt: row.next_run_at ?? null,
    deadLetteredAt: row.dead_lettered_at ?? null,
    lastError: row.last_error ?? null,
    failureType: row.failure_type ?? null,
    inputSnapshot: row.input_snapshot ?? {},
    output: row.output ?? {},
    error: row.error,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
    startedAt: row.started_at,
    completedAt: row.completed_at,
    canceledAt: row.canceled_at,
    steps: extras.steps?.map(mapCodegenJobStep),
    events: extras.events?.map(mapCodegenJobEvent),
  };
}

export function mapTaskNotification(row: TaskNotificationRow): TaskNotification {
  return {
    id: row.id,
    ownerId: row.owner_id,
    jobId: row.job_id,
    fileId: row.file_id,
    generationId: row.generation_id,
    kind: row.kind,
    title: row.title,
    message: row.message,
    readAt: row.read_at,
    createdAt: row.created_at,
  };
}

export function mapCodegenProviderConfig(
  row: CodegenProviderConfigRow,
): CodegenProviderConfig {
  return {
    provider: row.provider,
    enabled: row.enabled ?? true,
    maxPerMinute: Number(row.max_per_minute ?? 30),
    circuitThreshold: Number(row.circuit_threshold ?? 3),
    circuitOpenMs: Number(row.circuit_open_ms ?? 60_000),
    updatedBy: row.updated_by ?? null,
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function mapCodegenProviderConfigAudit(
  row: CodegenProviderConfigAuditRow,
): CodegenProviderConfigAudit {
  return {
    id: row.id,
    provider: row.provider,
    actorId: row.actor_id ?? null,
    actorEmail: row.actor_email ?? null,
    beforeConfig: row.before_config ?? {},
    afterConfig: row.after_config ?? {},
    reason: row.reason ?? null,
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
