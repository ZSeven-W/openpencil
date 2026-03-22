import {
  ipcMain,
  dialog,
  type BrowserWindow,
} from 'electron'
import { resolve, extname, sep } from 'node:path'
import { readFile, writeFile } from 'node:fs/promises'
import { app } from 'electron'

import {
  getUpdaterState,
  checkForAppUpdates,
  quitAndInstall,
  getAutoUpdateEnabled,
  setAutoUpdateEnabled,
  setUpdaterState,
  clearUpdateTimer,
  startUpdateTimer,
} from './auto-updater'
import { getLogDir } from './logger'

interface IpcDeps {
  getMainWindow: () => BrowserWindow | null
  getPendingFilePath: () => string | null
  clearPendingFilePath: () => void
  prefsCache: Record<string, string>
  schedulePrefsWrite: () => void
  writeAppSettings: (patch: { autoUpdate?: boolean }) => Promise<void>
}

export function setupIPC(deps: IpcDeps): void {
  const { getMainWindow, getPendingFilePath, clearPendingFilePath, prefsCache, schedulePrefsWrite, writeAppSettings } = deps

  ipcMain.handle('dialog:openFile', async () => {
    const mainWindow = getMainWindow()
    if (!mainWindow) return null
    const result = await dialog.showOpenDialog(mainWindow, {
      title: 'Open .op file',
      filters: [{ name: 'OpenPencil Files', extensions: ['op', 'pen'] }],
      properties: ['openFile'],
    })
    if (result.canceled || result.filePaths.length === 0) return null
    const filePath = result.filePaths[0]
    const content = await readFile(filePath, 'utf-8')
    return { filePath, content }
  })

  ipcMain.handle(
    'dialog:saveFile',
    async (_event, payload: { content: string; defaultPath?: string }) => {
      const mainWindow = getMainWindow()
      if (!mainWindow) return null
      const result = await dialog.showSaveDialog(mainWindow, {
        title: 'Save .op file',
        defaultPath: payload.defaultPath,
        filters: [{ name: 'OpenPencil Files', extensions: ['op'] }],
      })
      if (result.canceled || !result.filePath) return null
      await writeFile(result.filePath, payload.content, 'utf-8')
      return result.filePath
    },
  )

  ipcMain.handle(
    'dialog:saveToPath',
    async (_event, payload: { filePath: string; content: string }) => {
      const resolved = resolve(payload.filePath)
      if (resolved.includes('\0')) {
        throw new Error('Invalid file path')
      }
      const ext = extname(resolved).toLowerCase()
      if (ext !== '.op' && ext !== '.pen') {
        throw new Error('Only .op and .pen file extensions are allowed')
      }
      const allowedRoots = [app.getPath('home'), app.getPath('temp')]
      const inAllowedDir = allowedRoots.some(
        (root) => resolved === root || resolved.startsWith(root + sep),
      )
      if (!inAllowedDir) {
        throw new Error('File path must be within the user home or temp directory')
      }
      await writeFile(resolved, payload.content, 'utf-8')
      return resolved
    },
  )

  ipcMain.handle('file:getPending', () => {
    const filePath = getPendingFilePath()
    if (filePath) {
      clearPendingFilePath()
      return filePath
    }
    return null
  })

  ipcMain.handle('file:read', async (_event, filePath: string) => {
    const resolved = resolve(filePath)
    const ext = extname(resolved).toLowerCase()
    if (ext !== '.op' && ext !== '.pen') return null
    try {
      const content = await readFile(resolved, 'utf-8')
      return { filePath: resolved, content }
    } catch {
      return null
    }
  })

  // Theme sync for Windows/Linux title bar overlay
  ipcMain.handle(
    'theme:set',
    (_event, theme: 'dark' | 'light', colors?: { bg: string; fg: string }) => {
      const mainWindow = getMainWindow()
      if (!mainWindow || mainWindow.isDestroyed()) return
      const isWinOrLinux = process.platform === 'win32' || process.platform === 'linux'
      if (!isWinOrLinux) return
      const isLinux = process.platform === 'linux'
      const fallbackBg = theme === 'dark' ? '#111' : '#fff'
      const fallbackFg = theme === 'dark' ? '#d4d4d8' : '#3f3f46'
      mainWindow.setTitleBarOverlay({
        color: isLinux ? (colors?.bg || fallbackBg) : 'rgba(0,0,0,0)',
        symbolColor: colors?.fg || fallbackFg,
      })
    },
  )

  // Renderer preferences
  ipcMain.handle('prefs:getAll', () => ({ ...prefsCache }))

  ipcMain.handle('prefs:set', (_event, key: string, value: string) => {
    prefsCache[key] = value
    schedulePrefsWrite()
  })

  ipcMain.handle('prefs:remove', (_event, key: string) => {
    delete prefsCache[key]
    schedulePrefsWrite()
  })

  ipcMain.handle('log:getDir', () => getLogDir())

  // Updater IPC
  ipcMain.handle('updater:getState', () => getUpdaterState())
  ipcMain.handle('updater:checkForUpdates', async () => {
    await checkForAppUpdates(true)
    return getUpdaterState()
  })
  ipcMain.handle('updater:quitAndInstall', () => quitAndInstall())
  ipcMain.handle('updater:getAutoCheck', () => getAutoUpdateEnabled())

  ipcMain.handle('updater:setAutoCheck', async (_event, enabled: boolean) => {
    setAutoUpdateEnabled(enabled)
    await writeAppSettings({ autoUpdate: enabled })

    if (enabled) {
      startUpdateTimer()
      setUpdaterState({ status: 'idle' })
    } else {
      clearUpdateTimer()
      setUpdaterState({ status: 'disabled' })
    }
    return enabled
  })
}
