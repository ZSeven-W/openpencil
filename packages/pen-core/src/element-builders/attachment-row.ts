import type { ElementTree } from './helpers.js';

export interface AttachmentRowParams {
  /** File name (shown bold). */
  filename: string;
  /** Optional size label (e.g. "1.2 MB", "340 KB"). Rendered muted. */
  size?: string;
  /**
   * File-type icon. Default "file". Caller picks from lucide's
   * file-* set (file, file-text, file-image, file-video, file-audio,
   * file-archive, file-spreadsheet, file-code) so the row reads
   * semantically at a glance.
   */
  icon?: string;
  /**
   * When true, appends a small × icon on the right as a remove
   * affordance. Default true (matches every compose / upload UI).
   */
  removable?: boolean;
}

/**
 * File attachment row — compact list unit showing one uploaded file.
 * Pattern from email composers, chat message attachments, and form
 * upload summaries.
 *
 * Structure:
 *   frame(horizontal, width=fill_container, cornerRadius=8, bg=slate-50,
 *         padding=[10,12], gap=10, align=center, role='attachment-row')
 *     ├ icon_font(icon, 24×24, slate-500, role='attachment-icon')
 *     ├ frame(vertical, fill, gap=4, role='attachment-meta')
 *     │   ├ text(filename, 14/600, role='attachment-filename')
 *     │   └ text(size, 12/400 slate-500, role='attachment-size')   ← only if size given
 *     └ icon_font('x', 16×16, muted, role='attachment-remove')     ← only if removable
 *
 * Upload-in-progress state is intentionally NOT baked in — the
 * caller composes `add_progress_bar_v0` (or a batch_design
 * rectangle) directly below the row if needed. Keeping this builder
 * focused on the "done uploaded" 95% case avoids percentage-width
 * plumbing that pen-core doesn't natively support.
 */
export function buildAttachmentRow(params: AttachmentRowParams): ElementTree {
  const removable = params.removable !== false;
  const icon = params.icon ?? 'file';

  const metaChildren: ElementTree[] = [
    {
      type: 'text',
      name: 'Filename',
      role: 'attachment-filename',
      content: params.filename,
      fontSize: 14,
      fontWeight: 600,
      fill: [{ type: 'solid', color: '#0F172A' }],
    },
  ];
  if (params.size) {
    metaChildren.push({
      type: 'text',
      name: 'Size',
      role: 'attachment-size',
      content: params.size,
      fontSize: 12,
      fontWeight: 400,
      fill: [{ type: 'solid', color: '#64748B' }],
    });
  }

  const rowChildren: ElementTree[] = [
    {
      type: 'icon_font',
      name: 'Type Icon',
      role: 'attachment-icon',
      iconFontName: icon,
      iconFontFamily: 'lucide',
      width: 24,
      height: 24,
      fill: [{ type: 'solid', color: '#64748B' }],
    },
    {
      type: 'frame',
      name: 'Meta',
      role: 'attachment-meta',
      width: 'fill_container',
      height: 'fit_content',
      layout: 'vertical',
      gap: 4,
      children: metaChildren,
    },
  ];

  if (removable) {
    rowChildren.push({
      type: 'icon_font',
      name: 'Remove',
      role: 'attachment-remove',
      iconFontName: 'x',
      iconFontFamily: 'lucide',
      width: 16,
      height: 16,
      fill: [{ type: 'solid', color: '#94A3B8' }],
    });
  }

  return {
    type: 'frame',
    name: 'Attachment',
    role: 'attachment-row',
    width: 'fill_container',
    height: 'fit_content',
    cornerRadius: 8,
    layout: 'horizontal',
    alignItems: 'center',
    gap: 10,
    paddingTop: 10,
    paddingBottom: 10,
    paddingLeft: 12,
    paddingRight: 12,
    fill: [{ type: 'solid', color: '#F8FAFC' }],
    children: rowChildren,
  };
}
