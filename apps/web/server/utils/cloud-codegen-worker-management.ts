import type { SupabaseClient } from '@supabase/supabase-js';
import { createError } from 'h3';
import type {
  CloudCodegenJob,
  CodegenJobFailureType,
  CodegenProviderConfig,
  CodegenProviderConfigAudit,
  CodegenQueueAccess,
  CodegenQueueRole,
  CodegenWorkerRuntimeStats,
  CodegenWorkerOverview,
  CodegenWorkerOverviewMetrics,
} from '../../src/types/cloud';
import {
  mapCodegenJob,
  mapCodegenProviderConfig,
  mapCodegenProviderConfigAudit,
} from './cloud-file-mappers';
import { recordCloudActivity } from './cloud-activity-events';
import { toCodegenDbError } from './cloud-codegen-errors';

const JOB_SELECT =
  'id,file_id,owner_id,generation_id,status,framework,page_id,target_kind,node_ids,target_hash,document_revision,provider,model,priority,progress,attempts,max_attempts,locked_by,locked_until,last_heartbeat_at,next_run_at,dead_lettered_at,last_error,failure_type,input_snapshot,output,error,created_at,updated_at,started_at,completed_at,canceled_at';
const PROVIDER_CONFIG_SELECT =
  'provider,enabled,max_per_minute,circuit_threshold,circuit_open_ms,updated_by,created_at,updated_at';
const PROVIDER_CONFIG_AUDIT_SELECT =
  'id,provider,actor_id,actor_email,before_config,after_config,reason,created_at';

type CountResult<T> = { data: T[] | null; error: unknown; count?: number | null };
type DbQuery<T> = PromiseLike<CountResult<T>> & {
  eq: (column: string, value: unknown) => DbQuery<T>;
  gte: (column: string, value: string) => DbQuery<T>;
  lte: (column: string, value: string) => DbQuery<T>;
  in: (column: string, values: unknown[]) => DbQuery<T>;
  not: (column: string, operator: string, value: unknown) => DbQuery<T>;
  limit: (count: number) => DbQuery<T>;
  order: (column: string, options?: unknown) => DbQuery<T>;
  range: (from: number, to: number) => DbQuery<T>;
};
type CodegenJobRowForMap = Parameters<typeof mapCodegenJob>[0];
type ReplaySelectRow = {
  id: string;
  file_id: string;
  framework?: string | null;
  status?: string | null;
  dead_lettered_at?: string | null;
};

const ROLE_RANK: Record<CodegenQueueRole, number> = {
  viewer: 1,
  operator: 2,
  admin: 3,
};

function averageDurationMs(rows: Array<{ started_at: string | null; completed_at: string | null }>) {
  const durations = rows
    .map((row) => {
      if (!row.started_at || !row.completed_at) return null;
      const started = new Date(row.started_at).getTime();
      const completed = new Date(row.completed_at).getTime();
      return Number.isFinite(started) && Number.isFinite(completed) && completed >= started
        ? completed - started
        : null;
    })
    .filter((value): value is number => value !== null);
  if (durations.length === 0) return 0;
  return Math.round(durations.reduce((sum, value) => sum + value, 0) / durations.length);
}

function buildMetrics(rows: Array<{
  status: CloudCodegenJob['status'];
  dead_lettered_at?: string | null;
  started_at: string | null;
  completed_at: string | null;
}>): CodegenWorkerOverviewMetrics {
  const metrics = rows.reduce<CodegenWorkerOverviewMetrics>(
    (acc, row) => {
      acc.total += 1;
      acc[row.status] += 1;
      if (row.dead_lettered_at) acc.deadLettered += 1;
      return acc;
    },
    {
      total: 0,
      pending: 0,
      running: 0,
      succeeded: 0,
      failed: 0,
      canceled: 0,
      deadLettered: 0,
      averageDurationMs: 0,
      failureRate: 0,
    },
  );
  metrics.averageDurationMs = averageDurationMs(rows);
  metrics.failureRate = metrics.total > 0 ? metrics.failed / metrics.total : 0;
  return metrics;
}

