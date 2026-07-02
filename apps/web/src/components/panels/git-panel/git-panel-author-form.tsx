// apps/web/src/components/panels/git-panel/git-panel-author-form.tsx
//
// Inline name + email form for the first commit author identity. Per spec
// lines 285-292: shown once when the user attempts a commit with no cached
// identity. Phase 4a ships the component; Phase 4c's commit input owns the
// trigger (via showAuthorPrompt() when authorIdentity === null).
//
// The form validates name (non-empty) and email (must contain @). On
// submit, calls setAuthorIdentity which persists to OpenPencil prefs and
// updates the in-memory cache. Cancel hides the form without persisting.

import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import { useGitStore } from '@/stores/git-store';

const INPUT_CLASS =
  'h-8 bg-secondary border border-input rounded px-2 text-sm text-foreground focus:outline-none focus:border-ring';

interface ValidationErrors {
  name?: string;
  email?: string;
}

export function GitPanelAuthorForm() {
  const { t } = useTranslation();
  const setAuthorIdentity = useGitStore((s) => s.setAuthorIdentity);
  const hideAuthorPrompt = useGitStore((s) => s.hideAuthorPrompt);

  const [name, setName] = useState('');
  const [email, setEmail] = useState('');
  const [errors, setErrors] = useState<ValidationErrors>({});

  const validate = (): boolean => {
    const next: ValidationErrors = {};
    if (!name.trim()) next.name = t('git.author.validationName');
    if (!email.trim() || !email.includes('@')) next.email = t('git.author.validationEmail');
    setErrors(next);
    return Object.keys(next).length === 0;
  };

  const handleSubmit = async () => {
    if (!validate()) return;
    await setAuthorIdentity(name.trim(), email.trim());
    hideAuthorPrompt();
  };

  return (
    <div className="flex flex-col gap-3 p-4">
      <div className="text-sm font-medium text-foreground">{t('git.author.heading')}</div>
      <div className="text-xs text-muted-foreground">{t('git.author.subheading')}</div>

      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground" htmlFor="git-author-name">
          {t('git.author.nameLabel')}
        </label>
        <input
          id="git-author-name"
          type="text"
          value={name}
          onChange={(e) => setName(e.target.value)}
          placeholder={t('git.author.namePlaceholder')}
          className={INPUT_CLASS}
        />
        {errors.name && <div className="text-[11px] text-destructive">{errors.name}</div>}
      </div>

      <div className="flex flex-col gap-1">
        <label className="text-xs text-muted-foreground" htmlFor="git-author-email">
          {t('git.author.emailLabel')}
        </label>
        <input
          id="git-author-email"
          type="email"
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          placeholder={t('git.author.emailPlaceholder')}
          className={INPUT_CLASS}
        />
        {errors.email && <div className="text-[11px] text-destructive">{errors.email}</div>}
      </div>

      <div className="flex items-center justify-end gap-2 pt-1">
        <Button type="button" variant="ghost" size="sm" onClick={hideAuthorPrompt}>
          {t('git.author.cancel')}
        </Button>
        <Button type="button" variant="default" size="sm" onClick={() => void handleSubmit()}>
          {t('git.author.submit')}
        </Button>
      </div>
    </div>
  );
}
