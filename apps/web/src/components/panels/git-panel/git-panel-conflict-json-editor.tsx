// apps/web/src/components/panels/git-panel/git-panel-conflict-json-editor.tsx
//
// Inline manual JSON editor used by node conflict cards and field conflict
// cards. The textarea is strictly local — it does NOT write to store state on
// every keystroke. Only the parsed result of the "Apply" button is passed to
// the onSubmit callback.
//
// Validation rules:
//   - For node mode: JSON must parse to an object and keep the original nodeId.
//   - For field mode: JSON must parse to any valid JSON value (including
//     primitives, arrays, and objects).
//
// The submit button is disabled while JSON is invalid. Error messages are
// shown inline below the textarea.

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { safeParseJson, validateNodeJson } from './conflict-formatters';

export interface GitPanelConflictJsonEditorProps {
  /** Initial JSON string pre-filled in the textarea. */
  initialValue: string;
  /** 'node' — value must be a PenNode-like object with the original id.
   *  'field' — value can be any valid JSON (primitive, array, or object). */
  mode: 'node' | 'field';
  /** For mode='node': the nodeId the edited value must preserve. */
  nodeId?: string;
  /** Called with the parsed value when the user submits valid JSON. */
  onSubmit: (value: unknown) => void;
  /** Called when the user clicks Cancel. */
  onCancel: () => void;
}

export function GitPanelConflictJsonEditor({
  initialValue,
  mode,
  nodeId,
  onSubmit,
  onCancel,
}: GitPanelConflictJsonEditorProps) {
  const { t } = useTranslation();

  // Local textarea state — NOT synced to store on every keystroke.
  const [text, setText] = useState(initialValue);

  // Derive parse result from current text (derived state, not stored).
  const parseResult = safeParseJson(text);
  const nodeValidationError =
    parseResult.ok && mode === 'node' && nodeId
      ? validateNodeJson(parseResult.value, nodeId)
      : null;

  const isValid = parseResult.ok && nodeValidationError === null;
  const errorMessage = !parseResult.ok ? parseResult.error : (nodeValidationError ?? null);

  function handleSubmit() {
    if (!isValid || !parseResult.ok) return;
    onSubmit(parseResult.value);
  }

  return (
    <div className="flex flex-col gap-2" data-testid="conflict-json-editor">
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        className="w-full font-mono text-xs min-h-[120px] resize-y rounded border border-border bg-background p-2"
        spellCheck={false}
        aria-label={t('git.conflict.editor.textareaLabel')}
        data-testid="conflict-json-textarea"
      />
      {errorMessage !== null && (
        <p className="text-xs text-destructive" role="alert" data-testid="conflict-json-error">
          {t('git.conflict.editor.invalidJson')}: {errorMessage}
        </p>
      )}
      <div className="flex justify-end gap-2">
        <Button
          type="button"
          variant="outline"
          size="sm"
          onClick={onCancel}
          data-testid="conflict-json-cancel"
        >
          {t('git.conflict.editor.cancel')}
        </Button>
        <Button
          type="button"
          variant="default"
          size="sm"
          disabled={!isValid}
          onClick={handleSubmit}
          data-testid="conflict-json-apply"
        >
          {t('git.conflict.editor.apply')}
        </Button>
      </div>
    </div>
  );
}
