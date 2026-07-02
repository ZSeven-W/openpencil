/**
 * Image conversion utilities.
 * Image nodes in Figma are represented as RECTANGLE nodes with IMAGE fills.
 * The collectImageBlobs helper (in common.ts) handles blob detection.
 * Actual image node conversion is handled by convertRectangle in shape-converter.ts.
 *
 * This module is reserved for future image-specific conversion logic.
 */

export { collectImageBlobs } from './common.js';
