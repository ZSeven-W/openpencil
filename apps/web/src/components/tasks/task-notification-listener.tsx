import { useEffect, useState } from 'react';
import { CheckCircle2, X } from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useCodegenJobStore } from '@/stores/codegen-job-store';
import type { TaskNotification } from '@/types/cloud';

interface ToastState {
  id: string;
  title: string;
  message: string;
  notification: TaskNotification;
}

export function TaskNotificationListener() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const status = useCloudAuthStore((s) => s.status);
  const initialized = useCloudAuthStore((s) => s.initialized);
  const refreshNotifications = useCodegenJobStore((s) => s.refreshNotifications);
  const notifyUnreadCompletions = useCodegenJobStore((s) => s.notifyUnreadCompletions);
  const [toast, setToast] = useState<ToastState | null>(null);

  useEffect(() => {
    const handler = (event: Event) => {
      const detail = (event as CustomEvent<Omit<ToastState, 'id'>>).detail;
      setToast({
        id: detail.notification.id,
        title: detail.title,
        message: detail.message,
        notification: detail.notification,
      });
    };
    window.addEventListener('openpencil:task-notification', handler);
    return () => {
      window.removeEventListener('openpencil:task-notification', handler);
    };
  }, []);

  useEffect(() => {
    const cleanup = window.electronAPI?.onMenuAction?.((action) => {
      if (!action.startsWith('navigate:')) return;
      const route = action.slice('navigate:'.length);
      if (route === '/tasks') {
        void navigate({ to: '/tasks' });
        return;
      }
      const taskMatch = /^\/tasks\/([^/]+)$/.exec(route);
      if (taskMatch) {
        void navigate({ to: '/tasks/$jobId', params: { jobId: taskMatch[1] } });
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
    if (!initialized || status !== 'authenticated') return;
    let cancelled = false;
    const tick = async () => {
      if (cancelled) return;
      await refreshNotifications();
      if (!cancelled) await notifyUnreadCompletions();
    };
    void tick();
    const timer = window.setInterval(() => void tick(), 15_000);
    return () => {
      cancelled = true;
      window.clearInterval(timer);
    };
  }, [initialized, notifyUnreadCompletions, refreshNotifications, status]);

  useEffect(() => {
    if (!toast) return;
    const timer = window.setTimeout(() => setToast(null), 6000);
    return () => window.clearTimeout(timer);
  }, [toast]);

  if (!toast) return null;

  const openTarget = () => {
    const jobId = toast.notification.jobId;
    if (jobId) {
      void navigate({ to: '/tasks/$jobId', params: { jobId } });
      setToast(null);
      return;
    }
    const fileId = toast.notification.fileId;
    if (fileId) {
      void navigate({ to: '/editor/$fileId', params: { fileId } });
    } else {
      void navigate({ to: '/tasks' });
    }
    setToast(null);
  };

  return (
    <div className="fixed bottom-4 right-4 z-50 w-[320px] rounded-md border border-border bg-card p-3 text-card-foreground shadow-lg">
      <div className="flex items-start gap-3">
        <CheckCircle2 className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
        <button type="button" className="min-w-0 flex-1 text-left" onClick={openTarget}>
          <p className="text-sm font-medium">{toast.title}</p>
          <p className="mt-1 text-xs text-muted-foreground">{toast.message}</p>
          <p className="mt-2 text-xs font-medium text-primary">{t('tasks.notification.open')}</p>
        </button>
        <Button
          variant="ghost"
          size="icon-sm"
          aria-label={t('common.close')}
          onClick={() => setToast(null)}
        >
          <X size={14} />
        </Button>
      </div>
    </div>
  );
}
