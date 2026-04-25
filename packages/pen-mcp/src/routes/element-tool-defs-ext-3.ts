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
  {
    name: 'add_stat_card_v0',
    description:
      'Big-number stat card — standalone featured-metric tile: label (uppercase muted) + huge 32/700 ' +
      'value + optional delta line (tone-colored by trend) + optional corner icon. Dashboard KPI / ' +
      '"your XYZ today" pattern. Distinct from: `add_stat_grid_v0` (multi-cell side-by-side, smaller ' +
      'per-cell value), `add_metric_comparison_v0` (horizontal labeled KPI with inline trend arrow). ' +
      'Pick stat_card for "featured single metric"; use the others for compact rows. Use for "KPI card", ' +
      '"big number card", "metric tile", "stat widget", "关键指标卡", "数据大屏卡片". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: {
          type: 'string',
          description: 'Small label above the big number (rendered uppercase)',
        },
        value: {
          type: 'string',
          description: 'Primary metric string, e.g. "$12.4k" / "1,284" / "98.2%"',
        },
        icon: { type: 'string', description: 'Optional lucide icon shown in the top-right corner' },
        delta: {
          type: 'string',
          description: 'Optional delta text below the value (e.g. "+8% vs last week")',
        },
        trend: {
          type: 'string',
          enum: ['up', 'down', 'flat'],
          description:
            'Tone for the delta line. up=emerald, down=red, flat=slate (default). Value is always slate-900 regardless.',
        },
        width: { type: 'number', description: 'Card width in px (default 240, min 160)' },
        corner_radius: { type: 'number', description: 'Corner radius (default 16)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label', 'value'],
    },
  },
  {
    name: 'add_social_login_row_v0',
    description:
      'Social-auth provider button row — the "Continue with Google / Apple / Microsoft" pattern ' +
      'on login / signup screens. Two orientations: "vertical" (default, full-width stacked buttons ' +
      'with icon + "Continue with {Name}" label) and "horizontal" (compact icon-only square pills ' +
      'side-by-side). Known provider names (google / apple / microsoft / github / facebook / twitter ' +
      '/ x / linkedin / discord / slack / email / phone) auto-resolve to lucide icons; override via ' +
      '`providers[i].icon`. Use for "social login", "Sign in with Google", "OAuth buttons", "SSO row", ' +
      '"第三方登录", "社交登录". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        providers: {
          type: 'array',
          description:
            'Provider list, 1-6 items. Each item: { name: string, icon?: string }. Known names auto-map icons.',
          items: {
            type: 'object',
            properties: {
              name: { type: 'string' },
              icon: { type: 'string', description: 'Optional lucide icon override' },
            },
            required: ['name'],
          },
        },
        orientation: {
          type: 'string',
          enum: ['vertical', 'horizontal'],
          description:
            '"vertical" (default) = full-width stacked buttons w/ label. "horizontal" = compact icon-only pills.',
        },
        width: {
          type: 'number',
          description: 'Button width in px (default 320, min 200, vertical only)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['providers'],
    },
  },
  {
    name: 'add_pricing_card_v0',
    description:
      'SaaS pricing-tier card — the "Pro $29/month" column from pricing tables. Columns: tier name + ' +
      'optional description, big price (currency + amount + period), feature list with check icons, ' +
      'and a primary CTA at the bottom. Two emphases: "default" (white bg, slate border, slate CTA) ' +
      'and "featured" (accent border + CTA, auto-adds "Most popular" badge unless `badge` overrides). ' +
      'Use for "pricing card", "plan card", "SaaS tier", "subscription plan", "价格卡", "套餐卡". ' +
      'For a 3-column pricing section: call this 3× with different `tier`/`price`/`emphasis` values under ' +
      'the same section parent_id. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        tier: { type: 'string', description: 'Tier name (e.g. "Pro", "Team", "Enterprise")' },
        price: {
          type: 'string',
          description: 'Price amount (number only — currency rendered separately, e.g. "29", "0")',
        },
        currency: { type: 'string', description: 'Currency symbol before price (default "$")' },
        period: {
          type: 'string',
          description: 'Billing period after price (e.g. "/month", "/year", "/seat")',
        },
        features: {
          type: 'array',
          description: 'Feature list, 3-6 items typical. Each rendered with a leading check icon.',
          items: { type: 'string' },
        },
        description: {
          type: 'string',
          description: 'Small description beneath the tier name (e.g. "For growing teams")',
        },
        badge: {
          type: 'string',
          description:
            'Optional ribbon label (e.g. "Most popular"). Auto-shown on featured if omitted.',
        },
        cta: { type: 'string', description: 'Primary CTA label (default "Get started")' },
        emphasis: {
          type: 'string',
          enum: ['default', 'featured'],
          description: '"default" (slate) or "featured" (accent — highlights the recommended tier)',
        },
        width: { type: 'number', description: 'Card width in px (default 280, min 220)' },
        corner_radius: { type: 'number', description: 'Corner radius (default 16)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['tier', 'price'],
    },
  },
  {
    name: 'add_toast_v1',
    description:
      'Theme-aware variant of add_toast_v0 — same floating pill notification (fit_content, ' +
      'cornerRadius=24, optional leading icon + message) with an added `theme` param. ' +
      'theme="light" (default): byte-parity with v0 (dark pill #111827 + white fg). ' +
      'theme="dark": inverted contrast pill (#F1F5F9 light pill + #0F172A dark fg) — toasts use ' +
      'INVERTED contrast so a light pill sits on a dark surface. theme="system": emits ' +
      '$color-text-primary as bg + $color-surface as fg (inverted swap); render tracks themes.Mode. ' +
      'REQUIRES `applySemanticPalette(doc)` seeded when theme="system". Use when the spec says ' +
      '"dark-mode toast", "theme-aware snackbar", "supports light/dark toggle"; stick with ' +
      'add_toast_v0 for single-theme designs. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Toast message' },
        icon: { type: 'string', description: 'Optional leading lucide icon' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description:
            'Theme variant. "light" (default) = v0 parity (dark pill). "dark" = inverted light pill. "system" = $color-* refs, requires applySemanticPalette(doc).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message'],
    },
  },
  {
    name: 'add_range_slider_v0',
    description:
      'Single-thumb range slider — the "Volume 60%" / "Filter from N" horizontal control. Visual ' +
      'static representation (no interaction wiring). Optional label + value readout shown in a row ' +
      'above the track. Track renders as: filled accent portion (left of thumb) + 20×20 thumb with ' +
      'accent stroke + remaining slate portion (right of thumb). Value clamps to [min, max]. Use for ' +
      '"slider", "range input", "volume control", "opacity slider", "brightness slider", "滑块", ' +
      '"滑动条". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        value: { type: 'number', description: 'Current value (default mid-point)' },
        min: { type: 'number', description: 'Min value (default 0)' },
        max: { type: 'number', description: 'Max value (default 100)' },
        label: { type: 'string', description: 'Optional label above the track' },
        show_value: {
          type: 'boolean',
          description: 'When true, renders the current value on the right side of the header row',
        },
        value_suffix: {
          type: 'string',
          description: 'Optional suffix on rendered value (e.g. "%", "px", "°")',
        },
        width: { type: 'number', description: 'Track width in px (default 320, min 160)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
    },
  },
  {
    name: 'add_empty_chart_v1',
    description:
      'Theme-aware variant of add_empty_chart_v0 — same dashed-border "no data yet" tile shape with ' +
      'an added `theme` param controlling 5 colors (bg, border, icon, title, subtitle). ' +
      'theme="light" (default): byte-parity with v0 (slate-50 bg, slate-300 dashed border). ' +
      'theme="dark": hardcoded dark-mode palette (slate-800 bg, slate-600 border, slate-200 title) — ' +
      'use inside dark-theme dashboards so the empty slot matches surrounding card surfaces. ' +
      'theme="system": emits $color-surface-2 bg / $color-border stroke / $color-text-muted + ' +
      '$color-text-primary text refs; render tracks themes.Mode. REQUIRES `applySemanticPalette(doc)` ' +
      'seeded when theme="system". Use when the spec says "dark-mode empty chart", "empty state in ' +
      'dark dashboard", "theme-aware no-data placeholder". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width in px (default 320, min 120)' },
        height: { type: 'number', description: 'Height in px (default 200, min 100)' },
        title: { type: 'string', description: 'Headline above subtitle (default "No data yet")' },
        subtitle: { type: 'string', description: 'Hint beneath title' },
        icon: {
          type: 'string',
          description:
            'Lucide icon above title (default "bar-chart-2"; "line-chart" / "pie-chart" to match the widget it replaces)',
        },
        corner_radius: { type: 'number', description: 'Corner radius (default 12)' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description:
            'Theme variant. "light" (default) = v0 parity. "dark" = hardcoded dark hex. "system" = $color-* refs; requires applySemanticPalette(doc).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
    },
  },
  {
    name: 'add_phone_input_v0',
    description:
      'International phone input with leading country-code selector — the "+1 (555) …" pattern from ' +
      'every modern signup / login screen. Different from add_form_field_v0 (single text input, no ' +
      'prefix); use this when the spec calls for an international phone field with a country picker. ' +
      'Country selector renders as a button-shape (no actual menu); caller handles the picker UX as ' +
      'a separate concern. Set `value` to render the populated state (slate-900 text); omit for ' +
      'placeholder state (slate-400 text). Set `country_flag` (emoji or abbrev) to add a leading ' +
      'flag glyph next to the dial code. Use for "phone input", "phone field", "international phone", ' +
      '"country code input", "电话号码输入", "手机号输入". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Optional label above the input' },
        country_code: {
          type: 'string',
          description: 'Dial code shown in the leading button (default "+1")',
        },
        country_flag: {
          type: 'string',
          description: 'Optional flag emoji or country abbreviation shown next to the dial code',
        },
        placeholder: {
          type: 'string',
          description: 'Placeholder for the digits input (default "(555) 555-5555")',
        },
        value: {
          type: 'string',
          description:
            'Pre-filled phone digits (without country code). Omit for placeholder state.',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label',
        },
        width: { type: 'number', description: 'Field width in px (default 320, min 240)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
    },
  },
  {
    name: 'add_input_with_action_v0',
    description:
      'Input field with inline action button on the right — the "Subscribe to newsletter" / ' +
      '"Apply discount code" / "Send chat message" pattern. Different from add_form_field_v0 ' +
      '(label-above, no inline button) and add_search_bar_v0 (leading search icon, no trailing ' +
      'action). Two action variants: action_kind="text" (default, pill button with label like ' +
      '"Subscribe") or action_kind="icon" (44×44 square icon button — chat send arrow / search ' +
      'apply). Set `value` to render populated state (slate-900 text); omit for placeholder ' +
      'state (slate-400). Optional `leading_icon` adds an icon inside the input itself. Use for ' +
      '"newsletter signup", "apply discount code", "send message", "subscribe form", "promo code", ' +
      '"订阅输入", "发送消息输入". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        placeholder: { type: 'string', description: 'Placeholder text (e.g. "Enter email")' },
        value: {
          type: 'string',
          description: 'Pre-filled input value. Omit for placeholder state.',
        },
        action_label: {
          type: 'string',
          description: 'Button text when action_kind="text" (default "Submit")',
        },
        action_icon: {
          type: 'string',
          description: 'Lucide icon name when action_kind="icon" (default "arrow-right")',
        },
        action_kind: {
          type: 'string',
          enum: ['text', 'icon'],
          description:
            '"text" (default) = pill button with label. "icon" = 44×44 square icon button.',
        },
        leading_icon: {
          type: 'string',
          description: 'Optional lucide icon shown inside the input itself (left side)',
        },
        width: { type: 'number', description: 'Field width in px (default 400, min 280)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['placeholder'],
    },
  },
];
