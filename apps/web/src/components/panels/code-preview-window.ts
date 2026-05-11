import { PREVIEW_UNSUPPORTED_REASONS } from '@/services/cloud/codegen-preview';

export function getPreviewReasonKey(reason: string): string {
  if (reason === PREVIEW_UNSUPPORTED_REASONS.historyAssetsUnavailable) {
    return 'codePanel.preview.reason.historyAssetsUnavailable';
  }
  if (reason === PREVIEW_UNSUPPORTED_REASONS.vueTemplateRequired) {
    return 'codePanel.preview.reason.vueTemplateRequired';
  }
  if (reason === PREVIEW_UNSUPPORTED_REASONS.reactUnsupported) {
    return 'codePanel.preview.reason.reactUnsupported';
  }
  return 'codePanel.preview.reason.frameworkUnsupported';
}

export function openPreviewHtml(srcDoc: string): boolean {
  const win = window.open('', '_blank');
  if (!win) return false;
  win.opener = null;
  win.document.open();
  win.document.write(srcDoc);
  win.document.close();
  return true;
}