function pageOf(limit?: number, offset?: number) {
  return {
    limit: Math.min(Math.max(Number(limit ?? 50), 1), 100),
    offset: Math.max(Number(offset ?? 0), 0),
  };
}

function rangeEnd(offset: number, limit: number) {
  return offset + limit - 1;
}

function applyFailedJobFilters<T>(query: DbQuery<T>, input: {
  provider?: string;
  failureType?: CodegenJobFailureType;
  deadLetteredFrom?: string;
  deadLetteredTo?: string;
}) {
  let next = query;
  if (input.provider) next = next.eq('provider', input.provider);
  if (input.failureType) next = next.eq('failure_type', input.failureType);
  if (input.deadLetteredFrom) next = next.gte('dead_lettered_at', input.deadLetteredFrom);
  if (input.deadLetteredTo) next = next.lte('dead_lettered_at', input.deadLetteredTo);
  return next;
}

function canRole(role: CodegenQueueRole, required: CodegenQueueRole) {
  return ROLE_RANK[role] >= ROLE_RANK[required];
}

function accessForRole(role: CodegenQueueRole, bootstrapMode = false): CodegenQueueAccess {
  return {
    role,
    bootstrapMode,
    canViewWorkers: canRole(role, 'viewer'),
    canReplayFailed: canRole(role, 'operator'),
    canManageProviders: canRole(role, 'admin'),
  };
}

export async function getCodegenQueueAccess(input: {
  supabase: SupabaseClient | null;
  userId: string;
  userEmail?: string | null;
}): Promise<CodegenQueueAccess> {
  if (!input.supabase) return accessForRole('viewer');
  const row = await input.supabase
    .from('codegen_queue_members')
    .select('role')
    .eq('user_id', input.userId)
    .maybeSingle();
  if (row.error) throw toCodegenDbError(row.error, 'Failed to load queue access');
  if (row.data?.role) return accessForRole(row.data.role as CodegenQueueRole);

  const count = await input.supabase
    .from('codegen_queue_members')
    .select('id', { count: 'exact', head: true });
  if (count.error) throw toCodegenDbError(count.error, 'Failed to inspect queue access');
  if ((count.count ?? 0) === 0) return accessForRole('admin', true);
  return accessForRole('viewer');
}

export async function assertCodegenQueueRole(input: {
  supabase: SupabaseClient | null;
  userId: string;
  userEmail?: string | null;
  role: CodegenQueueRole;
}) {
  const access = await getCodegenQueueAccess(input);
  if (!canRole(access.role, input.role)) {
    throw createError({
      statusCode: 403,
      statusMessage: `Codegen queue ${input.role} access is required`,
    });
  }
  return access;
}

export async function updateCodegenJobPriority(input: {
  supabase: SupabaseClient;
  userId: string;
  jobId: string;
  priority: number;
}): Promise<CloudCodegenJob> {
  const updated = await input.supabase
    .from('codegen_jobs')
    .update({ priority: input.priority, updated_at: new Date().toISOString() })
    .eq('id', input.jobId)
    .eq('owner_id', input.userId)
    .eq('status', 'pending')
    .select(JOB_SELECT)
    .maybeSingle();
  if (updated.error) throw toCodegenDbError(updated.error, 'Failed to update codegen job priority');
  if (!updated.data) {
    throw createError({
      statusCode: 409,
      statusMessage: 'Only pending codegen jobs can change priority',
    });
  }
  return mapCodegenJob(updated.data);
}

export async function listCodegenProviderConfigs(input: {
  supabase: SupabaseClient | null;
  provider?: string;
  limit?: number;
  offset?: number;
}): Promise<CodegenProviderConfig[]> {
  if (!input.supabase) return [];
  const page = pageOf(input.limit, input.offset);
  let query = input.supabase
    .from('codegen_provider_configs')
    .select(PROVIDER_CONFIG_SELECT)
    .order('provider', { ascending: true })
    .range(page.offset, rangeEnd(page.offset, page.limit));
  if (input.provider) query = query.eq('provider', input.provider);
  const rows = await query;
  if (rows.error) throw toCodegenDbError(rows.error, 'Failed to list provider configs');
  return (rows.data ?? []).map(mapCodegenProviderConfig);
}

