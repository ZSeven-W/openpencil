// Extension tool definitions — shard 3 of 3 (shards 1 and 2 are
// `element-tool-defs-ext.ts` and `element-tool-defs-ext-2.ts`).
// All three shards are concatenated in `element-tool-defs.ts` to
// form the final registry. The three-way split keeps each file
// under the repo's 800-line ceiling while the element-tool family
// continues to grow past 67.
//
// When adding a new tool: pick whichever shard has the fewest tools
// to keep the split balanced. Run `wc -l` on all three files before
// committing.

import {
  schemaVersionProp,
  filePathProp,
  parentIdProp,
  pageIdProp,
} from './element-tool-def-props';

export const ELEMENT_TOOL_DEFINITIONS_EXT_3 = [
  {
    name: 'add_modal_shell_v1',
    description:
      'Theme-aware variant of add_modal_shell_v0 — same structure (scrim + centered card + title + ' +
      'optional subtitle) with an additional `theme` param controlling fill colors. ' +
      'theme="light" (default): byte-parity with v0 (white card, slate-500 muted subtitle, ' +
      'unstyled title). theme="dark": slate-800 card, slate-200 title, slate-400 muted subtitle — ' +
      'hardcoded dark palette, no ref resolution needed. theme="system": emits $color-surface / ' +
      '$color-text-primary / $color-text-muted refs; render color tracks themes.Mode at paint time. ' +
      'REQUIRES `applySemanticPalette(doc)` to have seeded the 14-token semantic palette when ' +
      'theme="system" — otherwise refs resolve to undefined. Scrim stays black in both themes ' +
      '(modal backdrops are always dark-dimming regardless of surface theme). ' +
      'Use when the spec says "dark-mode modal", "theme-aware dialog", "supports light/dark ' +
      'toggle"; stick with add_modal_shell_v0 for single-theme light designs (v0 is simpler to ' +
      'apply styling overrides on top of). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Modal title shown as heading' },
        subtitle: { type: 'string', description: 'Optional subtitle / description' },
        card_width: { type: 'number', description: 'Card width in px (default 400, min 280)' },
        card_padding: {
          type: 'number',
          description: 'Inner padding, all sides (default 24, min 12)',
        },
        scrim_opacity: {
          type: 'number',
          description: 'Backdrop scrim opacity 0..1 (default 0.5; 0 = no backdrop)',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description:
            'Theme variant. "light" (default) = v0 parity. "dark" = hardcoded dark hex. "system" = emits $color-* refs, requires applySemanticPalette(doc) seeded.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_date_picker_v0',
    description:
      'Date picker closed state — labeled input showing the selected date (or placeholder) plus a ' +
      'trailing calendar icon. Emits ONLY the closed trigger shape; for the open month-view panel ' +
      'use add_calendar_grid_v0 (typically shown inside a popover, not stacked directly below). ' +
      'Set `value` to render the populated state (slate-900 text); omit for placeholder state ' +
      '(slate-400 text). Set `clearable: true` to render a small X affordance when value is present ' +
      '(clear does nothing in static design; it signals the interaction shape). Use for "date picker", ' +
      '"date input", "date field", "picker closed", "日期选择器", "日期输入". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label shown above the input' },
        value: {
          type: 'string',
          description: 'Selected date text (e.g. "Jan 15, 2026"). Omit for placeholder state.',
        },
        placeholder: {
          type: 'string',
          description: 'Placeholder shown when value is empty (default "Select date")',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label',
        },
        clearable: {
          type: 'boolean',
          description: 'When true + value present, renders an X clear affordance (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_action_menu_v0',
    description:
      'Action / context menu panel — the floating card that drops from a "⋯ more" button or appears on ' +
      'right-click. Emits the OPEN state: vertical stack of padded rows (optional leading icon + ' +
      'label), white card with subtle stroke + shadow. Mark items with destructive=true to render ' +
      'red (e.g. "Delete"); use divider_before=true on an item to separate groups (like the "Share / ' +
      'Report / Delete" grouping). Positioning and show/hide animation are caller concerns. Use for ' +
      '"context menu", "dropdown menu", "more menu", "kebab menu", "action sheet", "下拉菜单", "操作菜单". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description:
            'Menu items. Each needs `label`; `icon` + `destructive` + `divider_before` optional.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              icon: { type: 'string', description: 'Optional lucide icon name' },
              destructive: {
                type: 'boolean',
                description: 'When true, label and icon render red (for Delete / Remove)',
              },
              divider_before: {
                type: 'boolean',
                description: 'When true, draws a 1px divider ABOVE this item (group boundary)',
              },
            },
            required: ['label'],
          },
        },
        width: { type: 'number', description: 'Panel width in px (default 200, min 140)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_empty_chart_v0',
    description:
      'Empty-state placeholder for a chart widget — a dashed-border tile in the same footprint as the ' +
      'real chart, with icon + "No data yet" title + hint subtitle. Use instead of an empty ' +
      'add_chart_line_v0 / add_chart_bars_v0 / add_chart_pie_v0 when the data array would be empty. ' +
      'Size defaults (320×200) match the line/bar chart default footprint; pass `icon` = ' +
      '"line-chart"/"pie-chart"/"bar-chart-2" to hint at the widget type it replaces. Use for ' +
      '"no data chart", "empty analytics tile", "chart placeholder", "空图表", "暂无数据". ' +
      'For non-chart empty states (inbox / onboarding / no results), use add_empty_state_v0. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width in px (default 320, min 120)' },
        height: { type: 'number', description: 'Height in px (default 200, min 100)' },
        title: { type: 'string', description: 'Headline above subtitle (default "No data yet")' },
        subtitle: {
          type: 'string',
          description:
            'Secondary hint line below title (default "Data will appear here once tracking begins.")',
        },
        icon: {
          type: 'string',
          description:
            'Lucide icon (default "bar-chart-2"; try "line-chart"/"pie-chart" to match widget)',
        },
        corner_radius: { type: 'number', description: 'Corner radius (default 12)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_chip_input_v0',
    description:
      'Multi-tag input field — labeled control that holds a variable number of pill-shaped chips ' +
      'plus an inline text cursor for adding the next tag. Wrap layout so chips flow onto multiple ' +
      'rows as they accumulate. Each chip has a small × icon for removal. Use for "tag input", ' +
      '"recipient list", "multi-select field", "email chips", "keyword tags", "标签输入", "多选标签". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label shown above the input' },
        chips: {
          type: 'array',
          description: 'Current chip values; each renders as a removable pill. Pass [] for empty.',
          items: { type: 'string' },
        },
        placeholder: {
          type: 'string',
          description: 'Caret placeholder text. Defaults to "Add tag…" when chips is empty.',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_faq_item_v0',
    description:
      'FAQ / accordion item — ONE question row in a frequently-asked list. Collapsed (expanded=false, ' +
      'default): shows header only (bold question + chevron-right). Expanded (expanded=true): shows ' +
      'header (chevron-down) + answer paragraph below. Caller stacks multiple items in a vertical parent ' +
      'to build the full list; set show_divider=true for 1px hairline below each row. Use for "FAQ", ' +
      '"accordion", "collapsible item", "expandable section", "Q&A", "常见问题", "折叠面板". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        question: { type: 'string', description: 'Question text (bold, shown in header)' },
        answer: {
          type: 'string',
          description: 'Answer paragraph (only rendered when expanded=true)',
        },
        expanded: {
          type: 'boolean',
          description: 'When true, renders expanded with answer + chevron-down (default false)',
        },
        show_divider: {
          type: 'boolean',
          description:
            'When true, appends a 1px slate-200 divider rect inside the row (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['question'],
    },
  },
  {
    name: 'add_pagination_v0',
    description:
      'Pagination bar for list/table footers: row of page-number pills flanked by optional prev/next ' +
      'arrow buttons. Active page renders as a filled pill (accent color, white text); inactive ' +
      'pages are ghost (no fill). Collapses long page ranges with "…" ellipses Google-style ' +
      '(always shows 1 and total, plus a ±siblings window around current). Use for "pagination", ' +
      '"page nav", "分页", "分页条". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total number of pages (>= 1)' },
        current: {
          type: 'number',
          description: '1-based current page (clamped to [1, total], default 1)',
        },
        siblings: {
          type: 'number',
          description: 'Pages shown on each side of current before ellipsis (default 1, min 0)',
        },
        show_arrows: {
          type: 'boolean',
          description: 'Include prev/next chevron buttons (default true)',
        },
        accent_color: {
          type: 'string',
          description: 'Hex color for active page pill (default #0F172A slate-900)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['total'],
    },
  },
  {
    name: 'add_otp_input_v0',
    description:
      'OTP / PIN code input — horizontal row of N square slots, one digit per slot. Common in 2FA ' +
      'verification, PIN unlock, email/phone confirmation flows. Renders the blank awaiting-input ' +
      'state by default, or a partial/full state if `digits` is supplied. The `focused_index` ' +
      'slot shows an accent-color 2px outline (the "currently typing here" visual). Use for ' +
      '"OTP input", "PIN code", "verification code", "6-digit code", "2FA code", "验证码", "PIN 码". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        length: {
          type: 'number',
          description: 'Number of code slots (clamped 4..8, default 6)',
        },
        digits: {
          type: 'array',
          description:
            'Optional digits — digits[i] fills slot i. Omit (or pass shorter array) for the blank / partial state.',
          items: { type: 'string' },
        },
        focused_index: {
          type: 'number',
          description: '0-based index of the slot with the accent outline (default 0)',
        },
        slot_size: {
          type: 'number',
          description: 'Slot side length in px (clamped 32..80, default 48)',
        },
        gap: { type: 'number', description: 'Gap between slots in px (clamped 0..24, default 12)' },
        accent_color: {
          type: 'string',
          description: 'Hex color for the focused-slot border (default #2563EB)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_chat_bubble_v0',
    description:
      'Chat message bubble for messaging / customer-support / AI chat UIs. Left-aligned (side=left, ' +
      'default): from-others bubble — slate-100 fill, slate-900 text, optional author label above, ' +
      'justified to flex-start. Right-aligned (side=right): from-self bubble — accent-color fill, ' +
      'white text, author suppressed (the self-bubble never carries "You:"), justified to flex-end. ' +
      'Optional timestamp below either side. Caller stacks multiple bubbles in a vertical parent to ' +
      'build the conversation. Use for "chat message", "message bubble", "conversation row", "聊天 ' +
      'bubble", "消息气泡". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Message body text (wraps within max_width)' },
        side: {
          type: 'string',
          enum: ['left', 'right'],
          description:
            'Alignment / color variant. "left" (default) = from-others (slate bg, author allowed). "right" = from-self (accent bg, no author).',
        },
        author: {
          type: 'string',
          description:
            'Sender display name. ONLY shown on side=left. Ignored on right (self bubbles never carry their own name).',
        },
        timestamp: {
          type: 'string',
          description: 'Optional relative time shown below the bubble (e.g. "2m", "Just now")',
        },
        max_width: {
          type: 'number',
          description: 'Bubble max width in px (clamped 160..480, default 280)',
        },
        accent_color: {
          type: 'string',
          description: 'Hex fill for self-side (side=right) bubbles (default #2563EB)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message'],
    },
  },
];
