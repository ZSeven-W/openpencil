// Extension tool definitions — shard 9 of 9 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5 / ext-6 / ext-7 / ext-8). Houses P3 batch-9 v1
// tools overflow from ext-8 (toolbar, tooltip, top_nav_bar, upload_dropzone,
// user_card, video_placeholder).
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

export const ELEMENT_TOOL_DEFINITIONS_EXT_9 = [
  {
    name: 'add_toolbar_v1',
    description:
      'Theme-aware desktop icon toolbar (v1). theme="light" (default): byte-parity ' +
      'with add_toolbar_v0. Surface, border, active-bg, icon fills tokenized. schemaVersion 1.0',
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
              icon: { type: 'string', description: 'Lucide icon slug.' },
              active: { type: 'boolean' },
              divider_after: { type: 'boolean' },
            },
            required: ['icon'],
          },
          description: 'Toolbar items.',
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
    name: 'add_tooltip_v1',
    description:
      'Theme-aware tooltip pill (v1). theme="light" (default): byte-parity ' +
      'with add_tooltip_v0. All modes identical — dark body (#111827) is ' +
      'intentional inverted-contrast pattern (spec §3.4). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        text: { type: 'string', description: 'Tooltip body text.' },
        position: {
          type: 'string',
          enum: ['top', 'bottom', 'left', 'right'],
          description: 'Position hint (sets role). Default "top".',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['text'],
    },
  },
  {
    name: 'add_top_nav_bar_v1',
    description:
      'Theme-aware mobile top navigation bar (v1). theme="light" (default): byte-parity ' +
      'with add_top_nav_bar_v0. All modes identical — no hardcoded surface colors. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Center title text.' },
        leading_icon: { type: 'string', description: 'Optional leading Lucide icon slug.' },
        trailing_icon: { type: 'string', description: 'Optional trailing Lucide icon slug.' },
        height: { type: 'number', description: 'Bar height in px. Default 56.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_upload_dropzone_v1',
    description:
      'Theme-aware file upload dropzone (v1). theme="light" (default): byte-parity ' +
      'with add_upload_dropzone_v0. bg (bgDeep), dashed stroke (border), icon (textMuted), ' +
      'title (textBody), subtitle (textMuted) tokenized. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width px. Default 480. Min 200.' },
        height: { type: 'number', description: 'Height px. Default 200. Min 120.' },
        title: {
          type: 'string',
          description: 'Main instruction. Default "Drop files to upload".',
        },
        subtitle: { type: 'string', description: 'Hint text. Default "or click to browse".' },
        icon: { type: 'string', description: 'Lucide icon. Default "upload-cloud".' },
        corner_radius: { type: 'number', description: 'Corner radius. Default 12.' },
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
    name: 'add_user_card_v1',
    description:
      'Theme-aware profile/contact card (v1). theme="light" (default): byte-parity ' +
      'with add_user_card_v0. name (textPrimary) and role (textMuted) tokenized. ' +
      'Avatar bg #3B82F6 and initial #FFFFFF hardcoded per spec §3.4. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        name: { type: 'string', description: 'Display name.' },
        role: { type: 'string', description: 'Optional secondary line (title/handle).' },
        initial: { type: 'string', description: 'Optional avatar initial (1-2 chars).' },
        avatar_size: {
          type: 'number',
          description: 'Avatar diameter px. Default 48. Clamped [32, 96].',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['name'],
    },
  },
  {
    name: 'add_video_placeholder_v1',
    description:
      'Theme-aware video placeholder (v1). theme="light" (default): byte-parity ' +
      'with add_video_placeholder_v0. All modes identical — dark bg (#334155), ' +
      'play icon (#FFFFFF), caption (#FFFFFFB3) are builder-private constants (spec §3.4). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        width: { type: 'number', description: 'Width px. Default 320. Min 80.' },
        height: { type: 'number', description: 'Height px. Default 180. Min 60.' },
        label: { type: 'string', description: 'Optional caption below play icon.' },
        corner_radius: { type: 'number', description: 'Corner radius. Default 12.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
];
