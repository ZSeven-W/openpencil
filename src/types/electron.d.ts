type UpdaterStatus =
  | 'disabled'
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'downloaded'
  | 'not-available'
  | 'error'

interface UpdaterState {
  status: UpdaterStatus
  currentVersion: string
  latestVersion?: string
  downloadProgress?: number
  releaseDate?: string
  error?: string
}

interface ElectronAPI {
  isElectron: true
  openFile: () => Promise<{ filePath: string; content: string } | null>
  saveFile: (
    content: string,
    defaultPath?: string,
  ) => Promise<string | null>
  saveToPath: (filePath: string, content: string) => Promise<string>
  onMenuAction: (callback: (action: string) => void) => () => void
  updater: {
    getState: () => Promise<UpdaterState>
    checkForUpdates: () => Promise<UpdaterState>
    quitAndInstall: () => Promise<boolean>
    onStateChange: (callback: (state: UpdaterState) => void) => () => void
  }
}

interface Window {
  electronAPI?: ElectronAPI
}
