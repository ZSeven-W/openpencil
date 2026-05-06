/**
 * Fixed 移动状态栏节点，复制自 Pencil 演示 (pencil-demo.pen)。
 *
 * The 视觉风格源自 iOS（4 条信号、风扇 WiFi、药丸电池）
 * 但用作所有平台的**通用移动模型状态栏**。
 * This is intentional — real-world design tools (Figma, Sketch) do the same;
 * iOS chrome 是移动 UI 模型的事实上的标准，无论
 * 目标平台。
 *
 * Structure：
 * Status 条（框架，w=fill_container，h=62，填充=[21,24,19,24]）
 * ├── Time（帧，fill_container，h=22）→“9:41”文本
 * └── Levels（帧，fill_container，h=22，间隙=7）
 * ├── Cellular Connection（路径 — 4 条信号）
 * ├── Wifi（路径 — 风扇图标）
 * └── Battery（框架，布局=无）
 * ├── Border（矩形，r=4.3，不透明度=0.35，描边）
 * ├── Cap（路径，不透明度=0.4）
 * └── Capacity（矩形，r=2.5，实心填充）
 *
 * Two 变体：深色 (#000) 表示浅色背景，白色 (#fff) 表示深色背景。
 */

import type { PenNode } from '@/types/pen';
import type { PenFill } from '@/types/styles';
import { nanoid } from 'nanoid';

// -- Path 几何形状（两种变体相同） ------------------------------

const CELLULAR_D =
  'M19.2 1.14623c0-0.63304-0.47756-1.14623-1.06667-1.14623l-1.06666 0c-0.5891 0-1.06667 0.51318-1.06667 1.14623l0 9.93396c0 0.63304 0.47756 1.14623 1.06667 1.14622l1.06666 0c0.5891 0 1.06667-0.51318 1.06667-1.14622l0-9.93396z m-7.43411 1.29905l1.06666 0c0.5891 0 1.06667 0.5255 1.06667 1.17374l0 7.43366c0 0.64824-0.47756 1.17374-1.06667 1.17373l-1.06666 0c-0.5891 0-1.06667-0.5255-1.06667-1.17373l0-7.43366c0-0.64824 0.47756-1.17374 1.06667-1.17374z m-4.33178 2.64905l-1.06666 0c-0.5891 0-1.06667 0.53219-1.06667 1.18868l0 4.75472c0 0.65649 0.47756 1.18868 1.06667 1.18867l1.06666 0.00001c0.5891 0 1.06667-0.53219 1.06667-1.18868l0-4.75472c0-0.65649-0.47756-1.18868-1.06667-1.18868z m-5.30078 2.44529l-1.06666 0c-0.5891 0-1.06667 0.52459-1.06667 1.1717l0 2.3434c0 0.64711 0.47756 1.1717 1.06667 1.1717l1.06666 0c0.5891 0 1.06667-0.52459 1.06667-1.1717l0-2.3434c0-0.64711-0.47756-1.1717-1.06667-1.1717z';

const WIFI_D =
  'M8.5713 2.46628c2.48711 0.00011 4.87912 0.92219 6.68163 2.57567 0.13573 0.12765 0.35269 0.12604 0.48637-0.00361l1.29749-1.26347c0.06769-0.06576 0.10543-0.15484 0.10487-0.24752-0.00056-0.09268-0.03938-0.18133-0.10786-0.24631-4.73101-4.37472-12.19473-4.37472-16.92574 0-0.06853 0.06494-0.10742 0.15356-0.10805 0.24624-0.00063 0.09268 0.03704 0.18178 0.10468 0.24759l1.29786 1.26347c0.1336 0.12985 0.35072 0.13146 0.48638 0.00361 1.80274-1.65359 4.19502-2.57567 6.68237-2.57567z m-0.00335 4.22028c1.35732-0.00008 2.6662 0.51165 3.67232 1.43578 0.13608 0.13116 0.35045 0.12831 0.4831-0.00641l1.28728-1.3193c0.06779-0.0692 0.1054-0.16308 0.10443-0.26063-0.00098-0.09755-0.04047-0.19063-0.10963-0.25843-3.06383-2.89085-7.80857-2.89085-10.8724 0-0.06921 0.06779-0.10869 0.16092-0.1096 0.2585-0.00091 0.09758 0.03684 0.19145 0.10477 0.26056l1.28691 1.3193c0.13265 0.13472 0.34702 0.13756 0.4831 0.00641 1.00545-0.92352 2.3133-1.43521 3.66972-1.43578z m2.52442 2.79355c0.00193 0.10535-0.03514 0.20692-0.10244 0.28073l-2.17666 2.45472c-0.06381 0.07214-0.1508 0.11274-0.24157 0.11275-0.09077 0-0.17776-0.0406-0.24157-0.11275l-2.17703-2.45472c-0.06725-0.07386-0.10425-0.17546-0.10225-0.28082 0.00199-0.10535 0.0428-0.20511 0.11279-0.27573 1.3901-1.31389 3.42602-1.31389 4.81612 0 0.06994 0.07067 0.11068 0.17047 0.11261 0.27582z';

