import { describe, expect, it } from 'vitest';
import { enrichNodeForAIConsumerView } from '../consumer-view-enrichment';

describe('consumer-view-enrichment', () => {
  it('adds image fill explain and keeps existing original size as the source of truth', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-1',
      type: 'rectangle',
      name: '背景11',
      x: 0.05,
      y: -0.39,
      width: 2560,
      height: 1600,
      fill: [
        {
          type: 'image',
          url: './assets/11-33.png',
          mode: 'stretch',
          originalSize: {
            width: 2644,
            height: 1696,
          },
          transform: {
            m00: 0.9682299494743347,
            m01: 0,
            m02: 0.019307976588606834,
            m10: 0,
            m11: 0.9433962106704712,
            m12: 0.041042111814022064,
          },
        },
      ],
    } as any);

    expect((enriched as any).fill[0]).toEqual({
      type: 'image',
      url: './assets/11-33.png',
      mode: 'stretch',
      originalSize: {
        width: 2644,
        height: 1696,
      },
      transform: {
        m00: 0.9682299494743347,
        m01: 0,
        m02: 0.019307976588606834,
        m10: 0,
        m11: 0.9433962106704712,
        m12: 0.041042111814022064,
      },
      explain: '这不是整图 stretch, 而是先裁剪原图后再映射到目标框',
    });
  });

  it('can still infer original size from axis-aligned transform when upstream size is missing', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-2',
      type: 'rectangle',
      name: 'Poster',
      width: 2560,
      height: 1600,
      fill: [
        {
          type: 'image',
          url: './assets/poster.png',
          mode: 'stretch',
          transform: {
            m00: 0.9682299494743347,
            m01: 0,
            m02: 0.019307976588606834,
            m10: 0,
            m11: 0.9433962106704712,
            m12: 0.041042111814022064,
          },
        },
      ],
    } as any);

    expect((enriched as any).fill[0].originalSize).toEqual({
      width: 2644,
      height: 1696,
    });
    expect((enriched as any).fill[0].explain).toBe(
      '这不是整图 stretch, 而是先裁剪原图后再映射到目标框',
    );
  });

  it('adds explain for gradients, auto-layout, clipContent, and image node objectFit', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'frame-1',
      type: 'frame',
      name: 'Hero',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'horizontal',
      gap: 24,
      padding: [32, 24],
      justifyContent: 'space_between',
      alignItems: 'center',
      clipContent: true,
      fill: [
        {
          type: 'linear_gradient',
          angle: 135,
          stops: [
            { offset: 0, color: '#111111' },
            { offset: 1, color: '#999999' },
          ],
        },
      ],
      children: [
        {
          id: 'image-1',
          type: 'image',
          name: 'Hero Image',
          src: './assets/hero.png',
          objectFit: 'crop',
          width: 320,
          height: 180,
        },
      ],
    } as any);

    expect((enriched as any).fill[0].explain).toBe(
      '这是线性渐变填充, 角度 135deg, 共 2 个色标, 表示颜色会沿该方向平滑过渡',
    );
    expect((enriched as any).explain).toBe(
      '这是一个横向 auto-layout 容器, 子元素间距为 24, 容器内边距为 32 24, 主轴对齐方式为 两端分布, 交叉轴对齐方式为 居中对齐。宽度会跟随父容器可用空间拉伸, 高度会由内容自动撑开。该容器会裁剪超出自身边界的子元素',
    );
    expect((enriched as any).children[0].explain).toBe(
      '这是图像节点, objectFit=crop 表示按 cover 铺满容器, 可能裁掉边缘。宽度固定为 320px, 高度固定为 180px',
    );
  });

  it('describes sizingBehavior hints such as fill_container(300) and fit_content(120)', () => {
    const enriched = enrichNodeForAIConsumerView({
      id: 'node-3',
      type: 'frame',
      width: 'fill_container(300)',
      height: 'fit_content(120)',
    } as any);

    expect((enriched as any).explain).toBe(
      '宽度会跟随父容器可用空间拉伸, 提示值约 300px, 高度会由内容自动撑开, 提示值约 120px',
    );
  });

  it('adds explain for effects and reusable/component-instance semantics', () => {
    const reusable = enrichNodeForAIConsumerView({
      id: 'component-1',
      type: 'frame',
      name: 'Card Component',
      reusable: true,
      slot: ['media', 'actions'],
      effects: [
        {
          type: 'shadow',
          offsetX: 0,
          offsetY: 4,
          blur: 12,
          spread: -2,
          color: 'rgba(0,0,0,0.12)',
        },
      ],
    } as any);

    expect((reusable as any).explain).toBe(
      '带有投影, 偏移 0px 4px, 模糊 12px, 扩散 -2px。这是一个可复用组件定义节点, 其他实例可以引用它。它声明了可插槽区域: media, actions',
    );

    const instance = enrichNodeForAIConsumerView({
      id: 'instance-1',
      type: 'ref',
      ref: 'component-1',
      descendants: {
        'child-1': { visible: false },
        'child-2': { opacity: 0.5 },
      },
    } as any);

    expect((instance as any).explain).toBe(
      '这是一个组件实例节点, 引用源节点 component-1。当前实例对 2 个后代节点带有覆写',
    );
  });

  it('adds explain for textGrowth, lineHeight, and text alignment semantics', () => {
    const textNode = enrichNodeForAIConsumerView({
      id: 'text-hero',
      type: 'text',
      content: 'Hello world',
      width: 'fill_container',
      textGrowth: 'fixed-width',
      lineHeight: 1.5,
      textAlign: 'center',
      textAlignVertical: 'middle',
    } as any);

    expect((textNode as any).explain).toBe(
      '这是文本节点, textGrowth=fixed-width 表示文本会按当前宽度换行, 高度随内容自动增长。行高倍率为 1.5。水平对齐方式为 居中。垂直对齐方式为 垂直居中。宽度会跟随父容器可用空间拉伸',
    );
  });

  it('adds explain for variable refs and theme overrides', () => {
    const themed = enrichNodeForAIConsumerView({
      id: 'node-theme',
      type: 'frame',
      opacity: '$opacity-soft',
      theme: {
        ColorScheme: 'Dark',
        Density: 'Compact',
      },
      fill: [{ type: 'solid', color: '$surface-bg' }],
      stroke: {
        thickness: '$border-width',
        fill: [{ type: 'solid', color: '$border-color' }],
      },
      effects: [
        {
          type: 'shadow',
          offsetX: '$shadow-x',
          offsetY: '$shadow-y',
          blur: '$shadow-blur',
          spread: '$shadow-spread',
          color: '$shadow-color',
        },
      ],
    } as any);

    expect((themed as any).explain).toBe(
      '带有投影效果。opacity 使用设计变量 $opacity-soft。填充颜色使用设计变量 $surface-bg。描边粗细使用设计变量 $border-width。描边颜色使用设计变量 $border-color。阴影颜色使用设计变量 $shadow-color。阴影模糊半径使用设计变量 $shadow-blur。阴影水平偏移使用设计变量 $shadow-x。阴影垂直偏移使用设计变量 $shadow-y。阴影扩散使用设计变量 $shadow-spread。这些值来自设计系统变量, 不是普通硬编码常量。该节点带有主题覆写上下文: ColorScheme=Dark, Density=Compact',
    );
  });
});
