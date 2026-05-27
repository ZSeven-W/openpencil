import { create } from 'zustand';
import i18n from '@/i18n';
import { appStorage } from '@/utils/app-storage';
import { reuseInFlightRequest, stableRequestKey } from '@/utils/in-flight-request';
import { CloudApiError } from '@/services/cloud/cloud-fetch';
import type {
  CloudCodegenJob,
  CloudListPage,
  CodegenProviderConfig,
  CodegenProviderConfigAudit,
  CodegenQueueAccess,
  CodegenResumeMode,
  CodegenWorkerRuntimeStats,
  CodegenWorkerOverview,
  CreateCloudCodegenJobInput,
  RerunCloudCodegenJobStepInput,
  TaskNotification,
} from '@/types/cloud';
import {
  batchCancelCloudCodegenJobs,
  cancelCloudCodegenJob,
  createCloudCodegenJob,
  deleteCloudCodegenJob,
  getCloudCodegenJob,
  getCloudCodegenQueueAccess,
  getCloudCodegenWorkerOverview,
  getCloudCodegenWorkerStats,
  listCloudCodegenProviderConfigAudits,
  listCloudCodegenProviderConfigs,
  listCloudCodegenJobsPage,
  listTaskNotificationsPage,
  markTaskNotificationRead,
  replayFailedCloudCodegenJobs,
  rerunCloudCodegenJobStep,
  resumeCloudCodegenJob,
  retryCloudCodegenJob,
  updateCloudCodegenJobPriority,
  updateCloudCodegenProviderConfig,
  deleteTaskNotification,
  type CloudCodegenWorkerOverviewFilters,
  type ReplayFailedCloudCodegenJobsInput,
} from '@/services/cloud/codegen-jobs';

interface CodegenJobState {
  jobs: CloudCodegenJob[];
  jobPage: CloudListPage;
  notifications: TaskNotification[];
  notificationPage: CloudListPage;
  providerConfigs: CodegenProviderConfig[];
  providerConfigAudits: CodegenProviderConfigAudit[];
  providerConfigAuditPage: { total: number; limit: number; offset: number };
  queueAccess?: CodegenQueueAccess;
  workerStats?: CodegenWorkerRuntimeStats;
  workerOverview?: CodegenWorkerOverview;
  loading: boolean;
  error?: string;
  lastCreatedJobId?: string;

  refreshJobs: (filters?: {
    fileId?: string;
    status?: CloudCodegenJob['status'];
    active?: boolean;
    deadLettered?: boolean;
    limit?: number;
    offset?: number;
    force?: boolean;
  }) => Promise<void>;
  loadJob: (
    jobId: string,
    options?: { force?: boolean; maxAgeMs?: number },
  ) => Promise<CloudCodegenJob | null>;
  createJob: (input: CreateCloudCodegenJobInput) => Promise<CloudCodegenJob | null>;
  cancelJob: (jobId: string) => Promise<void>;
  deleteJob: (jobId: string) => Promise<void>;
  batchCancelJobs: (jobIds: string[]) => Promise<void>;
  retryJob: (jobId: string) => Promise<void>;
  resumeJob: (jobId: string, stage?: CodegenResumeMode) => Promise<void>;
  rerunJobStep: (jobId: string, input: RerunCloudCodegenJobStepInput) => Promise<void>;
  updateJobPriority: (jobId: string, priority: number) => Promise<void>;
  refreshWorkerOverview: (
    filters?: CloudCodegenWorkerOverviewFilters & { force?: boolean },
  ) => Promise<void>;
  refreshWorkerStats: (filters?: { days?: number; force?: boolean }) => Promise<void>;
  refreshQueueAccess: (options?: { force?: boolean }) => Promise<void>;
  replayFailedJobs: (input?: ReplayFailedCloudCodegenJobsInput) => Promise<void>;
  refreshProviderConfigs: (filters?: {
    provider?: string;
    limit?: number;
    offset?: number;
    force?: boolean;
  }) => Promise<void>;
  refreshProviderConfigAudits: (filters?: {
    provider?: string;
    limit?: number;
    offset?: number;
    force?: boolean;
  }) => Promise<void>;
  saveProviderConfig: (
    provider: string,
    input: Pick<
      CodegenProviderConfig,
      'enabled' | 'maxPerMinute' | 'circuitThreshold' | 'circuitOpenMs'
    > & { reason?: string },
  ) => Promise<void>;
  refreshNotifications: (options?: {
    fileId?: string;
    limit?: number;
    offset?: number;
    force?: boolean;
  }) => Promise<void>;
  markNotificationRead: (id: string) => Promise<void>;
  deleteNotification: (id: string) => Promise<void>;
  notifyUnreadCompletions: () => Promise<void>;
  reset: () => void;
}

