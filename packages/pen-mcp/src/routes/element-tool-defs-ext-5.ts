// Extension tool definitions — shard 5 of 5 (siblings: base / ext /
// ext-2 / ext-3 / ext-4). Houses P3 batch-1 v1 tools (avatar, badge,
// divider, body_text, icon_label) + P3 batch-2 v1 tools (alert, bottom_nav,
// breadcrumb, activity_ring, carousel_dots, action_menu, attachment_row,
// calendar_grid, avatar_group, callout). Each shard caps at the repo's
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
  {
    name: 'add_alert_v1',
    description:
      'Theme-aware inline alert / callout banner (v1). theme="light" (default): byte-parity with ' +
      'add_alert_v0 (no fill attrs — semantic color applied via batch_design U-op). ' +
      'theme="dark" / "system": identical output (alert has no hardcoded color fills). ' +
      'fill_container row of [icon?] + message + [close-x?]. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Alert message text.' },
        icon: { type: 'string', description: 'Optional leading lucide icon slug.' },
        dismissible: { type: 'boolean', description: 'When true, adds a close × icon.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message'],
    },
  },
  {
    name: 'add_bottom_nav_v1',
    description:
      'Theme-aware bottom tab bar (v1). theme="light" (default): byte-parity with ' +
      'add_bottom_nav_v0 (no fill attrs — colors inherited from ambient theme). ' +
      'theme="dark" / "system": identical output (bottom nav has no hardcoded color fills). ' +
      '3-5 tab items, icon + label stack, active tab gets nav-item-active role + fontWeight 600. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Tab items. Each: { title, icon, active? }.',
          items: {
            type: 'object',
            properties: {
              title: { type: 'string' },
              icon: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['title', 'icon'],
          },
        },
        height: { type: 'number', description: 'Bar height px. Default 62.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_breadcrumb_v1',
    description:
      'Theme-aware breadcrumb trail (v1). theme="light" (default): byte-parity with ' +
      'add_breadcrumb_v0 (no fill attrs — text colors inherited from canvas default). ' +
      'theme="dark" / "system": identical output (breadcrumb has no hardcoded color fills). ' +
      'Interleaves item text with chevron-right separators. Last item gets fontWeight=600. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Breadcrumb items. Each: { label, active? }.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_activity_ring_v1',
    description:
      'Theme-aware Apple-style progress ring with centered text (v1). theme="light" (default): ' +
      'byte-parity with add_activity_ring_v0 (#000000 ring stroke placeholder). ' +
      'theme="dark" / "system": identical output (placeholder stroke is theme-independent). ' +
      'frame(cornerRadius=size/2) + stroke + centered text — never ellipse+sibling text. ' +
      'Override ring color via batch_design U-op. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        center_text: { type: 'string', description: 'Text shown in ring center (e.g. "72%").' },
        size: { type: 'number', description: 'Ring diameter px. Default 80.' },
        thickness: { type: 'number', description: 'Stroke thickness px. Default 8.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes produce identical output.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['center_text'],
    },
  },
  {
    name: 'add_carousel_dots_v1',
    description:
      'Theme-aware carousel pagination dots (v1). theme="light" (default): byte-parity with ' +
      'add_carousel_dots_v0 (active=#111827 pill, inactive=#D1D5DB dot). ' +
      'theme="dark": active=text-primary dark (#F1F5F9), inactive=border dark (#334155). ' +
      'theme="system": emits $color-text-primary / $color-border refs. ' +
      'Active dot is 16×6 pill; inactive 6×6 circle. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total number of dots.' },
        current: { type: 'number', description: 'Index of active dot (0-based). Default 0.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description:
            'Theme variant. Default "light". dark/system use semantic text-primary/border tokens.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['total'],
    },
  },
  {
    name: 'add_action_menu_v1',
    description:
      'Theme-aware action / context menu panel (v1). theme="light" (default): byte-parity with ' +
      'add_action_menu_v0 (surface=#FFFFFF, border=#E2E8F0, icon=#334155, label=#0F172A). ' +
      'theme="dark": dark surface + dark-mode fills. theme="system": $color-* refs. ' +
      'Destructive items use $color-destructive across all themes. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Menu items.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              icon: { type: 'string' },
              destructive: { type: 'boolean' },
              divider_before: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        width: { type: 'number', description: 'Panel width px. Default 200. Min 140.' },
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
    name: 'add_attachment_row_v1',
    description:
      'Theme-aware file attachment row (v1). theme="light" (default): byte-parity with ' +
      'add_attachment_row_v0 (surface=#F8FAFC, filename=#0F172A, size/icon=#64748B, remove=#94A3B8). ' +
      'theme="dark": dark fills. theme="system": $color-* refs. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        filename: { type: 'string', description: 'File name (shown bold).' },
        size: { type: 'string', description: 'Optional size label (e.g. "1.2 MB").' },
        icon: {
          type: 'string',
          description: 'Lucide file-* icon slug. Default "file".',
        },
        removable: { type: 'boolean', description: 'Adds × remove icon. Default true.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['filename'],
    },
  },
  {
    name: 'add_calendar_grid_v1',
    description:
      'Theme-aware month calendar grid (v1). theme="light" (default): byte-parity with ' +
      'add_calendar_grid_v0 (header=#6B7280, day=#111827, selected=#2563EB fill, today=#DBEAFE fill). ' +
      'theme="dark": dark fills via semantic palette. theme="system": $color-* refs. ' +
      '40px cells, weekday header + up to 6 week rows × 7. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        days_in_month: { type: 'number', description: 'Days in month. Default 30.' },
        start_day_offset: {
          type: 'number',
          description: 'Weekday of day 1 (0=Sun..6=Sat). Default 0.',
        },
        today: { type: 'number', description: 'Day number to highlight as today.' },
        selected_day: { type: 'number', description: 'Day number to show as selected.' },
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
    name: 'add_avatar_group_v1',
    description:
      'Theme-aware stacked avatar group (v1). theme="light" (default): byte-parity with ' +
      'add_avatar_group_v0 (ring=#FFFFFF, overflow bg=#F1F5F9, overflow text=#475569). ' +
      'theme="dark": ring=surface dark, overflow bg=surface-2 dark, overflow text=textMuted dark. ' +
      'theme="system": $color-* refs. Brand avatar palette (#3B82F6 etc.) stays hardcoded. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Avatar items. Each: { initial?, color? }.',
          items: {
            type: 'object',
            properties: {
              initial: { type: 'string' },
              color: { type: 'string' },
            },
          },
        },
        size: { type: 'number', description: 'Avatar diameter px. Default 32. Clamped 24..64.' },
        max_visible: {
          type: 'number',
          description: 'Max avatars before +N overflow tile. Default 4.',
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
    name: 'add_callout_v1',
    description:
      'Theme-aware inline doc callout (v1). theme="light" (default): byte-parity with ' +
      'add_callout_v0 (tone-keyed bg/fg: info=#DBEAFE/#1E40AF, success=#DCFCE7/#166534, ' +
      'warning=#FEF3C7/#92400E, danger=#FEE2E2/#991B1B, note=#F1F5F9/#0F172A). ' +
      'theme="dark": semantic alert palette. theme="system": $color-*-bg/$color-*-text refs. ' +
      'Use for docs, onboarding, "did you know" panels. Distinct from add_alert_v0 (banner). ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        body: { type: 'string', description: 'Body text. Required.' },
        title: { type: 'string', description: 'Optional bold heading line above the body.' },
        tone: {
          type: 'string',
          enum: ['info', 'success', 'warning', 'danger', 'note'],
          description: 'Color tone. Default "note" (slate).',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['body'],
    },
  },
];
