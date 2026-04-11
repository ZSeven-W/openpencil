import { useState, useCallback, useEffect } from 'react';
import { TooltipProvider } from '@/components/ui/tooltip';
import TopBar from './top-bar';
import Toolbar from './toolbar';
import BooleanToolbar from './boolean-toolbar';
import StatusBar from './status-bar';
import LayerPanel from '@/components/panels/layer-panel';
import RightPanel from '@/components/panels/right-panel';
import AIChatPanel, { AIChatMinimizedBar } from '@/components/panels/ai-chat-panel';
import VariablesPanel from '@/components/panels/variables-panel';
import DesignMdPanel from '@/components/panels/design-md-panel';
import ComponentBrowserPanel from '@/components/panels/component-browser-panel';
import ExportDialog from '@/components/shared/export-dialog';
import SaveDialog from '@/components/shared/save-dialog';
import AgentSettingsDialog from '@/components/shared/agent-settings-dialog';
import FigmaImportDialog from '@/components/shared/figma-import-dialog';
import UnsavedChangesDialog from '@/components/shared/unsaved-changes-dialog';
import type { UnsavedResult } from '@/components/shared/unsaved-changes-dialog';
import UpdateReadyBanner from './update-ready-banner';
import { useAIStore } from '@/stores/ai-store';
import { useCanvasStore } from '@/stores/canvas-store';
import { useGitStore } from '@/stores/git-store';
import { useDocumentStore } from '@/stores/document-store';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import { useUIKitStore } from '@/stores/uikit-store';
import { useThemePresetStore } from '@/stores/theme-preset-store';
import { useDesignMdStore } from '@/stores/design-md-store';
import { useElectronMenu } from '@/hooks/use-electron-menu';
import { useFigmaPaste } from '@/hooks/use-figma-paste';
import { useMcpSync } from '@/hooks/use-mcp-sync';
import { useFileDrop } from '@/hooks/use-file-drop';
import { initAppStorage } from '@/utils/app-storage';
import { getRecentFiles } from '@/utils/recent-files';
import SkiaCanvas from '@/canvas/skia/skia-canvas';

