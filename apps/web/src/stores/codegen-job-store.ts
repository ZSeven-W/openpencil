import { create } from 'zustand';
import i18n from '@/i18n';
import { appStorage } from '@/utils/app-storage';
import { CloudApiError } from '@/services/cloud/cloud-fetch';
import type {
  CloudCodegenJob,
  CodegenProviderConfig,
  CodegenProviderConfigAudit,
  CodegenQueueAccess,
  CodegenWorkerRuntimeStats,
  CodegenWorkerOverview,
  CreateCloudCodegenJobInput,
  TaskNotification,
} from '@/types/cloud';
import {
  batchCancelCloudCodegenJobs,
  cancelCloudCodegenJob,
  createCloudCodegenJob,
  getCloudCodegenJob,
  getCloudCodegenQueueAccess,
  getCloudCodegenWorkerOverview,
  getCloudCodegenWorkerStats,
  listCloudCodegenProviderConfigAudits,
  listCloudCodegenProviderConfigs,
  listCloudCodegenJobs,
  listTaskNotifications,
  markTaskNotificationRead,
  replayFailedCloudCodegenJobs,
  retryCloudCodegenJob,
  updateCloudCodegenJobPriority,
  updateCloudCodegenProviderConfig,
  type CloudCodegenWorkerOverviewFilters,
  type ReplayFailedCloudCodegenJobsInput,
} from '@/services/cloud/codegen-jobs';

interface CodegenJobState {
  jobs: CloudCodegenJob[];
  notifications: TaskNotification[];
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
    active?: boolean;
    deadLettered?: boolean;
  }) => Promise<void>;
  loadJob: (jobId: string) => Promise<CloudCodegenJob | null>;
  createJob: (input: CreateCloudCodegenJobInput) => Promise<CloudCodegenJob | null>;
  cancelJob: (jobId: string) => Promise<void>;
  batchCancelJobs: (jobIds: string[]) => Promise<void>;
  retryJob: (jobId: string) => Promise<void>;
  updateJobPriority: (jobId: string, priority: number) => Promise<void>;
  refreshWorkerOverview: (filters?: CloudCodegenWorkerOverviewFilters) => Promise<void>;
  refreshWorkerStats: (filters?: { days?: number }) => Promise<void>;
  refreshQueueAccess: () => Promise<void>;
  replayFailedJobs: (input?: ReplayFailedCloudCodegenJobsInput) => Promise<void>;
  refreshProviderConfigs: (filters?: { provider?: string; limit?: number; offset?: number }) => Promise<void>;
  refreshProviderConfigAudits: (filters?: {
    provider?: string;
    limit?: number;
    offset?: number;
  }) => Promise<void>;
  saveProviderConfig: (
    provider: string,
    input: Pick<
      CodegenProviderConfig,
      'enabled' | 'maxPerMinute' | 'circuitThreshold' | 'circuitOpenMs'
    > & { reason?: string },
  ) => Promise<void>;
  refreshNotifications: () => Promise<void>;
  markNotificationRead: (id: string) => Promise<void>;
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

async function showDesktopNotification(notification: TaskNotification) {
  if (typeof window === 'undefined') return;
  const nativeNotification = window.electronAPI?.notifications;
  if (!nativeNotification) return;
  const { title, message } = notificationI18n(notification);
  await nativeNotification.show({
    title,
    body: message,
    route: notification.jobId
      ? `/tasks/${notification.jobId}`
      : notification.fileId
        ? `/editor/${notification.fileId}`
        : '/tasks',
  });
}

export const useCodegenJobStore = create<CodegenJobState>((set) => ({
  jobs: [],
  notifications: [],
  providerConfigs: [],
  providerConfigAudits: [],
  providerConfigAuditPage: { total: 0, limit: 20, offset: 0 },
  queueAccess: undefined,
  workerStats: undefined,
  workerOverview: undefined,
  loading: false,
  error: undefined,
  lastCreatedJobId: undefined,

  refreshJobs: async (filters = {}) => {
    set({ loading: true, error: undefined });
    try {
      const jobs = await listCloudCodegenJobs({ ...filters, limit: 50 });
      set({ jobs, loading: false });
    } catch (err) {
      set({
        loading: false,
        error: taskErrorMessage(err, 'tasks.error.load'),
      });
    }
  },

  loadJob: async (jobId) => {
    set({ loading: true, error: undefined });
    try {
      const job = await getCloudCodegenJob(jobId);
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

  updateJobPriority: async (jobId, priority) => {
    try {
      const job = await updateCloudCodegenJobPriority(jobId, priority);
      set((state) => ({ jobs: upsertJob(state.jobs, job), error: undefined }));
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.updatePriority') });
    }
  },

  refreshWorkerOverview: async (filters = {}) => {
    try {
      const workerOverview = await getCloudCodegenWorkerOverview(filters);
      set({ workerOverview, error: undefined });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadWorkers') });
    }
  },

  refreshWorkerStats: async (filters = {}) => {
    try {
      const workerStats = await getCloudCodegenWorkerStats(filters);
      set({ workerStats, error: undefined });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadWorkerStats') });
    }
  },

  refreshQueueAccess: async () => {
    try {
      const queueAccess = await getCloudCodegenQueueAccess();
      set({ queueAccess, error: undefined });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadQueueAccess') });
    }
  },

  refreshProviderConfigs: async (filters = {}) => {
    try {
      const providerConfigs = await listCloudCodegenProviderConfigs(filters);
      set({ providerConfigs, error: undefined });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadProviderConfigs') });
    }
  },

  refreshProviderConfigAudits: async (filters = {}) => {
    try {
      const result = await listCloudCodegenProviderConfigAudits(filters);
      set({
        providerConfigAudits: result.data,
        providerConfigAuditPage: result.page,
        error: undefined,
      });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadProviderConfigAudits') });
    }
  },

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

  refreshNotifications: async () => {
    try {
      const notifications = await listTaskNotifications();
      set({ notifications });
    } catch (err) {
      set({ error: taskErrorMessage(err, 'tasks.error.loadNotifications') });
    }
  },

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

  reset: () =>
    set({
      jobs: [],
      notifications: [],
      providerConfigs: [],
      providerConfigAudits: [],
      providerConfigAuditPage: { total: 0, limit: 20, offset: 0 },
      queueAccess: undefined,
      workerStats: undefined,
      workerOverview: undefined,
      loading: false,
      error: undefined,
      lastCreatedJobId: undefined,
    }),
}));

export function getActiveCodegenJobs(jobs = useCodegenJobStore.getState().jobs) {
  return jobs.filter((job) => job.status === 'pending' || job.status === 'running');
}
