// @vitest-环境 jsdom
// apps/web/src/components/panels/git-panel/__tests__/git-panel-empty-state.test.tsx
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { render, screen, fireEvent, cleanup } from '@testing-library/react';

const initRepoMock = vi.fn(async () => {});
const openRepoMock = vi.fn(async () => {});
const enterCloneWizardMock = vi.fn(() => {});

vi.mock('@/stores/git-store', () => {
  return {
    useGitStore: (
      selector: (s: {
        initRepo: typeof initRepoMock;
        openRepo: typeof openRepoMock;
        enterCloneWizard: typeof enterCloneWizardMock;
      }) => unknown,
    ) =>
      selector({
        initRepo: initRepoMock,
        openRepo: openRepoMock,
        enterCloneWizard: enterCloneWizardMock,
      }),
  };
});

let mockedFilePath: string | null = '/tmp/test.op';
vi.mock('@/stores/document-store', () => {
  // Hook form: useDocumentStore((s) => s.filePath) — selector pattern.
  // Each 调用通过闭包读取当前的 mockedFilePath，因此单独
  // 测试可以在渲染之前改变它。
  const useDocumentStore = (selector: (s: { filePath: string | null }) => unknown) =>
    selector({ filePath: mockedFilePath });
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  (useDocumentStore as any).getState = () => ({ filePath: mockedFilePath });
  return { useDocumentStore };
});

vi.mock('react-i18next', () => ({
  useTranslation: () => ({ t: (k: string) => k }),
}));

import { GitPanelEmptyState } from '@/components/panels/git-panel/git-panel-empty-state';
import { TooltipProvider } from '@/components/ui/tooltip';

const renderWithProvider = (ui: React.ReactElement) =>
  render(<TooltipProvider>{ui}</TooltipProvider>);

describe('GitPanelEmptyState', () => {
  beforeEach(() => {
    mockedFilePath = '/tmp/test.op';
    vi.clearAllMocks();
    // Attach electronAPI 到现有的 jsdom 窗口 — 替换
    // 通过 vi.stubGlobal 的整个窗口对象会破坏 clearTimeout
    // 以及 Radix Tooltip 所依赖的其他浏览器 APIs。
// eslint-disable-next-line @typescript-eslint/no-explicit-any
    (window as any).electronAPI = {
      openDirectory: vi.fn(async () => '/tmp/repo'),
    };
  });

  afterEach(() => {
    cleanup();
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    delete (window as any).electronAPI;
  });

  it('disables the new card when currentFilePath is null', () => {
    mockedFilePath = null;
    renderWithProvider(<GitPanelEmptyState />);
    // The 新卡按钮已禁用 - 按可访问名称（标签键）查找。
    const newButton = screen.getByText('git.empty.newCard').closest('button');
    expect(newButton).toBeTruthy();
    expect(newButton?.hasAttribute('disabled')).toBe(true);
  });

  it('clicking the new card calls initRepo with the current file path', async () => {
    renderWithProvider(<GitPanelEmptyState />);
    const newButton = screen.getByText('git.empty.newCard').closest('button');
    expect(newButton).not.toBeNull();
    fireEvent.click(newButton!);
    // Wait 异步处理程序的勾号。
    await Promise.resolve();
    expect(initRepoMock).toHaveBeenCalledTimes(1);
    expect(initRepoMock).toHaveBeenCalledWith('/tmp/test.op');
  });

  it('clicking the open card calls electronAPI.openDirectory then openRepo', async () => {
    renderWithProvider(<GitPanelEmptyState />);
    const openButton = screen.getByText('git.empty.openCard').closest('button');
    expect(openButton).not.toBeNull();
    fireEvent.click(openButton!);
    // Wait 用于异步链。
    await Promise.resolve();
    await Promise.resolve();
    expect(openRepoMock).toHaveBeenCalledTimes(1);
    // openRepo 获取选择的目录+当前文件路径。
    expect(openRepoMock).toHaveBeenCalledWith('/tmp/repo', '/tmp/test.op');
  });

  it('clicking the clone card opens the clone wizard (Phase 6a)', () => {
    renderWithProvider(<GitPanelEmptyState />);
    const cloneButton = screen.getByText('git.empty.cloneCard').closest('button');
    expect(cloneButton).toBeTruthy();
    expect(cloneButton?.hasAttribute('disabled')).toBe(false);
    fireEvent.click(cloneButton!);
    expect(enterCloneWizardMock).toHaveBeenCalledTimes(1);
  });
});
