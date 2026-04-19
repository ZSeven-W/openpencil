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
      'Horizontal TOP TABS with active underline. Each tab gets padding=[12,16]; the active tab ' +
      'has a 2px bottom directional stroke and fontWeight=600. Use for "top tabs", "secondary nav", ' +
      'spec mentions "underline tabs" / "下划线 tab". For iOS-style pill tabs use ' +
      'add_segmented_control_v0 instead. schemaVersion 1.0',
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
