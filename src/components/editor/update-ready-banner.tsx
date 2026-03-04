import { useEffect, useState } from 'react'
import { AlertCircle, CheckCircle2, Download, Loader2, RefreshCw, Sparkles } from 'lucide-react'
import { useTranslation } from 'react-i18next'
import { Button } from '@/components/ui/button'

export default function UpdateReadyBanner() {
  const { t } = useTranslation()
  const [updateState, setUpdateState] = useState<UpdaterState | null>(null)
  const [isInstalling, setIsInstalling] = useState(false)
  const [isChecking, setIsChecking] = useState(false)
  const [dismissed, setDismissed] = useState(false)

  useEffect(() => {
    const updater = window.electronAPI?.updater
    if (!updater) return

    let mounted = true

    updater
      .getState()
      .then((state) => {
        if (mounted) setUpdateState(state)
      })
      .catch(() => {})

    const unsubscribe = updater.onStateChange((state) => {
      if (mounted) setUpdateState(state)
    })

    return () => {
      mounted = false
      unsubscribe()
    }
  }, [])

  useEffect(() => {
    if (!updateState) return
    if (updateState.status === 'available' || updateState.status === 'downloading' || updateState.status === 'downloaded' || updateState.status === 'error') {
      setDismissed(false)
    }
  }, [updateState])

  const handleInstall = async () => {
    if (!window.electronAPI?.updater) return

    setIsInstalling(true)
    const accepted = await window.electronAPI.updater.quitAndInstall()
    if (!accepted) {
      setIsInstalling(false)
    }
  }

  const handleCheckUpdates = async () => {
    if (!window.electronAPI?.updater) return
    setIsChecking(true)
    try {
      const next = await window.electronAPI.updater.checkForUpdates()
      setUpdateState(next)
    } finally {
      setIsChecking(false)
    }
  }

  if (!updateState) {
    return null
  }

  const visible =
    !dismissed
    && (updateState.status === 'checking'
      || updateState.status === 'available'
      || updateState.status === 'downloading'
      || updateState.status === 'downloaded'
      || updateState.status === 'error')

  if (!visible) {
    return null
  }

  const progress = Math.max(0, Math.min(100, Math.round(updateState.downloadProgress ?? 0)))
  const releaseDate = updateState.releaseDate
    ? new Date(updateState.releaseDate).toLocaleDateString()
    : null

  const titleByStatus: Partial<Record<UpdaterStatus, string>> = {
    checking: t('updater.title.checking'),
    available: t('updater.title.available'),
    downloading: t('updater.title.downloading'),
    downloaded: t('updater.title.downloaded'),
    error: t('updater.title.error'),
  }

  const subtitleByStatus: Partial<Record<UpdaterStatus, string>> = {
    checking: t('updater.subtitle.checking'),
    available: updateState.latestVersion ? t('updater.subtitle.available', { version: updateState.latestVersion }) : t('updater.subtitle.availableGeneric'),
    downloading: updateState.latestVersion
      ? t('updater.subtitle.downloading', { version: updateState.latestVersion })
      : t('updater.subtitle.downloadingGeneric'),
    downloaded: updateState.latestVersion
      ? t('updater.subtitle.downloaded', { version: updateState.latestVersion })
      : t('updater.subtitle.downloadedGeneric'),
    error: updateState.error || t('updater.subtitle.error'),
  }

  return (
    <div className="fixed top-5 right-5 z-50 app-region-no-drag">
      <div className="w-[460px] max-w-[calc(100vw-24px)] rounded-2xl border border-border/80 bg-card/95 backdrop-blur-md shadow-2xl overflow-hidden">
        <div className="px-5 py-4 border-b border-border/70 bg-gradient-to-r from-foreground/5 to-transparent">
          <div className="flex items-start justify-between gap-3">
            <div className="min-w-0">
              <div className="inline-flex items-center gap-2 text-[11px] uppercase tracking-[0.16em] text-muted-foreground">
                <Sparkles className="h-3.5 w-3.5" />
                {t('updater.softwareUpdate')}
              </div>
              <p className="mt-1 text-base font-semibold text-card-foreground">
                {titleByStatus[updateState.status] || t('updater.softwareUpdate')}
              </p>
              <p className="mt-1 text-sm text-muted-foreground leading-relaxed break-words">
                {subtitleByStatus[updateState.status]}
              </p>
            </div>
            <button
              type="button"
              onClick={() => setDismissed(true)}
              className="h-7 px-2 rounded-md text-xs text-muted-foreground hover:text-foreground hover:bg-muted transition-colors"
            >
              {t('updater.dismiss')}
            </button>
          </div>
        </div>

        <div className="px-5 py-4 space-y-3">
          <div className="grid grid-cols-2 gap-2 text-xs text-muted-foreground">
            <div className="rounded-lg border border-border/70 px-3 py-2">
              <span className="block text-[10px] uppercase tracking-wider mb-1">{t('updater.current')}</span>
              <span className="text-foreground font-medium">{updateState.currentVersion || t('updater.unknown')}</span>
            </div>
            <div className="rounded-lg border border-border/70 px-3 py-2">
              <span className="block text-[10px] uppercase tracking-wider mb-1">{t('updater.latest')}</span>
              <span className="text-foreground font-medium">{updateState.latestVersion || t('updater.checking')}</span>
            </div>
          </div>

          {(updateState.status === 'downloading' || updateState.status === 'available') && (
            <div className="rounded-lg border border-border/70 p-3">
              <div className="flex items-center justify-between text-xs text-muted-foreground mb-2">
                <span className="inline-flex items-center gap-1.5">
                  <Download className="h-3.5 w-3.5" />
                  {t('updater.downloadProgress')}
                </span>
                <span className="font-medium text-foreground">{progress}%</span>
              </div>
              <div className="h-2 rounded-full bg-muted overflow-hidden">
                <div
                  className="h-full bg-foreground/90 transition-all duration-300 ease-out"
                  style={{ width: `${progress}%` }}
                />
              </div>
            </div>
          )}

          {updateState.status === 'error' && (
            <div className="rounded-lg border border-red-500/30 bg-red-500/10 px-3 py-2 text-xs text-red-700 dark:text-red-300 inline-flex items-start gap-2">
              <AlertCircle className="h-4 w-4 mt-0.5 shrink-0" />
              <span className="leading-relaxed break-words">{updateState.error || t('updater.unknownError')}</span>
            </div>
          )}

          {updateState.status === 'downloaded' && (
            <div className="rounded-lg border border-emerald-500/25 bg-emerald-500/10 px-3 py-2 text-xs text-emerald-700 dark:text-emerald-300 inline-flex items-center gap-2">
              <CheckCircle2 className="h-4 w-4 shrink-0" />
              <span>{t('updater.restartHint')}</span>
            </div>
          )}

          {releaseDate && (
            <p className="text-[11px] text-muted-foreground">
              {t('updater.releaseDate', { date: releaseDate })}
            </p>
          )}

          <div className="flex items-center justify-end gap-2 pt-1">
            <Button
              variant="outline"
              onClick={handleCheckUpdates}
              disabled={isChecking || isInstalling}
              className="h-9"
            >
              {(isChecking || updateState.status === 'checking')
                ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('updater.checking')}
                  </>
                )
                : (
                  <>
                    <RefreshCw className="h-4 w-4" />
                    {t('updater.checkAgain')}
                  </>
                )}
            </Button>

            <Button
              onClick={handleInstall}
              disabled={isInstalling || updateState.status !== 'downloaded'}
              className="h-9 bg-foreground text-background hover:bg-foreground/90"
            >
              {isInstalling
                ? (
                  <>
                    <Loader2 className="h-4 w-4 animate-spin" />
                    {t('updater.installing')}
                  </>
                )
                : t('updater.restartInstall')}
            </Button>
          </div>
        </div>
      </div>
    </div>
  )
}
