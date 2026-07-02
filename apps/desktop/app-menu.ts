import { app, BrowserWindow, Menu } from 'electron';

function sendMenuAction(action: string): void {
  const win = BrowserWindow.getFocusedWindow() ?? BrowserWindow.getAllWindows()[0];
  win?.webContents.send('menu:action', action);
}

/** Recent files submenu — rebuilt each time the menu opens. */
function buildRecentFilesSubmenu(): Electron.MenuItemConstructorOptions[] {
  const recent = app.isReady() ? ((global as any).__recentFiles ?? []) : [];
  if (recent.length === 0) {
    return [{ label: 'No Recent Files', enabled: false }];
  }
  const items: Electron.MenuItemConstructorOptions[] = recent.map(
    (entry: { fileName: string; filePath: string }) => ({
      label: entry.fileName,
      click: () => sendMenuAction(`open-recent:${entry.filePath}`),
    }),
  );
  items.push({ type: 'separator' });
  items.push({
    label: 'Clear Recent Files',
    click: () => sendMenuAction('clear-recent-files'),
  });
  return items;
}

export function buildAppMenu(): void {
  const isMac = process.platform === 'darwin';

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
        {
          label: 'Open Recent',
          submenu: buildRecentFilesSubmenu(),
        },
        { type: 'separator' },
        {
          label: 'Save',
          accelerator: 'CmdOrCtrl+S',
          click: () => sendMenuAction('save'),
        },
        {
          label: 'Save As\u2026',
          accelerator: 'CmdOrCtrl+Shift+S',
          click: () => sendMenuAction('save-as'),
        },
        { type: 'separator' },
        {
          label: 'Export Image\u2026',
          // Use Cmd+Shift+P (P = Print/PDF/Picture). Cmd+Shift+E was being
          // swallowed at the OS level by Chinese IMEs / system tools on
          // macOS before reaching the renderer.
          //
          // `registerAccelerator: false` keeps the hint visible in the menu
          // but tells Electron NOT to register it with the OS — the
          // keystroke is handled by the renderer's capture-phase document
          // keydown listener in editor-layout.tsx, which avoids HMR/IPC
          // fragility.
          accelerator: 'CmdOrCtrl+Shift+P',
          registerAccelerator: false,
          click: () => sendMenuAction('export-image'),
        },
        { type: 'separator' },
        {
          label: 'Import Figma\u2026',
          accelerator: 'CmdOrCtrl+Shift+F',
          click: () => sendMenuAction('import-figma'),
        },
        ...(!isMac ? [{ type: 'separator' as const }, { role: 'quit' as const }] : []),
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
        // Reload / Force Reload / DevTools are dev-only — hide in packaged builds.
        ...(app.isPackaged
          ? []
          : [
              { role: 'reload' as const },
              { role: 'forceReload' as const },
              { role: 'toggleDevTools' as const },
              { type: 'separator' as const },
            ]),
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
          ? [{ type: 'separator' as const }, { role: 'front' as const }]
          : [{ role: 'close' as const }]),
      ],
    },
  ];

  Menu.setApplicationMenu(Menu.buildFromTemplate(template));
}
