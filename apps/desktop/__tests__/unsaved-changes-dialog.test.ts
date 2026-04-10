import { describe, expect, it } from 'bun:test';

import {
  buildUnsavedChangesDialogOptions,
  mapUnsavedChangesResponse,
} from '../unsaved-changes-dialog';

describe('unsaved changes dialog helpers', () => {
  it('builds a three-button yes/no/cancel dialog', () => {
    expect(
      buildUnsavedChangesDialogOptions({
        yesLabel: '是',
        noLabel: '否',
        cancelLabel: '取消',
        message: '关闭前是否要保存更改？',
        detail: '如果不保存，您的更改将会丢失。',
      }),
    ).toMatchObject({
      type: 'question',
      buttons: ['是', '否', '取消'],
      defaultId: 0,
      cancelId: 2,
      message: '关闭前是否要保存更改？',
      detail: '如果不保存，您的更改将会丢失。',
    });
  });

  it('maps button responses to save/discard/cancel actions', () => {
    expect(mapUnsavedChangesResponse(0)).toBe('save');
    expect(mapUnsavedChangesResponse(1)).toBe('discard');
    expect(mapUnsavedChangesResponse(2)).toBe('cancel');
    expect(mapUnsavedChangesResponse(99)).toBe('cancel');
  });
});
