import { Clock3, Grid2X2, Share2, Star, Trash2 } from 'lucide-react';
import type { CloudFileSort, CloudFileView, CloudFolder } from '@/types/cloud';

export const viewItems: Array<{ id: CloudFileView; labelKey: string; icon: typeof Grid2X2 }> = [
  { id: 'all', labelKey: 'cloudLibrary.view.all', icon: Grid2X2 },
  { id: 'recent', labelKey: 'cloudLibrary.view.recent', icon: Clock3 },
  { id: 'starred', labelKey: 'cloudLibrary.view.starred', icon: Star },
  { id: 'shared', labelKey: 'cloudLibrary.view.shared', icon: Share2 },
  { id: 'trash', labelKey: 'cloudLibrary.view.trash', icon: Trash2 },
];

export const sortLabelKeys: Record<CloudFileSort, string> = {
  updated_desc: 'cloudLibrary.sort.updated_desc',
  updated_asc: 'cloudLibrary.sort.updated_asc',
  name_asc: 'cloudLibrary.sort.name_asc',
  name_desc: 'cloudLibrary.sort.name_desc',
  created_desc: 'cloudLibrary.sort.created_desc',
};

export function formatDate(value: string): string {
  try {
    return new Intl.DateTimeFormat(undefined, {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value));
  } catch {
    return value;
  }
}

export function childFolders(folders: CloudFolder[], parentId: string | null): CloudFolder[] {
  return folders
    .filter((folder) => folder.parentId === parentId)
    .sort((a, b) => a.sortOrder - b.sortOrder || a.name.localeCompare(b.name));
}

export function folderPath(
  folders: CloudFolder[],
  folderId: string | null,
  rootLabel: string,
  fallbackLabel: string,
): string {
  if (!folderId) return rootLabel;
  const byId = new Map(folders.map((folder) => [folder.id, folder]));
  const names: string[] = [];
  let current = byId.get(folderId);
  while (current) {
    names.unshift(current.name);
    current = current.parentId ? byId.get(current.parentId) : undefined;
  }
  return names.length > 0 ? names.join(' / ') : fallbackLabel;
}

export function emptyStateForView(view: CloudFileView) {
  if (view === 'trash') {
    return {
      titleKey: 'cloudLibrary.empty.trash.title',
      descriptionKey: 'cloudLibrary.empty.trash.description',
      showCreateActions: false,
    };
  }
  if (view === 'shared') {
    return {
      titleKey: 'cloudLibrary.empty.shared.title',
      descriptionKey: 'cloudLibrary.empty.shared.description',
      showCreateActions: false,
    };
  }
  return {
    titleKey: 'cloudLibrary.empty.default.title',
    descriptionKey: 'cloudLibrary.empty.default.description',
    showCreateActions: true,
  };
}
