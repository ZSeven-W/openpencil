import type { PenDocument } from './pen.js';

export type ComponentCategory =
  | 'buttons'
  | 'inputs'
  | 'cards'
  | 'navigation'
  | 'layout'
  | 'feedback'
  | 'data-display'
  | 'other';

export interface KitComponent {
  /** 套件文档中可重复使用的 FrameNode 的 Node ID */
  id: string;
  /** Display 名称 */
  name: string;
  /** Category 用于在浏览器中进行组织 */
  category: ComponentCategory;
  /** Tags 用于搜索 */
  tags: string[];
  /** Component 预览尺寸 */
  width: number;
  height: number;
}

export interface UIKit {
  /** Unique 标识符 */
  id: string;
  /** Display 名称 */
  name: string;
  /** Optional 描述 */
  description?: string;
  /** Version 字符串 */
  version: string;
  /** Whether 这是应用程序附带的内置套件 */
  builtIn: boolean;
  /** Backing PenDocument 包含可重用节点 */
  document: PenDocument;
  /** Extracted 用于浏览的组件元数据 */
  components: KitComponent[];
}
