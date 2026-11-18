/** Live OpenPencil selection chip rendered above the DSH composer. */

import { useCallback, useSyncExternalStore } from 'react'
import type {} from '@deepseek-ai/dsh-client-ui-conversation/client'
import {
  getOpenPencilSelectionSnapshot,
  subscribeOpenPencilSelection,
  type OpenPencilLiveSelection,
} from './selection-store.js'
import type { PresentationLocale } from './index.js'

export const OPENPENCIL_SELECTION_DOCK_LAYOUT = {
  boxSizing: 'border-box',
  flex: 'none',
  width: 'calc(100% - var(--dsh-composer-side-clearance, 16px) - var(--dsh-composer-side-clearance, 16px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px))',
  maxWidth: 'calc(var(--dsh-composer-card-max-width, 780px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px) - var(--dsh-composer-dock-inset, 8px))',
  margin: '0 auto',
} as const satisfies React.CSSProperties

export function hasOpenPencilSelection(
  selection: OpenPencilLiveSelection | undefined,
): selection is OpenPencilLiveSelection {
  return (selection?.selectedIds.length ?? 0) > 0
}

export function selectionNodeLabel(selection: OpenPencilLiveSelection, locale: PresentationLocale): string {
  if (selection.selectedIds.length === 0) return locale === 'zh' ? '未选择画布节点' : 'No canvas node selected'
  if (selection.selectedIds.length > 1) {
    return locale === 'zh' ? `已选择 ${selection.selectedIds.length} 个节点` : `${selection.selectedIds.length} nodes selected`
  }
  const node = selection.nodes[0]
  return node?.name ?? node?.type ?? selection.selectedIds[0]!
}

export function selectionNodeDetail(selection: OpenPencilLiveSelection, locale: PresentationLocale): string {
  if (selection.selectedIds.length === 0) return locale === 'zh' ? '在右侧 OpenPencil 画布中选择节点' : 'Select a node on the OpenPencil canvas'
  const node = selection.nodes[0]
  if (selection.selectedIds.length > 1 || node === undefined) return selection.selectedIds.slice(0, 3).join(' · ')
  const dimensions = node.width === undefined || node.height === undefined
    ? undefined
    : `${Math.round(node.width)} × ${Math.round(node.height)}`
  return [node.type, dimensions, node.id].filter(Boolean).join(' · ')
}

const styles: Record<string, React.CSSProperties> = {
  root: {
    ...OPENPENCIL_SELECTION_DOCK_LAYOUT,
    display: 'flex', alignItems: 'center', gap: 10, minHeight: 42,
    padding: '7px 10px', border: '1px solid var(--dsw-alias-border-l2)',
    borderRadius: 9, background: 'var(--dsw-alias-bg-layer-1)', color: 'var(--dsw-alias-label-primary)',
  },
  icon: {
    width: 28, height: 28, flex: '0 0 28px', display: 'inline-flex', alignItems: 'center', justifyContent: 'center',
    borderRadius: 7, background: 'var(--dsw-alias-brand-primary, #3b82f6)', color: '#fff', fontSize: 15,
  },
  text: { minWidth: 0, display: 'flex', flexDirection: 'column', gap: 1 },
  title: { fontSize: 12, fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
  detail: { fontSize: 11, color: 'var(--dsw-alias-label-secondary)', overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' },
  target: { marginLeft: 'auto', fontSize: 11, color: 'var(--dsw-alias-label-secondary)', whiteSpace: 'nowrap' },
}

/**
 * Props the host passes to a `conversation.input.dock` entry, named locally.
 *
 * `PropsRuntime<'conversation.input.dock'>` cannot be used: the slot contract
 * lives in dsh-client-ui-conversation's `contract/slots.d.ts`, which its
 * `./client` entry does not re-export, so the SlotMap augmentation never
 * enters this compilation and the lookup degrades to `never`. The dock is a
 * session-scoped slot and this component reads only the session identity, so
 * that is all this declares.
 */
export type OpenPencilSelectionDockProps = {
  sessionId: string
  locale: PresentationLocale
}

export function OpenPencilSelectionDock({ sessionId, locale }: OpenPencilSelectionDockProps) {
  const subscribe = useCallback(
    (notify: () => void) => subscribeOpenPencilSelection(String(sessionId), notify),
    [sessionId],
  )
  const getSnapshot = useCallback(
    () => getOpenPencilSelectionSnapshot(String(sessionId)),
    [sessionId],
  )
  const snapshot = useSyncExternalStore(subscribe, getSnapshot, getSnapshot)
  const selection = snapshot.selection
  if (!hasOpenPencilSelection(selection)) return null
  return (
    <div style={styles.root} data-openpencil-selection-dock="true" role="status">
      <span style={styles.icon} aria-hidden="true">◇</span>
      <span style={styles.text}>
        <span style={styles.title}>{selectionNodeLabel(selection, locale)}</span>
        <span style={styles.detail}>{selectionNodeDetail(selection, locale)}</span>
      </span>
      <span style={styles.target}>{locale === 'zh' ? 'OpenPencil 修改目标' : 'OpenPencil edit target'}</span>
    </div>
  )
}
