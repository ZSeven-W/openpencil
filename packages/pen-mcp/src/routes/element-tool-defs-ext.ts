// Extension tool definitions — shard 1 of 2 (second shard is
// `element-tool-defs-ext-2.ts`). Both shards are concatenated in
// `element-tool-defs.ts` to form the final registry. Splitting the
// original single ext file in half keeps each shard under the
// repo's 800-line ceiling while the element-tool family continues
// to grow toward ~100.
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
];