function upsertJob(jobs: CloudCodegenJob[], job: CloudCodegenJob): CloudCodegenJob[] {
  const index = jobs.findIndex((item) => item.id === job.id);
  if (index < 0) return [job, ...jobs];
  const next = [...jobs];
  next[index] = job;
  return next;
}

const readRequests = new Map<string, Promise<unknown>>();
const jobDetailLoadedAt = new Map<string, number>();

const DEFAULT_JOB_DETAIL_CACHE_MS = 30_000;

const NOTIFIED_STORAGE_KEY = 'openpencil-task-notifications-shown';

function getShownNotificationIds(): Set<string> {
  try {
    return new Set(JSON.parse(appStorage.getItem(NOTIFIED_STORAGE_KEY) ?? '[]') as string[]);
  } catch {
    return new Set();
  }
}

function saveShownNotificationIds(ids: Set<string>) {
  appStorage.setItem(NOTIFIED_STORAGE_KEY, JSON.stringify(Array.from(ids).slice(-200)));
}

function isTerminalNotification(notification: TaskNotification): boolean {
  return (
    notification.kind === 'codegen_job_succeeded' ||
    notification.kind === 'codegen_job_failed' ||
    notification.kind === 'codegen_job_canceled'
  );
}

function notificationI18n(notification: TaskNotification): { title: string; message: string } {
  const framework = notification.message ?? '';
  if (notification.kind === 'codegen_job_succeeded') {
    return {
      title: i18n.t('tasks.notification.codegenSucceeded.title'),
      message: i18n.t('tasks.notification.codegenSucceeded.message', { framework }),
    };
  }
  if (notification.kind === 'codegen_job_failed') {
    return {
      title: i18n.t('tasks.notification.codegenFailed.title'),
      message: notification.message ?? i18n.t('tasks.notification.codegenFailed.message'),
    };
  }
  return {
    title: i18n.t('tasks.notification.codegenCanceled.title'),
    message: i18n.t('tasks.notification.codegenCanceled.message', { framework }),
  };
}

function taskErrorMessage(err: unknown, fallbackKey: string): string {
  if (err instanceof CloudApiError && err.code === 'migration_required') {
    const details = err.details as { migration?: string } | undefined;
    return i18n.t('tasks.error.migrationRequired', {
      migration: details?.migration ?? 'supabase/migrations/202605130001_background_codegen_jobs.sql',
    });
  }
  return err instanceof Error ? err.message : i18n.t(fallbackKey);
}

function showWebNotification(notification: TaskNotification) {
  if (typeof window === 'undefined') return;
  const event = new CustomEvent('openpencil:task-notification', {
    detail: { notification, ...notificationI18n(notification) },
  });
  window.dispatchEvent(event);
}

function forceReadOption(force?: boolean): { force: true } | undefined {
  return force ? { force: true } : undefined;
}

function hasLoadedJobDetail(job: CloudCodegenJob): boolean {
  return Array.isArray(job.steps) || Array.isArray(job.events);
}

function callWithOptionalOptions<Input, Result>(
  read: (input: Input, options?: { force?: boolean }) => Promise<Result>,
  input: Input,
  force?: boolean,
): Promise<Result> {
  const options = forceReadOption(force);
  return options ? read(input, options) : read(input);
}

function callWithOptionalReadOptions<Result>(
  read: (options?: { force?: boolean }) => Promise<Result>,
  force?: boolean,
): Promise<Result> {
  const options = forceReadOption(force);
  return options ? read(options) : read();
}

async function showDesktopNotification(notification: TaskNotification) {
  if (typeof window === 'undefined') return;
  const nativeNotification = window.electronAPI?.notifications;
  if (!nativeNotification) return;
  const { title, message } = notificationI18n(notification);
  const routeParams = new URLSearchParams({ view: 'notifications' });
  if (notification.fileId) routeParams.set('fileId', notification.fileId);
  await nativeNotification.show({
    title,
    body: message,
    route: `/tasks?${routeParams.toString()}`,
  });
}

