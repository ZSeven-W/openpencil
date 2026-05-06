/**
 * HTML Rendere
 *
 * r（可视化参考管道的 Stage 2）。 Renders 使用隐藏的 iframe +
 * html2canvas
 生成 HTML/CSS 的屏幕截图。 Runs 完全是客户端 — 不需要外部浏览器进程。
 */

import html2canvas from 'html2canvas';

/**
 * Render 将 HTML 字符串转换为 Base64 PNG 屏幕截图。
 * Creates 一个隐藏的 iframe，写入 HTML，并使用 html2canvas 捕获。
 *
 * @param html - Complete HTML 文档字符串
 * @param width - Viewport 宽度（以像素为单位）
 * @param height - Viewport 高度（以像素为单位）（0 = 基于内容自动）
 * @returns Base64 PNG 字符串（无数据：URL 前缀）
 */
export async function renderHtmlToScreenshot(
  html: string,
  width: number,
  height: number,
): Promise<string> {
  // Safety 检查 — 仅在浏览器中运行
  if (typeof document === 'undefined') {
    throw new Error('renderHtmlToScreenshot requires a browser environment');
  }

  const iframe = document.createElement('iframe');

  try {
    // Position 离屏
    iframe.style.cssText = `
      position: fixed;
      left: -9999px;
      top: 0;
      width: ${width}px;
      height: ${height > 0 ? `${height}px` : '4000px'};
      border: none;
      opacity: 0;
      pointer-events: none;
    `;
    document.body.appendChild(iframe);

    const iframeDoc = iframe.contentDocument;
    if (!iframeDoc) {
      throw new Error('Could not access iframe document');
    }

    // Write 将 HTML 放入 iframe（同源 blob）
    iframeDoc.open();
    iframeDoc.write(html);
    iframeDoc.close();

    // Wait 用于字体和渲染解决
    await waitForRender(iframeDoc);

    // Determine 如果高度为自动，则实际内容高度
    const captureHeight = height > 0 ? height : Math.min(iframeDoc.body.scrollHeight || 4000, 6000);

    // Resize iframe 到实际内容高度
    if (height <= 0) {
      iframe.style.height = `${captureHeight}px`;
      // Wait one more frame for resize to apply
      await new Promise<void>((resolve) => {
        requestAnimationFrame(() => {
          requestAnimationFrame(() => resolve());
        });
      });
    }

    // Capture with html2canvas
    const canvas = await html2canvas(iframeDoc.body, {
      width,
      height: captureHeight,
      windowWidth: width,
      windowHeight: captureHeight,
      useCORS: true,
      allowTaint: true,
      scale: 1, // 1x is sufficient for reference (saves memory/bandwidth)
      logging: false,
      backgroundColor: null, // Preserve transparency
    });

    // Convert to base64 PNG (strip the data:image/png;base64, prefix)
    const dataUrl = canvas.toDataURL('image/png');
    const base64 = dataUrl.replace(/^data:image\/png;base64,/, '');

    return base64;
  } finally {
    // Cleanup
    if (iframe.parentNode) {
      document.body.removeChild(iframe);
    }
  }
}

/**
 * Wait for the iframe document to finish rendering.
 * Waits for fonts, images, and layout to stabilize.
 */
async function waitForRender(doc: Document): Promise<void> {
  // Wait for fonts to load (if the document's fonts API is available)
  try {
    if (doc.fonts && typeof doc.fonts.ready === 'object') {
      await Promise.race([
        doc.fonts.ready,
        new Promise<void>((r) => setTimeout(r, 3000)), // Max 3s for fonts
      ]);
    }
  } catch {
    // Fonts API not available in iframe — continue anyway
  }

  // Wait for general rendering to stabilize (2 animation frames + small delay)
  await new Promise<void>((resolve) => {
    setTimeout(() => {
      requestAnimationFrame(() => {
        requestAnimationFrame(() => {
          resolve();
        });
      });
    }, 300); // 300ms for CSS transitions and layout
  });
}
