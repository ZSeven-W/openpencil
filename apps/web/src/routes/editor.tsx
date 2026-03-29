import { createFileRoute } from '@tanstack/react-router';
import { DesignProvider } from '@zseven-w/pen-react';
import EditorLayout from '@/components/editor/editor-layout';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { useBeforeUnload } from '@/hooks/use-before-unload';
import { useDocumentStore } from '@/stores/document-store';

export const Route = createFileRoute('/editor')({
  component: EditorPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
});

function EditorPage() {
  useKeyboardShortcuts();
  useBeforeUnload();

  const document = useDocumentStore((s) => s.document);
  const loadDocument = useDocumentStore((s) => s.applyExternalDocument);

  return (
    <DesignProvider document={document} onDocumentChange={loadDocument}>
      <EditorLayout />
    </DesignProvider>
  );
}
