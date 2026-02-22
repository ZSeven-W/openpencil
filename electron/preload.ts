import { contextBridge, ipcRenderer } from 'electron'

export interface ElectronAPI {
  isElectron: true
  openFile: () => Promise<{ filePath: string; content: string } | null>
  saveFile: (
    content: string,
    defaultPath?: string,
  ) => Promise<string | null>
  saveToPath: (filePath: string, content: string) => Promise<string>
}

const api: ElectronAPI = {
  isElectron: true,

  openFile: () => ipcRenderer.invoke('dialog:openFile'),

  saveFile: (content: string, defaultPath?: string) =>
    ipcRenderer.invoke('dialog:saveFile', { content, defaultPath }),

  saveToPath: (filePath: string, content: string) =>
    ipcRenderer.invoke('dialog:saveToPath', { filePath, content }),
}

contextBridge.exposeInMainWorld('electronAPI', api)
