/**
 * HTML Renderer (Stage 2 of visual reference pipeline).
 *
 * Renders generated HTML/CSS to a screenshot using a hidden iframe + html2canvas.
 * Runs entirely client-side — no external browser process needed.
 */

import html2canvas from 'html2canvas';

/**
 * Render an HTML string to a base64 PNG screenshot.
 * Creates a hidden iframe, writes the HTML, and captures with html2canvas.
 *
 * @param html - Complete HTML document string
 * @param width - Viewport width in pixels
 * @param height - Viewport height in pixels (0 = auto based on content)
 * @returns Base64 PNG string (without data: URL prefix)
 */
export async function renderHtmlToScreenshot(
  html: string,
  width: number,
  height: number,
): Promise<string> {
  // Safety check — only runs in browser
  if (typeof document === 'undefined') {
    throw new Error('renderHtmlToScreenshot requires a browser environment');
  }

  const iframe = document.createElement('iframe');

  try {
    // Position off-screen
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

    // Write the HTML into the iframe (same-origin blob)
    iframeDoc.open();
    iframeDoc.write(html);
    iframeDoc.close();

    // Wait for fonts and rendering to settle
    await waitForRender(iframeDoc);

    // Determine actual content height if height was auto
    const captureHeight = height > 0 ? height : Math.min(iframeDoc.body.scrollHeight || 4000, 6000);

    // Resize iframe to actual content height
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
