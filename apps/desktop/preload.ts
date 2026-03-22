import { contextBridge, ipcRenderer, type IpcRendererEvent } from 'electron'

export type UpdaterStatus =
  | 'disabled'
  | 'idle'
  | 'checking'
  | 'available'
  | 'downloading'
  | 'downloaded'
  | 'not-available'
  | 'error'

export interface UpdaterState {
  status: UpdaterStatus
  currentVersion: string
  latestVersion?: string
  downloadProgress?: number
  releaseDate?: string
  error?: string
}

export interface ElectronAPI {
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
  getLogDir: () => Promise<string>
  setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => void
  getPreferences: () => Promise<Record<string, string>>
  setPreference: (key: string, value: string) => Promise<void>
  removePreference: (key: string) => Promise<void>
  confirmClose: () => void
  updater: {
    getState: () => Promise<UpdaterState>
    checkForUpdates: () => Promise<UpdaterState>
    quitAndInstall: () => Promise<boolean>
    getAutoCheck: () => Promise<boolean>
    setAutoCheck: (enabled: boolean) => Promise<boolean>
    onStateChange: (callback: (state: UpdaterState) => void) => () => void
  }
}

const api: ElectronAPI = {
  isElectron: true,

  openFile: () => ipcRenderer.invoke('dialog:openFile'),

  saveFile: (content: string, defaultPath?: string) =>
    ipcRenderer.invoke('dialog:saveFile', { content, defaultPath }),

  saveToPath: (filePath: string, content: string) =>
    ipcRenderer.invoke('dialog:saveToPath', { filePath, content }),

  setTheme: (theme: 'dark' | 'light', colors?: { bg: string; fg: string }) =>
    ipcRenderer.invoke('theme:set', theme, colors),

  getPreferences: () => ipcRenderer.invoke('prefs:getAll'),

  setPreference: (key: string, value: string) => ipcRenderer.invoke('prefs:set', key, value),

  removePreference: (key: string) => ipcRenderer.invoke('prefs:remove', key),

  onMenuAction: (callback: (action: string) => void) => {
    const listener = (_event: IpcRendererEvent, action: string) => {
      callback(action)
    }
    ipcRenderer.on('menu:action', listener)
    return () => {
      ipcRenderer.removeListener('menu:action', listener)
    }
  },

  onOpenFile: (callback: (filePath: string) => void) => {
    const listener = (_event: IpcRendererEvent, filePath: string) => {
      callback(filePath)
    }
    ipcRenderer.on('file:open', listener)
    return () => {
      ipcRenderer.removeListener('file:open', listener)
    }
  },

  readFile: (filePath: string) => ipcRenderer.invoke('file:read', filePath),

  getPendingFile: () => ipcRenderer.invoke('file:getPending'),

  confirmClose: () => ipcRenderer.send('window:confirmClose'),

  getLogDir: () => ipcRenderer.invoke('log:getDir'),

  updater: {
    getState: () => ipcRenderer.invoke('updater:getState'),
    checkForUpdates: () => ipcRenderer.invoke('updater:checkForUpdates'),
    quitAndInstall: () => ipcRenderer.invoke('updater:quitAndInstall'),
    getAutoCheck: () => ipcRenderer.invoke('updater:getAutoCheck'),
    setAutoCheck: (enabled: boolean) => ipcRenderer.invoke('updater:setAutoCheck', enabled),
    onStateChange: (callback: (state: UpdaterState) => void) => {
      const listener = (_event: IpcRendererEvent, state: UpdaterState) => {
        callback(state)
      }
      ipcRenderer.on('updater:state', listener)
      return () => {
        ipcRenderer.removeListener('updater:state', listener)
      }
    },
  },
}

contextBridge.exposeInMainWorld('electronAPI', api)
