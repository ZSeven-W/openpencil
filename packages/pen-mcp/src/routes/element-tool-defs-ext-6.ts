// Extension tool definitions — shard 6 of 6 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5). Houses P3 batch-4 v1 tools
// (cookie_banner, data_table_row, date_picker, drawer_shell, empty_state,
// event_card, fab, faq_item, filter_group, form_field). Each shard caps
// at the repo's 800-line ceiling.
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
];
