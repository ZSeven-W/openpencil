import { useState, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Check, Key, Pencil, Trash2 } from 'lucide-react';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Switch } from '@/components/ui/switch';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { BuiltinProviderConfig } from '@/stores/agent-settings-store';
import { BuiltinProviderForm } from './builtin-provider-settings';

export function BuiltinProviderCard({ provider }: { provider: BuiltinProviderConfig }) {
  const { t } = useTranslation();
  const update = useAgentSettingsStore((s) => s.updateBuiltinProvider);
  const remove = useAgentSettingsStore((s) => s.removeBuiltinProvider);
  const persist = useAgentSettingsStore((s) => s.persist);
  const [editing, setEditing] = useState(false);

  const handleToggle = useCallback(
    (enabled: boolean) => {
      update(provider.id, { enabled });
      persist();
    },
    [provider.id, update, persist],
  );
  const handleRemove = useCallback(() => {
    remove(provider.id);
    persist();
  }, [provider.id, remove, persist]);
  const handleSave = useCallback(
    (data: Omit<BuiltinProviderConfig, 'id'>) => {
      update(provider.id, data);
      persist();
      setEditing(false);
    },
    [provider.id, update, persist],
  );

  if (editing) {
    return (
      <BuiltinProviderForm
        initial={provider}
        onSave={handleSave}
        onCancel={() => setEditing(false)}
      />
    );
  }

  const masked =
    provider.apiKey.length > 12
      ? provider.apiKey.slice(0, 7) + '***' + provider.apiKey.slice(-3)
      : '***';

  return (
    <div className="group">
      <div
        className={cn(
          'flex items-center gap-3 px-3.5 py-2.5 rounded-lg border transition-colors',
          provider.enabled
            ? 'bg-secondary/30 border-border'
            : 'border-transparent hover:bg-secondary/20',
        )}
      >
        <div
          className={cn(
            'w-9 h-9 rounded-lg flex items-center justify-center shrink-0 transition-colors',
            provider.enabled
              ? 'bg-foreground/8 text-foreground'
              : 'bg-secondary text-muted-foreground',
          )}
        >
          <Key size={18} />
        </div>
        <div className="flex-1 min-w-0">
          <span className="text-[13px] font-medium text-foreground leading-tight block">
            {provider.displayName}
          </span>
          <span className="text-[11px] text-muted-foreground leading-tight mt-0.5 block">
            {provider.model} &middot; {masked}
          </span>
          {provider.enabled && (
            <span className="text-[11px] text-green-500 leading-tight flex items-center gap-1 mt-0.5">
              <Check size={10} strokeWidth={2.5} />
              {t('builtin.ready')}
            </span>
          )}
        </div>
        <div className="flex items-center gap-1 shrink-0">
          <Switch checked={provider.enabled} onCheckedChange={handleToggle} className="mr-1" />
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={() => setEditing(true)}
            className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity"
          >
            <Pencil size={11} />
          </Button>
          <Button
            variant="ghost"
            size="icon-sm"
            onClick={handleRemove}
            className="h-6 w-6 opacity-0 group-hover:opacity-100 transition-opacity text-muted-foreground hover:text-destructive"
          >
            <Trash2 size={11} />
          </Button>
        </div>
      </div>
    </div>
  );
}

export default BuiltinProviderCard;
