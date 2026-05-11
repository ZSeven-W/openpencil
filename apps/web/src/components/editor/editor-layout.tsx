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
import { useCloudAutosave } from '@/hooks/use-cloud-autosave';
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

  // Phase 4c：在编辑器生命周期内只挂载一次 Git 自动保存订阅器。
  // `initAutosaveSubscriber()` 是幂等的，会先检查现有句柄，
  // 所以 React StrictMode 的双调用不会出问题。
  // 清理阶段会释放订阅器，避免开发期 HMR 或组件卸载时泄漏监听器。
  useEffect(() => {
    useGitStore.getState().initAutosaveSubscriber();
    return () => {
      useGitStore.getState().disposeAutosaveSubscriber();
    };
  }, []);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;

      // Cmd+J：切换 AI 面板最小化
      if (isMod && e.key === 'j') {
        e.preventDefault();
        toggleMinimize();
        return;
      }

      // Cmd+Shift+C：将右侧面板切换到代码选项卡
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'c') {
        e.preventDefault();
        useCanvasStore.getState().setRightPanelTab('code');
        return;
      }

      // Cmd+Shift+V：切换变量面板
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'v') {
        e.preventDefault();
        useCanvasStore.getState().toggleVariablesPanel();
        return;
      }

      // Cmd+Shift+D：切换设计系统面板
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'd') {
        e.preventDefault();
        useCanvasStore.getState().toggleDesignMdPanel();
        return;
      }

      // Cmd+Shift+K：切换 UIKit 浏览器
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'k') {
        e.preventDefault();
        useUIKitStore.getState().toggleBrowser();
        return;
      }

      // Cmd+Shift+F：打开 Figma 导入
      if (isMod && e.shiftKey && e.key.toLowerCase() === 'f') {
        e.preventDefault();
        useCanvasStore.getState().setFigmaImportDialogOpen(true);
        return;
      }

      // Cmd+,：打开代理设置
      if (isMod && e.key === ',') {
        e.preventDefault();
        useAgentSettingsStore.getState().setDialogOpen(true);
        return;
      }
    };
    window.addEventListener('keydown', handler);
    return () => window.removeEventListener('keydown', handler);
  }, [toggleMinimize]);

  // Cmd/Ctrl+Shift+P：打开全局导出对话框。
  //
  // 之前我们用的是 Cmd+Shift+E，但这个组合在部分 macOS 中文输入法
  // 或系统工具里会被静默吞掉，按键事件甚至到不了渲染层，
  // 因而不会触发 JS 处理器，也看不到任何日志。
  //
  // 这里改成 P（Print / PDF / Picture），因为它在应用内没有别的占用，
  // 也不容易被常见输入法拦截。
  //
  // 这个监听器注册在捕获阶段，这样它会尽可能早地触发，
  // 先于其他冒泡阶段监听器介入。
  // 同时用 `e.code === 'KeyP'` 保证与键盘布局无关：
  // 输入法可能改变 `e.key`，但不会改变 `e.code`。
  // 动作使用 `setExportDialogOpen(true)` 这种幂等写法，
  // 这样即使有多条路径同时触发，最终结果仍然是“打开”，而不是彼此抵消。
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

  // 处理 Electron 原生菜单动作
  useElectronMenu();

  // 处理 Figma 剪贴板粘贴
  useFigmaPaste();

  // MCP ↔ 画布实时同步
  useMcpSync();
  useCloudAutosave();

  // 处理拖放打开文件
  const isDragging = useFileDrop();

  // 恢复持久化设置（先初始化 appStorage，供 Electron IPC 缓存使用）
  useEffect(() => {
    initAppStorage().then(() => {
      useAgentSettingsStore.getState().hydrate();
      useUIKitStore.getState().hydrate();
      useCanvasStore.getState().hydrate();
      // 启动时把最近文件同步到 Electron 原生菜单
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

              {/* 浮动变量面板，锚定在工具栏右侧 */}
              {variablesPanelOpen && <VariablesPanel />}

              {/* 浮动设计系统面板 */}
              {designMdPanelOpen && <DesignMdPanel />}

              {/* 浮动 UIKit 浏览器面板 */}
              {browserOpen && <ComponentBrowserPanel />}

              {/* 底部栏：最小化 AI（左）+ 缩放控件（右） */}
              <div className="absolute bottom-2 left-2 right-2 z-10 flex items-center justify-between pointer-events-none">
                <div className="pointer-events-auto">
                  <AIChatMinimizedBar />
                </div>
                <div className="pointer-events-auto">
                  <StatusBar />
                </div>
              </div>

              {/* 展开的 AI 面板（浮动、可拖动） */}
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

        {/* 拖放区域遮罩 */}
        {isDragging && (
          <div className="fixed inset-0 z-50 border-2 border-dashed border-primary bg-primary/5 pointer-events-none" />
        )}
      </div>
    </TooltipProvider>
  );
}
