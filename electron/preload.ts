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
  updater: {
    getState: () => Promise<UpdaterState>
    checkForUpdates: () => Promise<UpdaterState>
    quitAndInstall: () => Promise<boolean>
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

  onMenuAction: (callback: (action: string) => void) => {
    const listener = (_event: IpcRendererEvent, action: string) => {
      callback(action)
    }
    ipcRenderer.on('menu:action', listener)
    return () => {
      ipcRenderer.removeListener('menu:action', listener)
    }
  },

  updater: {
    getState: () => ipcRenderer.invoke('updater:getState'),
    checkForUpdates: () => ipcRenderer.invoke('updater:checkForUpdates'),
    quitAndInstall: () => ipcRenderer.invoke('updater:quitAndInstall'),
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
