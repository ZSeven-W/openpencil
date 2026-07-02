export type UnsavedChangesDecision = 'save' | 'discard' | 'cancel';

export interface UnsavedChangesDialogLabels {
  message: string;
  detail?: string;
  yesLabel: string;
  noLabel: string;
  cancelLabel: string;
}

export function buildUnsavedChangesDialogOptions(labels: UnsavedChangesDialogLabels) {
  return {
    type: 'question' as const,
    buttons: [labels.yesLabel, labels.noLabel, labels.cancelLabel],
    defaultId: 0,
    cancelId: 2,
    message: labels.message,
    detail: labels.detail ?? '',
  };
}

export function mapUnsavedChangesResponse(response: number): UnsavedChangesDecision {
  if (response === 0) return 'save';
  if (response === 1) return 'discard';
  return 'cancel';
}
