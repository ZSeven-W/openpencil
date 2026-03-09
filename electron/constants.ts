/**
 * Shared constants for Electron main process and auto-updater.
 */

// GitHub publish target — used by auto-updater feed URL
export const GITHUB_OWNER = 'ZSeven-W'
export const GITHUB_REPO = 'openpencil'

// Port file for MCP sync discovery
export const PORT_FILE_DIR_NAME = '.openpencil'
export const PORT_FILE_NAME = '.port'

// Dev server
export const VITE_DEV_PORT = 3000

// Window defaults
export const WINDOW_WIDTH = 1440
export const WINDOW_HEIGHT = 900
export const WINDOW_MIN_WIDTH = 1024
export const WINDOW_MIN_HEIGHT = 600
export const TITLEBAR_OVERLAY_HEIGHT = 36
export const MACOS_TRAFFIC_LIGHT_POSITION = { x: 16, y: 11 }

// CSS padding for window controls (px)
export const MACOS_TRAFFIC_LIGHT_PAD = 74
export const WIN_CONTROLS_PAD = 140
export const LINUX_CONTROLS_PAD = 140

// Nitro server
export const NITRO_HOST = '127.0.0.1'
export const NITRO_FALLBACK_TIMEOUT_WIN = 6000
export const NITRO_FALLBACK_TIMEOUT_DEFAULT = 3000
