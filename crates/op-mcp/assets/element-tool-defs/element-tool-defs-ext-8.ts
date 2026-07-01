// Extension tool definitions — shard 8 of 8 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5 / ext-6 / ext-7). Houses P3 batch-7 v1 tools
// (search_bar, segmented_control, select, share_row, range_slider),
// P3 batch-8 v1 tools (sidebar_nav, skeleton, social_login_row, spinner,
// stat_card, stat_grid, status_badge, step_card, stepper, switch), and
// P3 batch-9 v1 tools (tabs, tag, text_button, textarea, timeline,
// toolbar, tooltip, top_nav_bar, upload_dropzone, user_card, video_placeholder).
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
  {
    name: 'add_sidebar_nav_v1',
    description:
      'Theme-aware vertical sidebar navigation (v1). theme="light" (default): byte-parity ' +
      'with add_sidebar_nav_v0. theme="dark": bg → surface (#1E293B); title + active label ' +
      '→ textPrimary; inactive label → textMuted; active item bg → surface2. ' +
      'theme="system": $color-* refs. Desktop dashboard / docs / admin left rail. schemaVersion 1.0',
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
              icon: { type: 'string', description: 'Lucide icon slug.' },
              active: { type: 'boolean', description: 'Marks item as currently selected.' },
            },
            required: ['label', 'icon'],
          },
          description: 'Navigation items. Required.',
        },
        title: { type: 'string', description: 'Optional brand/section title above items.' },
        width: {
          type: 'number',
          description: 'Sidebar width in px. Default 240. Clamped 180..320.',
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
    name: 'add_skeleton_v1',
    description:
      'Theme-aware loading skeleton (v1). theme="light" (default): byte-parity with ' +
      'add_skeleton_v0 (slate-200 rows). theme="dark": row fill → surface2 (#334155) — ' +
      'visible on dark bg. theme="system": $color-surface-2 ref. Stacked gray bars ' +
      'mimicking text lines. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        rows: {
          type: 'number',
          description: 'Number of skeleton rows (clamped 1..20). Default 3.',
        },
        row_height: {
          type: 'number',
          description: 'Row height in px (clamped 4..48). Default 16.',
        },
        row_gap: {
          type: 'number',
          description: 'Gap between rows in px (clamped 0..32). Default 12.',
        },
        last_row_short: {
          type: 'boolean',
          description: 'Last row is 60% width (paragraph-end pattern). Default true.',
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
    name: 'add_social_login_row_v1',
    description:
      'Theme-aware social-auth provider button row (v1). theme="light" (default): byte-parity ' +
      'with add_social_login_row_v0. theme="dark": button bg → surface; border → border; ' +
      'icon fill → textMuted; label → textPrimary. theme="system": $color-* refs. ' +
      '"Continue with Google / Apple / GitHub" pattern. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        providers: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              name: { type: 'string', description: 'Provider name (e.g. "google", "apple").' },
              icon: { type: 'string', description: 'Optional lucide icon override.' },
            },
            required: ['name'],
          },
          description: 'Providers to render (2-4 recommended, max 6). Required.',
        },
        orientation: {
          type: 'string',
          enum: ['vertical', 'horizontal'],
          description: 'Layout orientation. Default "vertical" (stacked full-width buttons).',
        },
        width: {
          type: 'number',
          description: 'Button width in px (vertical only). Default 320. Min 200.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['providers'],
    },
  },
  {
    name: 'add_spinner_v1',
    description:
      'Theme-aware loading spinner (v1). theme="light" (default): byte-parity with ' +
      'add_spinner_v0. All theme modes identical — track_color/active_color are caller-' +
      'overridable params (default #E2E8F0 track, #2563EB arc). Accepts theme for API ' +
      'consistency. 3/4-sweep arc; static still-frame. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        size: {
          type: 'number',
          description: 'Outer diameter in px (clamped 16..128). Default 32.',
        },
        thickness: {
          type: 'number',
          description: 'Stroke thickness in px (clamped 1..16). Default 3.',
        },
        track_color: { type: 'string', description: 'Static ring color. Default "#E2E8F0".' },
        active_color: { type: 'string', description: 'Active arc color. Default "#2563EB".' },
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
  {
    name: 'add_stat_card_v1',
    description:
      'Theme-aware big-number stat card (v1). theme="light" (default): byte-parity with ' +
      'add_stat_card_v0. theme="dark": bg → surface; border → border; label → textMuted; ' +
      'icon → textSubtle; value → textPrimary. Delta tones (success/error/flat) stay ' +
      'hardcoded — status semantics. theme="system": $color-* refs. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Metric label shown above value. Required.' },
        value: { type: 'string', description: 'Primary metric value (e.g. "$12.4k"). Required.' },
        icon: { type: 'string', description: 'Optional lucide icon in header corner.' },
        delta: { type: 'string', description: 'Optional delta text (e.g. "+8% vs last week").' },
        trend: {
          type: 'string',
          enum: ['up', 'down', 'flat'],
          description: 'Trend direction for delta tone. Default "flat".',
        },
        width: { type: 'number', description: 'Card width in px. Default 240. Min 160.' },
        corner_radius: { type: 'number', description: 'Corner radius. Default 16.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label', 'value'],
    },
  },
  {
    name: 'add_stat_grid_v1',
    description:
      'Theme-aware non-scrolling stat grid (v1). theme="light" (default): byte-parity with ' +
      'add_stat_grid_v0. All modes identical — no explicit fill colors in v0 (text inherits). ' +
      'Accepts theme for API consistency. 2-5 fill_container cells side-by-side. schemaVersion 1.0',
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
              value: { type: 'string', description: 'Metric value (e.g. "1,284").' },
              label: { type: 'string', description: 'Metric label.' },
              icon: { type: 'string', description: 'Optional lucide icon slug.' },
            },
            required: ['value', 'label'],
          },
          description: 'Stat items (2-5). Required.',
        },
        gap: { type: 'number', description: 'Gap between cells in px. Default 16.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_status_badge_v1',
    description:
      'Theme-aware status indicator pill (v1). theme="light" (default): byte-parity with ' +
      'add_status_badge_v0. All modes identical — dot colors are status semantics ' +
      '(success=emerald, warning=amber, error=red, info=blue, neutral=slate), kept ' +
      'hardcoded. Accepts theme for API consistency. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Status label text. Required.' },
        tone: {
          type: 'string',
          enum: ['success', 'warning', 'error', 'info', 'neutral'],
          description: 'Status tone. Default "neutral".',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_step_card_v1',
    description:
      'Theme-aware onboarding step card (v1). theme="light" (default): byte-parity with ' +
      'add_step_card_v0. theme="dark": title → textPrimary; description → textMuted; ' +
      'incomplete circle bg → surface. Accent (#2563EB) and check icon white stay hardcoded. ' +
      'theme="system": $color-* refs. Numbered circle + title + description. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        number: {
          description: 'Step index shown in circle (1-3 chars, e.g. 1, "01"). Required.',
          oneOf: [{ type: 'string' }, { type: 'number' }],
        },
        title: { type: 'string', description: 'Step title. Required.' },
        description: { type: 'string', description: 'Step description body. Required.' },
        completed: { type: 'boolean', description: 'Filled accent circle vs ring. Default false.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['number', 'title', 'description'],
    },
  },
  {
    name: 'add_stepper_v1',
    description:
      'Theme-aware horizontal numbered stepper (v1). theme="light" (default): byte-parity ' +
      'with add_stepper_v0. theme="dark": pending circle fill → border (#334155); pending ' +
      'number → textMuted; pending connector → border. Accent (#2563EB) and done-state ' +
      'white stay hardcoded. theme="system": $color-* refs for pending slots. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total step count. Required.' },
        current: { type: 'number', description: 'Current active step index (0-based). Default 0.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['total'],
    },
  },
  {
    name: 'add_switch_v1',
    description:
      'Theme-aware iOS/Material toggle switch (v1). theme="light" (default): byte-parity ' +
      'with add_switch_v0. All modes identical — #34C759 (iOS green active) and #E5E5EA ' +
      '(iOS gray inactive) are builder-private iOS HIG literals (spec §3.4), not tokenized. ' +
      '#FFFFFF thumb stays hardcoded. 51×31px, cornerRadius=16. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        active: { type: 'boolean', description: 'Switch on/off state. Default false.' },
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
  {
    name: 'add_tabs_v1',
    description:
      'Theme-aware horizontal tabs with underline (v1). theme="light" (default): byte-parity ' +
      'with add_tabs_v0. All modes identical — #2563EB underline is brand accent (spec §3.4). schemaVersion 1.0',
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
          description: 'Tab items. Mark one as active: true for the selected tab.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_tag_v1',
    description:
      'Theme-aware closable filter / selection chip (v1). theme="light" (default): byte-parity ' +
      'with add_tag_v0. All modes identical — tone bg/fg pairs are status semantic colors (spec §3.4). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Tag label text.' },
        removable: {
          type: 'boolean',
          description: 'Render trailing × icon. Default true.',
        },
        tone: {
          type: 'string',
          enum: ['default', 'accent', 'success', 'warning', 'error'],
          description: 'Color tone. Default "default" (slate).',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_text_button_v1',
    description:
      'Theme-aware padding-based text button (v1). theme="light" (default): byte-parity ' +
      'with add_text_button_v0. All modes identical — no hardcoded surface colors. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Button label text.' },
        leading_icon: {
          type: 'string',
          description: 'Optional Lucide icon slug shown before the label.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_textarea_v1',
    description:
      'Theme-aware multi-line text input (v1). theme="light" (default): byte-parity ' +
      'with add_textarea_v0. All modes identical — no hardcoded surface colors. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Field label.' },
        placeholder: { type: 'string', description: 'Placeholder text.' },
        rows: {
          type: 'number',
          description: 'Visible rows. Default 4. Clamped [2, 12].',
        },
        required: { type: 'boolean', description: 'Append * to label. Default false.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light". All modes identical for this tool.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label'],
    },
  },
  {
    name: 'add_timeline_v1',
    description:
      'Theme-aware vertical timeline (v1). theme="light" (default): byte-parity ' +
      'with add_timeline_v0. Active dot #2563EB is hardcoded (spec §3.4). ' +
      'Inactive dot/connector (border) and subtitle (textMuted) tokenized. schemaVersion 1.0',
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
              title: { type: 'string' },
              subtitle: { type: 'string' },
              active: { type: 'boolean' },
            },
            required: ['title'],
          },
          description: 'Timeline items (at least 1 required).',
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
];
// add_toolbar_v1 through add_video_placeholder_v1 → moved to ext-9 to stay under 800-line ceiling.
