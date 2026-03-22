import { createFileRoute } from '@tanstack/react-router'
import EditorLayout from '@/components/editor/editor-layout'
import { useKeyboardShortcuts } from '@/hooks/use-keyboard-shortcuts'
import { useBeforeUnload } from '@/hooks/use-before-unload'

export const Route = createFileRoute('/editor')({
  component: EditorPage,
  ssr: false,
  head: () => ({
    meta: [{ title: 'OpenPencil Editor' }],
  }),
})

function EditorPage() {
  useKeyboardShortcuts()
  useBeforeUnload()

  return <EditorLayout />
}
