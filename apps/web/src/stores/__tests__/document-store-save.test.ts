// @vitest-environment jsdom
// apps/web/src/stores/__tests__/document-store-save.test.ts
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { useDocumentStore } from '@/stores/document-store';
import { documentEvents } from '@/utils/document-events';
import * as fileOps from '@/utils/file-operations';

describe('useDocumentStore.save()', () => {
  let savedHandler: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    useDocumentStore.getState().newDocument();
    documentEvents._clear();
    savedHandler = vi.fn();
    documentEvents.on('saved', savedHandler);
    // Clean any window/electronAPI state
    delete (window as unknown as Record<string, unknown>).electronAPI;
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('returns null and does not emit when there is no file target and no fallback works', async () => {
    // No electronAPI, no FSA, no anchor click in jsdom — downloadDocument will
    // still succeed (it just creates a blob URL and clicks a synthetic anchor).
    // To simulate a true failure we mock downloadDocument to throw.
    const dl = vi.spyOn(fileOps, 'downloadDocument').mockImplementation(() => {
      throw new Error('cannot download');
    });
    const result = await useDocumentStore.getState().save();
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
    dl.mockRestore();
  });

  it('emits saved exactly once when the download fallback succeeds', async () => {
    const dl = vi.spyOn(fileOps, 'downloadDocument').mockImplementation(() => {
      // succeed silently
    });
    const result = await useDocumentStore.getState().save();
    expect(result).toBe('untitled.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    expect(savedHandler.mock.calls[0][0]).toMatchObject({
      filePath: null,
      fileName: 'untitled.op',
    });
    dl.mockRestore();
  });

  it('writes via writeToFilePath and emits saved when Electron path is set', async () => {
    const writeSpy = vi.spyOn(fileOps, 'writeToFilePath').mockImplementation(async () => {});
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: vi.fn(),
      saveToPath: vi.fn(),
    };
    useDocumentStore.setState({
      fileName: 'login.op',
      filePath: '/Users/foo/login.op',
      fileHandle: null,
    });
    const result = await useDocumentStore.getState().save();
    expect(writeSpy).toHaveBeenCalledWith('/Users/foo/login.op', expect.any(Object));
    expect(result).toBe('login.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    expect(savedHandler.mock.calls[0][0]).toMatchObject({
      filePath: '/Users/foo/login.op',
      fileName: 'login.op',
    });
    expect(useDocumentStore.getState().isDirty).toBe(false);
    writeSpy.mockRestore();
  });

  it('does NOT emit saved when writeToFilePath throws', async () => {
    const writeSpy = vi.spyOn(fileOps, 'writeToFilePath').mockImplementation(async () => {
      throw new Error('disk full');
    });
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: vi.fn(),
      saveToPath: vi.fn(),
    };
    useDocumentStore.setState({
      fileName: 'login.op',
      filePath: '/Users/foo/login.op',
      fileHandle: null,
    });
    const result = await useDocumentStore.getState().save();
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
    writeSpy.mockRestore();
    consoleError.mockRestore();
  });

  it('falls back to saveAs() when writeToFileHandle throws and clears the stale handle', async () => {
    const handleWriteSpy = vi.spyOn(fileOps, 'writeToFileHandle').mockImplementation(async () => {
      throw new Error('handle revoked');
    });
    const dl = vi.spyOn(fileOps, 'downloadDocument').mockImplementation(() => {});
    const consoleWarn = vi.spyOn(console, 'warn').mockImplementation(() => {});
    const fakeHandle = {} as FileSystemFileHandle;
    useDocumentStore.setState({
      fileName: 'login.op',
      filePath: null,
      fileHandle: fakeHandle,
    });
    const result = await useDocumentStore.getState().save();
    expect(handleWriteSpy).toHaveBeenCalled();
    expect(useDocumentStore.getState().fileHandle).toBeNull();
    // saveAs() succeeded via download fallback
    expect(result).toBe('login.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    handleWriteSpy.mockRestore();
    dl.mockRestore();
    consoleWarn.mockRestore();
  });

  it('emits exactly once via the FSA save-as path', async () => {
    const fakeHandle = {
      name: 'design.op',
    } as unknown as FileSystemFileHandle;
    const fsaSpy = vi.spyOn(fileOps, 'saveDocumentAs').mockResolvedValue({
      handle: fakeHandle,
      fileName: 'design.op',
    });
    // Pretend FSA is available
    (window as unknown as Record<string, unknown>).showSaveFilePicker = () => {};
    const result = await useDocumentStore.getState().saveAs();
    expect(fsaSpy).toHaveBeenCalled();
    expect(result).toBe('design.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    expect(useDocumentStore.getState().fileHandle).toBe(fakeHandle);
    expect(useDocumentStore.getState().fileName).toBe('design.op');
    expect(useDocumentStore.getState().isDirty).toBe(false);
    fsaSpy.mockRestore();
    delete (window as unknown as Record<string, unknown>).showSaveFilePicker;
  });

  it('returns null and does not emit when the FSA picker is cancelled', async () => {
    const fsaSpy = vi.spyOn(fileOps, 'saveDocumentAs').mockResolvedValue(null);
    (window as unknown as Record<string, unknown>).showSaveFilePicker = () => {};
    const result = await useDocumentStore.getState().saveAs();
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
    fsaSpy.mockRestore();
    delete (window as unknown as Record<string, unknown>).showSaveFilePicker;
  });

  it('emits exactly once via the Electron save-as path', async () => {
    const electronSaveFile = vi.fn().mockResolvedValue('/Users/foo/new.op');
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: electronSaveFile,
      saveToPath: vi.fn(),
    };
    const result = await useDocumentStore.getState().saveAs();
    expect(electronSaveFile).toHaveBeenCalled();
    expect(result).toBe('new.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    expect(savedHandler.mock.calls[0][0]).toMatchObject({
      filePath: '/Users/foo/new.op',
      fileName: 'new.op',
    });
    expect(useDocumentStore.getState().filePath).toBe('/Users/foo/new.op');
    expect(useDocumentStore.getState().fileName).toBe('new.op');
  });

  it('returns null and does not emit when the Electron save dialog is cancelled', async () => {
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: vi.fn().mockResolvedValue(null),
      saveToPath: vi.fn(),
    };
    const result = await useDocumentStore.getState().saveAs();
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
  });

  it('saveToNewPath writes to the given path and emits saved (Electron)', async () => {
    const writeSpy = vi.spyOn(fileOps, 'writeToFilePath').mockImplementation(async () => {});
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: vi.fn(),
      saveToPath: vi.fn(),
    };
    const result = await useDocumentStore.getState().saveToNewPath('/tmp/explicit.op');
    expect(writeSpy).toHaveBeenCalledWith('/tmp/explicit.op', expect.any(Object));
    expect(result).toBe('explicit.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
    expect(savedHandler.mock.calls[0][0]).toMatchObject({
      filePath: '/tmp/explicit.op',
      fileName: 'explicit.op',
    });
    expect(useDocumentStore.getState().filePath).toBe('/tmp/explicit.op');
    expect(useDocumentStore.getState().fileName).toBe('explicit.op');
    expect(useDocumentStore.getState().isDirty).toBe(false);
    writeSpy.mockRestore();
  });

  it('saveToNewPath returns null and does not emit in browser builds', async () => {
    const consoleError = vi.spyOn(console, 'error').mockImplementation(() => {});
    // No electronAPI set → isElectron() returns false
    const result = await useDocumentStore.getState().saveToNewPath('/tmp/explicit.op');
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
    consoleError.mockRestore();
  });

  it('saveAs(suggestedName) passes the explicit name through to the underlying picker', async () => {
    const electronSaveFile = vi
      .fn()
      .mockImplementation(async (_json: string, suggested: string) => {
        // Echo the suggested name back as the saved path so the test can assert it
        return `/Users/foo/${suggested}`;
      });
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: electronSaveFile,
      saveToPath: vi.fn(),
    };
    const result = await useDocumentStore.getState().saveAs('manual-typed-name');
    // The store appended .op since the typed name lacked the extension
    expect(electronSaveFile).toHaveBeenCalledWith(expect.any(String), 'manual-typed-name.op');
    expect(result).toBe('manual-typed-name.op');
    expect(savedHandler).toHaveBeenCalledTimes(1);
  });

  it('saveAs(suggestedName) does NOT mutate fileName when the user cancels', async () => {
    useDocumentStore.setState({
      fileName: 'before.op',
      filePath: null,
      fileHandle: null,
    });
    (window as unknown as Record<string, unknown>).electronAPI = {
      isElectron: true,
      saveFile: vi.fn().mockResolvedValue(null), // user cancelled
      saveToPath: vi.fn(),
    };
    const result = await useDocumentStore.getState().saveAs('attempted-new-name');
    expect(result).toBeNull();
    expect(savedHandler).not.toHaveBeenCalled();
    // Critical: fileName must be unchanged
    expect(useDocumentStore.getState().fileName).toBe('before.op');
  });
});
