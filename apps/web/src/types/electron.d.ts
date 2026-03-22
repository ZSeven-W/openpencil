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
  onOpenFile: (callback: (filePath: string) => void) => () => void
  readFile: (filePath: string) => Promise<{ filePath: string; content: string } | null>
  getPendingFile: () => Promise<string | null>
  confirmClose: () => void
  getLogDir: () => Promise<string>
  setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => void
  getPreferences: () => Promise<Record<string, string>>
  setPreference: (key: string, value: string) => Promise<void>
  removePreference: (key: string) => Promise<void>
  updater: {
    getState: () => Promise<UpdaterState>
    checkForUpdates: () => Promise<UpdaterState>
    quitAndInstall: () => Promise<boolean>
    getAutoCheck: () => Promise<boolean>
    setAutoCheck: (enabled: boolean) => Promise<boolean>
    onStateChange: (callback: (state: UpdaterState) => void) => () => void
  }
}

interface Window {
  electronAPI?: ElectronAPI
}