export async function updateCodegenProviderConfig(input: {
  supabase: SupabaseClient | null;
  actorId: string;
  actorEmail?: string | null;
  provider: string;
  enabled: boolean;
  maxPerMinute: number;
  circuitThreshold: number;
  circuitOpenMs: number;
  reason?: string;
}): Promise<CodegenProviderConfig> {
  if (!input.supabase) {
    throw createError({ statusCode: 503, statusMessage: 'Service role Supabase is not configured' });
  }
  await assertCodegenQueueRole({
    supabase: input.supabase,
    userId: input.actorId,
    userEmail: input.actorEmail,
    role: 'admin',
  });
  const before = await input.supabase
    .from('codegen_provider_configs')
    .select(PROVIDER_CONFIG_SELECT)
    .eq('provider', input.provider)
    .maybeSingle();
  if (before.error) throw toCodegenDbError(before.error, 'Failed to load provider config');

  const now = new Date().toISOString();
  const row = await input.supabase
    .from('codegen_provider_configs')
    .upsert({
      provider: input.provider,
      enabled: input.enabled,
      max_per_minute: input.maxPerMinute,
      circuit_threshold: input.circuitThreshold,
      circuit_open_ms: input.circuitOpenMs,
      updated_by: input.actorId,
      updated_at: now,
    })
    .select(PROVIDER_CONFIG_SELECT)
    .single();
  if (row.error || !row.data) {
    throw row.error
      ? toCodegenDbError(row.error, 'Failed to update provider config')
      : createError({ statusCode: 500, statusMessage: 'Failed to update provider config' });
  }
  const audit = await input.supabase.from('codegen_provider_config_audits').insert({
    provider: input.provider,
    actor_id: input.actorId,
    actor_email: input.actorEmail ?? null,
    before_config: before.data ?? {},
    after_config: row.data,
    reason: input.reason ?? null,
  });
  if (audit.error) throw toCodegenDbError(audit.error, 'Failed to audit provider config');
  return mapCodegenProviderConfig(row.data);
}

export async function listCodegenProviderConfigAudits(input: {
  supabase: SupabaseClient | null;
  provider?: string;
  limit?: number;
  offset?: number;
}): Promise<{ data: CodegenProviderConfigAudit[]; page: { total: number; limit: number; offset: number } }> {
  if (!input.supabase) return { data: [], page: { total: 0, ...pageOf(input.limit, input.offset) } };
  const page = pageOf(input.limit, input.offset);
  let query = input.supabase
    .from('codegen_provider_config_audits')
    .select(PROVIDER_CONFIG_AUDIT_SELECT, { count: 'exact' })
    .order('created_at', { ascending: false })
    .range(page.offset, rangeEnd(page.offset, page.limit));
  if (input.provider) query = query.eq('provider', input.provider);
  const rows = await query;
  if (rows.error) throw toCodegenDbError(rows.error, 'Failed to list provider config audits');
  return {
    data: (rows.data ?? []).map(mapCodegenProviderConfigAudit),
    page: { ...page, total: rows.count ?? 0 },
  };
}

