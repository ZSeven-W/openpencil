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
  projectId: string | null;
  folderId: string | null;
  name: string;
  thumbnailPath: string | null;
  revision: number;
  metadata?: Record<string, unknown>;
  starred: boolean;
  lastOpenedAt: string | null;
  deletedAt: string | null;
  createdAt: string;
  updatedAt: string;
  shareRole?: CloudShareRole;
  sharedByEmail?: string | null;
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
  actorId: string | null;
  sizeBytes: number;
  createdAt: string;
}

export type CloudActivityEventType =
  | 'file_created'
  | 'file_saved'
  | 'file_restored'
  | 'file_renamed'
  | 'file_moved'
  | 'file_shared'
  | 'file_share_revoked'
  | 'file_deleted'
  | 'file_restored_from_trash'
  | 'file_permanently_deleted'
  | 'codegen_created'
  | 'codegen_job_created'
  | 'codegen_job_succeeded'
  | 'codegen_job_failed'
  | 'codegen_job_canceled'
  | 'codegen_job_replayed'
  | 'codegen_promoted'
  | 'codegen_deleted'
  | 'version_labeled'
  | 'file_force_saved';

export interface CloudActivityEvent {
  id: string;
  fileId: string | null;
  generationId: string | null;
  actorId: string | null;
  ownerId: string;
  type: CloudActivityEventType;
  metadata?: Record<string, unknown>;
  createdAt: string;
}

export interface CloudActivityPage {
  data: CloudActivityEvent[];
  nextCursor: string | null;
  limit: number;
}

