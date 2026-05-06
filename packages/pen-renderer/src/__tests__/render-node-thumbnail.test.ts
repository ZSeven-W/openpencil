// packages/pen-renderer/src/__tests__/render-node-thumbnail.test.ts
//
// Output-renderNodeThumbnail 的形状和后备测试。 We 进行 NOT 测试
// 像素完美的输出（需要真正的 CanvasKit WASM 实例）。 Instead 我们
// 验证：
//   1. 当 CanvasKit 不可用时，Returns 优雅地为 null（测试环境）
//   2. Returns null 表示无效/空输入
//   3. Returns 当大小无效时为 null
//   4. The 函数已导出并可调用
//   5. resolveNodeForCanvas 使用文档变量 + 活动主题调用 (I6)

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import type { PenDocument, PenNode } from '@zseven-w/pen-types';
import * as penCore from '@zseven-w/pen-core';

// Mock @zseven-w/pen-core 允许监视 resolveNodeForCanvas，同时保持真实模块的所有其他导出完好无损
// 。
vi.mock('@zseven-w/pen-core', async () => {
  const actual = await vi.importActual<typeof penCore>('@zseven-w/pen-core');
  return { ...actual };
});

import { renderNodeThumbnail } from '../render-node-thumbnail';

// CanvasKit WASM 是 vitest/jsdom 环境中可用的 NOT。
// getCanvasKit() 返回 null，因此 renderNodeThumbnail 必须回退为 null
// for ALL inputs. These tests assert the graceful-fallback contract.

const makeRect = (id = 'rect-1'): PenNode =>
  ({
    id,
    type: 'rectangle',
    x: 0,
    y: 0,
    width: 100,
    height: 100,
  }) as PenNode;

const makeDoc = (): PenDocument =>
  ({
    id: 'doc-1',
    name: 'Test Document',
    children: [],
  }) as unknown as PenDocument;

