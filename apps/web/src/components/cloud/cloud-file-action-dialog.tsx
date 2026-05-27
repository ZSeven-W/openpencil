import { useEffect, useState } from 'react';
import type { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Field, FieldGroup, FieldLabel } from '@/components/ui/field';
import { Input } from '@/components/ui/input';
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import type { CloudShareRole } from '@/types/cloud';

export type CloudFileDialogAction =
  | { kind: 'new-project'; defaultValue: string }
  | { kind: 'new-folder'; defaultValue: string }
  | { kind: 'rename-project'; id: string; defaultValue: string }
  | { kind: 'rename-folder'; id: string; defaultValue: string }
  | { kind: 'rename-file'; id: string; defaultValue: string }
  | { kind: 'copy-file'; id: string; defaultValue: string }
  | { kind: 'share-file'; id: string; defaultValue: string };

export interface CloudFileDialogSubmit {
  name?: string;
  email?: string;
  role?: CloudShareRole;
}

interface CloudFileActionDialogProps {
  action: CloudFileDialogAction | null;
  onOpenChange: (open: boolean) => void;
  onSubmit: (values: CloudFileDialogSubmit) => Promise<void> | void;
  t: ReturnType<typeof useTranslation>['t'];
}

function dialogTitle(action: CloudFileDialogAction, t: ReturnType<typeof useTranslation>['t']) {
  if (action.kind === 'new-project') return t('cloudLibrary.prompt.newProjectName');
  if (action.kind === 'new-folder') return t('cloudLibrary.prompt.newFolderName');
  if (action.kind === 'rename-project') return t('cloudLibrary.prompt.renameProject');
  if (action.kind === 'rename-folder') return t('cloudLibrary.prompt.renameFolder');
  if (action.kind === 'rename-file') return t('cloudLibrary.prompt.renameFile');
  if (action.kind === 'copy-file') return t('cloudLibrary.prompt.copyFileAs');
  return t('cloudLibrary.prompt.shareWithEmail');
}

function isShareAction(action: CloudFileDialogAction | null) {
  return action?.kind === 'share-file';
}

export function CloudFileActionDialog({
  action,
  onOpenChange,
  onSubmit,
  t,
}: CloudFileActionDialogProps) {
  const [value, setValue] = useState('');
  const [role, setRole] = useState<CloudShareRole>('viewer');
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    setValue(action?.defaultValue ?? '');
    setRole('viewer');
    setSaving(false);
  }, [action]);

  const submit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const trimmed = value.trim();
    if (!action || !trimmed) return;
    setSaving(true);
    try {
      if (isShareAction(action)) {
        await onSubmit({ email: trimmed, role });
      } else {
        await onSubmit({ name: trimmed });
      }
      onOpenChange(false);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Dialog open={Boolean(action)} onOpenChange={onOpenChange}>
      <DialogContent>
        {action && (
          <form onSubmit={submit}>
            <DialogHeader>
              <DialogTitle>{dialogTitle(action, t)}</DialogTitle>
              <DialogDescription>{t('cloudLibrary.dialog.description')}</DialogDescription>
            </DialogHeader>
            <FieldGroup className="mt-5 gap-4">
              <Field>
                <FieldLabel htmlFor="cloud-file-dialog-value">
                  {isShareAction(action) ? t('cloudLibrary.share.email') : t('common.name')}
                </FieldLabel>
                <Input
                  id="cloud-file-dialog-value"
                  type={isShareAction(action) ? 'email' : 'text'}
                  value={value}
                  onChange={(event) => setValue(event.target.value)}
                  autoFocus
                  required
                />
              </Field>
              {isShareAction(action) && (
                <Field>
                  <FieldLabel>{t('cloudLibrary.share.roleLabel')}</FieldLabel>
                  <Select value={role} onValueChange={(next) => setRole(next as CloudShareRole)}>
                    <SelectTrigger className="w-full">
                      <SelectValue />
                    </SelectTrigger>
                    <SelectContent>
                      <SelectGroup>
                        <SelectItem value="viewer">
                          {t('cloudLibrary.share.role.viewer')}
                        </SelectItem>
                        <SelectItem value="editor">
                          {t('cloudLibrary.share.role.editor')}
                        </SelectItem>
                      </SelectGroup>
                    </SelectContent>
                  </Select>
                </Field>
              )}
            </FieldGroup>
            <DialogFooter className="mt-6">
              <Button type="button" variant="outline" onClick={() => onOpenChange(false)}>
                {t('common.cancel')}
              </Button>
              <Button type="submit" disabled={saving || !value.trim()}>
                {isShareAction(action) ? t('cloudLibrary.share.add') : t('common.save')}
              </Button>
            </DialogFooter>
          </form>
        )}
      </DialogContent>
    </Dialog>
  );
}
