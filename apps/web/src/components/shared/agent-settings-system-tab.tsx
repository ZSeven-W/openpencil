import { useState, useEffect, useCallback } from 'react';
import { useTranslation } from 'react-i18next';
import { Switch } from '@/components/ui/switch';

export function SystemTab() {
  const { t } = useTranslation();
  const [autoUpdateEnabled, setAutoUpdateEnabled] = useState(true);
  const [isElectron, setIsElectron] = useState(false);

  useEffect(() => {
    setIsElectron(!!window.electronAPI);
  }, []);

  useEffect(() => {
    if (!window.electronAPI?.updater?.getAutoCheck) return;
    window.electronAPI.updater
      .getAutoCheck()
      .then(setAutoUpdateEnabled)
      .catch((err) => console.error('[auto-update getAutoCheck]', err));
  }, []);

  const handleAutoUpdateToggle = useCallback(async (enabled: boolean) => {
    setAutoUpdateEnabled(enabled);
    try {
      await window.electronAPI?.updater?.setAutoCheck?.(enabled);
    } catch (err) {
      console.error('[auto-update toggle]', err);
    }
  }, []);

  return (
    <div>
      <h3 className="text-[15px] font-semibold text-foreground mb-4">{t('settings.system')}</h3>
      {isElectron && (
        <div className="flex items-center justify-between px-3.5 py-2.5 rounded-lg border border-border bg-secondary/20">
          <div>
            <span className="text-[13px] text-foreground block leading-tight">
              {t('agents.autoUpdate')}
            </span>
            <span className="text-[11px] text-muted-foreground mt-0.5 block">
              {t('settings.autoUpdateDesc')}
            </span>
          </div>
          <Switch checked={autoUpdateEnabled} onCheckedChange={handleAutoUpdateToggle} />
        </div>
      )}
      {!isElectron && (
        <div className="rounded-lg border border-border bg-secondary/20 px-4 py-6 text-center">
          <p className="text-[13px] text-muted-foreground">{t('settings.systemDesktopOnly')}</p>
        </div>
      )}
    </div>
  );
}