export async function replayFailedCodegenJobs(input: {
  supabase: SupabaseClient;
  adminSupabase?: SupabaseClient | null;
  userId: string;
  userEmail?: string | null;
  jobIds?: string[];
  limit?: number;
  provider?: string;
  failureType?: CodegenJobFailureType;
  deadLetteredFrom?: string;
  deadLetteredTo?: string;
}): Promise<CloudCodegenJob[]> {
  if (input.adminSupabase) {
    await assertCodegenQueueRole({
      supabase: input.adminSupabase,
      userId: input.userId,
      userEmail: input.userEmail,
      role: 'operator',
    });
  }
  const uniqueJobIds = Array.from(new Set(input.jobIds ?? [])).filter(Boolean);
  if (uniqueJobIds.length > 50) {
    throw createError({ statusCode: 400, statusMessage: 'At most 50 jobs can be replayed at once' });
  }

  let selectQuery = input.supabase
    .from('codegen_jobs')
    .select('id,file_id,framework,status,dead_lettered_at')
    .eq('owner_id', input.userId)
    .eq('status', 'failed') as unknown as DbQuery<ReplaySelectRow>;

  if (uniqueJobIds.length > 0) {
    selectQuery = selectQuery.in('id', uniqueJobIds);
  } else {
    const filteredQuery = applyFailedJobFilters(
      selectQuery.not('dead_lettered_at', 'is', null),
      input,
    );
    selectQuery = filteredQuery.limit(input.limit ?? 50);
  }

  const selected = await selectQuery;
  if (selected.error) throw toCodegenDbError(selected.error, 'Failed to select failed jobs');
  const selectedRows = selected.data ?? [];
  const ids = selectedRows.map((row: { id: string }) => row.id);
  if (ids.length === 0) return [];

  const now = new Date().toISOString();
  const updated = await input.supabase
    .from('codegen_jobs')
    .update({
      status: 'pending',
      generation_id: null,
      progress: 0,
      attempts: 0,
      locked_by: null,
      locked_until: null,
      last_heartbeat_at: null,
      next_run_at: now,
      dead_lettered_at: null,
      last_error: null,
      failure_type: null,
      output: {},
      error: null,
      started_at: null,
      completed_at: null,
      canceled_at: null,
      updated_at: now,
    })
    .eq('owner_id', input.userId)
    .in('id', ids)
    .select(JOB_SELECT);
  if (updated.error) throw toCodegenDbError(updated.error, 'Failed to replay failed jobs');

  const replayMetadata = {
    actorId: input.userId,
    filters: {
      jobIds: uniqueJobIds,
      provider: input.provider,
      failureType: input.failureType,
      deadLetteredFrom: input.deadLetteredFrom,
      deadLetteredTo: input.deadLetteredTo,
    },
    replayedAt: now,
  };
  const events = await input.supabase.from('codegen_job_events').insert(
    ids.map((jobId) => ({
      job_id: jobId,
      type: 'replay_queued',
      message: 'Failed codegen job replay queued.',
      metadata: replayMetadata,
    })),
  );
  if (events.error) throw toCodegenDbError(events.error, 'Failed to create replay events');

  for (const row of selectedRows as Array<{ id: string; file_id: string; framework?: string | null }>) {
    await recordCloudActivity({
      supabase: input.supabase,
      ownerId: input.userId,
      actorId: input.userId,
      fileId: row.file_id,
      type: 'codegen_job_replayed',
      metadata: { jobId: row.id, framework: row.framework ?? null, ...replayMetadata },
    });
  }

  return (updated.data ?? []).map((row) => mapCodegenJob(row));
}

