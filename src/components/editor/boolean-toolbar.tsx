import { useCallback } from 'react'
import { useTranslation } from 'react-i18next'
import {
  SquaresUnite,
  SquaresSubtract,
  SquaresIntersect,
} from 'lucide-react'
import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from '@/components/ui/tooltip'
import { useCanvasStore } from '@/stores/canvas-store'
import { useDocumentStore } from '@/stores/document-store'
import { useHistoryStore } from '@/stores/history-store'
import type { PenNode } from '@/types/pen'
import {
  canBooleanOp,
  executeBooleanOp,
  type BooleanOpType,
} from '@/utils/boolean-ops'

const OPS = [
  { type: 'union' as BooleanOpType, icon: SquaresUnite, labelKey: 'layerMenu.booleanUnion', shortcut: '\u2318\u2325U' },
  { type: 'subtract' as BooleanOpType, icon: SquaresSubtract, labelKey: 'layerMenu.booleanSubtract', shortcut: '\u2318\u2325S' },
  { type: 'intersect' as BooleanOpType, icon: SquaresIntersect, labelKey: 'layerMenu.booleanIntersect', shortcut: '\u2318\u2325I' },
] as const

export default function BooleanToolbar() {
  const { t } = useTranslation()
  const selectedIds = useCanvasStore((s) => s.selection.selectedIds)

  const nodes = selectedIds
    .map((id) => useDocumentStore.getState().getNodeById(id))
    .filter((n): n is PenNode => n != null)

  const show = canBooleanOp(nodes)

  const handleOp = useCallback((opType: BooleanOpType) => {
    const { selectedIds } = useCanvasStore.getState().selection
    const currentNodes = selectedIds
      .map((id) => useDocumentStore.getState().getNodeById(id))
      .filter((n): n is PenNode => n != null)

    if (!canBooleanOp(currentNodes)) return
    const result = executeBooleanOp(currentNodes, opType)
    if (!result) return

    useHistoryStore.getState().pushState(useDocumentStore.getState().document)
    for (const id of selectedIds) {
      useDocumentStore.getState().removeNode(id)
    }
    useDocumentStore.getState().addNode(null, result)
    useCanvasStore.getState().setSelection([result.id], result.id)
  }, [])

  if (!show) return null

  return (
    <div className="absolute top-2 left-14 z-10 bg-card border border-border rounded-xl flex items-center py-1 px-1 gap-0.5 shadow-lg">
      {OPS.map((op) => (
        <Tooltip key={op.type}>
          <TooltipTrigger asChild>
            <button
              type="button"
              onClick={() => handleOp(op.type)}
              aria-label={t(op.labelKey)}
              className="inline-flex items-center justify-center h-7 w-7 rounded-lg transition-colors text-muted-foreground hover:bg-muted hover:text-foreground"
            >
              <op.icon size={16} strokeWidth={1.5} />
            </button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {t(op.labelKey)}
            <kbd className="ml-1.5 inline-flex h-4 items-center rounded border border-border/50 bg-muted px-1 font-mono text-[10px] text-muted-foreground">
              {op.shortcut}
            </kbd>
          </TooltipContent>
        </Tooltip>
      ))}
    </div>
  )
}