export default function EditorLayout() {
  const toggleMinimize = useAIStore((s) => s.toggleMinimize);
  const hasSelection = useCanvasStore((s) => s.selection.activeId !== null);
  const layerPanelOpen = useCanvasStore((s) => s.layerPanelOpen);
  const variablesPanelOpen = useCanvasStore((s) => s.variablesPanelOpen);
  const designMdPanelOpen = useCanvasStore((s) => s.designMdPanelOpen);
  const figmaImportOpen = useCanvasStore((s) => s.figmaImportDialogOpen);
  const closeFigmaImport = useCallback(() => {
    useCanvasStore.getState().setFigmaImportDialogOpen(false);
  }, []);
  const browserOpen = useUIKitStore((s) => s.browserOpen);
  const saveDialogOpen = useDocumentStore((s) => s.saveDialogOpen);
  const closeSaveDialog = useCallback(() => {
    useDocumentStore.getState().setSaveDialogOpen(false);
  }, []);
  const exportOpen = useCanvasStore((s) => s.exportDialogOpen);
  const [unsavedDialog, setUnsavedDialog] = useState<{
    open: boolean;
    fileName: string;
    onResult: (result: UnsavedResult) => void;
  }>({ open: false, fileName: '', onResult: () => {} });

  const closeExport = useCallback(() => {
    useCanvasStore.getState().setExportDialogOpen(false);
  }, []);

  const showUnsavedDialog = useCallback((fileName: string): Promise<UnsavedResult> => {
    return new Promise((resolve) => {
      setUnsavedDialog({
        open: true,
        fileName,
        onResult: (result) => {
          setUnsavedDialog((prev) => ({ ...prev, open: false }));
          resolve(result);
        },
      });
    });
  }, []);

  useEffect(() => {
    (window as any).__showUnsavedDialog = showUnsavedDialog;
    return () => {
      delete (window as any).__showUnsavedDialog;
    };
  }, [showUnsavedDialog]);

  // Phase 4c: mount the Git autosave subscriber once for the editor's
  // lifetime. initAutosaveSubscriber is idempotent (checks for existing
  // handle), so React StrictMode's double-invocation is safe. The cleanup
  // disposes the subscriber so dev-time HMR and unmounts don't leak
  // handlers.
  useEffect(() => {
    useGitStore.getState().initAutosaveSubscriber();
    return () => {
      useGitStore.getState().disposeAutosaveSubscriber();
    };
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;

      // Cmd+J: toggle AI panel minimize
      if (isMod && e.key === 'j') {
        e.preventDefault();
        toggleMinimize();
        return;
      }

      // Cmd+Shift+C: switch right panel to code tab
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'c') {
        e.preventDefault();
        useCanvasStore.getState().setRightPanelTab('code');
        return;
      }

      // Cmd+Shift+V: toggle variables panel
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'v') {
        e.preventDefault();
        useCanvasStore.getState().toggleVariablesPanel();
        return;
      }

      // Cmd+Shift+D: toggle design system panel
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        useCanvasStore.getState().toggleDesignMdPanel();
        return;
      }

      // Cmd+Shift+K: toggle UIKit browser
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        useUIKitStore.getState().toggleBrowser();
        return;
      }

      // Cmd+Shift+F: open Figma import
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        useCanvasStore.getState().setFigmaImportDialogOpen(true);
        return;
      }

      // Cmd+,: open agent settings
      if (isMod && e.key === ',') {
        e.preventDefault();
        useAgentSettingsStore.getState().setDialogOpen(true);
        return;
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [toggleMinimize]);

  // Cmd/Ctrl+Shift+P: open the global export dialog.
  //
  // We previously used Cmd+Shift+E, but that combo is silently swallowed
  // by some macOS Chinese IMEs / system tools before the keystroke ever
  // reaches the renderer (no JS handler fires, no logs appear). P (for
  // "Print/PDF/Picture") is unused everywhere else in the app and is not
  // intercepted by common IMEs.
  //
  // Registered as a *capture-phase document listener* so it fires earliest
  // in the event chain, before any bubble-phase listener can interfere.
  // Uses `e.code === 'KeyP'` for keyboard-layout independence (an IME state
  // can change `e.key` but not `e.code`). The action is
  // `setExportDialogOpen(true)` (idempotent) rather than a toggle, so even
  // if multiple paths fire the dialog still ends up open instead of
  // cancelling itself out.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;
      if (!isMod || !e.shiftKey) return;
      if (e.code !== 'KeyP' && e.key.toLowerCase() !== 'p') return;
      e.preventDefault();
      e.stopImmediatePropagation();
      useCanvasStore.getState().setExportDialogOpen(true);
    };
    document.addEventListener('keydown', handler, { capture: true });
    return () => document.removeEventListener('keydown', handler, { capture: true });
  }, []);

  // Handle Electron native menu actions
  useElectronMenu();

  // Handle Figma clipboard paste
  useFigmaPaste();

  // MCP ↔ canvas real-time sync
  useMcpSync();

  // Drag-and-drop file open
  const isDragging = useFileDrop();

  // Hydrate persisted settings (init appStorage first for Electron IPC cache)
  useEffect(() => {
    initAppStorage().then(() => {
      useAgentSettingsStore.getState().hydrate();
      useUIKitStore.getState().hydrate();
      useCanvasStore.getState().hydrate();
      // Sync recent files to Electron native menu on startup
      const recent = getRecentFiles();
      if (recent.length > 0 && window.electronAPI?.syncRecentFiles) {
        const forMenu = recent
          .filter((f) => f.filePath)
          .map((f) => ({ fileName: f.fileName, filePath: f.filePath! }));
        window.electronAPI.syncRecentFiles(forMenu);
      }
      useThemePresetStore.getState().hydrate();
      useDesignMdStore.getState().hydrate();
    });
  }, []);

  return (
    <TooltipProvider delayDuration={300}>
      <div className="h-screen flex flex-col bg-background">
        <UpdateReadyBanner />
        <TopBar />
        <div className="flex-1 flex flex-col overflow-hidden">
          <div className="flex-1 flex overflow-hidden">
            {layerPanelOpen && <LayerPanel />}
            <div className="flex-1 flex flex-col min-w-0 relative">
              <SkiaCanvas />
              <Toolbar />
              <BooleanToolbar />

              {/* Floating variables panel — anchored to the right of the toolbar */}
              {variablesPanelOpen && <VariablesPanel />}

              {/* Floating design system panel */}
              {designMdPanelOpen && <DesignMdPanel />}

              {/* Floating UIKit browser panel */}
              {browserOpen && <ComponentBrowserPanel />}

              {/* Bottom bar: minimized AI (left) + zoom controls (right) */}
              <div className="absolute bottom-2 left-2 right-2 z-10 flex items-center justify-between pointer-events-none">
                <div className="pointer-events-auto">
                  <AIChatMinimizedBar />
                </div>
                <div className="pointer-events-auto">
                  <StatusBar />
                </div>
              </div>

              {/* Expanded AI panel (floating, draggable) */}
              <AIChatPanel />
            </div>
            {hasSelection && <RightPanel />}
          </div>
        </div>
        <ExportDialog open={exportOpen} onClose={closeExport} />
        <SaveDialog open={saveDialogOpen} onClose={closeSaveDialog} />
        <AgentSettingsDialog />
        <FigmaImportDialog open={figmaImportOpen} onClose={closeFigmaImport} />
        <UnsavedChangesDialog
          open={unsavedDialog.open}
          fileName={unsavedDialog.fileName}
          onResult={unsavedDialog.onResult}
        />

        {/* Drop zone overlay */}
        {isDragging && (
          <div className="fixed inset-0 z-50 border-2 border-dashed border-primary bg-primary/5 pointer-events-none" />
        )}
      </div>
    </TooltipProvider>
  );
}
