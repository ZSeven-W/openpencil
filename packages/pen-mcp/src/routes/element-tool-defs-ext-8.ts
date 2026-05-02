// Extension tool definitions — shard 8 of 8 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5 / ext-6 / ext-7). Houses P3 batch-7 v1 tools
// (search_bar, segmented_control, select, share_row, range_slider).
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

export const ELEMENT_TOOL_DEFINITIONS_EXT_8 = [
  {
    name: 'add_search_bar_v1',
    description:
      'Theme-aware search bar (v1). theme="light" (default): byte-parity with ' +
      'add_search_bar_v0 (no explicit fill). theme="dark": adds surface2 fill (#334155) ' +
      'so bar is visible against dark page bg. theme="system": $color-surface-2 ref. ' +
      'Height=44, cornerRadius=22 (iOS HIG); fill_container width. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        placeholder: { type: 'string', description: 'Placeholder text. Default "Search...".' },
        leading_icon: {
          type: 'string',
          description: 'Lucide icon slug for leading icon. Default "search".',
        },
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
    name: 'add_segmented_control_v1',
    description:
      'Theme-aware segmented control (v1). theme="light" (default): byte-parity with ' +
      'add_segmented_control_v0. theme="dark": track → surface2 (#334155); active seg ' +
      '→ surface (#1E293B); active label → textPrimary; inactive → textMuted. ' +
      'theme="system": $color-* refs. iOS pill-style tabs; 32px height. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
          description: 'Segment items. At most one should have active=true. Required.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_select_v1',
    description:
      'Theme-aware dropdown select — closed state (v1). theme="light" (default): byte-parity ' +
      'with add_select_v0. theme="dark": placeholder → textSubtle (#64748B). ' +
      'theme="system": $color-text-subtle ref for placeholder. ' +
      'Label + input frame + trailing chevron. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label text. Required.' },
        value: {
          type: 'string',
          description: 'Currently selected value. Omit to show placeholder.',
        },
        placeholder: {
          type: 'string',
          description: 'Placeholder shown when no value selected. Default "Select…".',
        },
        trailing_icon: {
          type: 'string',
          description: 'Trailing icon slug. Default "chevron-down".',
        },
        required: { type: 'boolean', description: 'Appends * to label when true.' },
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
    name: 'add_share_row_v1',
    description:
      'Theme-aware social-share button row (v1). theme="light" (default): byte-parity with ' +
      'add_share_row_v0. theme="dark": icon button bg → surface2 (#334155); icon fill ' +
      '+ label → textMuted (#94A3B8). theme="system": $color-* refs. ' +
      'Horizontal list of 40×40 circular icon buttons each labeled below. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        targets: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Share target label (e.g. "Twitter").' },
              icon: { type: 'string', description: 'Lucide icon slug.' },
            },
            required: ['label', 'icon'],
          },
          description: 'Share targets. Required.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['targets'],
    },
  },
  {
    name: 'add_range_slider_v1',
    description:
      'Theme-aware single-thumb range slider (v1). theme="light" (default): byte-parity ' +
      'with add_range_slider_v0. theme="dark": accent stays brand-invariant; thumb bg → ' +
      'surface (#1E293B); remaining track → border (#334155); label → textPrimary; ' +
      'value text → textMuted. theme="system": $color-* refs for non-accent fields. ' +
      'Static design representation at given value. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        value: { type: 'number', description: 'Current value. Clamped to [min, max]. Default 50.' },
        min: { type: 'number', description: 'Min value. Default 0.' },
        max: { type: 'number', description: 'Max value. Default 100.' },
        label: { type: 'string', description: 'Optional label shown above the track.' },
        show_value: {
          type: 'boolean',
          description: 'Show current value as right-aligned text. Default false.',
        },
        value_suffix: {
          type: 'string',
          description: 'Suffix for rendered value (e.g. "%", "px"). Default "".',
        },
        width: {
          type: 'number',
          description: 'Slider track width in px. Min 160. Default 320.',
        },
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
];
