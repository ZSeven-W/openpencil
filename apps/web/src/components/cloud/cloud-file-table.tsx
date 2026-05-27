import { useMemo } from 'react';
import {
  ArchiveRestore,
  Copy,
  FileJson,
  FolderInput,
  MoreHorizontal,
  Pencil,
  Share2,
  Star,
  Trash2,
} from 'lucide-react';
import {
  flexRender,
  getCoreRowModel,
  useReactTable,
  type ColumnDef,
} from '@tanstack/react-table';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { Checkbox } from '@/components/ui/checkbox';
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
import { CloudFileActionsMenu } from './cloud-file-actions-menu';
import { cn } from '@/lib/utils';
import type { CloudFileSummary } from '@/types/cloud';

interface CloudFileTableProps {
  files: CloudFileSummary[];
  selectedFileIds: string[];
  actionsFileId: string | null;
  operatingIds: Record<string, string | undefined>;
  isTrashView: boolean;
  canMoveFile: (file: CloudFileSummary) => boolean;
  formatDate: (value: string) => string;
  folderPath: (folderId: string | null) => string;
  onToggleSelected: (fileId: string) => void;
  onOpen: (fileId: string) => void;
  onShowDetails: (fileId: string) => void;
  onToggleFavorite: (file: CloudFileSummary) => void;
  onRename: (file: CloudFileSummary) => void;
  onCopy: (file: CloudFileSummary) => void;
  onMove: (file: CloudFileSummary) => void;
  onShare: (file: CloudFileSummary) => void;
  onDelete: (file: CloudFileSummary) => void;
  onRestore: (file: CloudFileSummary) => void;
  onPermanentDelete: (file: CloudFileSummary) => void;
  onToggleActions: (fileId: string | null) => void;
  onToggleAll?: (fileIds: string[]) => void;
  pagination?: {
    page: number;
    pageSize: number;
    total: number;
    onPageChange: (page: number) => void;
  };
}

const headerClassByColumn: Record<string, string> = {
  select: 'w-10',
  name: 'min-w-[200px]',
  revision: 'w-[110px]',
  updated: 'w-[150px]',
  actions: 'w-[200px] text-right',
};

const cellClassByColumn: Record<string, string> = {
  revision: 'text-xs text-muted-foreground',
  updated: 'text-xs text-muted-foreground',
};

