import { useMemo, type MouseEvent } from 'react';
import { FileCode2, RotateCcw, Trash2 } from 'lucide-react';
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
} from '@tanstack/react-table';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Checkbox } from '@/components/ui/checkbox';
import { Input } from '@/components/ui/input';
import { Progress } from '@/components/ui/progress';
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
import type { CloudCodegenJob } from '@/types/cloud';
import {
  describeJobKind,
  describePipelineMode,
  clampProgress,
  durationBetween,
  formatDuration,
  formatProviderModel,
  formatTime,
  getTaskFileDisplay,
  getTaskPageDisplay,
  getVisibleStatus,
  isCancellable,
  StatusBadge,
  statusIcon,
  TASK_PAGE_SIZE,
} from './task-center-utils';

interface TaskTableProps {
  jobs: CloudCodegenJob[];
  total: number;
  page: number;
  selectedJobIds: Set<string>;
  allSelected: boolean;
  someSelected: boolean;
  priorityDrafts: Record<string, string>;
  onToggleSelected: (jobId: string) => void;
  onToggleSelectedPage: () => void;
  onPageChange: (page: number) => void;
  onCancel: (jobId: string) => void;
  onDelete: (jobId: string) => void;
  onRetry: (jobId: string) => void;
  onOpenFile: (job: CloudCodegenJob) => void;
  onOpenDetail: (jobId: string) => void;
  onPriorityChange: (jobId: string, value: string) => void;
  onSavePriority: (job: CloudCodegenJob) => void;
  t: ReturnType<typeof useTranslation>['t'];
}

function stopRowClick(event: MouseEvent<HTMLElement>) {
  event.stopPropagation();
}

const headerClassByColumn: Record<string, string> = {
  select: 'w-10',
  status: 'w-[128px]',
  file: 'w-[168px]',
  page: 'w-[140px]',
  model: 'w-[168px]',
  duration: 'w-[120px]',
  priority: 'w-[112px]',
  actions: 'w-[230px] text-right',
};

const cellClassByColumn: Record<string, string> = {
  status: 'text-xs text-muted-foreground',
  file: 'max-w-[180px] text-xs',
  page: 'max-w-[150px] text-xs',
  model: 'max-w-[180px] truncate text-xs text-muted-foreground',
  duration: 'text-xs text-muted-foreground',
};

