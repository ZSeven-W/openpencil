import {
  app,
  BrowserWindow,
  Menu,
  ipcMain,
  dialog,
  type BrowserWindowConstructorOptions,
} from 'electron'
import { autoUpdater } from 'electron-updater'
import { GitHubProvider } from 'electron-updater/out/providers/GitHubProvider'
import { execSync } from 'node:child_process'
import { fork, type ChildProcess } from 'node:child_process'
import { createServer } from 'node:net'
import { join, resolve, extname } from 'node:path'
import { homedir } from 'node:os'
import { readFile, writeFile, mkdir, unlink } from 'node:fs/promises'

let mainWindow: BrowserWindow | null = null
let nitroProcess: ChildProcess | null = null
let serverPort = 0
let autoUpdateEnabled = true
let updateCheckTimer: ReturnType<typeof setInterval> | null = null
let pendingFilePath: string | null = null

const isDev = !app.isPackaged
const SETTINGS_PATH = join(homedir(), '.openpencil', 'settings.json')

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

const MacGitHubUpdateProvider = class {
  constructor(options: unknown, updater: unknown, runtimeOptions: unknown) {
    const provider = new (GitHubProvider as any)(options, updater, runtimeOptions) as any
    if (process.platform === 'darwin') {
      provider.getDefaultChannelName = () =>
        process.arch === 'arm64' ? 'latest-mac-arm64' : 'latest-mac'
      provider.getCustomChannelName = (channel: string) => channel
    }
    return provider
  }
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

  if (process.platform === 'darwin') {
    autoUpdater.setFeedURL({
      provider: 'custom',
      updateProvider: MacGitHubUpdateProvider as any,
      owner: 'ZSeven-W',
      repo: 'openpencil',
      releaseType: 'release',
    } as any)
  }

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

  if (autoUpdateEnabled) {
    // Delay first check until app startup work is done.
    setTimeout(() => {
      void checkForAppUpdates(true)
    }, 5000)

    // Poll for new versions while app is running.
    updateCheckTimer = setInterval(() => {
      void checkForAppUpdates(false)
    }, 60 * 60 * 1000)
    updateCheckTimer.unref()
  }
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
// App settings (~/.openpencil/settings.json)
// ---------------------------------------------------------------------------

interface AppSettings {
  autoUpdate?: boolean
}

async function readAppSettings(): Promise<AppSettings> {
  try {
    const raw = await readFile(SETTINGS_PATH, 'utf-8')
    return JSON.parse(raw)
  } catch {
    return {}
  }
}

async function writeAppSettings(patch: Partial<AppSettings>): Promise<void> {
  const current = await readAppSettings()
  const merged = { ...current, ...patch }
  await mkdir(PORT_FILE_DIR, { recursive: true })
  await writeFile(SETTINGS_PATH, JSON.stringify(merged, null, 2), 'utf-8')
}

// ---------------------------------------------------------------------------
// Port file for MCP sync discovery (~/.openpencil/.port)
// ---------------------------------------------------------------------------

const PORT_FILE_DIR = join(homedir(), '.openpencil')
const PORT_FILE_PATH = join(PORT_FILE_DIR, '.port')

async function writePortFile(port: number): Promise<void> {
  try {
    await mkdir(PORT_FILE_DIR, { recursive: true })
    await writeFile(
      PORT_FILE_PATH,
      JSON.stringify({ port, pid: process.pid, timestamp: Date.now() }),
      'utf-8',
    )
  } catch {
    // Non-critical — MCP sync will fall back to file I/O
  }
}

async function cleanupPortFile(): Promise<void> {
  try {
    await unlink(PORT_FILE_PATH)
  } catch {
    // Ignore if already removed
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
// Linux window-controls side detection
// ---------------------------------------------------------------------------

/**
 * Detect whether Linux DE places window controls on the left or right.
 * Uses gsettings (GNOME/Cinnamon/MATE) as primary check, defaults to right
 * for KDE/XFCE and when detection fails.
 */
function getLinuxControlsSide(): 'left' | 'right' {
  try {
    const layout = execSync(
      'gsettings get org.gnome.desktop.wm.preferences button-layout',
      { encoding: 'utf-8', timeout: 3000 },
    )
      .trim()
      .replace(/'/g, '')
    // Format: "close,minimize,maximize:" → left, ":minimize,maximize,close" → right
    const colonIndex = layout.indexOf(':')
    if (colonIndex < 0) return 'right'
    const beforeColon = layout.slice(0, colonIndex)
    if (
      beforeColon.includes('close') ||
      beforeColon.includes('minimize') ||
      beforeColon.includes('maximize')
    ) {
      return 'left'
    }
    return 'right'
  } catch {
    return 'right'
  }
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

function createWindow(): void {
  const isWinOrLinux = process.platform === 'win32' || process.platform === 'linux'

  const windowOptions: BrowserWindowConstructorOptions = {
    width: 1440,
    height: 900,
    minWidth: 1024,
    minHeight: 600,
    title: 'OpenPencil',
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'hidden',
    ...(isWinOrLinux
      ? {
          titleBarOverlay: {
            color: 'rgba(0,0,0,0)',
            symbolColor: '#a1a1aa',
            height: 36,
          },
        }
      : {}),
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

  // Hide native menu bar on Windows/Linux (shortcuts still work via Alt key)
  if (isWinOrLinux) {
    mainWindow.setAutoHideMenuBar(true)
    mainWindow.setMenuBarVisibility(false)
  }

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
    if (process.platform === 'win32') {
      await mainWindow.webContents.insertCSS(
        '.electron-win-controls-pad { margin-right: 140px; }',
      )
    }
    if (process.platform === 'linux') {
      const side = getLinuxControlsSide()
      if (side === 'left') {
        await mainWindow.webContents.insertCSS(
          '.electron-traffic-light-pad { margin-left: 140px; }',
        )
      } else {
        await mainWindow.webContents.insertCSS(
          '.electron-win-controls-pad { margin-right: 140px; }',
        )
      }
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
      const resolved = resolve(payload.filePath)
      if (resolved.includes('\0')) {
        throw new Error('Invalid file path')
      }
      const ext = extname(resolved).toLowerCase()
      if (ext !== '.op' && ext !== '.pen') {
        throw new Error('Only .op and .pen file extensions are allowed')
      }
      // Directory allowlist: only allow writes under user home or OS temp
      const allowedRoots = [app.getPath('home'), app.getPath('temp')]
      const inAllowedDir = allowedRoots.some(
        (root) => resolved === root || resolved.startsWith(root + '/') || resolved.startsWith(root + '\\'),
      )
      if (!inAllowedDir) {
        throw new Error('File path must be within the user home or temp directory')
      }
      await writeFile(resolved, payload.content, 'utf-8')
      return resolved
    },
  )

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
  ipcMain.handle('theme:set', (_event, theme: 'dark' | 'light') => {
    if (!mainWindow || mainWindow.isDestroyed()) return
    const isWinOrLinux = process.platform === 'win32' || process.platform === 'linux'
    if (!isWinOrLinux) return
    mainWindow.setTitleBarOverlay({
      color: 'rgba(0,0,0,0)',
      symbolColor: theme === 'dark' ? '#a1a1aa' : '#71717a',
    })
  })

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

  ipcMain.handle('updater:getAutoCheck', () => autoUpdateEnabled)

  ipcMain.handle('updater:setAutoCheck', async (_event, enabled: boolean) => {
    autoUpdateEnabled = enabled
    await writeAppSettings({ autoUpdate: enabled })

    if (enabled) {
      // Start polling if not already running
      if (!updateCheckTimer) {
        updateCheckTimer = setInterval(() => {
          void checkForAppUpdates(false)
        }, 60 * 60 * 1000)
        updateCheckTimer.unref()
      }
      setUpdaterState({ status: 'idle' })
    } else {
      // Stop polling
      if (updateCheckTimer) {
        clearInterval(updateCheckTimer)
        updateCheckTimer = null
      }
      setUpdaterState({ status: 'disabled' })
    }
    return enabled
  })
}

// ---------------------------------------------------------------------------
// Application menu
// ---------------------------------------------------------------------------

function sendMenuAction(action: string): void {
  const win = BrowserWindow.getFocusedWindow() ?? mainWindow
  win?.webContents.send('menu:action', action)
}

function buildAppMenu(): void {
  const isMac = process.platform === 'darwin'

  const template: Electron.MenuItemConstructorOptions[] = [
    // macOS app menu
    ...(isMac
      ? [
          {
            label: app.name,
            submenu: [
              { role: 'about' as const },
              { type: 'separator' as const },
              { role: 'services' as const },
              { type: 'separator' as const },
              { role: 'hide' as const },
              { role: 'hideOthers' as const },
              { role: 'unhide' as const },
              { type: 'separator' as const },
              { role: 'quit' as const },
            ],
          },
        ]
      : []),

    // File menu
    {
      label: 'File',
      submenu: [
        {
          label: 'New',
          accelerator: 'CmdOrCtrl+N',
          click: () => sendMenuAction('new'),
        },
        {
          label: 'Open\u2026',
          accelerator: 'CmdOrCtrl+O',
          click: () => sendMenuAction('open'),
        },
        { type: 'separator' },
        {
          label: 'Save',
          accelerator: 'CmdOrCtrl+S',
          click: () => sendMenuAction('save'),
        },
        { type: 'separator' },
        {
          label: 'Import Figma\u2026',
          accelerator: 'CmdOrCtrl+Shift+F',
          click: () => sendMenuAction('import-figma'),
        },
        ...(!isMac
          ? [{ type: 'separator' as const }, { role: 'quit' as const }]
          : []),
      ],
    },

    // Edit menu (role-based for native text input support)
    {
      label: 'Edit',
      submenu: [
        {
          label: 'Undo',
          accelerator: 'CmdOrCtrl+Z',
          click: () => sendMenuAction('undo'),
        },
        {
          label: 'Redo',
          accelerator: 'CmdOrCtrl+Shift+Z',
          click: () => sendMenuAction('redo'),
        },
        { type: 'separator' },
        { role: 'cut' },
        { role: 'copy' },
        { role: 'paste' },
        ...(isMac ? [{ role: 'pasteAndMatchStyle' as const }] : []),
        { role: 'selectAll' },
      ],
    },

    // View menu
    {
      label: 'View',
      submenu: [
        { role: 'reload' },
        { role: 'forceReload' },
        { role: 'toggleDevTools' },
        { type: 'separator' },
        { role: 'resetZoom' },
        { role: 'zoomIn' },
        { role: 'zoomOut' },
        { type: 'separator' },
        { role: 'togglefullscreen' },
      ],
    },

    // Window menu
    {
      label: 'Window',
      submenu: [
        { role: 'minimize' },
        { role: 'zoom' },
        ...(isMac
          ? [
              { type: 'separator' as const },
              { role: 'front' as const },
            ]
          : [{ role: 'close' as const }]),
      ],
    },
  ]

  Menu.setApplicationMenu(Menu.buildFromTemplate(template))
}

// ---------------------------------------------------------------------------
// File association: open .op files
// ---------------------------------------------------------------------------

/** Extract .op file path from command-line arguments. */
function getFilePathFromArgs(args: string[]): string | null {
  for (const arg of args) {
    if (arg.endsWith('.op') || arg.endsWith('.pen')) {
      return arg
    }
  }
  return null
}

/** Send a file path to the renderer for loading. */
function sendOpenFile(filePath: string): void {
  if (mainWindow && !mainWindow.isDestroyed()) {
    mainWindow.webContents.send('file:open', filePath)
  } else {
    pendingFilePath = filePath
  }
}

// macOS: open-file fires when user double-clicks a .op file
app.on('open-file', (event, filePath) => {
  event.preventDefault()
  if (app.isReady()) {
    sendOpenFile(filePath)
  } else {
    pendingFilePath = filePath
  }
})

// Single instance lock (Windows/Linux: second instance passes file path as arg)
const gotTheLock = app.requestSingleInstanceLock()

if (!gotTheLock) {
  app.quit()
} else {
  app.on('second-instance', (_event, argv) => {
    const filePath = getFilePathFromArgs(argv)
    if (filePath) {
      sendOpenFile(filePath)
    }
    // Focus existing window
    if (mainWindow) {
      if (mainWindow.isMinimized()) mainWindow.restore()
      mainWindow.focus()
    }
  })
}

// ---------------------------------------------------------------------------
// App lifecycle
// ---------------------------------------------------------------------------

app.on('ready', async () => {
  fixPath()
  setupIPC()
  buildAppMenu()

  if (!isDev) {
    try {
      serverPort = await startNitroServer()
      console.log(`Nitro server started on port ${serverPort}`)
      await writePortFile(serverPort)
    } catch (err) {
      console.error('Failed to start Nitro server:', err)
      app.quit()
      return
    }
  } else {
    // Dev mode: Vite dev server runs on port 3000
    await writePortFile(3000)
  }

  createWindow()

  // Check for file to open: pending open-file event or CLI args (Windows/Linux)
  if (!pendingFilePath) {
    pendingFilePath = getFilePathFromArgs(process.argv)
  }
  if (pendingFilePath) {
    const fileToOpen = pendingFilePath
    pendingFilePath = null
    // Wait for renderer to be ready, then send
    mainWindow?.webContents.once('did-finish-load', () => {
      mainWindow?.webContents.send('file:open', fileToOpen)
    })
  }

  if (!isDev) {
    const settings = await readAppSettings()
    autoUpdateEnabled = settings.autoUpdate !== false
    if (autoUpdateEnabled) {
      setupAutoUpdater()
    } else {
      setUpdaterState({ status: 'disabled' })
    }
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

app.on('before-quit', async () => {
  await cleanupPortFile()
  if (nitroProcess) {
    nitroProcess.kill()
    nitroProcess = null
  }
})
