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
];
