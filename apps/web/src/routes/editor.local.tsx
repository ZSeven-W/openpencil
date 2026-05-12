import { createFileRoute } from '@tanstack/react-router';
import EditorLayout from '@/components/editor/editor-layout';
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts';
import { useBeforeUnload } from '@/hooks/use-before-unload';

export const Route = createFileRoute('/editor/local')({
  component: LocalEditorPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Local Editor' }],
  }),
});

function LocalEditorPage() {
  useKeyboardShortcuts();
  useBeforeUnload();
  return <EditorLayout />;
}
