import { useEffect, useState } from 'react';
import { ShieldCheck, Trash2 } from 'lucide-react';
import { useTranslation } from 'react-i18next';
import { Button } from '@/components/ui/button';
import {
  createCloudFileShare,
  listCloudFileShares,
  revokeCloudFileShare,
} from '@/services/cloud/cloud-files';
import type { CloudFileShare, CloudFileSummary, CloudShareRole } from '@/types/cloud';

interface CloudFileSharingSectionProps {
  file: CloudFileSummary;
}

export function CloudFileSharingSection({ file }: CloudFileSharingSectionProps) {
  const { t } = useTranslation();
  const [shares, setShares] = useState<CloudFileShare[]>([]);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [email, setEmail] = useState('');
  const [role, setRole] = useState<CloudShareRole>('viewer');

  const isSharedWithMe = Boolean(file.shareRole);

  useEffect(() => {
    if (isSharedWithMe) return;
    let cancelled = false;
    setLoading(true);
    setError(null);
    listCloudFileShares(file.id)
      .then((items) => {
        if (!cancelled) setShares(items);
      })
      .catch((err) => {
        if (!cancelled) {
          setError(err instanceof Error ? err.message : t('cloudLibrary.share.errorLoad'));
        }
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, [file.id, isSharedWithMe]);

  const addShare = async () => {
    const trimmed = email.trim();
    if (!trimmed) return;
    setSaving(true);
    setError(null);
    try {
      const created = await createCloudFileShare({ fileId: file.id, email: trimmed, role });
      setShares((items) => [created, ...items.filter((item) => item.id !== created.id)]);
      setEmail('');
      setRole('viewer');
    } catch (err) {
      setError(err instanceof Error ? err.message : t('cloudLibrary.share.errorAdd'));
    } finally {
      setSaving(false);
    }
  };

  const revokeShare = async (shareId: string) => {
    setError(null);
    try {
      await revokeCloudFileShare({ fileId: file.id, shareId });
      setShares((items) => items.filter((item) => item.id !== shareId));
    } catch (err) {
      setError(err instanceof Error ? err.message : t('cloudLibrary.share.errorRevoke'));
    }
  };

  if (isSharedWithMe) {
    return (
      <section className="mt-5 border-t border-border pt-4 text-xs">
        <p className="mb-2 font-medium">{t('cloudLibrary.share.title')}</p>
        <div className="rounded-md border border-border bg-card p-3">
          <p className="flex items-center gap-2 font-medium">
            <ShieldCheck size={14} className="text-primary" />
            {t('cloudLibrary.share.sharedWithYou')}
          </p>
          <p className="mt-1 text-muted-foreground">
            {t('cloudLibrary.share.yourRole', {
              role:
                file.shareRole === 'editor'
                  ? t('cloudLibrary.share.role.editor')
                  : t('cloudLibrary.share.role.viewer'),
            })}
          </p>
        </div>
      </section>
    );
  }

  return (
    <section className="mt-5 border-t border-border pt-4 text-xs">
      <p className="mb-3 font-medium">{t('cloudLibrary.share.title')}</p>
      <div className="space-y-2">
        <input
          aria-label={t('cloudLibrary.share.email')}
          value={email}
          onChange={(event) => setEmail(event.target.value)}
          placeholder="teammate@example.com"
          className="h-8 w-full rounded-md border border-input bg-background px-2 text-xs outline-none focus:border-ring"
        />
        <div className="flex gap-2">
          <select
            aria-label={t('cloudLibrary.share.roleLabel')}
            value={role}
            onChange={(event) => setRole(event.target.value as CloudShareRole)}
            className="h-8 min-w-0 flex-1 rounded-md border border-input bg-background px-2 text-xs outline-none focus:border-ring"
          >
            <option value="viewer">{t('cloudLibrary.share.role.viewer')}</option>
            <option value="editor">{t('cloudLibrary.share.role.editor')}</option>
          </select>
          <Button size="sm" onClick={() => void addShare()} disabled={saving || !email.trim()}>
            {t('cloudLibrary.share.add')}
          </Button>
        </div>
      </div>

      {error && <p className="mt-3 text-xs text-destructive">{error}</p>}
      {loading ? (
        <p className="mt-3 text-xs text-muted-foreground">{t('cloudLibrary.share.loading')}</p>
      ) : shares.length === 0 ? (
        <p className="mt-3 rounded-md border border-border bg-card px-3 py-2 text-muted-foreground">
          {t('cloudLibrary.share.empty')}
        </p>
      ) : (
        <div className="mt-3 space-y-2">
          {shares.map((share) => {
            const recipient =
              share.sharedWithEmail ??
              share.sharedWithUserId ??
              t('cloudLibrary.share.unknownRecipient');
            return (
              <div
                key={share.id}
                className="flex items-center justify-between gap-2 rounded-md border border-border bg-card px-3 py-2"
              >
                <div className="min-w-0">
                  <p className="truncate font-medium">{recipient}</p>
                  <p className="capitalize text-muted-foreground">
                    {t(`cloudLibrary.share.role.${share.role}`)}
                  </p>
                </div>
                <Button
                  variant="ghost"
                  size="icon-sm"
                  className="hover:text-destructive"
                  aria-label={t('cloudLibrary.share.revokeFor', { recipient })}
                  onClick={() => void revokeShare(share.id)}
                >
                  <Trash2 size={13} />
                </Button>
              </div>
            );
          })}
        </div>
      )}
    </section>
  );
}