export async function getCodegenWorkerOverview(input: {
  supabase: SupabaseClient;
  adminSupabase: SupabaseClient | null;
  userId: string;
  workerLimit?: number;
  workerOffset?: number;
  providerLimit?: number;
  providerOffset?: number;
  failedLimit?: number;
  failedOffset?: number;
  provider?: string;
  failureType?: CodegenJobFailureType;
  deadLetteredFrom?: string;
  deadLetteredTo?: string;
}): Promise<CodegenWorkerOverview> {
  const workerPage = pageOf(input.workerLimit, input.workerOffset);
  const providerPage = pageOf(input.providerLimit, input.providerOffset);
  const failedPage = pageOf(input.failedLimit, input.failedOffset);

  const jobs = await input.supabase
    .from('codegen_jobs')
    .select('status,provider,dead_lettered_at,started_at,completed_at,created_at,updated_at')
    .eq('owner_id', input.userId)
    .order('created_at', { ascending: false })
    .limit(500);
  if (jobs.error) throw toCodegenDbError(jobs.error, 'Failed to load worker metrics');

  let failedQuery = input.supabase
    .from('codegen_jobs')
    .select(JOB_SELECT, { count: 'exact' })
    .eq('owner_id', input.userId)
    .eq('status', 'failed')
    .not('dead_lettered_at', 'is', null) as unknown as DbQuery<CodegenJobRowForMap>;
  const filteredFailedQuery = applyFailedJobFilters(
    failedQuery,
    input,
  );
  failedQuery = filteredFailedQuery
    .order('dead_lettered_at', { ascending: false })
    .range(failedPage.offset, rangeEnd(failedPage.offset, failedPage.limit));
  const failedRows = await failedQuery;
  if (failedRows.error) throw toCodegenDbError(failedRows.error, 'Failed to load failed jobs');

  const admin = input.adminSupabase;
  const [workerRows, providerRows] = admin
    ? await Promise.all([
        admin
          .from('codegen_worker_heartbeats')
          .select('worker_id,hostname,pid,metadata,started_at,last_heartbeat_at', { count: 'exact' })
          .order('last_heartbeat_at', { ascending: false })
          .range(workerPage.offset, rangeEnd(workerPage.offset, workerPage.limit)),
        admin
          .from('codegen_provider_health')
          .select(
            'provider,total_requests,failed_requests,consecutive_failures,last_failure_type,last_error,circuit_open_until,last_request_at,updated_at',
            { count: 'exact' },
          )
          .order('updated_at', { ascending: false })
          .range(providerPage.offset, rangeEnd(providerPage.offset, providerPage.limit)),
      ])
    : [{ data: [], error: null, count: 0 }, { data: [], error: null, count: 0 }];

  if (workerRows.error) throw toCodegenDbError(workerRows.error, 'Failed to load workers');
  if (providerRows.error) throw toCodegenDbError(providerRows.error, 'Failed to load providers');

  const now = Date.now();
  return {
    workers: (workerRows.data ?? []).map((row: {
      worker_id: string;
      hostname?: string | null;
      pid?: number | null;
      metadata?: Record<string, unknown> | null;
      started_at: string;
      last_heartbeat_at: string;
    }) => {
      const lastHeartbeatAt = row.last_heartbeat_at;
      const heartbeatMs = new Date(lastHeartbeatAt).getTime();
      return {
        workerId: row.worker_id,
        hostname: row.hostname ?? null,
        pid: row.pid ?? null,
        metadata: row.metadata ?? {},
        startedAt: row.started_at,
        lastHeartbeatAt,
        active: Number.isFinite(heartbeatMs) && now - heartbeatMs < 60_000,
      };
    }),
    metrics: buildMetrics(jobs.data ?? []),
    providers: (providerRows.data ?? []).map((row: {
      provider: string;
      total_requests?: number | null;
      failed_requests?: number | null;
      consecutive_failures?: number | null;
      last_failure_type?: string | null;
      last_error?: string | null;
      circuit_open_until?: string | null;
      last_request_at?: string | null;
      updated_at?: string | null;
    }) => {
      const total = Number(row.total_requests ?? 0);
      const failed = Number(row.failed_requests ?? 0);
      return {
        provider: row.provider,
        total,
        failed,
        failureRate: total > 0 ? failed / total : 0,
        consecutiveFailures: Number(row.consecutive_failures ?? 0),
        lastFailureType: row.last_failure_type ?? null,
        lastError: row.last_error ?? null,
        circuitOpenUntil: row.circuit_open_until ?? null,
        lastRequestAt: row.last_request_at ?? null,
        updatedAt: row.updated_at ?? null,
      };
    }),
    failedJobs: ((failedRows.data ?? []) as CodegenJobRowForMap[]).map((row) => mapCodegenJob(row)),
    pages: {
      workers: { ...workerPage, total: workerRows.count ?? 0 },
      providers: { ...providerPage, total: providerRows.count ?? 0 },
      failedJobs: { ...failedPage, total: failedRows.count ?? 0 },
    },
  };
}