export interface CloudProject {
  id: string;
  name: string;
  description: string | null;
  icon: string | null;
  color: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CloudFolder {
  id: string;
  projectId: string;
  parentId: string | null;
  name: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export type CloudShareRole = 'viewer' | 'editor';

export interface CloudFileShare {
  id: string;
  fileId: string;
  ownerId: string;
  sharedWithUserId: string | null;
  sharedWithEmail: string | null;
  role: CloudShareRole;
  createdAt: string;
  updatedAt: string;
}

export type CloudFileView = 'all' | 'recent' | 'starred' | 'shared' | 'trash';
export type CloudFileLayout = 'list' | 'grid';
export type CloudFileSort = 'updated_desc' | 'updated_asc' | 'name_asc' | 'name_desc' | 'created_desc';

export interface CloudSaveConflict {
  code: 'revision_conflict';
  fileId: string;
  expectedRevision: number;
  serverRevision: number | null;
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

export type CodegenFileRole =
  | 'entry'
  | 'page'
  | 'component'
  | 'style'
  | 'config'
  | 'asset'
  | 'other';

export interface SaveCodegenFileInput {
  path: string;
  language: string;
  role: CodegenFileRole;
  content: string;
  orderIndex?: number;
}

export interface CloudCodegenFile extends SaveCodegenFileInput {
  id: string;
  generationId: string;
  sizeBytes: number;
  createdAt: string;
}

export interface SaveCodeGenerationInput extends CodegenTarget {
  fileId: string;
  framework: Framework;
  documentRevision: number;
  status: 'done' | 'degraded' | 'failed';
  finalCode?: string;
  entryFile?: string;
  degraded: boolean;
  assets: SaveCodegenAssetInput[];
  files?: SaveCodegenFileInput[];
  model?: string;
  provider?: string;
  error?: string;
  chunks: CloudCodegenChunk[];
  metadata?: Record<string, unknown>;
}

export interface CloudCodeGeneration extends CodegenTarget {
  id: string;
  fileId: string;
  framework: Framework;
  documentRevision: number;
  status: 'done' | 'degraded' | 'failed';
  finalCode: string | null;
  entryFile?: string | null;
  degraded: boolean;
  assetsManifest: CodegenAssetManifestEntry[];
  metadata?: Record<string, unknown>;
  model: string | null;
  provider: string | null;
  error: string | null;
  createdAt: string;
  completedAt: string | null;
  promotedAt?: string | null;
}

export interface CloudCodeGenerationDetail extends CloudCodeGeneration {
  files?: CloudCodegenFile[];
  chunks: CloudCodegenChunk[];
}

export type CodegenJobStatus = 'pending' | 'running' | 'succeeded' | 'failed' | 'canceled';
export type CodegenJobKind = 'full_generation' | 'patch_generation';
export type CodegenJobFailureType =
  | 'rate_limit'
  | 'timeout'
  | 'provider_error'
  | 'canceled'
  | 'execution_error';
export type CodegenJobAgentRole =
  | 'planner'
  | 'page_codegen'
  | 'reviewer_repair'
  | 'bundler_persist';

export interface CloudCodegenJobStep {
  id: string;
  jobId: string;
  agentRole: CodegenJobAgentRole;
  attempt: number;
  status: CodegenJobStatus;
  progress: number;
  input: Record<string, unknown>;
  output: Record<string, unknown>;
  error: string | null;
  startedAt: string | null;
  completedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface CloudCodegenJobEvent {
  id: string;
  jobId: string;
  type: string;
  message: string | null;
  metadata: Record<string, unknown>;
  createdAt: string;
}

export interface CloudCodegenJob {
  id: string;
  fileId: string;
  ownerId: string;
  generationId: string | null;
  jobKind: CodegenJobKind;
  status: CodegenJobStatus;
  framework: Framework;
  pageId: string;
  targetKind: CodegenTargetKind;
  nodeIds: string[];
  targetHash: string;
  documentRevision: number;
  provider: string | null;
  model: string | null;
  priority: number;
  progress: number;
  attempts: number;
  maxAttempts: number;
  lockedBy: string | null;
  lockedUntil: string | null;
  lastHeartbeatAt?: string | null;
  nextRunAt?: string | null;
  deadLetteredAt?: string | null;
  lastError?: string | null;
  failureType?: CodegenJobFailureType | null;
  inputSnapshot: Record<string, unknown>;
  output: Record<string, unknown>;
  error: string | null;
  createdAt: string;
  updatedAt: string;
  startedAt: string | null;
  completedAt: string | null;
  canceledAt: string | null;
  steps?: CloudCodegenJobStep[];
  events?: CloudCodegenJobEvent[];
}

export interface CodegenWorkerHeartbeat {
  workerId: string;
  hostname?: string | null;
  pid?: number | null;
  metadata?: Record<string, unknown>;
  startedAt?: string | null;
  lastHeartbeatAt: string;
  active: boolean;
}

export interface CodegenProviderHealth {
  provider: string;
  total: number;
  failed: number;
  failureRate: number;
  consecutiveFailures: number;
  lastFailureType?: string | null;
  lastError?: string | null;
  circuitOpenUntil?: string | null;
  lastRequestAt?: string | null;
  updatedAt?: string | null;
}

export interface CodegenProviderConfig {
  provider: string;
  enabled: boolean;
  maxPerMinute: number;
  circuitThreshold: number;
  circuitOpenMs: number;
  updatedBy: string | null;
  createdAt: string;
  updatedAt: string;
}

export type CodegenQueueRole = 'viewer' | 'operator' | 'admin';

export interface CodegenQueueAccess {
  role: CodegenQueueRole;
  bootstrapMode: boolean;
  canViewWorkers: boolean;
  canReplayFailed: boolean;
  canManageProviders: boolean;
}

export interface CodegenProviderConfigAudit {
  id: string;
  provider: string;
  actorId: string | null;
  actorEmail: string | null;
  beforeConfig: Record<string, unknown>;
  afterConfig: Record<string, unknown>;
  reason: string | null;
  createdAt: string;
}

export interface CodegenWorkerStatsBucket {
  bucket: string;
  total: number;
  pending: number;
  running: number;
  succeeded: number;
  failed: number;
  canceled: number;
  averageDurationMs: number;
  failureRate: number;
}

export interface CodegenProviderStats {
  provider: string;
  total: number;
  failed: number;
  succeeded: number;
  averageDurationMs: number;
  failureRate: number;
}

export interface CodegenWorkerRuntimeStats {
  buckets: CodegenWorkerStatsBucket[];
  providers: CodegenProviderStats[];
  workers: {
    active: number;
    stale: number;
  };
}

export interface CodegenWorkerOverviewMetrics {
  total: number;
  pending: number;
  running: number;
  succeeded: number;
  failed: number;
  canceled: number;
  deadLettered: number;
  averageDurationMs: number;
  failureRate: number;
}

export interface CodegenQueuePage {
  total: number;
  limit: number;
  offset: number;
}

export interface CodegenWorkerOverview {
  workers: CodegenWorkerHeartbeat[];
  metrics: CodegenWorkerOverviewMetrics;
  providers: CodegenProviderHealth[];
  failedJobs: CloudCodegenJob[];
  pages: {
    workers: CodegenQueuePage;
    providers: CodegenQueuePage;
    failedJobs: CodegenQueuePage;
  };
}

export interface CreateCloudCodegenJobInput extends CodegenTarget {
  jobKind?: CodegenJobKind;
  fileId: string;
  framework: Framework;
  documentRevision: number;
  model: string;
  provider: string;
  nodes: unknown[];
  variables?: Record<string, unknown>;
  baseGenerationId?: string;
  patchInstruction?: string;
}

export type TaskNotificationKind =
  | 'codegen_job_succeeded'
  | 'codegen_job_failed'
  | 'codegen_job_canceled';

export interface TaskNotification {
  id: string;
  ownerId: string;
  jobId: string | null;
  fileId: string | null;
  generationId: string | null;
  kind: TaskNotificationKind;
  title: string;
  message: string | null;
  readAt: string | null;
  createdAt: string;
}
