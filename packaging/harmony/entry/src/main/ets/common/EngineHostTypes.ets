/**
 * Which of the TopBar's painted traffic-light dots the user pressed. The
 * engine owns no window, so it can only name the intent.
 */
export enum WindowControlKind {
  CLOSE = 0,
  MINIMIZE = 1,
  ZOOM = 2,
}

/** Main-thread handlers the page installs for engine-requested shell work. */
export interface ShellActionSink {
  onOpenDocument: () => void;
  onImportImageOrSvg: () => void;
  onExportDocument: () => void;
  /** The engine asked for the Save / Save As file picker (action 11). */
  onSaveDocument: () => void;
  onOpenAccountCenter: () => void;
  onRequestLogin: () => void;
  onOpenLanguagePicker: () => void;
  /** A TopBar traffic-light dot asked the shell to drive the window. */
  onWindowControl: (kind: WindowControlKind) => void;
  /**
   * The engine gained or lost text-input focus. The page moves the hidden
   * IME proxy's ArkUI focus to match — that proxy is what makes the app a
   * real input client, which a SURFACE XComponent is not.
   */
  onImeFocusChanged: (focused: boolean) => void;
  /** The engine produced a device-login verification URL. */
  onOpenLoginUi: (url: string) => void;
  /** The engine terminated the login flow itself. */
  onCloseLoginUi: () => void;
  /** The engine's backdrop needs light system glyphs. */
  onSystemChromeAppearance: (prefersLightIcons: boolean) => void;
  /** Long-press over a focused engine input: show Paste at `(x, y)` vp. */
  onEditorPasteMenu: (x: number, y: number) => void;
}
