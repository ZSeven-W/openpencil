import { useEffect, useState } from 'react'
import { Loader2 } from 'lucide-react'
import { Button } from '@/components/ui/button'

export default function UpdateReadyBanner() {
  const [updateState, setUpdateState] = useState<UpdaterState | null>(null)
  const [isInstalling, setIsInstalling] = useState(false)

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

  const handleInstall = async () => {
    if (!window.electronAPI?.updater) return

    setIsInstalling(true)
    const accepted = await window.electronAPI.updater.quitAndInstall()
    if (!accepted) {
      setIsInstalling(false)
    }
  }

  if (!updateState || updateState.status !== 'downloaded') {
    return null
  }

  return (
    <div className="fixed top-6 left-1/2 -translate-x-1/2 z-50 app-region-no-drag">
      <div className="w-[760px] max-w-[calc(100vw-32px)] rounded-2xl border border-border bg-card/95 backdrop-blur-md shadow-2xl px-6 py-5 sm:px-8 sm:py-6 flex items-center justify-between gap-5 sm:gap-8">
        <div className="min-w-0">
          <p className="text-2xl sm:text-3xl font-semibold text-card-foreground leading-tight">
            Update Ready To Install
          </p>
          <p className="mt-2 text-sm sm:text-lg text-muted-foreground">
            {updateState.latestVersion ? `Version ${updateState.latestVersion} has been downloaded.` : 'A new version has been downloaded.'}
          </p>
          <p className="text-sm sm:text-lg text-muted-foreground">
            Can take 10-15 seconds for the app to relaunch automatically.
          </p>
        </div>

        <Button
          onClick={handleInstall}
          disabled={isInstalling}
          className="h-11 sm:h-12 px-4 sm:px-6 text-sm sm:text-lg font-semibold whitespace-nowrap bg-foreground text-background hover:bg-foreground/90"
        >
          {isInstalling
            ? (
              <>
                <Loader2 className="h-5 w-5 animate-spin" />
                Installing...
              </>
            )
            : 'Restart & Install'}
        </Button>
      </div>
    </div>
  )
}
