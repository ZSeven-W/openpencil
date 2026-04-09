import { useClipboardShortcuts } from './use-clipboard-shortcuts';
import { useEditShortcuts } from './use-edit-shortcuts';
import { useHistoryShortcuts } from './use-history-shortcuts';
import { useToolShortcuts } from './use-tool-shortcuts';

export function useKeyboardShortcuts() {
  useToolShortcuts();
  useClipboardShortcuts();
  useHistoryShortcuts();
  useEditShortcuts();
}
