// Extension tool definitions — shard 5 of 5 (siblings: base / ext /
// ext-2 / ext-3 / ext-4). Houses P3 batch-1 v1 tools (avatar, badge,
// divider, body_text, icon_label). Each shard caps at the repo's
// 800-line ceiling.
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

export const ELEMENT_TOOL_DEFINITIONS_EXT_5 = [
  {
    name: 'add_avatar_v1',
    description:
      'Theme-aware circular avatar (v1). theme="light" (default): byte-parity with ' +
      'add_avatar_v0 (no fill attrs — inherits from parent context). theme="dark" / "system": ' +
      'identical output (avatar has no hardcoded color fills). Accepts theme param for API ' +
      'consistency across all v1 tools. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        initial: { type: 'string', description: 'Optional centered initial (1-2 chars).' },
        size: { type: 'number', description: 'Avatar diameter px. Default 40.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_badge_v1',
    description:
      'Theme-aware short inline badge / pill / tag (v1). theme="light" (default): byte-parity ' +
      'with add_badge_v0 (no fill attrs — caller applies colors via batch_design U-op). ' +
      'theme="dark" / "system": identical output (badge has no hardcoded color fills). ' +
      'cornerRadius=999 (full pill), fontSize=11, fontWeight=600. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Badge text (e.g. "NEW", "BETA", "42").' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_divider_v1',
    description:
      'Theme-aware hairline divider (v1). theme="light" (default): byte-parity with ' +
      'add_divider_v0 (no fill attrs — colors inherited from ambient theme). theme="dark" / ' +
      '"system": identical output (divider has no hardcoded color fills). Horizontal = ' +
      'fill_container × thickness px; vertical = thickness × fill_container. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        orientation: {
          type: 'string',
          enum: ['horizontal', 'vertical'],
          description: 'Divider direction. Default "horizontal".',
        },
        thickness: { type: 'number', description: 'Thickness in px. Default 1.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: [],
    },
  },
  {
    name: 'add_body_text_v1',
    description:
      'Theme-aware body / description text (v1). theme="light" (default): byte-parity with ' +
      'add_body_text_v0 (no fill attrs — text color inherited from canvas default). ' +
      'theme="dark" / "system": identical output (body text has no hardcoded color fills). ' +
      'CJK auto-detection applies in all modes (lineHeight=1.6 + letterSpacing=0 for CJK, ' +
      'lineHeight=1.5 for Latin). Always Inter, width=fill_container, textGrowth=fixed-width. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        content: { type: 'string', description: 'Paragraph text content.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['content'],
    },
  },
  {
    name: 'add_icon_label_v1',
    description:
      'Theme-aware atomic icon + label pair (v1). theme="light" (default): byte-parity with ' +
      'add_icon_label_v0 (no fill attrs — icon and text colors inherited from parent context). ' +
      'theme="dark" / "system": identical output (icon_label has no hardcoded color fills). ' +
      'Horizontal layout, alignItems=center, icon leads (16×16 lucide), label 14/500. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        icon: { type: 'string', description: 'Lucide icon slug (e.g. "star", "user").' },
        label: { type: 'string', description: 'Label text.' },
        gap: { type: 'number', description: 'Gap between icon and label px. Default 8.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['icon', 'label'],
    },
  },
];
