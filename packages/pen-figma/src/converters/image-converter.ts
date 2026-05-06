/**
 * Image 转换实用程序
 * 。 Figma 中的 Image 节点表示为带有
 * IMAGE 填充的 RECTANGLE 节点。 The collectImageBlobs 帮助程序（在 common.ts 中）处理 blob
 * 检测。 Actual 图像节点转换由 shape-converter.ts 中的 convertRectangle 处理。 This
 *
 * 模块保留用于将来的图像特定转换逻辑。
 */

export { collectImageBlobs } from './common.js';
