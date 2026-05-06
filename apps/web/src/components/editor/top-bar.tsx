import { useCallback, useEffect, useState } from 'react';
import { appStorage, initAppStorage } from '@/utils/app-storage';
import type { ComponentType, SVGProps } from 'react';
import {
  PanelLeft,
  Folder,
  ChevronDown,
  Sun,
  Moon,
  Maximize,
  Minimize,
  Blocks,
} from 'lucide-react';
import { useTranslation } from 'react-i18next';
import ClaudeLogo from '@/components/icons/claude-logo';
import OpenAILogo from '@/components/icons/openai-logo';
import OpenCodeLogo from '@/components/icons/opencode-logo';
import CopilotLogo from '@/components/icons/copilot-logo';
import GeminiLogo from '@/components/icons/gemini-logo';
import FigmaLogo from '@/components/icons/figma-logo';
import FileMenu from '@/components/shared/file-menu';
import LanguageSelector from '@/components/shared/language-selector';
import { GitButton } from '@/components/editor/git-button';
import { cn } from '@/lib/utils';
import { Button } from '@/components/ui/button';
import { Tooltip, TooltipTrigger, TooltipContent } from '@/components/ui/tooltip';
import { useCanvasStore } from '@/stores/canvas-store';
import { useDocumentStore } from '@/stores/document-store';
import {
  supportsFileSystemAccess,
  isElectron,
  openDocumentFS,
  openDocument,
} from '@/utils/file-operations';
import { syncCanvasPositionsToStore } from '@/canvas/skia-engine-ref';
import { zoomToFitContent } from '@/canvas/skia-engine-ref';
import { parseAndPrepareImportedDocument } from '@/utils/import-pen-document';
import { addRecentFile } from '@/utils/recent-files';
import { useAgentSettingsStore } from '@/stores/agent-settings-store';
import type { AIProviderType } from '@/types/agent-settings';

/** 用离屏 canvas 把 CSS 颜色值（oklch/rgb 等）转换成 `#rrggbb`。 */
function cssToHex(raw: string): string | null {
  const v = raw.trim();
  if (!v) return null;
  try {
    const ctx = document.createElement('canvas').getContext('2d');
    if (!ctx) return null;
    ctx.fillStyle = v;
    const hex = ctx.fillStyle; // 浏览器标准化为#rrggbb
    return hex.startsWith('#') ? hex : null;
  } catch {
    return null;
  }
}

const PROVIDER_ICONS: Record<AIProviderType, ComponentType<SVGProps<SVGSVGElement>>> = {
  anthropic: ClaudeLogo,
  openai: OpenAILogo,
  opencode: OpenCodeLogo,
  copilot: CopilotLogo,
  gemini: GeminiLogo,
};

const PROVIDER_ORDER: AIProviderType[] = ['anthropic', 'openai', 'opencode', 'copilot', 'gemini'];

function AgentStatusButton() {
  const { t } = useTranslation();
  const providers = useAgentSettingsStore((s) => s.providers);
  const mcpIntegrations = useAgentSettingsStore((s) => s.mcpIntegrations);
  const connectedTypes = PROVIDER_ORDER.filter((tp) => providers[tp].isConnected);
  const agentCount = connectedTypes.length;
  const mcpCount = mcpIntegrations.filter((m) => m.enabled).length;
  const hasAny = agentCount > 0 || mcpCount > 0;

  const tooltipParts: string[] = [];
  if (agentCount > 0) tooltipParts.push(`${agentCount} agent${agentCount !== 1 ? 's' : ''}`);
  if (mcpCount > 0) tooltipParts.push(`${mcpCount} MCP`);

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="sm"
          onClick={() => useAgentSettingsStore.getState().setDialogOpen(true)}
          className="h-7 px-2 text-muted-foreground hover:text-foreground"
        >
          {hasAny ? (
            <div className="flex items-center gap-1.5">
              {agentCount > 0 && (
                <div className="flex items-center -space-x-1.5">
                  {connectedTypes.map((type) => {
                    const Icon = PROVIDER_ICONS[type];
                    return (
                      <div
                        key={type}
                        className="w-5 h-5 rounded-md bg-foreground/10 flex items-center justify-center ring-1 ring-card"
                      >
                        <Icon className="w-3 h-3" />
                      </div>
                    );
                  })}
                </div>
              )}
              {agentCount === 0 && <Blocks size={14} strokeWidth={1.5} />}
              <span className="w-1.5 h-1.5 rounded-full bg-emerald-500 shrink-0" />
              <span className="text-[11px] text-muted-foreground hidden sm:inline">
                {tooltipParts.join(' · ')}
              </span>
            </div>
          ) : (
            <div className="flex items-center gap-1.5">
              <Blocks size={14} strokeWidth={1.5} />
              <span className={cn('text-[11px]', 'hidden sm:inline')}>
                {t('topbar.agentsAndMcp')}
              </span>
            </div>
          )}
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">
        {hasAny
          ? tooltipParts.join(' · ') + ' ' + t('topbar.connected')
          : t('topbar.setupAgentsMcp')}
      </TooltipContent>
    </Tooltip>
  );
}