export async function getCodegenWorkerRuntimeStats(input: {
  supabase: SupabaseClient;
  adminSupabase: SupabaseClient | null;
  userId: string;
  days?: number;
}): Promise<CodegenWorkerRuntimeStats> {
  const days = Math.min(Math.max(Number(input.days ?? 7), 1), 30);
  const since = new Date(Date.now() - days * 24 * 60 * 60 * 1000).toISOString();
  const jobs = await input.supabase
    .from('codegen_jobs')
    .select('status,provider,started_at,completed_at,created_at')
    .eq('owner_id', input.userId)
    .gte('created_at', since)
    .order('created_at', { ascending: true })
    .limit(1000);
  if (jobs.error) throw toCodegenDbError(jobs.error, 'Failed to load worker stats');

  const workerRows = input.adminSupabase
    ? await input.adminSupabase
      .from('codegen_worker_heartbeats')
      .select('last_heartbeat_at')
      .order('last_heartbeat_at', { ascending: false })
      .limit(500)
    : { data: [], error: null };
  if (workerRows.error) throw toCodegenDbError(workerRows.error, 'Failed to load worker stats');

  const now = Date.now();
  const workers = (workerRows.data ?? []).reduce(
    (acc, row: { last_heartbeat_at?: string | null }) => {
      const heartbeat = row.last_heartbeat_at ? new Date(row.last_heartbeat_at).getTime() : 0;
      if (Number.isFinite(heartbeat) && now - heartbeat < 60_000) acc.active += 1;
      else acc.stale += 1;
      return acc;
    },
    { active: 0, stale: 0 },
  );

  type Acc = {
    total: number;
    pending: number;
    running: number;
    succeeded: number;
    failed: number;
    canceled: number;
    durations: number[];
  };
  const bucketMap = new Map<string, Acc>();
  const providerMap = new Map<string, Acc>();
  const makeAcc = (): Acc => ({
    total: 0,
    pending: 0,
    running: 0,
    succeeded: 0,
    failed: 0,
    canceled: 0,
    durations: [],
  });
  const add = (acc: Acc, row: {
    status: CloudCodegenJob['status'];
    started_at?: string | null;
    completed_at?: string | null;
  }) => {
    acc.total += 1;
    acc[row.status] += 1;
    if (row.started_at && row.completed_at) {
      const started = new Date(row.started_at).getTime();
      const completed = new Date(row.completed_at).getTime();
      if (Number.isFinite(started) && Number.isFinite(completed) && completed >= started) {
        acc.durations.push(completed - started);
      }
    }
  };
  const finish = (acc: Acc) => ({
    total: acc.total,
    pending: acc.pending,
    running: acc.running,
    succeeded: acc.succeeded,
    failed: acc.failed,
    canceled: acc.canceled,
    averageDurationMs: acc.durations.length
      ? Math.round(acc.durations.reduce((sum, value) => sum + value, 0) / acc.durations.length)
      : 0,
    failureRate: acc.total > 0 ? acc.failed / acc.total : 0,
  });

  for (const row of jobs.data ?? []) {
    const bucket = String(row.created_at).slice(0, 10);
    const provider = row.provider ?? 'unknown';
    const bucketAcc = bucketMap.get(bucket) ?? makeAcc();
    const providerAcc = providerMap.get(provider) ?? makeAcc();
    add(bucketAcc, row);
    add(providerAcc, row);
    bucketMap.set(bucket, bucketAcc);
    providerMap.set(provider, providerAcc);
  }

  return {
    buckets: Array.from(bucketMap.entries()).map(([bucket, acc]) => ({ bucket, ...finish(acc) })),
    providers: Array.from(providerMap.entries()).map(([provider, acc]) => ({
      provider,
      ...finish(acc),
    })),
    workers,
  };
}
