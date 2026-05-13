import { Cloud, FilePlus2, FolderOpen, HardDrive } from 'lucide-react';
import { useNavigate } from '@tanstack/react-router';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useDocumentStore } from '@/stores/document-store';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';

export function DesktopStartPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();

  const openCloudFiles = () => {
    void navigate({ to: '/cloud' });
  };

  const createLocalFile = () => {
    useDocumentStore.getState().newDocument();
    void navigate({ to: '/editor/local' });
  };

  const openLocalFile = async () => {
    const result = await window.electronAPI?.openFile?.();
    if (!result) return;

    const fileName = result.filePath.split(/[/\\]/).pop() || 'untitled.op';
    const prepared = parseAndPrepareImportedDocument(result.content, {
      fileName,
      filePath: result.filePath,
    });
    if (!prepared) return;

    useDocumentStore.getState().loadDocument(prepared.doc, fileName, null, result.filePath);
    void navigate({ to: '/editor/local' });
  };

  return (
    <div className="min-h-screen bg-background text-foreground">
      <div className="mx-auto flex min-h-screen max-w-5xl flex-col justify-center px-6 py-10">
        <div className="mb-8">
          <div className="flex items-center gap-3">
            <div className="flex h-10 w-10 items-center justify-center border border-border bg-card">
              <HardDrive className="h-5 w-5 text-primary" />
            </div>
            <div>
              <h1 className="text-xl font-semibold tracking-normal">
                {t('desktopStart.title')}
              </h1>
              <p className="mt-1 text-sm text-muted-foreground">
                {t('desktopStart.subtitle')}
              </p>
            </div>
          </div>
        </div>

        <div className="grid gap-3 md:grid-cols-2">
          <section className="border border-border bg-card p-5">
            <div className="flex items-start gap-3">
              <div className="flex h-9 w-9 items-center justify-center border border-border bg-background">
                <Cloud className="h-4 w-4 text-primary" />
              </div>
              <div className="min-w-0">
                <h2 className="text-sm font-semibold">{t('desktopStart.cloudTitle')}</h2>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">
                  {t('desktopStart.cloudDescription')}
                </p>
              </div>
            </div>
            <Button className="mt-5 w-full justify-center" onClick={openCloudFiles}>
              <Cloud className="h-4 w-4" />
              {t('desktopStart.openCloudFiles')}
            </Button>
          </section>

          <section className="border border-border bg-card p-5">
            <div className="flex items-start gap-3">
              <div className="flex h-9 w-9 items-center justify-center border border-border bg-background">
                <FolderOpen className="h-4 w-4 text-primary" />
              </div>
              <div className="min-w-0">
                <h2 className="text-sm font-semibold">{t('desktopStart.localTitle')}</h2>
                <p className="mt-1 text-sm leading-6 text-muted-foreground">
                  {t('desktopStart.localDescription')}
                </p>
              </div>
            </div>
            <div className="mt-5 grid grid-cols-2 gap-2">
              <Button variant="outline" className="justify-center" onClick={createLocalFile}>
                <FilePlus2 className="h-4 w-4" />
                {t('desktopStart.newLocalFile')}
              </Button>
              <Button variant="outline" className="justify-center" onClick={() => void openLocalFile()}>
                <FolderOpen className="h-4 w-4" />
                {t('desktopStart.openLocalFile')}
              </Button>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
}
