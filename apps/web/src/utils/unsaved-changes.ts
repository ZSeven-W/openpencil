import i18n from '@/i18n';
import { useDocumentStore } from '@/stores/document-store';
import { isElectron } from '@/utils/file-operations';
import { saveCurrentDocument } from '@/utils/save-current-document';

export async function confirmContinueWithUnsavedChanges(): Promise<boolean> {
  if (!useDocumentStore.getState().isDirty) return true;

  if (isElectron() && window.electronAPI?.confirmUnsavedChanges) {
    const decision = await window.electronAPI.confirmUnsavedChanges({
      message: i18n.t('topbar.closeConfirmMessage'),
      detail: i18n.t('topbar.closeConfirmDetail'),
      yesLabel: i18n.t('common.yes'),
      noLabel: i18n.t('common.no'),
      cancelLabel: i18n.t('common.cancel'),
    });
    if (decision === 'save') {
      return saveCurrentDocument();
    }
    return decision === 'discard';
  }

  return window.confirm(i18n.t('topbar.closeConfirmMessage'));
}