export default function TopBar() {
  const { t } = useTranslation();
  const toggleLayerPanel = useCanvasStore((s) => s.toggleLayerPanel);
  const layerPanelOpen = useCanvasStore((s) => s.layerPanelOpen);
  const fileName = useDocumentStore((s) => s.fileName);
  const isDirty = useDocumentStore((s) => s.isDirty);

  const [theme, setTheme] = useState<'dark' | 'light'>('dark');
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [fileMenuOpen, setFileMenuOpen] = useState(false);
  const [saveIndicator, setSaveIndicator] = useState(false);

  // 读取计算后的 `--card` / `--card-foreground`，
  // 转成十六进制后同步给 Electron 标题栏覆盖层。
  const syncOverlayColors = useCallback((t: 'dark' | 'light') => {
    if (!window.electronAPI?.setTheme) return;
    // 切换 class 后等一帧，确保 CSS 已经生效。
    requestAnimationFrame(() => {
      const s = getComputedStyle(document.documentElement);
      const bg = cssToHex(s.getPropertyValue('--card'));
      const fg = cssToHex(s.getPropertyValue('--card-foreground'));
      window.electronAPI!.setTheme(t, bg && fg ? { bg, fg } : undefined);
    });
  }, []);

  // hydration 之后恢复上次保存的主题。
  // 在 Electron 里要先执行 `initAppStorage()`，
  // 因为 `appStorage.getItem()` 是同步读取，它依赖 IPC 缓存已准备好。
  useEffect(() => {
    const restore = async () => {
      await initAppStorage();
      const saved = appStorage.getItem('openpencil-theme');
      if (saved === 'light') {
        document.documentElement.classList.add('light');
        setTheme('light');
        syncOverlayColors('light');
      } else {
        syncOverlayColors('dark');
      }
    };
    restore();
  }, [syncOverlayColors]);

  // 监听全屏状态变化
  useEffect(() => {
    const handler = () => setIsFullscreen(!!document.fullscreenElement);
    document.addEventListener('fullscreenchange', handler);
    return () => document.removeEventListener('fullscreenchange', handler);
  }, []);

  const toggleTheme = useCallback(() => {
    const next = theme === 'dark' ? 'light' : 'dark';
    if (next === 'light') {
      document.documentElement.classList.add('light');
    } else {
      document.documentElement.classList.remove('light');
    }
    setTheme(next);
    syncOverlayColors(next);
    appStorage.setItem('openpencil-theme', next);
  }, [theme, syncOverlayColors]);

  const toggleFullscreen = useCallback(() => {
    if (document.fullscreenElement) {
      document.exitFullscreen();
    } else {
      document.documentElement.requestFullscreen();
    }
  }, []);

  // 纯保存动作：直接委托给 store。
  // `handleNew` / `handleOpenRecent` 在丢弃当前文档前会调用它，
  // 而“保存成功提示闪烁”的 UI 反馈由下面的 `handleSaveWithFeedback` 负责。
  const handleSave = useCallback(async (): Promise<string | null> => {
    try {
      syncCanvasPositionsToStore();
    } catch (err) {
      console.error('[Save] syncCanvasPositionsToStore failed:', err);
    }
    return useDocumentStore.getState().save();
  }, []);

  const handleSaveWithFeedback = useCallback(async () => {
    const savedName = await handleSave();
    if (!savedName) {
      // 用户取消了保存对话框，或保存本身失败了。
      // 这里不要写入最近文件，也不要触发保存成功提示。
      return;
    }
    const filePath = useDocumentStore.getState().filePath;
    addRecentFile({ fileName: savedName, filePath: filePath ?? null });
    setSaveIndicator(true);
    setTimeout(() => setSaveIndicator(false), 2000);
  }, [handleSave]);

  const handleSaveAs = useCallback(async () => {
    try {
      syncCanvasPositionsToStore();
    } catch (err) {
      console.error('[SaveAs] syncCanvasPositionsToStore failed:', err);
    }
    // 直接调用 `saveAs()`。
    // 这里不会预先改动 `filePath` / `fileHandle`，
    // store 只会在用户确认且写入成功后更新状态。
    const savedName = await useDocumentStore.getState().saveAs();
    if (!savedName) return;
    const filePath = useDocumentStore.getState().filePath;
    addRecentFile({ fileName: savedName, filePath: filePath ?? null });
    setSaveIndicator(true);
    setTimeout(() => setSaveIndicator(false), 2000);
  }, []);

  const handleNew = useCallback(async () => {
    if (useDocumentStore.getState().isDirty) {
      const showDialog = (window as any).__showUnsavedDialog;
      if (showDialog) {
        const result = await showDialog(
          useDocumentStore.getState().fileName || t('common.untitled'),
        );
        if (result === 'cancel') return;
        if (result === 'save') await handleSaveWithFeedback();
      }
    }
    useDocumentStore.getState().newDocument();
    requestAnimationFrame(() => zoomToFitContent());
  }, [t, handleSaveWithFeedback]);

  const handleOpenRecent = useCallback(
    async (filePath: string) => {
      if (!isElectron()) return;
      if (useDocumentStore.getState().isDirty) {
        const showDialog = (window as any).__showUnsavedDialog;
        if (showDialog) {
          const result = await showDialog(
            useDocumentStore.getState().fileName || t('common.untitled'),
          );
          if (result === 'cancel') return;
          if (result === 'save') await handleSaveWithFeedback();
        }
      }
      window.electronAPI!.readFile(filePath).then((result) => {
        if (!result) return;
        try {
          const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op';
          const prepared = parseAndPrepareImportedDocument(result.content, {
            fileName: name,
            filePath: result.filePath,
          });
          if (!prepared) return;
          const { doc } = prepared;
          useDocumentStore.getState().loadDocument(doc, name, null, result.filePath);
          requestAnimationFrame(() => zoomToFitContent());
        } catch {
          /* 非法文件 */
        }
      });
    },
    [t, handleSaveWithFeedback],
  );

  const handleOpen = useCallback(async () => {
    if (useDocumentStore.getState().isDirty) {
      const showDialog = (window as any).__showUnsavedDialog;
      if (showDialog) {
        const result = await showDialog(
          useDocumentStore.getState().fileName || t('common.untitled'),
        );
        if (result === 'cancel') return;
        if (result === 'save') await handleSaveWithFeedback();
      }
    }
    if (isElectron()) {
      window.electronAPI!.openFile().then((result) => {
        if (!result) return;
        try {
          const name = result.filePath.split(/[/\\]/).pop() || 'untitled.op';
          const prepared = parseAndPrepareImportedDocument(result.content, {
            fileName: name,
            filePath: result.filePath,
          });
          if (!prepared) return;
          const { doc } = prepared;
          useDocumentStore.getState().loadDocument(doc, name, null, result.filePath);
          requestAnimationFrame(() => zoomToFitContent());
        } catch {
          /* 非法文件 */
        }
      });
    } else if (supportsFileSystemAccess()) {
      openDocumentFS().then((result) => {
        if (result) {
          useDocumentStore.getState().loadDocument(result.doc, result.fileName, result.handle);
          requestAnimationFrame(() => zoomToFitContent());
        }
      });
    } else {
      openDocument().then((result) => {
        if (result) {
          useDocumentStore.getState().loadDocument(result.doc, result.fileName);
          requestAnimationFrame(() => zoomToFitContent());
        }
      });
    }
  }, [t, handleSaveWithFeedback]);

  const displayName = fileName ?? t('common.untitled');

  return (
    <div className="h-10 bg-card border-b border-border flex items-center px-2 shrink-0 select-none app-region-drag">
      {/* 左侧区域 */}
      <div className="flex items-center gap-0.5 app-region-no-drag electron-traffic-light-pad">
        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              onClick={toggleLayerPanel}
              className={layerPanelOpen ? 'text-foreground' : 'text-muted-foreground'}
            >
              <PanelLeft size={15} strokeWidth={1.5} />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {layerPanelOpen ? t('topbar.hideLayers') : t('topbar.showLayers')}
          </TooltipContent>
        </Tooltip>

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        {/* 文件菜单下拉触发器 */}
        <div className="relative">
          <Tooltip>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="sm"
                className="text-muted-foreground h-7 px-1.5 gap-0.5"
                onClick={() => setFileMenuOpen((v) => !v)}
              >
                <Folder size={15} strokeWidth={1.5} />
                <ChevronDown size={10} className="text-muted-foreground/60" />
              </Button>
            </TooltipTrigger>
            <TooltipContent side="bottom">{t('topbar.open')}</TooltipContent>
          </Tooltip>
          <FileMenu
            open={fileMenuOpen}
            onClose={() => setFileMenuOpen(false)}
            onNew={handleNew}
            onOpen={handleOpen}
            onSave={handleSaveWithFeedback}
            onSaveAs={handleSaveAs}
            onExport={() => useCanvasStore.getState().setExportDialogOpen(true)}
            onOpenRecent={handleOpenRecent}
          />
        </div>

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-muted-foreground"
              onClick={() => useCanvasStore.getState().setFigmaImportDialogOpen(true)}
            >
              <FigmaLogo className="w-3.5 h-3.5" />
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">{t('topbar.importFigma')}</TooltipContent>
        </Tooltip>
      </div>

      {/* 中间区域：文件名 + Git 状态指示器 */}
      <div className="flex min-w-0 flex-1 items-center justify-center gap-1.5">
        <span className="truncate text-xs leading-none text-foreground" suppressHydrationWarning>
          {displayName}
        </span>
        {isDirty && (
          <span className="text-xs leading-none text-muted-foreground">{t('topbar.edited')}</span>
        )}
        {saveIndicator && (
          <span className="text-[10px] leading-none text-emerald-500 animate-pulse">
            {t('fileMenu.saved')}
          </span>
        )}
        <div className="app-region-no-drag flex items-center">
          <GitButton />
        </div>
      </div>

      {/* 右侧区域 */}
      <div className="flex items-center gap-0.5 app-region-no-drag electron-win-controls-pad">
        <AgentStatusButton />

        <div className="w-px h-3.5 bg-border/60 mx-1" />

        <LanguageSelector />

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-muted-foreground"
              onClick={toggleTheme}
            >
              {theme === 'dark' ? (
                <Sun size={15} strokeWidth={1.5} />
              ) : (
                <Moon size={15} strokeWidth={1.5} />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {theme === 'dark' ? t('topbar.lightMode') : t('topbar.darkMode')}
          </TooltipContent>
        </Tooltip>

        <Tooltip>
          <TooltipTrigger asChild>
            <Button
              variant="ghost"
              size="icon-sm"
              className="text-muted-foreground"
              onClick={toggleFullscreen}
            >
              {isFullscreen ? (
                <Minimize size={15} strokeWidth={1.5} />
              ) : (
                <Maximize size={15} strokeWidth={1.5} />
              )}
            </Button>
          </TooltipTrigger>
          <TooltipContent side="bottom">
            {isFullscreen ? t('topbar.exitFullscreen') : t('topbar.fullscreen')}
          </TooltipContent>
        </Tooltip>
      </div>
    </div>
  );
}
