import {
  app,
  BrowserWindow,
  ipcMain,
  dialog,
  type BrowserWindowConstructorOptions,
} from 'electron'
import { execSync } from 'node:child_process'
import { fork, type ChildProcess } from 'node:child_process'
import { createServer } from 'node:net'
import { join, extname } from 'node:path'
import { homedir } from 'node:os'
import { readFile, writeFile, mkdir, unlink } from 'node:fs/promises'

import { buildAppMenu } from './app-menu'
import {
  PORT_FILE_DIR_NAME,
  PORT_FILE_NAME,
  VITE_DEV_PORT,
  WINDOW_WIDTH,
  WINDOW_HEIGHT,
  WINDOW_MIN_WIDTH,
  WINDOW_MIN_HEIGHT,
  TITLEBAR_OVERLAY_HEIGHT,
  MACOS_TRAFFIC_LIGHT_POSITION,
  MACOS_TRAFFIC_LIGHT_PAD,
  WIN_CONTROLS_PAD,
  LINUX_CONTROLS_PAD,
  NITRO_HOST,
  NITRO_FALLBACK_TIMEOUT_WIN,
  NITRO_FALLBACK_TIMEOUT_DEFAULT,
} from './constants'
import {
  setupAutoUpdater,
  broadcastUpdaterState,
  setUpdaterState,
  clearUpdateTimer,
  setAutoUpdateEnabled,
} from './auto-updater'
import { initLogger, log } from './logger'
import { setupIPC } from './ipc-handlers'

let mainWindow: BrowserWindow | null = null
let nitroProcess: ChildProcess | null = null
let serverPort = 0
let pendingFilePath: string | null = null

const isDev = !app.isPackaged
// Settings stored in platform-standard app data dir (Electron-managed):
// macOS: ~/Library/Application Support/OpenPencil/
// Windows: %APPDATA%\OpenPencil\
// Linux: ~/.config/OpenPencil/
const SETTINGS_PATH = join(app.getPath('userData'), 'settings.json')
const PREFS_PATH = join(app.getPath('userData'), 'preferences.json')

// ---------------------------------------------------------------------------
// Renderer preferences (replaces localStorage which is origin-scoped)
// ---------------------------------------------------------------------------

let prefsCache: Record<string, string> = {}
let prefsDirty = false
let prefsWriteTimer: ReturnType<typeof setTimeout> | null = null

async function loadPrefs(): Promise<void> {
  try {
    const raw = await readFile(PREFS_PATH, 'utf-8')
    prefsCache = JSON.parse(raw)
  } catch {
    prefsCache = {}
  }
}

function schedulePrefsWrite(): void {
  if (prefsWriteTimer) return
  prefsDirty = true
  prefsWriteTimer = setTimeout(async () => {
    prefsWriteTimer = null
    if (!prefsDirty) return
    prefsDirty = false
    try {
      await mkdir(app.getPath('userData'), { recursive: true })
      await writeFile(PREFS_PATH, JSON.stringify(prefsCache, null, 2), 'utf-8')
    } catch (err) {
      log.error(`[prefs] Failed to write preferences: ${err}`)
    }
  }, 500)
}

// ---------------------------------------------------------------------------
// Fix PATH for GUI apps (shell PATH not inherited)
// ---------------------------------------------------------------------------

function fixPath(): void {
  if (process.platform === 'win32') {
    // Windows GUI apps inherit PATH from the system, but common tool install
    // dirs (npm global, scoop, cargo, etc.) may be missing in packaged apps.
    const home = homedir()
    const extraDirs = [
      join(home, 'AppData', 'Roaming', 'npm'),          // npm global
      join(home, 'AppData', 'Local', 'Programs', 'Microsoft VS Code', 'bin'), // VS Code CLI
      join(home, '.cargo', 'bin'),                        // Rust/cargo
      join(home, 'scoop', 'shims'),                       // scoop
      join(home, '.bun', 'bin'),                          // bun
    ]
    const current = process.env.PATH || ''
    const existing = new Set(current.split(';').map((p) => p.toLowerCase()))
    const additions = extraDirs.filter((d) => !existing.has(d.toLowerCase()))
    if (additions.length > 0) {
      process.env.PATH = [...additions, current].join(';')
    }
    return
  }

  if (process.platform !== 'darwin' && process.platform !== 'linux') return

  try {
    const shell = process.env.SHELL || (process.platform === 'darwin' ? '/bin/zsh' : '/bin/bash')
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
    // Packaged app may not have a login shell — add common tool dirs as fallback
    const home = homedir()
    const fallbackDirs = [
      join(home, '.local', 'bin'),
      join(home, '.cargo', 'bin'),
      join(home, '.bun', 'bin'),
      '/usr/local/bin',
      '/opt/homebrew/bin',
    ]
    const current = process.env.PATH || ''
    const existing = new Set(current.split(':'))
    const additions = fallbackDirs.filter((d) => !existing.has(d))
    if (additions.length > 0) {
      process.env.PATH = [...additions, current].join(':')
    }
  }
}

