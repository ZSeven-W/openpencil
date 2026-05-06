// apps/web/src/components/panels/git-panel/git-panel-conflict-json-editor.tsx
//
// Inline 手册 JSON 节点冲突卡和字段冲突所使用的编辑器
// 卡片。 The 文本区域是严格本地的 — 它确实写入 NOT 来存储状态
// 每次击键。 Only “Apply”按钮的解析结果被传递到
// onSubmit 回调。
//
// Validation 规则：
//   - For 节点模式：JSON 必须解析为对象并保留原来的 nodeId。
//   - For 字段模式：JSON 必须解析为任何有效的 JSON 值（包括
// 基元、数组和对象）。
//
// The 提交按钮被禁用，而 JSON 无效。 Error 消息是
// 显示在文本区域下方内联。

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { safeParseJson, validateNodeJson } from './conflict-formatters';

export interface GitPanelConflictJsonEditorProps {
  /** Initial JSON 预填充在文本区域中的字符串。 */
  initialValue: string;
  /** 'node' — 值必须是具有原始 id 的类似 PenNode 的对象。 'field' — 值可以是任何有效的 JSON
   * （基元、数组或对象）。 */
  mode: 'node' | 'field';
  /** For mode='node'：编辑值必须保留 nodeId。 */
  nodeId?: string;
  /** Called 与用户提交有效的 JSON 时解析的值。 */
  onSubmit: (value: unknown) => void;
  /** 当用户单击 Cancel 时，会出现 Called。 */
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

  // Local 文本区域状态 — NOT 在每次击键时同步存储。
  const [text, setText] = useState(initialValue);

  // Derive 从当前文本解析结果（派生状态，未存储）。
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
