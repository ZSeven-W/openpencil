import { createFileRoute } from '@tanstack/react-router'
import EditorLayout from '@/components/editor/EditorLayout'
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts'

export const Route = createFileRoute('/editor')({
  component: EditorPage,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
})

function EditorPage() {
  useKeyboardShortcuts()

  return <EditorLayout />
}
