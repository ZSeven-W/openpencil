import { useToolShortcuts } from './use-tool-shortcuts';
import { useClipboardShortcuts } from './use-clipboard-shortcuts';
import { useHistoryShortcuts } from './use-history-shortcuts';
import { useEditShortcuts } from './use-edit-shortcuts';

export function useKeyboardShortcuts() {
  useToolShortcuts();
  useClipboardShortcuts();
  useHistoryShortcuts();
  useEditShortcuts();
}