describe('renderNodeThumbnail — output shape / fallback contract', () => {
  it('returns null in test/Node.js environment (no CanvasKit)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: 128,
    });
    // In 测试环境，CanvasKit 不可用 → null
    expect(result).toBeNull();
  });

  it('returns null for a null node', async () => {
    const result = await renderNodeThumbnail(null as unknown as PenNode, {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('returns null for an invalid size (zero)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: 0,
    });
    expect(result).toBeNull();
  });

  it('returns null for an invalid size (negative)', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: -10,
    });
    expect(result).toBeNull();
  });

  it('returns null for a NaN size', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
      size: NaN,
    });
    expect(result).toBeNull();
  });

  it('accepts pageId: null without throwing', async () => {
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    // In test env → null，但不能抛出异常。
    expect(result).toBeNull();
  });

  it('uses default size of 128 when size is omitted', async () => {
    // Just 验证它不会抛出 — 测试环境中的输出为 null。
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('handles a ref-type node gracefully (ref resolution may return empty)', async () => {
    const refNode = {
      id: 'ref-1',
      type: 'ref',
      ref: 'non-existent-component',
      x: 0,
      y: 0,
    } as unknown as PenNode;
    const result = await renderNodeThumbnail(refNode, {
      document: makeDoc(),
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('handles a doc with children without throwing', async () => {
    const doc: PenDocument = {
      id: 'doc-2',
      name: 'Doc With Children',
      children: [makeRect('comp-1')],
    } as unknown as PenDocument;
    const result = await renderNodeThumbnail(makeRect(), {
      document: doc,
      pageId: 'page-1',
    });
    expect(result).toBeNull();
  });

  it('result when non-null must be a data URL string (structural contract)', async () => {
    // This 测试记录了 CanvasKit IS 可用时预期的非空合约。 In 测试环境模拟返回 null，因此我们仅使用类型断言来验证类型约定
    // - 这里不可能进行运行时断言。
    const result = await renderNodeThumbnail(makeRect(), {
      document: makeDoc(),
      pageId: null,
    });
    // In 测试环境始终为空。 In 一个实时环境，它将是字符串 |无效的。
    if (result !== null) {
      expect(typeof result).toBe('string');
      expect(result).toMatch(/^data:/);
    } else {
      expect(result).toBeNull();
    }
  });

  // ---------------------------------------------------------------------------
  // Gap 2：$可变分辨率
// ---------------------------------------------------------------------------

  it('does not throw when node has a $variable fill reference and document has variables', async () => {
    // Node 带有 $color-primary 填充参考。
    const nodeWithVar = {
      id: 'var-node-1',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 100,
      height: 100,
      fill: [{ type: 'solid', color: '$color-primary' }],
    } as unknown as PenNode;

    const docWithVars: PenDocument = {
      id: 'doc-vars',
      name: 'Doc With Variables',
      children: [],
      variables: {
        'color-primary': { value: '#ff0000' },
      },
    } as unknown as PenDocument;

    // In 测试环境 CanvasKit 不可用，因此结果为 null，但 resolveNodeForCanvas
    // 代码路径必须执行而不抛出异常。
    const result = await renderNodeThumbnail(nodeWithVar, {
      document: docWithVars,
      pageId: null,
    });
    expect(result).toBeNull();
  });

  it('does not throw when document has themes and node uses themed variable', async () => {
    const nodeWithVar = {
      id: 'themed-node-1',
      type: 'rectangle',
      x: 0,
      y: 0,
      width: 50,
      height: 50,
      fill: [{ type: 'solid', color: '$bg-color' }],
    } as unknown as PenNode;

    const docWithThemes: PenDocument = {
      id: 'doc-themed',
      name: 'Themed Doc',
      children: [],
      variables: {
        'bg-color': {
          value: [
            { theme: { Mode: 'Light' }, value: '#ffffff' },
            { theme: { Mode: 'Dark' }, value: '#000000' },
          ],
        },
      },
      themes: { Mode: ['Light', 'Dark'] },
    } as unknown as PenDocument;

    const result = await renderNodeThumbnail(nodeWithVar, {
      document: docWithThemes,
      pageId: null,
    });
    // Still 在测试环境中为 null，但不得抛出。
    expect(result).toBeNull();
  });

  // ---------------------------------------------------------------------------
  // I6：用文档变量+主题调用 Verify resolveNodeForCanvas
// ---------------------------------------------------------------------------

  describe('resolveNodeForCanvas invocation (I6)', () => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    let spy: ReturnType<typeof vi.spyOn<any, any>>;

    beforeEach(() => {
      spy = vi.spyOn(penCore, 'resolveNodeForCanvas');
    });

    afterEach(() => {
      spy.mockRestore();
    });

    it('calls resolveNodeForCanvas with the node, document variables, and active theme', async () => {
      const nodeWithVar = {
        id: 'spy-node-1',
        type: 'rectangle',
        x: 0,
        y: 0,
        width: 100,
        height: 100,
        fill: [{ type: 'solid', color: '$color-primary' }],
      } as unknown as PenNode;

      const docWithVars: PenDocument = {
        id: 'doc-spy',
        name: 'Doc With Variables',
        children: [],
        variables: {
          'color-primary': { value: '#ff0000' },
        },
      } as unknown as PenDocument;

      await renderNodeThumbnail(nodeWithVar, {
        document: docWithVars,
        pageId: null,
      });

      // resolveNodeForCanvas 必须已被调用 - 确认变量解析代码路径执行而不是被静默绕过。
      expect(spy).toHaveBeenCalled();
      // The 第一个参数应该是节点（或引用解析的等效项）。
      const [calledNode, calledVars] = spy.mock.calls[0] as [PenNode, Record<string, unknown>];
      expect(calledNode).toMatchObject({ id: 'spy-node-1' });
      // Variables 必须是文档的变量映射。
      expect(calledVars).toEqual(docWithVars.variables);
    });

    it('calls resolveNodeForCanvas with active theme derived from document themes', async () => {
      const node = {
        id: 'spy-node-2',
        type: 'rectangle',
        x: 0,
        y: 0,
        width: 50,
        height: 50,
        fill: [{ type: 'solid', color: '$bg' }],
      } as unknown as PenNode;

      const docWithThemes: PenDocument = {
        id: 'doc-spy-themed',
        name: 'Themed Doc',
        children: [],
        variables: {
          bg: {
            value: [
              { theme: { Mode: 'Light' }, value: '#fff' },
              { theme: { Mode: 'Dark' }, value: '#000' },
            ],
          },
        },
        themes: { Mode: ['Light', 'Dark'] },
      } as unknown as PenDocument;

      await renderNodeThumbnail(node, {
        document: docWithThemes,
        pageId: null,
      });

      expect(spy).toHaveBeenCalled();
      // Third 参数是从 getDefaultTheme 派生的活动主题对象。
      const [, , calledTheme] = spy.mock.calls[0] as [PenNode, unknown, Record<string, string>];
      // getDefaultTheme({'Mode': ['Light','Dark']}) 返回 { Mode: 'Light' }
      expect(calledTheme).toMatchObject({ Mode: 'Light' });
    });
  });
});
