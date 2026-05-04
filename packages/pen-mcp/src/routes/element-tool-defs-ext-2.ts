// Extension tool definitions — shard 2 of 3 (shards 1 and 3 are
// `element-tool-defs-ext.ts` and `element-tool-defs-ext-3.ts`).
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
      'When the canvas runs its auto-search pass, an image_search_query (or label) lets it ' +
      'fetch a relevant photo to replace the gray box; without one it falls back to a ' +
      'generic stock photo. schemaVersion 1.0',
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
        image_search_query: {
          type: 'string',
          description:
            '2-3 English keywords for the auto-search pass to fetch a photo with ' +
            '(e.g. "burger fries", "modern office workspace", "yoga sunset"). Strongly ' +
            'recommended for restaurant cards, product photos, hero banners, etc — ' +
            'without it the pipeline searches for the label or a generic placeholder.',
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
    name: 'add_tag_v0',
    description:
      'Single closable tag — filter / selection / applied-criteria chip. Pill body with a label ' +
      'and (by default) a trailing × close icon. tone enum picks the color pair: default (slate), ' +
      'accent (blue), success (green), warning (amber), error (red). Distinct from add_badge_v0 ' +
      '(read-only static label, no × affordance, smaller font) and add_chip_input_v0 (multi-tag ' +
      'input FIELD with inline caret). Use for "filter chip", "selected criterion", "applied tag", ' +
      '"category pill", "标签", "筛选标签", "可移除标签". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Tag text (e.g. "Status: Active", "Plan: Pro").' },
        removable: {
          type: 'boolean',
          description: 'Render the trailing × close icon. Default true.',
        },
        tone: {
          type: 'string',
          enum: ['default', 'accent', 'success', 'warning', 'error'],
          description: 'Color tone (bg + fg pair). Default "default" (slate-100 / slate-600).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_data_table_row_v0',
    description:
      'Desktop / dashboard data-table row — N evenly-spaced cells laid out horizontally (no ' +
      'vertical dividers, modern Linear / Stripe styling). Set `header: true` for the column-' +
      'header row (smaller, bolder, slate-500). Set `selected: true` on a body row to tint it ' +
      'slate-50. For row separators stack `add_divider_v0` between successive rows. Distinct ' +
      'from add_list_row_v0 (iOS / mobile leading-icon list cell). Use for "table row", "data ' +
      'row", "customer row", "order row", "table header", "数据表行", "表格行". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        columns: {
          type: 'array',
          description:
            'Cells in column order. Each cell renders as a fill_container frame so all columns ' +
            'share remaining width evenly.',
          items: {
            type: 'object',
            properties: {
              content: {
                type: 'string',
                description: 'Cell text (e.g. "Sarah Lee", "$1,240", "Active").',
              },
            },
            required: ['content'],
          },
        },
        header: {
          type: 'boolean',
          description: 'Render as the table header row (12/600 slate-500, 40px tall).',
        },
        selected: {
          type: 'boolean',
          description:
            'Tint a body row slate-50 to mark hover / selected state. Ignored on header.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['columns'],
    },
  },
  {
    name: 'add_avatar_group_v0',
    description:
      'Stacked avatar tile group — team / collaborator / "+N more" presence indicator. ' +
      'Renders up to `max_visible` filled circles (initial optional, fill rotates through a default ' +
      'palette so distinct items read apart) plus a slate "+N" tile when items overflow. Each ' +
      'tile gets a 2px white ring so they stay visually separated at the 4px gap (pen-core flex ' +
      "doesn't allow negative gap, so true overlap isn't possible — the ring is the affordance). " +
      'Distinct from add_avatar_v0 (single tile). Use for "team avatars", "5 contributors", ' +
      '"online users", "+N more", "团队成员", "在线用户". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Avatars in render order. Items past max_visible collapse into a "+N" tile.',
          items: {
            type: 'object',
            properties: {
              initial: {
                type: 'string',
                description: 'Centered initial (e.g. "JD"). Omit for an empty colored disk.',
              },
              color: {
                type: 'string',
                description: 'Optional hex fill. Falls back to a rotating palette per index.',
              },
            },
            required: [],
          },
        },
        size: {
          type: 'number',
          description: 'Avatar diameter in px. Default 32. Clamped 24..64.',
        },
        max_visible: {
          type: 'number',
          description:
            'Cap on rendered avatars; the rest collapse into a single "+N" tile at the end. ' +
            'Default 4. Clamped 1..10.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_sidebar_nav_v0',
    description:
      'Persistent vertical sidebar navigation — desktop / dashboard left rail. Stack of icon+' +
      'label rows with an optional brand/title row at top. Active item gets a slate-100 pill ' +
      'background + bolder darker label; inactive items have no fill and a muted slate label. ' +
      'Distinct from add_bottom_nav_v0 (mobile bottom tab bar — horizontal flow, label below ' +
      'icon) and add_top_nav_bar_v0 (mobile single-row header). Use for "sidebar", "side nav", ' +
      '"dashboard nav", "admin rail", "docs sidebar", "侧边栏", "侧边导航". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Nav items in vertical order. Set one item.active=true to mark current.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Nav item label (e.g. "Dashboard")' },
              icon: {
                type: 'string',
                description: 'Lucide icon slug (e.g. "home", "users", "settings")',
              },
              active: {
                type: 'boolean',
                description: 'Marks this row as the current page (slate-100 pill bg + bolder).',
              },
            },
            required: ['label', 'icon'],
          },
        },
        title: {
          type: 'string',
          description: 'Optional brand / section title row above the items (16/700).',
        },
        width: {
          type: 'number',
          description: 'Sidebar width in px. Default 240. Clamped 180..320.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
];
