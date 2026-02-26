import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  type BrowserWindowConstructorOptions,
} from 'electron'
import { autoUpdater } from 'electron-updater'
import { execSync } from 'node:child_process'
import { fork, type ChildProcess } from 'node:child_process'
import { createServer } from 'node:net'
import { join } from 'node:path'
import { readFile, writeFile } from 'node:fs/promises'

let mainWindow: BrowserWindow | null = null
let nitroProcess: ChildProcess | null = null
let serverPort = 0

const isDev = !app.isPackaged

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

let updaterState: UpdaterState = {
  status: isDev ? 'disabled' : 'idle',
  currentVersion: app.getVersion(),
}

let lastUpdateCheckAt = 0

function broadcastUpdaterState(): void {
  for (const win of BrowserWindow.getAllWindows()) {
    if (!win.isDestroyed()) {
      win.webContents.send('updater:state', updaterState)
    }
  }
}

function setUpdaterState(next: Partial<UpdaterState>): void {
  updaterState = {
    ...updaterState,
    ...next,
    currentVersion: app.getVersion(),
  }
  broadcastUpdaterState()
}

async function checkForAppUpdates(force = false): Promise<void> {
  if (isDev) return

  const now = Date.now()
  if (!force && now - lastUpdateCheckAt < 60 * 1000) {
    return
  }
  lastUpdateCheckAt = now

  try {
    await autoUpdater.checkForUpdates()
  } catch (err) {
    const error = err instanceof Error ? err.message : String(err)
    setUpdaterState({ status: 'error', error })
  }
}

function setupAutoUpdater(): void {
  if (isDev) return

  autoUpdater.autoDownload = true
  autoUpdater.autoInstallOnAppQuit = true
  autoUpdater.allowPrerelease = true

  autoUpdater.on('checking-for-update', () => {
    setUpdaterState({ status: 'checking', error: undefined, downloadProgress: undefined })
  })

  autoUpdater.on('update-available', (info) => {
    setUpdaterState({
      status: 'available',
      latestVersion: info.version,
      releaseDate: info.releaseDate,
      error: undefined,
    })
  })

  autoUpdater.on('download-progress', (progress) => {
    setUpdaterState({
      status: 'downloading',
      downloadProgress: Math.round(progress.percent),
      error: undefined,
    })
  })

  autoUpdater.on('update-downloaded', (info) => {
    setUpdaterState({
      status: 'downloaded',
      latestVersion: info.version,
      releaseDate: info.releaseDate,
      downloadProgress: 100,
      error: undefined,
    })
  })

  autoUpdater.on('update-not-available', (info) => {
    setUpdaterState({
      status: 'not-available',
      latestVersion: info.version,
      downloadProgress: undefined,
      error: undefined,
    })
  })

  autoUpdater.on('error', (err) => {
    setUpdaterState({
      status: 'error',
      error: err?.message ?? String(err),
    })
  })

  // Delay first check until app startup work is done.
  setTimeout(() => {
    void checkForAppUpdates(true)
  }, 5000)

  // Poll for new versions while app is running.
  setInterval(() => {
    void checkForAppUpdates(false)
  }, 60 * 60 * 1000).unref()
}

// ---------------------------------------------------------------------------
// Fix PATH for macOS GUI apps (Finder doesn't inherit shell PATH)
// ---------------------------------------------------------------------------

