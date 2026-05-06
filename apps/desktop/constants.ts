/**
 * Shared Electron 主进程和自动更新程序的常量。
 */

// GitHub 发布目标 — 由自动更新程序 feed URL 使用
export const GITHUB_OWNER = 'ZSeven-W';
export const GITHUB_REPO = 'openpencil';

// 用于 MCP 同步发现的 Port 文件
export const PORT_FILE_DIR_NAME = '.openpencil';
export const PORT_FILE_NAME = '.port';

// Dev 服务器
export const VITE_DEV_PORT = 3000;

// Window 默认值
export const WINDOW_WIDTH = 1440;
export const WINDOW_HEIGHT = 900;
export const WINDOW_MIN_WIDTH = 1024;
export const WINDOW_MIN_HEIGHT = 600;
export const TITLEBAR_OVERLAY_HEIGHT = 36;
export const MACOS_TRAFFIC_LIGHT_POSITION = { x: 16, y: 11 };

// CSS 窗口控件的填充（px）
export const MACOS_TRAFFIC_LIGHT_PAD = 74;
export const WIN_CONTROLS_PAD = 140;
export const LINUX_CONTROLS_PAD = 140;

// Nitro 服务器
export const NITRO_HOST = '127.0.0.1';
export const NITRO_FALLBACK_TIMEOUT_WIN = 6000;
export const NITRO_FALLBACK_TIMEOUT_DEFAULT = 3000;