export function CloudFileTable({
  files,
  selectedFileIds,
  actionsFileId,
  operatingIds,
  isTrashView,
  canMoveFile,
  formatDate,
  folderPath,
  onToggleSelected,
  onOpen,
  onShowDetails,
  onToggleFavorite,
  onRename,
  onCopy,
  onMove,
  onShare,
  onDelete,
  onRestore,
  onPermanentDelete,
  onToggleActions,
  onToggleAll,
  pagination,
}: CloudFileTableProps) {
  const { t } = useTranslation();
  const allSelected = files.length > 0 && files.every((file) => selectedFileIds.includes(file.id));
  const someSelected = files.some((file) => selectedFileIds.includes(file.id));
  const columns = useMemo<ColumnDef<CloudFileSummary>[]>(
    () => [
      {
        id: 'select',
        header: () => (
          <Checkbox
            aria-label={t('common.selectAll')}
            checked={allSelected ? true : someSelected ? 'indeterminate' : false}
            onCheckedChange={() => onToggleAll?.(files.map((file) => file.id))}
          />
        ),
        cell: ({ row }) => {
          const file = row.original;
          return (
            <Checkbox
              aria-label={t('cloudLibrary.action.selectFile', { name: file.name })}
              checked={selectedFileIds.includes(file.id)}
              onCheckedChange={() => onToggleSelected(file.id)}
            />
          );
        },
      },
      {
        id: 'name',
        header: () => t('common.name'),
        cell: ({ row }) => {
          const file = row.original;
          return (
            <Button
              type="button"
              variant="ghost"
              aria-label={t('cloudLibrary.action.openFile', { name: file.name })}
              className="h-auto min-w-0 justify-start p-0 text-left"
              onClick={() => onOpen(file.id)}
              disabled={isTrashView}
            >
              <span className="flex size-7 shrink-0 items-center justify-center rounded border border-border bg-background">
                <FileJson size={15} className="text-muted-foreground" />
              </span>
              <span className="min-w-0">
                <span className="block truncate text-xs font-medium">{file.name}</span>
                <span className="block truncate text-xs text-muted-foreground">
                  {folderPath(file.folderId)}
                </span>
              </span>
            </Button>
          );
        },
      },
      {
        id: 'revision',
        header: () => t('cloudLibrary.details.revision'),
        cell: ({ row }) => `rev ${row.original.revision}`,
      },
      {
        id: 'updated',
        header: () => t('cloudLibrary.details.updated'),
        cell: ({ row }) => formatDate(row.original.updatedAt),
      },
      {
        id: 'actions',
        header: () => t('cloudLibrary.table.actions'),
        cell: ({ row }) => {
          const file = row.original;
          const busy = Boolean(operatingIds[file.id]);
          const canMove = canMoveFile(file);
          return (
            <>
              <div className="flex justify-end gap-1">
                {isTrashView ? (
                  <>
                    <IconButton
                      label={t('cloudLibrary.action.restoreFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onRestore(file)}
                    >
                      <ArchiveRestore size={14} />
                    </IconButton>
                    <ConfirmIconButton
                      label={t('cloudLibrary.action.permanentlyDeleteFile', { name: file.name })}
                      disabled={busy}
                      title={t('cloudLibrary.confirm.permanentlyDeleteFile', { name: file.name })}
                      onConfirm={() => onPermanentDelete(file)}
                    >
                      <Trash2 size={14} />
                    </ConfirmIconButton>
                  </>
                ) : (
                  <>
                    <IconButton
                      label={t('cloudLibrary.action.detailsFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onShowDetails(file.id)}
                    >
                      <FileJson size={14} />
                    </IconButton>
                    <IconButton
                      label={t('cloudLibrary.action.favoriteFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onToggleFavorite(file)}
                    >
                      <Star size={14} className={cn(file.starred && 'fill-current text-primary')} />
                    </IconButton>
                    <IconButton
                      label={t('cloudLibrary.action.renameFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onRename(file)}
                    >
                      <Pencil size={14} />
                    </IconButton>
                    <IconButton
                      label={t('cloudLibrary.action.copyFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onCopy(file)}
                    >
                      <Copy size={14} />
                    </IconButton>
                    {canMove && (
                      <IconButton
                        label={t('cloudLibrary.action.moveFile', { name: file.name })}
                        disabled={busy}
                        onClick={() => onMove(file)}
                      >
                        <FolderInput size={14} />
                      </IconButton>
                    )}
                    <IconButton
                      label={t('cloudLibrary.action.shareFile', { name: file.name })}
                      disabled={busy}
                      onClick={() => onShare(file)}
                    >
                      <Share2 size={14} />
                    </IconButton>
                    <CloudFileActionsMenu
                      file={file}
                      open={actionsFileId === file.id}
                      onOpenChange={(open) => onToggleActions(open ? file.id : null)}
                      trigger={
                        <Button
                          variant="ghost"
                          size="icon-sm"
                          aria-label={t('cloudLibrary.action.moreActionsFor', { name: file.name })}
                          disabled={busy}
                        >
                          <MoreHorizontal size={14} />
                        </Button>
                      }
                      isTrash={isTrashView}
                      canMove={canMove}
                      onOpen={() => onOpen(file.id)}
                      onDetails={() => onShowDetails(file.id)}
                      onFavorite={() => onToggleFavorite(file)}
                      onRename={() => onRename(file)}
                      onCopy={() => onCopy(file)}
                      onMove={() => onMove(file)}
                      onShare={() => onShare(file)}
                      onDelete={() => onDelete(file)}
                      onRestore={() => onRestore(file)}
                      onPermanentDelete={() => onPermanentDelete(file)}
                    />
                    <ConfirmIconButton
                      label={t('cloudLibrary.action.deleteFile', { name: file.name })}
                      disabled={busy}
                      title={t('cloudLibrary.confirm.deleteFile', { name: file.name })}
                      onConfirm={() => onDelete(file)}
                    >
                      <Trash2 size={14} />
                    </ConfirmIconButton>
                  </>
                )}
              </div>
            </>
          );
        },
      },
    ],
    [
      actionsFileId,
      allSelected,
      canMoveFile,
      files,
      folderPath,
      formatDate,
      isTrashView,
      onCopy,
      onDelete,
      onMove,
      onOpen,
      onPermanentDelete,
      onRename,
      onRestore,
      onShare,
      onShowDetails,
      onToggleActions,
      onToggleAll,
      onToggleFavorite,
      onToggleSelected,
      operatingIds,
      selectedFileIds,
      someSelected,
      t,
    ],
  );
  const table = useReactTable({
    data: files,
    columns,
    getCoreRowModel: getCoreRowModel(),
    getRowId: (row) => row.id,
  });

  return (
    <div className="overflow-hidden rounded border border-border bg-card">
      <Table className="min-w-[720px]">
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
              className="relative h-10"
              data-state={selectedFileIds.includes(row.original.id) ? 'selected' : undefined}
              onContextMenu={(event) => {
                event.preventDefault();
                onToggleActions(row.original.id);
              }}
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
      {pagination && (
        <div className="border-t border-border px-2.5 py-1.5">
          <WorkbenchPagination
            page={pagination.page}
            pageSize={pagination.pageSize}
            total={pagination.total}
            onPageChange={pagination.onPageChange}
            labels={{
              range: (values) => t('tasks.pageRange', values),
              previous: t('tasks.previousPage'),
              next: t('tasks.nextPage'),
            }}
          />
        </div>
      )}
    </div>
  );
}

function ConfirmIconButton({
  label,
  disabled,
  title,
  children,
  onConfirm,
}: {
  label: string;
  disabled: boolean;
  title: string;
  children: React.ReactNode;
  onConfirm: () => void;
}) {
  const { t } = useTranslation();
  return (
    <ConfirmActionButton
      variant="ghost"
      size="icon-sm"
      ariaLabel={label}
      disabled={disabled}
      title={title}
      description={title}
      confirmLabel={t('common.delete')}
      cancelLabel={t('common.cancel')}
      className="text-muted-foreground hover:text-destructive"
      onConfirm={onConfirm}
    >
      {children}
    </ConfirmActionButton>
  );
}

function IconButton({
  label,
  disabled,
  destructive,
  children,
  onClick,
}: {
  label: string;
  disabled: boolean;
  destructive?: boolean;
  children: React.ReactNode;
  onClick: () => void;
}) {
  return (
    <Button
      variant="ghost"
      size="icon-sm"
      aria-label={label}
      disabled={disabled}
      className={destructive ? 'text-muted-foreground hover:text-destructive' : undefined}
      onClick={onClick}
    >
      {children}
    </Button>
  );
}