// ---------------------------------------------------------------------------
// App settings
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
  await mkdir(app.getPath('userData'), { recursive: true })
  await writeFile(SETTINGS_PATH, JSON.stringify(merged, null, 2), 'utf-8')
}

// ---------------------------------------------------------------------------
// Port file for MCP sync discovery (~/.openpencil/.port)
// ---------------------------------------------------------------------------

const PORT_FILE_DIR = join(homedir(), PORT_FILE_DIR_NAME)
const PORT_FILE_PATH = join(PORT_FILE_DIR, PORT_FILE_NAME)

async function writePortFile(port: number): Promise<void> {
  try {
    await mkdir(PORT_FILE_DIR, { recursive: true })
    await writeFile(
      PORT_FILE_PATH,
      JSON.stringify({ port, pid: process.pid, timestamp: Date.now() }),
      'utf-8',
    )
  } catch (err) {
    log.error(`[port-file] Failed to write port file: ${err}`)
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
    server.listen(0, NITRO_HOST, () => {
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
        HOST: NITRO_HOST,
        PORT: String(port),
        NITRO_HOST: NITRO_HOST,
        NITRO_PORT: String(port),
        ELECTRON_RESOURCES_PATH: process.resourcesPath,
      },
      stdio: 'pipe',
    })

    nitroProcess = child

    child.stdout?.on('data', (data: Buffer) => {
      const msg = data.toString()
      log.info(`[nitro] ${msg.trimEnd()}`)
      // Resolve once Nitro reports it's listening
      if (msg.includes('Listening') || msg.includes('ready')) {
        resolve(port)
      }
    })

    child.stderr?.on('data', (data: Buffer) => {
      log.error(`[nitro:err] ${data.toString().trimEnd()}`)
    })

    child.on('error', reject)
    child.on('exit', (code) => {
      if (code !== 0 && code !== null) {
        log.error(`Nitro exited with code ${code}`)
      }
      nitroProcess = null
      // Auto-restart Nitro server if it crashes while app is running
      if (code !== 0 && code !== null && mainWindow && !mainWindow.isDestroyed()) {
        log.info('[nitro] Restarting server after crash...')
        startNitroServer()
          .then((newPort) => {
            serverPort = newPort
            writePortFile(newPort)
            log.info(`[nitro] Restarted on port ${newPort}`)
            if (mainWindow && !mainWindow.isDestroyed()) {
              mainWindow.loadURL(`http://${NITRO_HOST}:${newPort}/editor`)
            }
          })
          .catch((err) => {
            log.error(`[nitro] Failed to restart: ${err}`)
          })
      }
    })

    // Fallback: if no stdout "ready" message comes, wait then resolve anyway.
    // Use longer timeout on Windows (slower process creation).
    const fallbackMs = process.platform === 'win32' ? NITRO_FALLBACK_TIMEOUT_WIN : NITRO_FALLBACK_TIMEOUT_DEFAULT
    setTimeout(() => resolve(port), fallbackMs)
  })
}

// ---------------------------------------------------------------------------
// Linux window-controls side detection
// ---------------------------------------------------------------------------

/** Cached result for Linux controls side detection. */
let cachedLinuxControlsSide: 'left' | 'right' | null = null

/**
 * Detect whether Linux DE places window controls on the left or right.
 * Uses gsettings (GNOME/Cinnamon/MATE) as primary check, checks XDG_CURRENT_DESKTOP
 * for known right-side DEs, then defaults to right. Result is cached.
 */
