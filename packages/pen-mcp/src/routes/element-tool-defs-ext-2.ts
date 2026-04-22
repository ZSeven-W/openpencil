// Extension tool definitions — shard 2 of 2 (the first shard is in
// `element-tool-defs-ext.ts`). Both shards are concatenated in
// `element-tool-defs.ts` to form the final registry. The split keeps
// each file under the repo's 800-line ceiling while the element-tool
// family continues to grow.
//
// When adding a new tool: pick whichever shard has the fewer tools
// to keep the split balanced. Run `wc -l` on both files before
// committing.

import {
  schemaVersionProp,
  filePathProp,
  parentIdProp,
  pageIdProp,
} from './element-tool-def-props';

export const ELEMENT_TOOL_DEFINITIONS_EXT_2 = [
  {
    name: 'add_calendar_grid_v0',
    description:
      'Month calendar grid — weekday header (Sun..Sat) + up to 6 week rows × 7 cells. ' +
      '`start_day_offset` blanks leading cells so day 1 lands on the correct weekday (0=Sun..6=Sat). ' +
      '`today` gets a light-blue tint; `selected_day` gets a solid primary fill (selected wins when ' +
      "they overlap). Fixed cell size 40px — pen-core has no grid primitive so it's emitted as " +
      'nested vertical-of-horizontal frames. Use for "calendar", "date picker grid", "month view", ' +
      '"日历". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        days_in_month: {
          type: 'number',
          description: 'Days in this month (clamped 1..31, default 30)',
        },
        start_day_offset: {
          type: 'number',
          description:
            'Weekday of day 1 (0=Sunday..6=Saturday, clamped; default 0). Blank cells fill the offset.',
        },
        today: {
          type: 'number',
          description: 'Optional day-of-month to highlight as today (light tint)',
        },
        selected_day: {
          type: 'number',
          description: 'Optional day-of-month to mark as selected (solid primary fill)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_metric_comparison_v0',
    description:
      'KPI with trend: label (small, slate) above value (big 28/700) with optional arrow + ' +
      'change amount on the right. trend enum (up/down/flat) drives arrow icon (trending-up/-down/' +
      'minus) + color (emerald/red/slate). The "$12k ↑ 8%" card-cell pattern. Distinct from ' +
      'add_metric_row_v0 which is a scroll row of label+value cells without trend. Use for ' +
      '"KPI trend", "dashboard metric with change", "仪表盘带趋势指标". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Small label above the value (e.g. "Revenue")' },
        value: {
          type: 'string',
          description: 'Formatted metric (caller formats — "$12,480" / "98.7%" / "2.3k")',
        },
        change: {
          type: 'string',
          description: 'Change amount ("8%" / "1.2k"). Triggers arrow + colored change text.',
        },
        trend: {
          type: 'string',
          enum: ['up', 'down', 'flat'],
          description:
            'up=green+trending-up, down=red+trending-down, flat=slate+minus. Default flat.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label', 'value'],
    },
  },
  {
    name: 'add_notification_row_v0',
    description:
      'Notification list row: leading icon + title (inline with optional timestamp + optional ' +
      'unread red dot) + optional body preview line. Distinct from add_list_row_v0 (no timestamp ' +
      'affordance, no unread marker). Use for "notification item", "alert row", "通知条目". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Notification title (bold, one line)' },
        body: {
          type: 'string',
          description: 'Optional body preview (one line, truncated visually)',
        },
        timestamp: { type: 'string', description: 'Optional relative time ("2h ago", "Now")' },
        icon: { type: 'string', description: 'Leading lucide icon slug. Default "bell".' },
        unread: {
          type: 'boolean',
          description: 'When true, renders a small red dot next to the title (unread marker)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_status_badge_v0',
    description:
      'Semantic status indicator: small colored dot + short label (e.g. "● Online", "● Busy", ' +
      '"● Error"). ALWAYS has a dot — that is what makes it visually a status, distinguishing ' +
      'from the more general add_badge_v0 (pill label without dot). tone picks dot color: ' +
      'success (green) / warning (amber) / error (red) / info (blue) / neutral (slate, default). ' +
      'Use for "status", "presence", "health indicator", "状态". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Label text (e.g. "Online")' },
        tone: {
          type: 'string',
          enum: ['success', 'warning', 'error', 'info', 'neutral'],
          description: 'Dot color tone. Default "neutral" (slate gray).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_spinner_v0',
    description:
      'Loading spinner (static): a ring + 3/4-sweep active arc. No animation — pen-core is ' +
      'still-frame, so use size + thickness to match the desired visual and rely on the app to ' +
      'animate if needed. Use for "loading spinner", "progress circle", "loader", "加载圈". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        size: { type: 'number', description: 'Outer diameter in px (default 32, clamped 16..128)' },
        thickness: {
          type: 'number',
          description: 'Stroke thickness in px (default 3, clamped 1..16)',
        },
        track_color: {
          type: 'string',
          description: 'Track ring color (default #E2E8F0 slate-200)',
        },
        active_color: {
          type: 'string',
          description: 'Active arc color (default #2563EB blue-600)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_tooltip_v0',
    description:
      'Tooltip pill: small dark (#111827) pill with white text, typical hover-hint appearance. ' +
      'Only emits the open-state body — caller positions it. position ("top"/"bottom"/"left"/"right") ' +
      'encodes a role hint for downstream positioning logic but the visual body is identical. ' +
      'NO arrow pointer (pen-core has no clean triangle primitive); compose one via batch_design ' +
      'rectangle + rotate if needed. Use for "tooltip", "help hint", "hover label", "提示浮层". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        text: { type: 'string', description: 'Tooltip body text (1-2 short lines)' },
        position: {
          type: 'string',
          enum: ['top', 'bottom', 'left', 'right'],
          description: 'Position hint; sets `tooltip-<position>` on outer role. Default "top".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['text'],
    },
  },
  {
    name: 'add_image_placeholder_v0',
    description:
      'Image placeholder — gray box with centered lucide icon + optional caption label. ' +
      'The "this will be an image later" affordance. Use for "photo slot", "hero image area", ' +
      '"upload zone", "cover placeholder", "图片占位". Separate from G() (which fetches real ' +
      'images via search) — this emits a visual placeholder frame, not an image node. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width in px (default 200, min 40)' },
        height: { type: 'number', description: 'Height in px (default 140, min 40)' },
        label: {
          type: 'string',
          description: 'Optional caption below the icon (e.g. "Upload", "Hero image")',
        },
        icon: {
          type: 'string',
          description: 'Lucide icon name (default "image"). Common: image-plus, video, camera',
        },
        corner_radius: {
          type: 'number',
          description: 'Corner radius (default 8). Use larger for card-style, 0 for sharp.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_video_placeholder_v0',
    description:
      'Video placeholder — dark slate (#334155) box with centered white play triangle + optional ' +
      'caption (white/70). Default 320×180 (16:9). The "future video embed" affordance. Structurally ' +
      'similar to add_image_placeholder_v0 but semantically distinct: dark bg + play icon reads as ' +
      '"play me later", not "picture coming". Use for "video slot", "video embed placeholder", ' +
      '"upcoming video", "视频占位". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width in px (default 320, min 80)' },
        height: { type: 'number', description: 'Height in px (default 180 for 16:9, min 60)' },
        label: { type: 'string', description: 'Optional caption (e.g. "Coming soon")' },
        corner_radius: { type: 'number', description: 'Corner radius (default 12)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_comment_v0',
    description:
      'Single comment row: circular avatar + (author + timestamp inline header) + body text. ' +
      'Social / UGC / feedback list unit. Compose multiple comments by calling this N times ' +
      'inside a vertical parent. Does NOT handle threaded replies / like count / action menu — ' +
      'compose those via batch_design. Use for "comment", "reply", "feedback row", "评论". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        author: { type: 'string', description: 'Author display name (shown bold inline)' },
        timestamp: { type: 'string', description: 'Relative time string ("2h ago", "Just now")' },
        body: { type: 'string', description: 'Comment body text (multi-line supported)' },
        avatar_initial: {
          type: 'string',
          description: '1-2 char initial for the avatar circle. Omit for blank placeholder.',
        },
        avatar_size: {
          type: 'number',
          description: 'Avatar diameter in px (default 40, min 24)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['author', 'body'],
    },
  },
  {
    name: 'add_modal_shell_v0',
    description:
      'Modal dialog shell: full-bleed dimmed backdrop + centered card (rounded, shadowed) ' +
      'containing a title + optional subtitle. BODY content (form fields, CTA button, etc.) ' +
      'is composed by the caller into the `modal-shell-card` role via a follow-up insert. ' +
      'This tool emits ONLY the chrome (scrim + card + header) — `shell` in the name is deliberate. ' +
      'Use for "modal", "dialog", "popup chrome", "confirm dialog shell", "模态框". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Modal heading text' },
        subtitle: { type: 'string', description: 'Optional description below title' },
        card_width: {
          type: 'number',
          description: 'Centered card width in px (default 400, min 280)',
        },
        card_padding: { type: 'number', description: 'Card inner padding (default 24, min 12)' },
        scrim_opacity: {
          type: 'number',
          description: 'Backdrop dim opacity 0..1 (default 0.5). 0 = borderless no-scrim.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_chart_line_v0',
    description:
      'Line-chart skeleton: polyline through N data points (normalized to max), optional dots at ' +
      'each vertex. fit_content width = values.length × point_spacing (default 32px). Use for ' +
      '"line chart", "trend chart", "折线图". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        values: {
          type: 'array',
          items: { type: 'number' },
          description: 'Data points. Non-finite / negative values clamp to 0. Must not be empty.',
        },
        point_spacing: {
          type: 'number',
          description: 'Width per data point slot (px, default 32, min 8)',
        },
        chart_height: { type: 'number', description: 'Chart height in px (default 160, min 40)' },
        dots: { type: 'boolean', description: 'Emit a filled dot at each vertex (default true)' },
        stroke_color: { type: 'string', description: 'Line stroke hex color (default #2563EB)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['values'],
    },
  },
  {
    name: 'add_chart_pie_v0',
    description:
      'Pie-chart skeleton: N colored slice ellipses via startAngle/sweepAngle arc support (NOT ' +
      'stacked full ellipses — that would be the ring anti-pattern). Slice angles sum to 360°. ' +
      'Set inner_radius_ratio > 0 for a donut. Use for "pie chart", "donut chart", "饼图". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        values: {
          type: 'array',
          items: { type: 'number' },
          description:
            'Slice values (any scale; normalized internally). All-zero input throws. Non-finite / negative clamp to 0.',
        },
        diameter: {
          type: 'number',
          description: 'Pie diameter (px, width=height; default 160, min 40)',
        },
        colors: {
          type: 'array',
          items: { type: 'string' },
          description:
            'Optional per-slice hex colors. If shorter than values.length, default palette fills the rest.',
        },
        inner_radius_ratio: {
          type: 'number',
          description: 'Donut hole radius as fraction of outer (0..0.9). Default 0 (full pie).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['values'],
    },
  },
  {
    name: 'add_select_v0',
    description:
      'Dropdown select (display / closed state). Label above input box matching add_form_field_v0 ' +
      'shape, but the input trailing slot is ALWAYS a chevron-down icon affordance. When `value` ' +
      'is set: input shows value text (black). When absent: input shows placeholder text (gray ' +
      '#94A3B8). The OPEN state (menu list) is NOT modeled here — compose via batch_design when ' +
      'needed. Use for "dropdown", "select", "picker", "下拉选择". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label text' },
        value: {
          type: 'string',
          description: 'Currently selected value text. Omit to show placeholder instead.',
        },
        placeholder: {
          type: 'string',
          description: 'Placeholder when value is absent. Default "Select…".',
        },
        trailing_icon: {
          type: 'string',
          description: 'Trailing icon. Default "chevron-down" (closed state).',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label text',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_skeleton_v0',
    description:
      'Loading skeleton placeholder. N stacked gray rectangles (cornerRadius=4) mimicking future ' +
      'text lines while content fetches. Last row defaults to ~60% width (220px) to suggest an ' +
      'unfinished paragraph. Use for "loading state", "placeholder", "shimmer". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        rows: {
          type: 'number',
          description: 'Number of skeleton rows (clamped 1..20, default 3)',
        },
        row_height: {
          type: 'number',
          description: 'Height per row in px (clamped 4..48, default 16)',
        },
        row_gap: {
          type: 'number',
          description: 'Gap between rows in px (clamped 0..32, default 12)',
        },
        last_row_short: {
          type: 'boolean',
          description: 'When true (default), last row is ~60% width to look unfinished',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_textarea_v0',
    description:
      'Multi-line text input for notes, bio, feedback. Same vertical label-above-input shape as ' +
      'add_form_field_v0 but input height grows by `rows` (default 4, clamped 2..12). Input area ' +
      'is vertical layout with placeholder top-aligned — matches native iOS/Material multi-line. ' +
      'Use for "description", "comments", "bio", "message". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label text' },
        placeholder: { type: 'string', description: 'Optional input placeholder' },
        rows: {
          type: 'number',
          description: 'Visible text rows (clamped 2..12, default 4). Initial visible height only.',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label text',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
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
];
