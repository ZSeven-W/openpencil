// Extension tool definitions — shard 6 of 6 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5). Houses P3 batch-4 v1 tools
// (cookie_banner, data_table_row, date_picker, drawer_shell, empty_state,
// event_card, fab, faq_item, filter_group, form_field) and P3 batch-5 v1
// tools (icon_button, image_placeholder, inbox_message, inline_action,
// input_with_action, invite_row, kbd, legend_item, link, list_row).
// Each shard caps at the repo's 800-line ceiling.
//
// When adding a new tool: pick whichever shard has the fewest tools
// to keep the split balanced. Run `wc -l` on all shard files before
// committing.

import {
  schemaVersionProp,
  filePathProp,
  parentIdProp,
  pageIdProp,
} from './element-tool-def-props';

export const ELEMENT_TOOL_DEFINITIONS_EXT_6 = [
  {
    name: 'add_cookie_banner_v1',
    description:
      'Theme-aware cookie consent banner (v1). theme="light" (default): byte-parity with ' +
      'add_cookie_banner_v0. theme="dark": card bg → slate-800, title → slate-50, body → slate-400, ' +
      'decline bg → slate-700, accept bg → accent. theme="system": $color-* refs. ' +
      'GDPR/CCPA disclosure card with title, body, accept/decline buttons, optional settings link. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Banner headline. Default "We use cookies".' },
        body: { type: 'string', description: 'Disclosure paragraph text.' },
        accept_label: { type: 'string', description: 'Accept button label. Default "Accept all".' },
        decline_label: { type: 'string', description: 'Decline button label. Default "Reject".' },
        show_settings_link: {
          type: 'boolean',
          description: 'When true, render a "Cookie settings" link. Default false.',
        },
        settings_label: {
          type: 'string',
          description: 'Settings link label. Default "Cookie settings".',
        },
        width: { type: 'number', description: 'Banner width in px. Default 720. Min 320.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_data_table_row_v1',
    description:
      'Theme-aware data-table row (v1). theme="light" (default): byte-parity with ' +
      'add_data_table_row_v0. theme="dark": header text → slate-400, body text → slate-50, ' +
      'selected row → slate-900. theme="system": $color-* refs. ' +
      'N evenly-fill_container cells laid out horizontally. Header rows use 12/600 typography. ' +
      'Body rows use 14/400 typography. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        columns: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              content: { type: 'string' },
            },
            required: ['content'],
          },
          description: 'Cell columns. Required (1+).',
        },
        header: {
          type: 'boolean',
          description: 'Render as header row (12/600 slate-500). Default false.',
        },
        selected: {
          type: 'boolean',
          description: 'Mark as selected/hover row (tinted bg). Default false.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['columns'],
    },
  },
  {
    name: 'add_date_picker_v1',
    description:
      'Theme-aware date picker closed state (v1). theme="light" (default): byte-parity with ' +
      'add_date_picker_v0. theme="dark": input bg → slate-800, stroke → slate-600, value → slate-50, ' +
      'placeholder/clear → slate-500, calendar icon → slate-400. theme="system": $color-* refs. ' +
      'Labeled input with value/placeholder and calendar icon. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label. Required.' },
        value: { type: 'string', description: 'Selected date text (e.g. "Jan 15, 2026").' },
        placeholder: {
          type: 'string',
          description: 'Placeholder when empty. Default "Select date".',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to label.',
        },
        clearable: {
          type: 'boolean',
          description: 'Show X clear button when value is set. Default false.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_drawer_shell_v1',
    description:
      'Theme-aware slide-in drawer shell (v1). theme="light" (default): byte-parity with ' +
      'add_drawer_shell_v0. theme="dark": drawer bg → slate-800, header border → slate-600, ' +
      'title → slate-50, close icon → slate-400. theme="system": $color-* refs. ' +
      'Full-height side panel with title + close button. Distinct from modal_shell (centered) and ' +
      'action_menu (compact dropdown). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Drawer title text. Required.' },
        side: {
          type: 'string',
          enum: ['right', 'left'],
          description: 'Side the drawer slides from. Default "right".',
        },
        width: { type: 'number', description: 'Drawer width px. Default 400. Clamped 280..640.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_empty_state_v1',
    description:
      'Theme-aware empty-state block (v1). theme="light" (default): byte-parity with ' +
      'add_empty_state_v0. theme="dark" / "system": identical output (no hardcoded colors in v0). ' +
      'Accepts theme param for API consistency. Vertical centered stack: icon? + title + subtitle? + CTA?. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Empty state title. Required.' },
        subtitle: { type: 'string', description: 'Subtitle text.' },
        icon: { type: 'string', description: 'Lucide icon name (e.g. "inbox").' },
        cta_label: { type: 'string', description: 'Call-to-action button label.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_event_card_v1',
    description:
      'Theme-aware calendar event tile (v1). theme="light" (default): byte-parity with ' +
      'add_event_card_v0. theme="dark": card bg → slate-800, stroke → slate-600, date column → slate-700, ' +
      'day/title → slate-50, meta → slate-400. theme="system": $color-* refs. ' +
      'Left date column (month+day) + right text stack (title+time?+location?). ' +
      'Accent color for month strip header is caller-supplied (brand-invariant). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        month: { type: 'string', description: 'Short month code (e.g. "OCT"). Required.' },
        day: {
          type: ['string', 'number'],
          description: 'Day-of-month (e.g. "15"). Required.',
        },
        title: { type: 'string', description: 'Event title. Required.' },
        time: { type: 'string', description: 'Time string (e.g. "2:00 PM – 3:30 PM").' },
        location: { type: 'string', description: 'Location string.' },
        accent: {
          type: 'string',
          description: 'Accent hex for the month strip bg. Default "#2563EB".',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['month', 'day', 'title'],
    },
  },
  {
    name: 'add_fab_v1',
    description:
      'Theme-aware floating action button (v1). theme="light" (default): byte-parity with ' +
      'add_fab_v0. theme="dark" / "system": FAB bg → accent (brand-invariant). ' +
      'Icon is always white (white on accent — brand decision). ' +
      'Circular button 56×56 (default) with centered lucide icon. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        icon: { type: 'string', description: 'Lucide icon name. Required.' },
        size: { type: 'number', description: 'Button diameter px. Default 56.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['icon'],
    },
  },
  {
    name: 'add_faq_item_v1',
    description:
      'Theme-aware FAQ / accordion item (v1). theme="light" (default): byte-parity with ' +
      'add_faq_item_v0. theme="dark": chevron/answer → slate-400, divider → slate-600, ' +
      'question → slate-50. theme="system": $color-* refs. ' +
      'Collapsed: header only (question + chevron-right). Expanded: header + answer below. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        question: { type: 'string', description: 'Question text. Required.' },
        answer: { type: 'string', description: 'Answer text (only shown when expanded=true).' },
        expanded: {
          type: 'boolean',
          description: 'When true, renders chevron-down and answer. Default false.',
        },
        show_divider: {
          type: 'boolean',
          description: 'Render divider below the row. Default false.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['question'],
    },
  },
  {
    name: 'add_filter_group_v1',
    description:
      'Theme-aware sidebar filter group / facet (v1). theme="light" (default): byte-parity with ' +
      'add_filter_group_v0. theme="dark": title → slate-50, label → slate-300, count → slate-500, ' +
      'unselected box → slate-800 + slate-600, selected box → accent. theme="system": $color-* refs. ' +
      'Heading + vertical checkbox-style option rows with optional counts. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Group heading (e.g. "Category"). Required.' },
        options: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              count: { type: 'number', description: 'Optional count badge value.' },
              selected: { type: 'boolean', description: 'Whether selected. Default false.' },
            },
            required: ['label'],
          },
          description: 'Facet options. Required (1+).',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title', 'options'],
    },
  },
  {
    name: 'add_form_field_v1',
    description:
      'Theme-aware form field (v1). theme="light" (default): byte-parity with add_form_field_v0. ' +
      'theme="dark" / "system": identical output (no hardcoded colors in v0). ' +
      'Accepts theme param for API consistency. Label above 48px input box, ' +
      'optional leading/trailing icons. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label. Required.' },
        placeholder: { type: 'string', description: 'Placeholder text.' },
        leading_icon: { type: 'string', description: 'Lucide icon name for leading icon.' },
        trailing_icon: { type: 'string', description: 'Lucide icon name for trailing icon.' },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to label.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_icon_button_v1',
    description:
      'Theme-aware icon-only button (v1). No hardcoded colors in v0; light/dark/system output ' +
      'identical (byte-parity with v0 in all modes). Accepts theme param for API consistency. ' +
      '44×44 default (Apple HIG / Material min-hit-target) with flex-centered icon. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        icon: { type: 'string', description: 'Lucide icon slug. Required.' },
        size: { type: 'number', description: 'Frame size in px. Default 44.' },
        icon_size: { type: 'number', description: 'Icon size in px. Default 24.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['icon'],
    },
  },
  {
    name: 'add_image_placeholder_v1',
    description:
      'Theme-aware image placeholder (v1). theme="light" (default): byte-parity with ' +
      'add_image_placeholder_v0. theme="dark": bg → bgDeep, icon → textMuted, label → textMuted. ' +
      'theme="system": $color-* refs. Gray box + centered icon + optional caption. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width in px. Default 200. Min 40.' },
        height: { type: 'number', description: 'Height in px. Default 140. Min 40.' },
        label: { type: 'string', description: 'Optional caption below the icon.' },
        icon: { type: 'string', description: 'Lucide icon name. Default "image".' },
        corner_radius: { type: 'number', description: 'Corner radius. Default 8.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_inbox_message_v1',
    description:
      'Theme-aware inbox / email list row (v1). theme="light" (default): byte-parity with ' +
      'add_inbox_message_v0. theme="dark": primary → textPrimary, timestamp → textSubtle, ' +
      'preview → textMuted, unread dot → accent. theme="system": $color-* refs. ' +
      'sender + subject + optional preview + timestamp + optional unread dot. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        from: { type: 'string', description: 'Sender name. Required.' },
        subject: { type: 'string', description: 'Email subject line. Required.' },
        preview: { type: 'string', description: 'Optional body preview text.' },
        timestamp: { type: 'string', description: 'Time / date label (e.g. "10:42 AM").' },
        unread: {
          type: 'boolean',
          description: 'When true, renders bold typography + unread indicator dot.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['from', 'subject'],
    },
  },
  {
    name: 'add_inline_action_v1',
    description:
      'Theme-aware inline status + action row (v1). theme="light" (default): byte-parity with ' +
      'add_inline_action_v0. theme="dark": icon/message → textBody, CTA → accent. ' +
      'theme="system": $color-* refs. Message text left, blue text button right. ' +
      'Use for "Comment deleted • Undo" — NOT floating toast. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Status / context message. Required.' },
        action_label: { type: 'string', description: 'Action label (e.g. "Undo"). Required.' },
        icon: { type: 'string', description: 'Optional leading Lucide icon slug.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message', 'action_label'],
    },
  },
  {
    name: 'add_input_with_action_v1',
    description:
      'Theme-aware input field with inline action button (v1). theme="light" (default): byte-parity ' +
      'with add_input_with_action_v0. theme="dark": input bg → surface, stroke → border, ' +
      'text → textPrimary, placeholder/icon → textMuted; button bg → accent (brand-invariant), ' +
      'button text/icon → white. theme="system": $color-* refs. ' +
      'Two action variants: text (pill button with label) or icon (44×44 square). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        placeholder: { type: 'string', description: 'Placeholder text. Required.' },
        value: { type: 'string', description: 'Pre-filled input value.' },
        action_label: { type: 'string', description: 'Button label for action_kind="text".' },
        action_icon: { type: 'string', description: 'Lucide icon for action_kind="icon".' },
        action_kind: {
          type: 'string',
          enum: ['text', 'icon'],
          description: 'Action button kind. Default "text".',
        },
        leading_icon: { type: 'string', description: 'Leading icon inside the input.' },
        width: { type: 'number', description: 'Total field width in px. Default 400. Min 280.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['placeholder'],
    },
  },
  {
    name: 'add_invite_row_v1',
    description:
      'Theme-aware pending invite list row (v1). theme="light" (default): byte-parity with ' +
      'add_invite_row_v0. theme="dark": avatar bg → surface2, text → textPrimary/textMuted, ' +
      'action → accent; status pills use alertColors (pending→warning, expired→danger, ' +
      'accepted→success). theme="system": $color-* refs. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        email: { type: 'string', description: 'Invitee email. Required.' },
        role: { type: 'string', description: 'Optional invited role (e.g. "Editor").' },
        status: {
          type: 'string',
          enum: ['pending', 'expired', 'accepted'],
          description: 'Invite status. Default "pending".',
        },
        action_label: {
          type: 'string',
          description: 'Trailing action label. Default "Resend".',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['email'],
    },
  },
  {
    name: 'add_kbd_v1',
    description:
      'Theme-aware keyboard shortcut (v1). theme="light" (default): byte-parity with ' +
      'add_kbd_v0. theme="dark": key bg → surface2, stroke → border. theme="system": $color-* refs. ' +
      'Each key becomes a bordered cell; entries joined with separator text. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        keys: {
          type: 'array',
          items: { type: 'string' },
          description: 'Key glyphs (e.g. ["⌘", "K"] or ["Ctrl", "Shift", "P"]). Required.',
        },
        separator: { type: 'string', description: 'Separator text between keys. Default "+".' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['keys'],
    },
  },
  {
    name: 'add_legend_item_v1',
    description:
      'Theme-aware chart legend entry (v1). theme="light" (default): byte-parity with ' +
      'add_legend_item_v0. theme="dark": label → textBody, value → textPrimary. ' +
      'theme="system": $color-* refs. Marker color (caller-supplied) is kept as-is in all modes. ' +
      'colored marker + label + optional right-aligned value. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Legend label text. Required.' },
        color: { type: 'string', description: 'Marker fill hex (e.g. "#2563EB"). Required.' },
        value: { type: 'string', description: 'Optional value shown right of label.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label', 'color'],
    },
  },
  {
    name: 'add_link_v1',
    description:
      'Theme-aware inline text link (v1). No hardcoded colors in v0; light/dark/system output ' +
      'identical (byte-parity with v0 in all modes). Accepts theme param for API consistency. ' +
      'Optional trailing icon ("Learn more →"). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Link text. Required.' },
        trailing_icon: { type: 'string', description: 'Optional trailing Lucide icon slug.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_list_row_v1',
    description:
      'Theme-aware iOS / Material-style list row (v1). No hardcoded colors in v0; light/dark/system ' +
      'output identical (byte-parity with v0 in all modes). Accepts theme param for API consistency. ' +
      '[optional leading icon] + [title/subtitle text stack] + [optional trailing icon]. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Row title text. Required.' },
        subtitle: { type: 'string', description: 'Optional subtitle text.' },
        leading_icon: { type: 'string', description: 'Optional leading Lucide icon slug.' },
        trailing_icon: {
          type: 'string',
          description: 'Optional trailing icon (e.g. "chevron-right").',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
];
