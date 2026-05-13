import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { describe, expect, it } from 'vitest';

const ROOT = process.cwd().endsWith('/apps/web') ? join(process.cwd(), '../..') : process.cwd();

const AUDITED_FILES = [
  'apps/web/src/components/shared/language-selector.tsx',
  'apps/web/src/components/cloud/auth-gate.tsx',
  'apps/web/src/components/cloud/cloud-file-library.tsx',
  'apps/web/src/components/cloud/cloud-file-details-panel.tsx',
  'apps/web/src/components/cloud/cloud-file-sharing-section.tsx',
  'apps/web/src/components/cloud/cloud-move-target-panel.tsx',
  'apps/web/src/components/cloud/cloud-save-conflict-dialog.tsx',
  'apps/web/src/components/cloud/cloud-version-history-panel.tsx',
  'apps/web/src/components/panels/code-file-tree.tsx',
  'apps/web/src/components/panels/code-local-output-actions.tsx',
  'apps/web/src/components/panels/code-panel.tsx',
  'apps/web/src/components/tasks/task-center.tsx',
  'apps/web/src/components/tasks/worker-management.tsx',
  'apps/web/src/components/tasks/task-notification-listener.tsx',
];

const REQUIRED_SNIPPETS = [
  { file: 'apps/web/src/components/shared/language-selector.tsx', text: 'loadLocale(code)' },
  { file: 'apps/web/src/components/cloud/auth-gate.tsx', text: "t('auth.cloudTitle')" },
  { file: 'apps/web/src/components/cloud/cloud-file-library.tsx', text: "t('cloudLibrary.title')" },
  {
    file: 'apps/web/src/components/cloud/cloud-save-conflict-dialog.tsx',
    text: "t('cloudConflict.title')",
  },
  {
    file: 'apps/web/src/components/cloud/cloud-version-history-panel.tsx',
    text: "t('versionHistory.title')",
  },
  { file: 'apps/web/src/components/panels/code-panel.tsx', text: "t('codePanel.generate'," },
  { file: 'apps/web/src/components/tasks/task-center.tsx', text: "t('tasks.title')" },
  {
    file: 'apps/web/src/components/tasks/task-notification-listener.tsx',
    text: "t('tasks.notification.open')",
  },
  {
    file: 'apps/web/src/components/cloud/cloud-file-details-panel.tsx',
    text: "t('cloudLibrary.details.openFileTasks')",
  },
  { file: 'apps/web/src/components/tasks/task-center.tsx', text: "t('tasks.retry')" },
  {
    file: 'apps/web/src/components/tasks/worker-management.tsx',
    text: "t('tasks.workerManagement')",
  },
];

describe('i18n hardcoded copy audit', () => {
  it('keeps language switching, cloud library, and code panel behind locale keys', () => {
    for (const { file, text } of REQUIRED_SNIPPETS) {
      const source = readFileSync(join(ROOT, file), 'utf-8');
      expect(source).toContain(text);
    }
  });

  it('tracks the core files that must stay localized as they evolve', () => {
    for (const file of AUDITED_FILES) {
      expect(readFileSync(join(ROOT, file), 'utf-8').length).toBeGreaterThan(100);
    }
  });
});