function getLinuxControlsSide(): 'left' | 'right' {
  if (cachedLinuxControlsSide) return cachedLinuxControlsSide

  let result: 'left' | 'right' = 'right'

  // Try gsettings (works for GNOME, Cinnamon, MATE, Budgie)
  try {
    const layout = execSync(
      'gsettings get org.gnome.desktop.wm.preferences button-layout',
      { encoding: 'utf-8', timeout: 3000 },
    )
      .trim()
      .replace(/'/g, '')
    const colonIndex = layout.indexOf(':')
    if (colonIndex >= 0) {
      const beforeColon = layout.slice(0, colonIndex)
      if (
        beforeColon.includes('close') ||
        beforeColon.includes('minimize') ||
        beforeColon.includes('maximize')
      ) {
        result = 'left'
      }
    }
  } catch {
    // gsettings not available — check desktop environment
    const desktop = (process.env.XDG_CURRENT_DESKTOP || '').toLowerCase()
    // KDE, XFCE, LXQt default to right. elementary OS defaults to left.
    if (desktop.includes('pantheon')) {
      result = 'left'
    }
  }

  cachedLinuxControlsSide = result
  return result
}

// ---------------------------------------------------------------------------
// Window
// ---------------------------------------------------------------------------

function createWindow(): void {
  const isWinOrLinux = process.platform === 'win32' || process.platform === 'linux'

  const windowOptions: BrowserWindowConstructorOptions = {
    width: WINDOW_WIDTH,
    height: WINDOW_HEIGHT,
    minWidth: WINDOW_MIN_WIDTH,
    minHeight: WINDOW_MIN_HEIGHT,
    title: 'OpenPencil',
    titleBarStyle: process.platform === 'darwin' ? 'hiddenInset' : 'hidden',
    ...(isWinOrLinux
      ? {
          titleBarOverlay: {
            // Windows supports transparent overlay; Linux uses solid color (updated via theme:set IPC)
            color: process.platform === 'win32' ? 'rgba(0,0,0,0)' : '#1a1a1a',
            symbolColor: '#d4d4d8',
            height: TITLEBAR_OVERLAY_HEIGHT,
          },
        }
      : {}),
    webPreferences: {
      preload: join(__dirname, 'preload.cjs'),
      contextIsolation: true,
      nodeIntegration: false,
      // Persist localStorage/cookies in a fixed partition so data survives
      // across random Nitro server port changes (origin-independent storage).
      partition: 'persist:openpencil',
    },
  }

  if (process.platform === 'darwin') {
    windowOptions.trafficLightPosition = MACOS_TRAFFIC_LIGHT_POSITION
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
    ? `http://localhost:${VITE_DEV_PORT}/editor`
    : `http://${NITRO_HOST}:${serverPort}/editor`

  // Inject traffic-light padding CSS then show window (no flash)
  mainWindow.webContents.on('did-finish-load', async () => {
    if (!mainWindow) return
    if (process.platform === 'darwin') {
      await mainWindow.webContents.insertCSS(
        `.electron-traffic-light-pad { margin-left: ${MACOS_TRAFFIC_LIGHT_PAD}px; }` +
        '.electron-fullscreen .electron-traffic-light-pad { margin-left: 0; }',
      )
    }
    if (process.platform === 'win32') {
      await mainWindow.webContents.insertCSS(
        `.electron-win-controls-pad { margin-right: ${WIN_CONTROLS_PAD}px; }`,
      )
    }
    if (process.platform === 'linux') {
      const side = getLinuxControlsSide()
      if (side === 'left') {
        await mainWindow.webContents.insertCSS(
          `.electron-traffic-light-pad { margin-left: ${LINUX_CONTROLS_PAD}px; }`,
        )
      } else {
        await mainWindow.webContents.insertCSS(
          `.electron-win-controls-pad { margin-right: ${LINUX_CONTROLS_PAD}px; }`,
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

  // ---------------------------------------------------------------------------
  // Unsaved-changes close confirmation
  // ---------------------------------------------------------------------------
  let forceClose = false

  mainWindow.on('close', (event) => {
    if (forceClose || !mainWindow || mainWindow.isDestroyed()) return

    // preventDefault must be called synchronously — check dirty state after
    event.preventDefault()

    const win = mainWindow
    win.webContents
      .executeJavaScript(
        '({ dirty: window.__documentIsDirty === true,' +
        ' message: typeof window.__i18nT === "function" ? window.__i18nT("topbar.closeConfirmMessage") : "",' +
        ' detail: typeof window.__i18nT === "function" ? window.__i18nT("topbar.closeConfirmDetail") : "",' +
        ' save: typeof window.__i18nT === "function" ? window.__i18nT("common.save") : "",' +
        ' dontSave: typeof window.__i18nT === "function" ? window.__i18nT("topbar.dontSave") : "",' +
        ' cancel: typeof window.__i18nT === "function" ? window.__i18nT("common.cancel") : "" })',
      )
      .then((result: { dirty: boolean; message: string; detail: string; save: string; dontSave: string; cancel: string }) => {
        if (!result.dirty) {
          // Not dirty — allow close
          forceClose = true
          win.close()
          return
        }

        // Show native save dialog with i18n strings
        return dialog
          .showMessageBox(win, {
            type: 'question',
            buttons: [
              result.save || 'Save',
              result.dontSave || "Don't Save",
              result.cancel || 'Cancel',
            ],
            defaultId: 0,
            cancelId: 2,
            message: result.message || 'Do you want to save changes before closing?',
            detail: result.detail || 'Your changes will be lost if you don\'t save them.',
          })
          .then(({ response }) => {
            if (response === 0) {
              // Save — tell renderer to save then confirm close
              win.webContents.send('menu:action', 'save-and-close')
            } else if (response === 1) {
              // Don't Save — force close
              forceClose = true
              win.close()
            }
            // Cancel (response === 2) — do nothing, window stays open
          })
      })
      .catch(() => {
        // Page not loaded or crashed — allow close
        forceClose = true
        win.close()
      })
  })

  // Renderer confirms save completed → force close
  ipcMain.on('window:confirmClose', () => {
    forceClose = true
    mainWindow?.close()
  })

  mainWindow.on('closed', () => {
    mainWindow = null
  })
}

// ---------------------------------------------------------------------------
// IPC: native file dialogs & updater (extracted to ipc-handlers.ts)
// ---------------------------------------------------------------------------

function initIPC(): void {
  setupIPC({
    getMainWindow: () => mainWindow,
    getPendingFilePath: () => pendingFilePath,
    clearPendingFilePath: () => { pendingFilePath = null },
    prefsCache,
    schedulePrefsWrite,
    writeAppSettings,
  })
}

// ---------------------------------------------------------------------------
// File association: open .op files
// ---------------------------------------------------------------------------

/** Extract .op file path from command-line arguments. */
function getFilePathFromArgs(args: string[]): string | null {
  for (const arg of args) {
    // Skip flags and the Electron binary/script path
    if (arg.startsWith('-') || arg.startsWith('--')) continue
    const ext = extname(arg).toLowerCase()
    if (ext === '.op' || ext === '.pen') {
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
  await initLogger(app.getPath('userData'))
  fixPath()
  await loadPrefs()
  initIPC()
  buildAppMenu()

  if (!isDev) {
    try {
      serverPort = await startNitroServer()
      log.info(`Nitro server started on port ${serverPort}`)
      await writePortFile(serverPort)
    } catch (err) {
      log.error(`Failed to start Nitro server: ${err}`)
      dialog.showErrorBox(
        'OpenPencil',
        `Failed to start the application server.\n\n${err instanceof Error ? err.message : String(err)}\n\nThe application will now quit.`,
      )
      app.quit()
      return
    }
  } else {
    // Dev mode: Vite dev server runs on port 3000
    await writePortFile(VITE_DEV_PORT)
  }

  createWindow()

  // Check for file to open: pending open-file event or CLI args (Windows/Linux).
  // The file path is stored in pendingFilePath and pulled by the renderer
  // via file:getPending IPC when the React app mounts (useElectronMenu hook).
  if (!pendingFilePath) {
    pendingFilePath = getFilePathFromArgs(process.argv)
  }

  if (!isDev) {
    const settings = await readAppSettings()
    const autoUpdate = settings.autoUpdate !== false
    setAutoUpdateEnabled(autoUpdate)
    if (autoUpdate) {
      setupAutoUpdater()
    } else {
      setUpdaterState({ status: 'disabled' })
    }
  }
})

app.on('window-all-closed', () => {
  app.quit()
})

app.on('activate', () => {
  if (mainWindow === null) {
    createWindow()
  }
})

app.on('before-quit', () => {
  clearUpdateTimer()
  killNitroProcess()
  cleanupPortFile().catch(() => {})
})

/** Platform-aware Nitro process termination. */
function killNitroProcess(): void {
  if (!nitroProcess) return
  const pid = nitroProcess.pid
  if (process.platform === 'win32') {
    // SIGTERM is unreliable on Windows; use taskkill for proper tree-kill
    try {
      if (pid) {
        execSync(`taskkill /pid ${pid} /T /F`, { stdio: 'ignore' })
      }
    } catch { /* process may have already exited */ }
  } else {
    // SIGTERM may not be processed before the main process exits,
    // leaving orphan Nitro processes. Kill the entire process group
    // with SIGKILL for reliable cleanup.
    try {
      if (pid) process.kill(-pid, 'SIGKILL')
    } catch { /* process may have already exited */ }
    try {
      nitroProcess.kill('SIGKILL')
    } catch { /* ignore */ }
  }
  nitroProcess = null
}

// Ensure child process cleanup on unexpected termination (Linux/macOS signals)
for (const signal of ['SIGINT', 'SIGTERM', 'SIGHUP'] as const) {
  process.on(signal, () => {
    killNitroProcess()
    cleanupPortFile().finally(() => process.exit(0))
  })
}
