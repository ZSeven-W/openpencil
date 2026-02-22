interface ElectronAPI {
  isElectron: true
  openFile: () => Promise<{ filePath: string; content: string } | null>
  saveFile: (
    content: string,
    defaultPath?: string,
  ) => Promise<string | null>
  saveToPath: (filePath: string, content: string) => Promise<string>
}

interface Window {
  electronAPI?: ElectronAPI
}