export function TaskTable({
  jobs,
  total,
  page,
  selectedJobIds,
  allSelected,
  someSelected,
  priorityDrafts,
  onToggleSelected,
  onToggleSelectedPage,
  onPageChange,
  onCancel,
  onDelete,
  onRetry,
  onOpenFile,
  onOpenDetail,
  onPriorityChange,
  onSavePriority,
  t,
}: TaskTableProps) {
  const columns = useMemo<ColumnDef<CloudCodegenJob>[]>(
    () => [
      {
        id: 'select',
        header: () => (
          <Checkbox
            aria-label={t('common.selectAll')}
            checked={allSelected ? true : someSelected ? 'indeterminate' : false}
            onCheckedChange={() => onToggleSelectedPage()}
          />
        ),
        cell: ({ row }) => {
          const job = row.original;
          return (
            <div onClick={stopRowClick}>
              <Checkbox
                checked={selectedJobIds.has(job.id)}
                onCheckedChange={() => onToggleSelected(job.id)}
                aria-label={t('tasks.selectJob', { id: job.id })}
              />
            </div>
          );
        },
      },
      {
        id: 'task',
        header: () => t('tasks.table.task'),
        cell: ({ row }) => {
          const job = row.original;
          return (
            <div className="min-w-0">
              <div className="flex items-center gap-2">
                {statusIcon(job.status)}
                <h2 className="truncate text-xs font-semibold">
                  {t('tasks.codegenTitle', { framework: job.framework })}
                </h2>
                <Badge variant="outline">{describeJobKind(job, t)}</Badge>
                <Badge variant="secondary">{describePipelineMode(job, t)}</Badge>
              </div>
              <p className="mt-1 text-xs text-muted-foreground">
                {formatTime(job.createdAt)}
                {job.attempts > 0 &&
                  ` · ${t('tasks.attempts', {
                    attempts: job.attempts,
                    maxAttempts: job.maxAttempts,
                  })}`}
              </p>
              <Progress className="mt-2 h-1" value={clampProgress(job.progress)} />
              {job.error && <p className="mt-2 text-xs text-destructive">{job.error}</p>}
            </div>
          );
        },
      },
      {
        id: 'status',
        header: () => t('tasks.table.status'),
        cell: ({ row }) => (
          <StatusBadge status={row.original.status}>
            {getVisibleStatus(row.original, t)}
          </StatusBadge>
        ),
      },
      {
        id: 'file',
        header: () => t('tasks.table.file'),
        cell: ({ row }) => <TaskTargetCell {...getTaskFileDisplay(row.original)} />,
      },
      {
        id: 'page',
        header: () => t('tasks.table.page'),
        cell: ({ row }) => <TaskTargetCell {...getTaskPageDisplay(row.original)} />,
      },
      {
        id: 'model',
        header: () => t('tasks.table.model'),
        cell: ({ row }) => formatProviderModel(row.original),
      },
      {
        id: 'duration',
        header: () => t('tasks.table.duration'),
        cell: ({ row }) =>
          formatDuration(
            durationBetween(row.original.startedAt ?? row.original.createdAt, row.original.completedAt),
          ),
      },
      {
        id: 'priority',
        header: () => t('tasks.priority'),
        cell: ({ row }) => {
          const job = row.original;
          return (
            <div onClick={stopRowClick}>
              {job.status === 'pending' ? (
                <div className="flex items-center gap-1">
                  <label className="sr-only" htmlFor={`priority-${job.id}`}>
                    {t('tasks.priority')}
                  </label>
                  <Input
                    id={`priority-${job.id}`}
                    type="number"
                    className="h-7 w-16 text-xs"
                    value={priorityDrafts[job.id] ?? String(job.priority)}
                    onChange={(event) => onPriorityChange(job.id, event.target.value)}
                  />
                  <Button variant="outline" size="sm" onClick={() => onSavePriority(job)}>
                    {t('tasks.savePriority')}
                  </Button>
                </div>
              ) : (
                <span className="text-xs text-muted-foreground">{job.priority}</span>
              )}
            </div>
          );
        },
      },
      {
        id: 'actions',
        header: () => t('cloudLibrary.table.actions'),
        cell: ({ row }) => {
          const job = row.original;
          return (
            <div className="flex justify-end gap-1" onClick={stopRowClick}>
              {job.status === 'failed' && (
                <Button variant="outline" size="sm" onClick={() => onRetry(job.id)}>
                  <RotateCcw data-icon="inline-start" />
                  {t('tasks.retry')}
                </Button>
              )}
              {isCancellable(job) && (
                <Button variant="outline" size="sm" onClick={() => onCancel(job.id)}>
                  {t('tasks.cancel')}
                </Button>
              )}
              <Button
                variant="ghost"
                size="icon-sm"
                aria-label={t('tasks.openFile')}
                onClick={() => onOpenFile(job)}
              >
                <FileCode2 />
              </Button>
              <Button variant="ghost" size="sm" onClick={() => onOpenDetail(job.id)}>
                {t('tasks.details')}
              </Button>
              <ConfirmActionButton
                variant="ghost"
                size="icon-sm"
                ariaLabel={t('tasks.deleteTask')}
                title={t('tasks.deleteTask')}
                description={t('tasks.confirm.deleteTask')}
                confirmLabel={t('common.delete')}
                cancelLabel={t('common.cancel')}
                className="text-muted-foreground hover:text-destructive"
                onConfirm={() => onDelete(job.id)}
              >
                <Trash2 />
              </ConfirmActionButton>
            </div>
          );
        },
      },
    ],
    [
      allSelected,
      onCancel,
      onDelete,
      onOpenDetail,
      onOpenFile,
      onPriorityChange,
      onRetry,
      onSavePriority,
      onToggleSelected,
      onToggleSelectedPage,
      priorityDrafts,
      selectedJobIds,
      someSelected,
      t,
    ],
  );
  const table = useReactTable({
    data: jobs,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
  });

  return (
    <div className="overflow-hidden rounded border border-border bg-card">
      <Table className="min-w-[1160px]">
        <TableHeader>
          {table.getHeaderGroups().map((headerGroup) => (
            <TableRow key={headerGroup.id} className="h-8 hover:bg-transparent">
              {headerGroup.headers.map((header) => (
                <TableHead key={header.id} className={headerClassByColumn[header.column.id]}>
                  {header.isPlaceholder
                    ? null
                    : flexRender(header.column.columnDef.header, header.getContext())}
                </TableHead>
              ))}
            </TableRow>
          ))}
        </TableHeader>
        <TableBody>
          {table.getRowModel().rows.map((row) => (
            <TableRow
              key={row.id}
              className="h-12 cursor-pointer"
              data-state={selectedJobIds.has(row.original.id) ? 'selected' : undefined}
              onClick={() => onOpenDetail(row.original.id)}
            >
              {row.getVisibleCells().map((cell) => (
                <TableCell key={cell.id} className={cellClassByColumn[cell.column.id]}>
                  {flexRender(cell.column.columnDef.cell, cell.getContext())}
                </TableCell>
              ))}
            </TableRow>
          ))}
        </TableBody>
      </Table>
      <div className="border-t border-border px-2.5 py-1.5">
        <WorkbenchPagination
          page={page}
          pageSize={TASK_PAGE_SIZE}
          total={total}
          onPageChange={onPageChange}
          labels={{
            range: (values) => t('tasks.pageRange', values),
            previous: t('tasks.previousPage'),
            next: t('tasks.nextPage'),
          }}
        />
      </div>
    </div>
  );
}

function TaskTargetCell({ primary, secondary }: { primary: string; secondary: string }) {
  return (
    <div className="min-w-0">
      <p className="truncate font-medium text-foreground">{primary}</p>
      {secondary && <p className="mt-0.5 truncate text-muted-foreground">{secondary}</p>}
    </div>
  );
}