function fixPath(): void {
  if (process.platform !== 'darwin' && process.platform !== 'linux') return

  try {
    const shell = process.env.SHELL || '/bin/zsh'
    const shellPath = execSync(`${shell} -ilc 'echo -n "$PATH"'`, {
      encoding: 'utf-8',
      timeout: 5000,
    }).trim()
    if (shellPath) {
      const current = process.env.PATH || ''
      process.env.PATH = [...new Set([...shellPath.split(':'), ...current.split(':')])]
        .filter(Boolean)
        .join(':')
    }
  } catch {
    // Packaged app may not have a login shell — ignore
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function getFreePorts(): Promise<number> {
  return new Promise((resolve, reject) => {
    const server = createServer()
    server.listen(0, '127.0.0.1', () => {
      const addr = server.address()
      if (addr && typeof addr === 'object') {
        const { port } = addr
        server.close(() => resolve(port))
      } else {
        reject(new Error('Failed to get free port'))
      }
    })
    server.on('error', reject)
  })
}

function getServerEntry(): string {
  if (isDev) {
    // In dev, the Nitro output lives at .output/server/index.mjs
    return join(app.getAppPath(), '.output', 'server', 'index.mjs')
  }
  // In production, extraResources copies .output into the resources folder
  return join(process.resourcesPath, 'server', 'index.mjs')
}

// ---------------------------------------------------------------------------
// Nitro server
// ---------------------------------------------------------------------------

async function startNitroServer(): Promise<number> {
  const port = await getFreePorts()
  const entry = getServerEntry()

  return new Promise((resolve, reject) => {
    const child = fork(entry, [], {
      env: {
        ...process.env,
        HOST: '127.0.0.1',
        PORT: String(port),
        NITRO_HOST: '127.0.0.1',
        NITRO_PORT: String(port),
        ELECTRON_RESOURCES_PATH: process.resourcesPath,
      },
      stdio: 'pipe',
    })

    nitroProcess = child

    child.stdout?.on('data', (data: Buffer) => {
      const msg = data.toString()
      console.log('[nitro]', msg)
      // Resolve once Nitro reports it's listening
      if (msg.includes('Listening') || msg.includes('ready')) {
        resolve(port)
      }
    })

    child.stderr?.on('data', (data: Buffer) => {
      console.error('[nitro:err]', data.toString())
    })

    child.on('error', reject)
    child.on('exit', (code) => {
      if (code !== 0 && code !== null) {
        console.error(`Nitro exited with code ${code}`)
      }
      nitroProcess = null
    })

    // Fallback: if no stdout "ready" message comes, wait then resolve anyway
    setTimeout(() => resolve(port), 3000)
  })
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

function createWindow(): void {
  const windowOptions: BrowserWindowConstructorOptions = {
    width: 1440,
    height: 900,
    minWidth: 1024,
    minHeight: 600,
    title: 'OpenPencil',
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'default',
    webPreferences: {
      preload: join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
    },
  }

  if (process.platform === 'darwin') {
    windowOptions.trafficLightPosition = { x: 16, y: 11 }
  }

  // Start hidden to avoid visual flash before CSS injection
  windowOptions.show = false

  mainWindow = new BrowserWindow(windowOptions)

  const url = isDev
    ? 'http://localhost:3000/editor'
    : `http://127.0.0.1:${serverPort}/editor`

  // Inject traffic-light padding CSS then show window (no flash)
  mainWindow.webContents.on('did-finish-load', async () => {
    if (!mainWindow) return
    if (process.platform === 'darwin') {
      await mainWindow.webContents.insertCSS(
        '.electron-traffic-light-pad { margin-left: 74px; }' +
        '.electron-fullscreen .electron-traffic-light-pad { margin-left: 0; }',
      )
    }
    mainWindow.show()
    broadcastUpdaterState()
  })

  // Toggle fullscreen class to remove traffic-light padding in fullscreen
  if (process.platform === 'darwin') {
    mainWindow.on('enter-full-screen', () => {
      mainWindow?.webContents.executeJavaScript(
        'document.body.classList.add("electron-fullscreen")',
      )
    })
    mainWindow.on('leave-full-screen', () => {
      mainWindow?.webContents.executeJavaScript(
        'document.body.classList.remove("electron-fullscreen")',
      )
    })
  }

  mainWindow.loadURL(url)

  if (isDev) {
    mainWindow.webContents.openDevTools({ mode: 'detach' })
  }

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

// ---------------------------------------------------------------------------
// IPC: native file dialogs
// ---------------------------------------------------------------------------

function setupIPC(): void {
  ipcMain.handle('dialog:openFile', async () => {
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
      await writeFile(payload.filePath, payload.content, 'utf-8')
      return payload.filePath
    },
  )

  ipcMain.handle('updater:getState', () => updaterState)

  ipcMain.handle('updater:checkForUpdates', async () => {
    await checkForAppUpdates(true)
    return updaterState
  })

  ipcMain.handle('updater:quitAndInstall', () => {
    if (!isDev && updaterState.status === 'downloaded') {
      autoUpdater.quitAndInstall()
      return true
    }
    return false
  })
}

// ---------------------------------------------------------------------------
// App lifecycle
// ---------------------------------------------------------------------------

app.on('ready', async () => {
  fixPath()
  setupIPC()

  if (!isDev) {
    try {
      serverPort = await startNitroServer()
      console.log(`Nitro server started on port ${serverPort}`)
    } catch (err) {
      console.error('Failed to start Nitro server:', err)
      app.quit()
      return
    }
  }

  createWindow()

  if (!isDev) {
    setupAutoUpdater()
  }
})

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit()
  }
})

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow()
  }
})

app.on('before-quit', () => {
  if (nitroProcess) {
    nitroProcess.kill()
    nitroProcess = null
  }
})