export const useCodegenJobStore = create<CodegenJobState>((set, get) => ({
  jobs: [],
  jobPage: { total: 0, limit: 10, offset: 0 },
  notifications: [],
  notificationPage: { total: 0, limit: 10, offset: 0 },
  providerConfigs: [],
  providerConfigAudits: [],
  providerConfigAuditPage: { total: 0, limit: 10, offset: 0 },
  queueAccess: undefined,
  workerStats: undefined,
  workerOverview: undefined,
  loading: false,
  error: undefined,
  lastCreatedJobId: undefined,

  refreshJobs: async (filters = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('jobs', { ...filters, limit: filters.limit ?? 10 }), async () => {
      const { force, ...apiFilters } = filters;
      set({ loading: true, error: undefined });
      try {
        const result = await callWithOptionalOptions(
          listCloudCodegenJobsPage,
          { ...apiFilters, limit: filters.limit ?? 10 },
          force,
        );
        set({ jobs: result.data, jobPage: result.page, loading: false });
      } catch (err) {
        set({
          loading: false,
          error: taskErrorMessage(err, 'tasks.error.load'),
        });
      }
    }),

  loadJob: async (jobId, options = {}) => {
    const maxAgeMs = Math.max(0, options.maxAgeMs ?? DEFAULT_JOB_DETAIL_CACHE_MS);
    if (!options.force && maxAgeMs > 0) {
      const cached = get().jobs.find((item) => item.id === jobId);
      const loadedAt = jobDetailLoadedAt.get(jobId) ?? 0;
      if (cached && hasLoadedJobDetail(cached) && Date.now() - loadedAt <= maxAgeMs) {
        return cached;
      }
    }

    return reuseInFlightRequest(
      readRequests,
      stableRequestKey('job', { jobId, force: options.force === true }),
      async () => {
        set({ loading: true, error: undefined });
        try {
          const job = options.force
            ? await getCloudCodegenJob(jobId, { force: true })
            : await getCloudCodegenJob(jobId);
          jobDetailLoadedAt.set(job.id, Date.now());
          set((state) => ({
            loading: false,
            jobs: upsertJob(state.jobs, job),
          }));
          return job;
        } catch (err) {
          set({
            loading: false,
            error: taskErrorMessage(err, 'tasks.error.load'),
          });
          return null;
        }
      },
    );
  },

  createJob: async (input) => {
    set({ loading: true, error: undefined });
    try {
      const job = await createCloudCodegenJob(input);
      set((state) => ({
        loading: false,
        lastCreatedJobId: job.id,
        jobs: upsertJob(state.jobs, job),
      }));
      return job;
    } catch (err) {
      set({
        loading: false,
        error: taskErrorMessage(err, 'tasks.error.create'),
      });
      return null;
    }
  },

  cancelJob: async (jobId) => {
    try {
      const job = await cancelCloudCodegenJob(jobId);
      set((state) => ({ jobs: upsertJob(state.jobs, job) }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.cancel') });
    }
  },

  deleteJob: async (jobId) => {
    try {
      await deleteCloudCodegenJob(jobId);
      jobDetailLoadedAt.delete(jobId);
      set((state) => ({
        jobs: state.jobs.filter((job) => job.id !== jobId),
        jobPage: { ...state.jobPage, total: Math.max(0, state.jobPage.total - 1) },
        notifications: state.notifications.filter((item) => item.jobId !== jobId),
        error: undefined,
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.delete') });
    }
  },

  batchCancelJobs: async (jobIds) => {
    const uniqueJobIds = Array.from(new Set(jobIds)).filter(Boolean);
    if (uniqueJobIds.length === 0) return;
    try {
      const jobs = await batchCancelCloudCodegenJobs(uniqueJobIds);
      set((state) => ({
        jobs: jobs.reduce((current, job) => upsertJob(current, job), state.jobs),
        error: undefined,
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.batchCancel') });
    }
  },

  retryJob: async (jobId) => {
    try {
      const job = await retryCloudCodegenJob(jobId);
      set((state) => ({ jobs: upsertJob(state.jobs, job), error: undefined }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.retry') });
    }
  },

  resumeJob: async (jobId, stage) => {
    try {
      const job = await resumeCloudCodegenJob(jobId, stage ? { stage } : {});
      set((state) => ({ jobs: upsertJob(state.jobs, job), error: undefined }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.resume') });
    }
  },

  rerunJobStep: async (jobId, input) => {
    try {
      const job = await rerunCloudCodegenJobStep(jobId, input);
      set((state) => ({ jobs: upsertJob(state.jobs, job), error: undefined }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.rerunStep') });
    }
  },

  updateJobPriority: async (jobId, priority) => {
    try {
      const job = await updateCloudCodegenJobPriority(jobId, priority);
      set((state) => ({ jobs: upsertJob(state.jobs, job), error: undefined }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.updatePriority') });
    }
  },

  refreshWorkerOverview: async (filters = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('worker-overview', filters), async () => {
      const { force, ...apiFilters } = filters;
      try {
        const workerOverview = await callWithOptionalOptions(
          getCloudCodegenWorkerOverview,
          apiFilters,
          force,
        );
        set({ workerOverview, error: undefined });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadWorkers') });
      }
    }),

  refreshWorkerStats: async (filters = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('worker-stats', filters), async () => {
      const { force, ...apiFilters } = filters;
      try {
        const workerStats = await callWithOptionalOptions(
          getCloudCodegenWorkerStats,
          apiFilters,
          force,
        );
        set({ workerStats, error: undefined });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadWorkerStats') });
      }
    }),

  refreshQueueAccess: async (options = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('queue-access', options), async () => {
      try {
        const queueAccess = await callWithOptionalReadOptions(
          getCloudCodegenQueueAccess,
          options.force,
        );
        set({ queueAccess, error: undefined });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadQueueAccess') });
      }
    }),

  refreshProviderConfigs: async (filters = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('provider-configs', filters), async () => {
      const { force, ...apiFilters } = filters;
      try {
        const providerConfigs = await callWithOptionalOptions(
          listCloudCodegenProviderConfigs,
          apiFilters,
          force,
        );
        set({ providerConfigs, error: undefined });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadProviderConfigs') });
      }
    }),

  refreshProviderConfigAudits: async (filters = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('provider-config-audits', filters), async () => {
      const { force, ...apiFilters } = filters;
      try {
        const result = await callWithOptionalOptions(
          listCloudCodegenProviderConfigAudits,
          apiFilters,
          force,
        );
        set({
          providerConfigAudits: result.data,
          providerConfigAuditPage: result.page,
          error: undefined,
        });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadProviderConfigAudits') });
      }
    }),

  saveProviderConfig: async (provider, input) => {
    try {
      const config = await updateCloudCodegenProviderConfig(provider, input);
      set((state) => ({
        providerConfigs: [
          config,
          ...state.providerConfigs.filter((item) => item.provider !== config.provider),
        ].sort((a, b) => a.provider.localeCompare(b.provider)),
        error: undefined,
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.saveProviderConfig') });
    }
  },

  replayFailedJobs: async (input = {}) => {
    try {
      const jobs = await replayFailedCloudCodegenJobs(input);
      set((state) => ({
        jobs: jobs.reduce((current, job) => upsertJob(current, job), state.jobs),
        error: undefined,
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.replayFailed') });
    }
  },

  refreshNotifications: async (options = {}) =>
    reuseInFlightRequest(readRequests, stableRequestKey('notifications', options), async () => {
      try {
        const { force, ...apiOptions } = options;
        const result = await callWithOptionalOptions(
          listTaskNotificationsPage,
          { ...apiOptions, limit: apiOptions.limit ?? 10 },
          force,
        );
        set({ notifications: result.data, notificationPage: result.page });
      } catch (err) {
        set({ error: taskErrorMessage(err, 'tasks.error.loadNotifications') });
      }
    }),

  markNotificationRead: async (id) => {
    try {
      const notification = await markTaskNotificationRead(id);
      set((state) => ({
        notifications: state.notifications.map((item) =>
          item.id === notification.id ? notification : item,
        ),
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.updateNotification') });
    }
  },

  deleteNotification: async (id) => {
    try {
      await deleteTaskNotification(id);
      set((state) => ({
        notifications: state.notifications.filter((item) => item.id !== id),
        error: undefined,
      }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.updateNotification') });
    }
  },

  notifyUnreadCompletions: async () => {
    const shownIds = getShownNotificationIds();
    const unread = useCodegenJobStore
      .getState()
      .notifications.filter(
        (item) => !item.readAt && isTerminalNotification(item) && !shownIds.has(item.id),
      );
    for (const notification of unread) {
      showWebNotification(notification);
      await showDesktopNotification(notification).catch(() => {});
      shownIds.add(notification.id);
    }
    if (unread.length > 0) saveShownNotificationIds(shownIds);
  },

  reset: () => {
    readRequests.clear();
    jobDetailLoadedAt.clear();
    set({
      jobs: [],
      jobPage: { total: 0, limit: 10, offset: 0 },
      notifications: [],
      notificationPage: { total: 0, limit: 10, offset: 0 },
      providerConfigs: [],
      providerConfigAudits: [],
      providerConfigAuditPage: { total: 0, limit: 10, offset: 0 },
      queueAccess: undefined,
      workerStats: undefined,
      workerOverview: undefined,
      loading: false,
      error: undefined,
      lastCreatedJobId: undefined,
    });
  },
}));

export function getActiveCodegenJobs(jobs = useCodegenJobStore.getState().jobs) {
  return jobs.filter((job) => job.status === 'pending' || job.status === 'running');
}
