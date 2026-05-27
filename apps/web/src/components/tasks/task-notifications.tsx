import { Trash2 } from 'lucide-react';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table';
import { ConfirmActionButton } from '@/components/workbench/confirm-action-button';
import { WorkbenchPagination } from '@/components/workbench/workbench-pagination';
import type { TaskNotification } from '@/types/cloud';
import { formatTime, NOTIFICATION_PAGE_SIZE, renderNotificationCopy } from './task-center-utils';

interface NotificationRecordsProps {
  notifications: TaskNotification[];
  total: number;
  page: number;
  onPageChange: (page: number) => void;
  onOpen: (notification: TaskNotification) => void;
  onDelete: (id: string) => void;
  t: ReturnType<typeof useTranslation>['t'];
}

export function NotificationRecords({
  notifications,
  total,
  page,
  onPageChange,
  onOpen,
  onDelete,
  t,
}: NotificationRecordsProps) {
  return (
    <div className="overflow-hidden rounded border border-border bg-card">
      <div className="flex min-h-8 items-center justify-between border-b border-border px-3 py-2">
        <div>
          <h2 className="text-sm font-semibold">{t('tasks.notificationRecords')}</h2>
          <p className="mt-1 text-xs text-muted-foreground">
            {t('tasks.notificationRecordsSubtitle')}
          </p>
        </div>
      </div>
      {total === 0 ? (
        <div className="p-8 text-center text-sm text-muted-foreground">
          {t('tasks.notificationsEmpty')}
        </div>
      ) : (
        <>
          <Table className="min-w-[620px]">
            <TableHeader>
              <TableRow className="h-8 hover:bg-transparent">
                <TableHead>{t('tasks.notifications')}</TableHead>
                <TableHead className="w-[150px]">{t('tasks.table.created')}</TableHead>
                <TableHead className="w-[150px] text-right">
                  {t('cloudLibrary.table.actions')}
                </TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {notifications.map((item) => {
                const copy = renderNotificationCopy(item, t);
                return (
                  <TableRow key={item.id} data-state={!item.readAt ? 'selected' : undefined}>
                    <TableCell>
                      <Button
                        type="button"
                        variant="ghost"
                        className="h-auto min-w-0 justify-start p-0 text-left"
                        onClick={() => onOpen(item)}
                      >
                        <span className="min-w-0">
                          <span className="block truncate text-sm font-medium">{copy.title}</span>
                          {copy.message && (
                            <span className="mt-1 block truncate text-xs text-muted-foreground">
                              {copy.message}
                            </span>
                          )}
                        </span>
                      </Button>
                    </TableCell>
                    <TableCell className="text-xs text-muted-foreground">
                      {formatTime(item.createdAt)}
                    </TableCell>
                    <TableCell>
                      <div className="flex justify-end gap-1">
                        <Button variant="ghost" size="sm" onClick={() => onOpen(item)}>
                          {t('tasks.notification.open')}
                        </Button>
                        <ConfirmActionButton
                          variant="ghost"
                          size="icon-sm"
                          ariaLabel={t('tasks.deleteNotification')}
                          title={t('tasks.deleteNotification')}
                          description={t('tasks.confirm.deleteNotification')}
                          confirmLabel={t('common.delete')}
                          cancelLabel={t('common.cancel')}
                          className="text-muted-foreground hover:text-destructive"
                          onConfirm={() => onDelete(item.id)}
                        >
                          <Trash2 />
                        </ConfirmActionButton>
                      </div>
                    </TableCell>
                  </TableRow>
                );
              })}
            </TableBody>
          </Table>
          <div className="border-t border-border px-2.5 py-1.5">
            <WorkbenchPagination
              page={page}
              pageSize={NOTIFICATION_PAGE_SIZE}
              total={total}
              onPageChange={onPageChange}
              labels={{
                range: (values) => t('tasks.pageRange', values),
                previous: t('tasks.previousPage'),
                next: t('tasks.nextPage'),
              }}
            />
          </div>
        </>
      )}
    </div>
  );
}
