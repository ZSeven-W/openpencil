import { useEffect } from 'react';
import { useLocation, useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { toast } from 'sonner';
import { Button } from '@/components/ui/button';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';

const INITIAL_NOTIFICATION_POLL_DELAY_MS = 5_000;
const NOTIFICATION_POLL_INTERVAL_MS = 15_000;

function hasTaskJobSearch(search: unknown): boolean {
  if (!search) return false;
  if (typeof search === 'string') {
    return new URLSearchParams(search.startsWith('?') ? search.slice(1) : search).has('jobId');
  }
  if (typeof search !== 'object') return false;
  const value = (search as Record<string, unknown>).jobId;
  return typeof value === 'string' && value.trim().length > 0;
}

export function shouldPollTaskNotifications(pathname: string, search: unknown): boolean {
  if (pathname === '/tasks' || pathname.startsWith('/tasks/')) return false;
  if (hasTaskJobSearch(search)) return false;
  return true;
}

export function TaskNotificationListener() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const status = useCloudAuthStore((s) => s.status);
  const initialized = useCloudAuthStore((s) => s.initialized);
  const refreshNotifications = useCodegenJobStore((s) => s.refreshNotifications);
  const notifyUnreadCompletions = useCodegenJobStore((s) => s.notifyUnreadCompletions);
  const search =
    typeof (location as { searchStr?: unknown }).searchStr === 'string'
      ? (location as { searchStr: string }).searchStr
      : (location as { search?: unknown }).search;
  const shouldPoll = shouldPollTaskNotifications(location.pathname, search);

  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (
        event as CustomEvent<{
          title: string;
          message: string;
          notification: { id: string; fileId?: string | null };
        }>
      ).detail;
      toast(detail.title, {
        id: detail.notification.id,
        description: detail.message,
        action: (
          <Button
            type="button"
            size="sm"
            variant="outline"
            onClick={() => {
              const fileId = detail.notification.fileId ?? undefined;
              void navigate({
                to: '/tasks',
                search: { ...(fileId ? { fileId } : {}), view: 'notifications' },
              });
            }}
          >
            {t('tasks.notification.open')}
          </Button>
        ),
      });
    };
    window.addEventListener('openpencil:task-notification', handler);
    return () => {
      window.removeEventListener('openpencil:task-notification', handler);
    };
  }, [navigate, t]);

  useEffect(() => {
    const cleanup = window.electronAPI?.onMenuAction?.((action) => {
      if (!action.startsWith('navigate:')) return;
      const route = action.slice('navigate:'.length);
      if (route.startsWith('/tasks?')) {
        const params = new URLSearchParams(route.slice('/tasks?'.length));
        const fileId = params.get('fileId') ?? undefined;
        void navigate({
          to: '/tasks',
          search: { ...(fileId ? { fileId } : {}), view: 'notifications' },
        });
        return;
      }
      if (route === '/tasks') {
        void navigate({ to: '/tasks' });
        return;
      }
      const taskMatch = /^\/tasks\/([^/]+)$/.exec(route);
      if (taskMatch) {
        void navigate({ to: '/tasks', search: { jobId: taskMatch[1] } });
        return;
      }
      const match = /^\/editor\/([^/]+)$/.exec(route);
      if (match) {
        void navigate({ to: '/editor/$fileId', params: { fileId: match[1] } });
      }
    });
    return () => cleanup?.();
  }, [navigate]);

  useEffect(() => {
    if (!initialized || status !== 'authenticated' || !shouldPoll) return;
    let cancelled = false;
    const tick = async () => {
      if (cancelled) return;
      await refreshNotifications({ limit: 10 });
      if (!cancelled) await notifyUnreadCompletions();
    };
    const initialTimer = window.setTimeout(() => void tick(), INITIAL_NOTIFICATION_POLL_DELAY_MS);
    const intervalTimer = window.setInterval(
      () => void tick(),
      NOTIFICATION_POLL_INTERVAL_MS,
    );
    return () => {
      cancelled = true;
      window.clearTimeout(initialTimer);
      window.clearInterval(intervalTimer);
    };
  }, [initialized, notifyUnreadCompletions, refreshNotifications, shouldPoll, status]);

  return null;
}
