// Extension tool definitions (added after the initial 19 base tools).
// Kept separate from element-tool-defs.ts so that file stays under the
// 800-line repo limit as the element-tool family grows toward ~100.

const schemaVersionProp = {
  type: 'string' as const,
  enum: ['1.0'],
  description:
    'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
};

const filePathProp = {
  type: 'string' as const,
  description: 'Path to .op file, or omit for live canvas',
};

const parentIdProp = {
  type: 'string' as const,
  description: 'Target parent node id (must exist in the document). Omit for root-level insertion.',
};

const pageIdProp = {
  type: 'string' as const,
  description: 'Target page ID (defaults to first page)',
};

export const ELEMENT_TOOL_DEFINITIONS_EXT = [
  {
    name: 'add_switch_v0',
    description:
      'iOS/Material toggle switch. Fixed 51×31 track (iOS HIG) with 27×27 white thumb. ' +
      'active=true → iOS green track + thumb right via justifyContent=flex-end. ' +
      'Use for "toggle", "switch", "on/off control". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        active: {
          type: 'boolean',
          description: 'Whether the switch is turned on (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_checkbox_v0',
    description:
      'Checkbox + inline label. 20×20 box (cornerRadius=4) with optional check icon when ' +
      'checked=true; otherwise empty with 1.5px stroke. Horizontal layout, gap=8. ' +
      'Use for "checkbox", "agreement", "select option". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Label shown next to the checkbox' },
        checked: {
          type: 'boolean',
          description: 'Whether the checkbox is checked (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_radio_v0',
    description:
      'Radio button + inline label. 20×20 ring (cornerRadius=10) with centered 10×10 dot ' +
      'when selected=true. Build radio groups by stacking multiple add_radio_v0 in a vertical parent. ' +
      'Use for "radio", "single choice option". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Label shown next to the radio' },
        selected: {
          type: 'boolean',
          description: 'Whether the radio is selected (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_tabs_v0',
    description:
      'Horizontal TOP TABS with active underline. Every tab uses width=fill_container so the bar ' +
      'splits evenly (Twitter/Material pattern, avoids the fill_container-in-fit_content layout ' +
      'trap that would blow up the active tab). Each tab is a vertical frame: [padded content ' +
      'wrapper] + [2px sibling rectangle underline when active]. Active tab also gets fontWeight=600. ' +
      'Underline is a sibling rect rather than a directional stroke because PenStroke only supports ' +
      'uniform/array thickness. Use for "top tabs", "secondary nav", "underline tabs", "下划线 tab". ' +
      'For iOS-style pill tabs use add_segmented_control_v0 instead. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Tab items. Each needs label; set active=true on the current tab.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_segmented_control_v0',
    description:
      'iOS pill-style segmented control. Container 32px high, cornerRadius=8, gray-100 fill. ' +
      'Each segment gets width=fill_container so the pill distributes equally (overflow-safe, ' +
      'never scrolls). Active segment floats white on top. Use for "iOS segmented control", ' +
      '"pill tabs", "filter toggle group", "iOS 分段控制". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Segments. Each needs label; mark active=true on the current segment.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_alert_v0',
    description:
      'Inline alert/callout banner. One-row layout: [icon?] + message + [close-x when dismissible]. ' +
      'width=fill_container, cornerRadius=8. Semantic color (info/success/warning/error) applied by ' +
      'caller via follow-up batch_design U-op. Use for "banner", "callout", "notification bar", ' +
      '"告知条". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Alert body text' },
        icon: { type: 'string', description: 'Optional 20×20 lucide icon name' },
        dismissible: {
          type: 'boolean',
          description: 'If true, appends a trailing close (x) icon (default false)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message'],
    },
  },
  {
    name: 'add_toast_v0',
    description:
      'Floating pill-shaped notification. Dark fill, cornerRadius=24, width=fit_content so it ' +
      'does not stretch across the canvas. Caller positions at bottom/top of screen via parent_id. ' +
      'Use for "toast", "snackbar", "popup notification", "轻提示". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Toast body text' },
        icon: { type: 'string', description: 'Optional 18×18 lucide icon name' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message'],
    },
  },
  {
    name: 'add_progress_bar_v0',
    description:
      'Linear progress bar. Fixed-pixel bar_width (default 240) so the fill can be computed as a ' +
      'deterministic sub-width (value/100 × bar_width) — pen-core has no percent/flex-basis sizing. ' +
      'value clamped 0-100. Use for "progress bar", "loading bar", "线性进度条". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        value: {
          type: 'number',
          description: 'Progress percentage 0-100 (default 50, clamped)',
        },
        bar_width: {
          type: 'number',
          description: 'Total track width in pixels (default 240)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_fab_v0',
    description:
      'Floating action button — circular 56×56 (Material FAB default) with centered icon at 43% of ' +
      'the button size. Caller handles positioning (pen-core has no reliable absolute positioning). ' +
      'Use for "FAB", "floating action button", "新建按钮", "compose button". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        icon: { type: 'string', description: 'lucide icon name for the FAB' },
        size: { type: 'number', description: 'Button diameter in pixels (default 56)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['icon'],
    },
  },
  {
    name: 'add_breadcrumb_v0',
    description:
      'Breadcrumb trail: interleaves item text with chevron-right separators (e.g. Home › Settings › ' +
      'Billing). The last item (or any item marked active=true) gets fontWeight=600 + active role. ' +
      'Use for "breadcrumb", "nav path", "面包屑". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Crumb items. Each needs label; mark the current crumb active=true.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_stepper_v0',
    description:
      'Horizontal numbered stepper: (1)───(2)───(3). Circles are 24×24 with step index (1-based ' +
      'display). Connectors use rectangle(fill_container, h=2) so they fill space between adjacent ' +
      'circles (pen-core splits fill_container siblings equally). Done circles + connectors through ' +
      'current use primary fill; rest gray. Use for "stepper", "progress steps", "wizard nav", ' +
      '"步骤条". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total step count (>= 1)' },
        current: {
          type: 'number',
          description: '0-indexed current step (default 0, clamped to 0..total-1)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['total'],
    },
  },
  {
    name: 'add_empty_state_v0',
    description:
      'Empty-state block: icon + title + optional subtitle + optional CTA button, stacked vertically ' +
      'with alignItems=center and padding=[48,24]. Use for "empty list", "no results", ' +
      '"nothing here yet", "first-run state", "空状态". Locks the 4-piece structure so weak ' +
      'models never emit a broken sparse layout. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Primary message (18/600)' },
        subtitle: { type: 'string', description: 'Optional secondary message (14/400)' },
        icon: { type: 'string', description: 'Optional 48×48 lucide icon name' },
        cta_label: {
          type: 'string',
          description: 'Optional CTA button label. Omit for a message-only empty state.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_rating_stars_v0',
    description:
      'Star rating row (e.g. review 4/5). Emits `total` lucide star icons; the first `filled` get ' +
      "role='star-filled' and the rest role='star-empty' so a follow-up batch_design U-op can apply " +
      'semantic colors. Style-Guide orthogonal — no bundled gold / gray palette. Use for "rating", ' +
      '"review stars", "评分". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        filled: {
          type: 'number',
          description: 'Number of filled stars (clamped to [0, total])',
        },
        total: {
          type: 'number',
          description: 'Total star count (default 5)',
        },
        size: {
          type: 'number',
          description: 'Per-star icon size in px (default 16)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['filled'],
    },
  },
  {
    name: 'add_link_v0',
    description:
      'Inline text link, optionally with trailing icon ("Learn more →"). Emits horizontal fit_content ' +
      "frame with role='link' / 'link-label' / 'link-icon'. Underline + color applied by follow-up " +
      'batch_design U-op. Use for "learn more link", "CTA text link", "read more". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Link text' },
        trailing_icon: {
          type: 'string',
          description:
            'Optional lucide icon name to append (e.g. "arrow-right"). Omit for plain text.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_kbd_v0',
    description:
      'Keyboard shortcut display ("⌘ K" / "Ctrl + Shift + P"). Each entry in `keys` becomes a ' +
      "bordered cell with role='kbd-key'; entries are joined with `separator` text between them " +
      '(default "+"). Use for "keyboard shortcut", "hotkey", "快捷键". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        keys: {
          type: 'array',
          description: 'Key glyphs in display order (e.g. ["⌘","K"] or ["Ctrl","Shift","P"])',
          items: { type: 'string' },
        },
        separator: {
          type: 'string',
          description: 'Glyph rendered between keys (default "+"). Pass "" for no separator.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['keys'],
    },
  },
  {
    name: 'add_carousel_dots_v0',
    description:
      'Carousel pagination dots. `total` dots laid out horizontally; the `current` dot (0-indexed, ' +
      'clamped) is stretched into a 16×6 pill, inactive dots are 6×6 circles. Emitted as ' +
      'frame+cornerRadius (not ellipse) per the layout.md §RING rule. Use for "carousel dots", ' +
      '"pagination indicator", "slide indicator", "轮播指示". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total dot count (>= 1)' },
        current: {
          type: 'number',
          description: '0-indexed current slide (default 0, clamped to 0..total-1)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['total'],
    },
  },
  {
    name: 'add_price_v0',
    description:
      'Price display ("$29/month" typography). Three inline text parts: currency (20/500), big ' +
      'amount (40/700), optional period (14/500). `amount` is a STRING so callers can pass ' +
      'pre-formatted values ("1,299" / "29.99"). Use for "pricing card price", "plan cost", ' +
      '"monthly fee", "定价". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        amount: {
          type: 'string',
          description: 'Formatted amount, e.g. "29" / "1,299" / "29.99"',
        },
        currency: {
          type: 'string',
          description: 'Currency glyph (default "$"). Pass "€" / "¥" / "£" as needed.',
        },
        period: {
          type: 'string',
          description: 'Optional trailing period like "/month" or "/yr". Omit for one-time prices.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['amount'],
    },
  },
  {
    name: 'add_quote_block_v0',
    description:
      'Quoted passage block — rounded padded container with quote text above an optional attribution ' +
      'line ("— Author"). fill_container so it wraps naturally. No left vertical bar: pen-core has no ' +
      'alignItems=stretch, so a bar sibling would collide with the fit_content circular-dep rule. ' +
      'role="quote-block" is enough for the style engine to apply a bar via batch_design U-op later. ' +
      'Use for "quote", "testimonial quote", "pull quote", "引言". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        quote: { type: 'string', description: 'Quoted text (wraps multi-line)' },
        author: {
          type: 'string',
          description: 'Optional attribution. Rendered as "— <author>" in a smaller weight.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['quote'],
    },
  },
  {
    name: 'add_code_block_v0',
    description:
      'Preformatted code block. fill_container frame (padding=[12,16], cornerRadius=8, gray-50 fill) ' +
      'with one text child that preserves `code` verbatim including newlines. `language` becomes part ' +
      'of the frame name only — no syntax highlighting (pen-core has no highlighter). Font family is ' +
      'unset (renderer default); inject JetBrains Mono via batch_design U-op if needed. Use for ' +
      '"code snippet", "preformatted text", "代码块". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        code: { type: 'string', description: 'Raw code text (newlines preserved)' },
        language: {
          type: 'string',
          description: 'Optional language hint ("typescript", "python"). Shown in frame name only.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['code'],
    },
  },
  {
    name: 'add_color_swatch_v0',
    description:
      'Design-system color swatch: colored square (default 64×64, cornerRadius=12) with optional ' +
      'label underneath. `color` accepts literal hex ("#2563EB") or a $variable ref ("$color-primary") ' +
      'which pen-core resolves per active theme. Use for "color token display", "palette card", ' +
      '"swatch", "色板". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        color: {
          type: 'string',
          description: 'Hex string or $variable ref used as solid fill on the square',
        },
        label: {
          type: 'string',
          description: 'Optional token name / description shown below the swatch',
        },
        size: { type: 'number', description: 'Square side length in px (default 64, min 16)' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['color'],
    },
  },
  {
    name: 'add_chart_bars_v0',
    description:
      'Horizontal bar-chart skeleton — one rectangle per `values` entry, bottom-aligned via ' +
      'alignItems=flex-end so bars read as axis-anchored. Heights scale to max(values); zero-valued ' +
      'bars get a 2px floor so pen-core does not collapse them. Negative / non-finite values clamp ' +
      'to 0. No axes / labels / grid — caller stitches those via batch_design U-op. Use for ' +
      '"bar chart", "histogram skeleton", "weekly steps chart", "柱状图". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        values: {
          type: 'array',
          description: 'Bar values in display order (≥0). Max value determines chart scale.',
          items: { type: 'number' },
        },
        bar_width: {
          type: 'number',
          description: 'Per-bar width in px (default 24, min 4)',
        },
        gap: {
          type: 'number',
          description: 'Inter-bar gap in px (default 12, min 0)',
        },
        chart_height: {
          type: 'number',
          description: 'Tallest-bar height in px, also sets frame height (default 160, min 40)',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['values'],
    },
  },
  {
    name: 'add_timeline_v0',
    description:
      'Vertical timeline — per-item row of (24×24 dot + fixed 24px connector) + ' +
      '(title [+ subtitle]) content column. Connector is a fixed-height rectangle (NOT ' +
      'fill_container — pen-core has no minHeight/stretch, so a fill_container connector ' +
      'would collapse to 0 when content col is shorter than the dot). Icon col uses ' +
      "fit_content = 24 dot + 24 connector = 48 (gap=0 between them) to drive the row's " +
      'cross-axis height. NO row padding, NO outer-timeline gap, NO icon-col gap — so the ' +
      'connector IS the FULL 24px inter-item spacing and dots land flush against both ' +
      'connector ends. Last item drops the connector. `active` items get primary fill on ' +
      'the dot. Known limitation: content taller than 48px (e.g. wrapped 3-line title) ' +
      'extends past the connector, creating a small visual break before the next dot — ' +
      'for that case build the timeline via batch_design instead. Use for "timeline", ' +
      '"activity history", "vertical stepper", "时间线", "动态". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description:
            'Timeline entries top-to-bottom. `active=true` highlights the dot; `subtitle` is optional.',
          items: {
            type: 'object',
            properties: {
              title: { type: 'string' },
              subtitle: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['title'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
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
];
