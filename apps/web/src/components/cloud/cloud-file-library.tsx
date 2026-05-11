import { useEffect, useRef, useState } from 'react';
import { useNavigate } from '@tanstack/react-router';
import { FileJson, LogOut, Plus, RefreshCw, Trash2, Upload } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { useCloudAuthStore } from '@/stores/cloud-auth-store';
import { useCloudFileStore } from '@/stores/cloud-file-store';
import { createEmptyDocument } from '@/stores/document-store';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import type { PenDocument } from '@/types/pen';

function formatDate(value: string): string {
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

export function CloudFileLibrary() {
  const navigate = useNavigate();
  const user = useCloudAuthStore((s) => s.user);
  const signOut = useCloudAuthStore((s) => s.signOut);
  const files = useCloudFileStore((s) => s.files);
  const loading = useCloudFileStore((s) => s.loading);
  const error = useCloudFileStore((s) => s.error);
  const loadFiles = useCloudFileStore((s) => s.loadFiles);
  const createFile = useCloudFileStore((s) => s.createFile);
  const deleteFile = useCloudFileStore((s) => s.deleteFile);
  const resetFiles = useCloudFileStore((s) => s.reset);
  const inputRef = useRef<HTMLInputElement>(null);
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    void loadFiles();
  }, [loadFiles]);

  const openFile = (fileId: string) => {
    void navigate({ to: '/editor/$fileId', params: { fileId } });
  };

  const createNew = async () => {
    setCreating(true);
    try {
      const id = await createFile('Untitled', createEmptyDocument() as PenDocument);
      if (id) openFile(id);
    } finally {
      setCreating(false);
    }
  };

  const importOp = async (file: File) => {
    const text = await file.text();
    const prepared = parseAndPrepareImportedDocument(text, { fileName: file.name });
    if (!prepared) return;
    const name = file.name.replace(/\.(pen|op|json)$/i, '') || 'Imported design';
    const id = await createFile(name, prepared.doc, 'import');
    if (id) openFile(id);
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <header className="border-b border-border bg-card">
        <div className="mx-auto flex h-14 max-w-6xl items-center justify-between px-6">
          <div className="flex items-center gap-3">
            <FileJson size={22} className="text-primary" />
            <div>
              <h1 className="text-sm font-semibold">OpenPencil Cloud</h1>
              <p className="text-xs text-muted-foreground">{user?.email}</p>
            </div>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="sm" onClick={() => void loadFiles()}>
              <RefreshCw size={15} />
              Refresh
            </Button>
            <Button variant="ghost" size="sm" onClick={() => inputRef.current?.click()}>
              <Upload size={15} />
              Import .op
            </Button>
            <Button size="sm" onClick={() => void createNew()} disabled={creating}>
              <Plus size={15} />
              New
            </Button>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={() => {
                resetFiles();
                void signOut();
              }}
              aria-label="Sign out"
            >
              <LogOut size={15} />
            </Button>
          </div>
          <input
            ref={inputRef}
            type="file"
            accept=".op,.pen,.json"
            className="hidden"
            onChange={(event) => {
              const file = event.currentTarget.files?.[0];
              event.currentTarget.value = '';
              if (file) void importOp(file);
            }}
          />
        </div>
      </header>

      <main className="mx-auto max-w-6xl px-6 py-8">
        {error && (
          <div className="mb-4 border border-destructive/40 bg-destructive/10 px-3 py-2 text-sm text-destructive">
            {error}
          </div>
        )}

        {loading && files.length === 0 ? (
          <div className="py-20 text-center text-sm text-muted-foreground">Loading files...</div>
        ) : files.length === 0 ? (
          <div className="border border-border bg-card p-10 text-center">
            <p className="text-sm font-medium">No cloud files yet</p>
            <p className="mt-1 text-sm text-muted-foreground">
              Create a new design or import an existing `.op` file.
            </p>
            <div className="mt-5 flex justify-center gap-2">
              <Button onClick={() => void createNew()}>
                <Plus size={15} />
                New design
              </Button>
              <Button variant="outline" onClick={() => inputRef.current?.click()}>
                <Upload size={15} />
                Import .op
              </Button>
            </div>
          </div>
        ) : (
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3">
            {files.map((file) => (
              <div
                key={file.id}
                className="group border border-border bg-card p-4 transition-colors hover:border-primary/50"
              >
                <button
                  type="button"
                  className="block w-full text-left"
                  onClick={() => openFile(file.id)}
                >
                  <div className="mb-4 flex h-28 items-center justify-center border border-border bg-background">
                    <FileJson size={28} className="text-muted-foreground" />
                  </div>
                  <p className="truncate text-sm font-medium">{file.name}</p>
                  <p className="mt-1 text-xs text-muted-foreground">
                    rev {file.revision} · {formatDate(file.updatedAt)}
                  </p>
                </button>
                <div className="mt-3 flex justify-end">
                  <Button
                    variant="ghost"
                    size="icon-sm"
                    className="text-muted-foreground hover:text-destructive"
                    aria-label={`Delete ${file.name}`}
                    onClick={() => void deleteFile(file.id)}
                  >
                    <Trash2 size={14} />
                  </Button>
                </div>
              </div>
            ))}
          </div>
        )}
      </main>
    </div>
  );
}