const CAP_D =
  'M0 0l0 4c0.80473-0.33878 1.32804-1.12687 1.32804-2 0-0.87313-0.52331-1.66122-1.32804-2';

// -- Helpers ------------------------------------------------------------------

function solidFill(color: string): PenFill[] {
  return [{ type: 'solid', color }];
}

// -- Builder ------------------------------------------------------------------

type StatusBarVariant = 'dark' | 'light';

/**
 * Creates 一个带有新鲜 IDs 的移动状态栏节点树。
 *
 * @param variant - “深色”（黑色图标，用于浅色背景）或“浅色”（白色图标，用于深色背景）
 */
export function createMobileStatusBar(variant: StatusBarVariant = 'dark'): PenNode {
  const fg = variant === 'dark' ? '#000000ff' : '#ffffffff';
  const fgFill = solidFill(fg);
  return {
    id: nanoid(),
    type: 'frame',
    name: 'Status bar',
    role: 'status-bar',
    width: 'fill_container',
    height: 62,
    padding: [21, 24, 19, 24],
    gap: 154,
    justifyContent: 'center',
    alignItems: 'center',
    children: [
      // Time 部分（左）
      {
        id: nanoid(),
        type: 'frame',
        name: 'Time',
        width: 'fill_container',
        height: 22,
        padding: [1.5, 0, 0, 0],
        gap: 10,
        justifyContent: 'center',
        alignItems: 'center',
        children: [
          {
            id: nanoid(),
            type: 'text',
            name: 'Time',
            content: '9:41',
            fill: fgFill,
            fontFamily: 'Inter',
            fontSize: 17,
            fontWeight: 600,
            lineHeight: 1.2941176470588236,
            textAlign: 'center',
          } as PenNode,
        ],
      } as PenNode,
      // Levels 部分（右 — 信号、wifi、电池）
      {
        id: nanoid(),
        type: 'frame',
        name: 'Levels',
        width: 'fill_container',
        height: 22,
        padding: [1, 1, 0, 0],
        gap: 7,
        justifyContent: 'center',
        alignItems: 'center',
        children: [
          // Cellular
          {
            id: nanoid(),
            type: 'path',
            name: 'Cellular Connection',
            d: CELLULAR_D,
            width: 19.2,
            height: 12.226,
            fill: fgFill,
            fillRule: 'evenodd',
          } as PenNode,
          // Wifi
          {
            id: nanoid(),
            type: 'path',
            name: 'Wifi',
            d: WIFI_D,
            width: 17.142,
            height: 12.328,
            fill: fgFill,
            fillRule: 'evenodd',
          } as PenNode,
          // Battery 帧
          {
            id: nanoid(),
            type: 'frame',
            name: 'Battery',
            width: 27.328,
            height: 13,
            layout: 'none',
            children: [
              // Border
              {
                id: nanoid(),
                type: 'rectangle',
                name: 'Border',
                x: 0,
                y: 0,
                width: 25,
                height: 13,
                cornerRadius: 4.3,
                opacity: 0.35,
                stroke: { align: 'inside', fill: fgFill, thickness: 1 },
              } as PenNode,
              // Cap
              {
                id: nanoid(),
                type: 'path',
                name: 'Cap',
                d: CAP_D,
                x: 26,
                y: 4.5,
                width: 1.328,
                height: 4.075,
                fill: fgFill,
                opacity: 0.4,
              } as PenNode,
              // Capacity
              {
                id: nanoid(),
                type: 'rectangle',
                name: 'Capacity',
                x: 2,
                y: 2,
                width: 21,
                height: 9,
                cornerRadius: 2.5,
                fill: fgFill,
              } as PenNode,
            ],
          } as PenNode,
        ],
      } as PenNode,
    ],
  } as PenNode;
}

/**
 * Determines
 * 基于背景颜色的最佳状态栏变体。 Returns 'light'（白色图标）用于深色背景，'dark'（黑色图标）用于浅色背景。
 */
export function inferStatusBarVariant(bgColor?: string | unknown): StatusBarVariant {
  // Defensive：当上游 PenNode 尚未进行变量解析时，调用者偶尔会传递非字符串值（例如填充对象或 `$variable`
  // 引用形状对象）。 Bail 为安全默认值，而不是抛出 `bgColor.replace is not a function`。
  if (typeof bgColor !== 'string' || !bgColor) return 'dark';
  const hex = bgColor.replace(/^#/, '').slice(0, 6);
  if (hex.length !== 6) return 'dark';
  const r = parseInt(hex.slice(0, 2), 16);
  const g = parseInt(hex.slice(2, 4), 16);
  const b = parseInt(hex.slice(4, 6), 16);
  // Relative 亮度（简化）
  const luminance = (0.299 * r + 0.587 * g + 0.114 * b) / 255;
  return luminance < 0.5 ? 'light' : 'dark';
}
