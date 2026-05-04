// Extension tool definitions — shard 7 of 8 (siblings: base / ext /
// ext-2 / ext-3 / ext-4 / ext-5 / ext-6 / ext-8). Houses P3 batch-6 v1 tools
// (metric_comparison, metric_row, nav_chip_row, notification_row,
// otp_input, pagination, phone_input, price, pricing_card, profile_header)
// and P3 batch-7 v1 tools (progress_bar, quote_block, radio, rating_stars,
// section_header).
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

export const ELEMENT_TOOL_DEFINITIONS_EXT_7 = [
  {
    name: 'add_metric_comparison_v1',
    description:
      'Theme-aware KPI with trend indicator (v1). theme="light" (default): byte-parity with ' +
      'add_metric_comparison_v0. theme="dark": label → textMuted; trend up → success, ' +
      'down → destructive, flat → textMuted. theme="system": $color-* refs. ' +
      'Big value + small label + optional arrow + percent change. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Metric label text. Required.' },
        value: {
          type: 'string',
          description:
            'Main metric value (pre-formatted string, e.g. "$12,480" / "98.7%"). Required.',
        },
        change: {
          type: 'string',
          description: 'Numeric change amount (e.g. "8%" / "1.2k"). Optional.',
        },
        trend: {
          type: 'string',
          enum: ['up', 'down', 'flat'],
          description: 'Trend direction — picks arrow icon + color. Default "flat".',
        },
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
    name: 'add_metric_row_v1',
    description:
      'Theme-aware horizontal scroll row of metric tiles (v1). No hardcoded colors in v0; ' +
      'light/dark/system modes are identical (byte-parity with v0 in all modes). ' +
      'Accepts theme param for API consistency across all v1 tools. ' +
      'Each tile: label 12/500 + value 28/700 + optional icon. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Array of metric tile items.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              value: { type: 'string' },
              icon: { type: 'string', description: 'Optional Lucide icon slug.' },
            },
            required: ['label', 'value'],
          },
        },
        tile_width: { type: 'number', description: 'Tile width in px. Default 120.' },
        gap: { type: 'number', description: 'Gap between tiles. Default 12.' },
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
    name: 'add_nav_chip_row_v1',
    description:
      'Theme-aware horizontal scroll row of nav chips (v1). No hardcoded colors in v0; ' +
      'light/dark/system modes are identical (byte-parity with v0 in all modes). ' +
      'Accepts theme param for API consistency across all v1 tools. ' +
      'Icon + label per chip, active state → bolder label. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Array of nav chip items.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              icon: { type: 'string', description: 'Optional Lucide icon slug.' },
              active: { type: 'boolean', description: 'Whether this chip is active.' },
            },
            required: ['label'],
          },
        },
        chip_width: { type: 'number', description: 'Chip width in px. Default 72.' },
        gap: { type: 'number', description: 'Gap between chips. Default 12.' },
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
    name: 'add_notification_row_v1',
    description:
      'Theme-aware notification list row (v1). theme="light" (default): byte-parity with ' +
      'add_notification_row_v0. theme="dark": timestamp → textSubtle, body → textBody, ' +
      'unread dot → destructive. theme="system": $color-* refs. ' +
      'Leading icon + title + optional timestamp + optional body preview + optional unread dot. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Notification title text. Required.' },
        body: { type: 'string', description: 'Optional body preview text.' },
        timestamp: { type: 'string', description: 'Optional timestamp label (e.g. "2h ago").' },
        icon: {
          type: 'string',
          description: 'Lucide icon slug for leading affordance. Default "bell".',
        },
        unread: {
          type: 'boolean',
          description: 'When true, renders a small unread-indicator dot.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_otp_input_v1',
    description:
      'Theme-aware OTP / PIN code input (v1). theme="light" (default): byte-parity with ' +
      'add_otp_input_v0. theme="dark": slot bg → surface, filled border → borderStrong, ' +
      'empty border → border, digit text → textPrimary; focused border = accent_color (invariant). ' +
      'theme="system": $color-* refs. Horizontal row of N square slots (4..8 digits). ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        length: {
          type: 'number',
          description: 'Number of digit slots. Clamped 4..8. Default 6.',
        },
        digits: {
          type: 'array',
          items: { type: 'string' },
          description: 'Pre-filled digit strings. Shorter arrays leave remaining slots empty.',
        },
        focused_index: {
          type: 'number',
          description: '0-based index of the focused slot. Default 0.',
        },
        slot_size: { type: 'number', description: 'Slot size in px. Clamped 32..80. Default 48.' },
        gap: { type: 'number', description: 'Gap between slots. Clamped 0..24. Default 12.' },
        accent_color: {
          type: 'string',
          description: 'Accent hex for focused slot border. Default #2563EB.',
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
    name: 'add_pagination_v1',
    description:
      'Theme-aware pagination bar (v1). theme="light" (default): byte-parity with ' +
      'add_pagination_v0. theme="dark": arrow/inactive → textBody, ellipsis → textMuted; ' +
      'active pill bg = accent_color (brand-invariant), active text stays #FFFFFF. ' +
      'theme="system": $color-* refs for arrow/ellipsis/inactive fills. ' +
      'Numbered pills + prev/next arrows, Google-style ellipses for big ranges. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        total: { type: 'number', description: 'Total number of pages. Clamped >= 1. Required.' },
        current: {
          type: 'number',
          description: '1-based index of the current page. Clamped [1, total]. Default 1.',
        },
        siblings: {
          type: 'number',
          description:
            'Pages to show each side of current before collapsing to ellipsis. Default 1.',
        },
        show_arrows: {
          type: 'boolean',
          description: 'Include prev/next arrow buttons. Default true.',
        },
        accent_color: {
          type: 'string',
          description: 'Active page pill fill color. Default #0F172A (slate-900).',
        },
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
    name: 'add_phone_input_v1',
    description:
      'Theme-aware phone number input with country-code selector (v1). ' +
      'theme="light" (default): byte-parity with add_phone_input_v0. ' +
      'theme="dark": field bg → surface, stroke/divider → border, label → textBody, ' +
      'code → textPrimary, chevron → textMuted, placeholder → textSubtle, value → textPrimary. ' +
      'theme="system": $color-* refs. "+1 (555) …" pattern with country dial code button. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Optional label above the input.' },
        country_code: {
          type: 'string',
          description: 'Country code in the leading button. Default "+1".',
        },
        country_flag: { type: 'string', description: 'Optional flag emoji or abbreviation.' },
        placeholder: {
          type: 'string',
          description: 'Digits placeholder. Default "(555) 555-5555".',
        },
        value: { type: 'string', description: 'Pre-filled phone digits (populated state).' },
        required: {
          type: 'boolean',
          description: 'Appends " *" to the label when true.',
        },
        width: {
          type: 'number',
          description: 'Field width in px. Min 240. Default 320.',
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
    name: 'add_price_v1',
    description:
      'Theme-aware price display (v1). No hardcoded colors in v0; light/dark/system modes are ' +
      'identical (byte-parity with v0 in all modes). Accepts theme param for API consistency. ' +
      'Currency 20/500 + amount 40/700 + optional period 14/500, baseline-aligned. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        amount: {
          type: 'string',
          description: 'Price amount (pre-formatted string, e.g. "29", "99"). Required.',
        },
        currency: { type: 'string', description: 'Currency symbol. Default "$".' },
        period: { type: 'string', description: 'Billing period (e.g. "/month").' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['amount'],
    },
  },
  {
    name: 'add_pricing_card_v1',
    description:
      'Theme-aware SaaS pricing tier card (v1). theme="light" (default): byte-parity with ' +
      'add_pricing_card_v0. theme="dark": card bg → surface, border → border (featured: ' +
      'accent invariant), tier/currency/amount → textPrimary, description/period → textMuted, ' +
      'feature label → textBody; CTA bg stays brand-invariant. theme="system": $color-* refs. ' +
      'Tier name + big price + feature list + CTA button. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        tier: { type: 'string', description: 'Tier name (e.g. "Pro", "Team"). Required.' },
        price: {
          type: 'string',
          description: 'Price amount — number only (e.g. "29", "0"). Required.',
        },
        currency: { type: 'string', description: 'Currency symbol. Default "$".' },
        period: { type: 'string', description: 'Billing period (e.g. "/month").' },
        features: {
          type: 'array',
          items: { type: 'string' },
          description: 'Feature list (3-6 items typical).',
        },
        description: { type: 'string', description: 'Optional description beneath tier name.' },
        badge: {
          type: 'string',
          description:
            'Optional ribbon badge label. Featured tier auto-gets "Most popular" if omitted.',
        },
        cta: { type: 'string', description: 'CTA label. Default "Get started".' },
        emphasis: {
          type: 'string',
          enum: ['default', 'featured'],
          description: '"featured" adds accent border + CTA. Default "default".',
        },
        width: { type: 'number', description: 'Card width in px. Min 220. Default 280.' },
        corner_radius: { type: 'number', description: 'Corner radius. Default 16.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['tier', 'price'],
    },
  },
  {
    name: 'add_profile_header_v1',
    description:
      'Theme-aware large profile header (v1). theme="light" (default): byte-parity with ' +
      'add_profile_header_v0. theme="dark": name → textPrimary, handle → textMuted, ' +
      'bio → textBody; avatar bg (#3B82F6) and initial text (white) stay brand-invariant. ' +
      'theme="system": $color-* refs for name/handle/bio. ' +
      'Centered avatar + display name + optional handle / bio block. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        name: { type: 'string', description: 'Display name. Required.' },
        handle: { type: 'string', description: 'Optional handle (e.g. "@sarah").' },
        bio: { type: 'string', description: 'Optional bio / role line.' },
        initial: { type: 'string', description: 'Optional avatar initial (1-2 chars).' },
        avatar_size: {
          type: 'number',
          description: 'Avatar diameter in px. Clamped 64..160. Default 96.',
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
    name: 'add_progress_bar_v1',
    description:
      'Theme-aware linear progress bar (v1). theme="light" (default): byte-parity with ' +
      'add_progress_bar_v0. theme="dark": track bg → surface2 (#334155); fill stays ' +
      'brand-invariant accent. theme="system": $color-surface-2 ref for track. ' +
      'Fixed-pixel track; fill width = value/100 × bar_width. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        value: {
          type: 'number',
          description: 'Progress value 0..100. Default 50.',
        },
        bar_width: {
          type: 'number',
          description: 'Track width in px. Default 240.',
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
    name: 'add_quote_block_v1',
    description:
      'Theme-aware quoted passage block (v1). theme="light" (default): byte-parity with ' +
      'add_quote_block_v0. theme="dark": container bg → surface (#1E293B). ' +
      'theme="system": $color-surface ref. Rounded container, quote text + optional ' +
      'attribution. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        quote: { type: 'string', description: 'Quote text. Required.' },
        author: { type: 'string', description: 'Optional attribution (e.g. "Jane Doe").' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['quote'],
    },
  },
  {
    name: 'add_radio_v1',
    description:
      'Theme-aware radio button + label (v1). theme="light" (default): byte-parity with ' +
      'add_radio_v0. theme="dark": accent stays brand-invariant; unselected ring → ' +
      'border token (#334155). theme="system": $color-border ref for unselected ring. ' +
      '20×20 outer ring; 10×10 inner dot when selected. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Radio label text. Required.' },
        selected: { type: 'boolean', description: 'Whether the radio is selected. Default false.' },
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
    name: 'add_rating_stars_v1',
    description:
      'Theme-aware star rating row (v1). theme="light" (default): byte-parity with ' +
      'add_rating_stars_v0. theme="dark"/"system": identical (no explicit colors in v0 ' +
      '— icon_font nodes inherit canvas text color). Accepts theme for API consistency. ' +
      'star-filled / star-empty roles for downstream color U-ops. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        filled: {
          type: 'number',
          description: 'Number of filled stars. Clamped to [0, total]. Required.',
        },
        total: { type: 'number', description: 'Total number of stars. Default 5.' },
        size: { type: 'number', description: 'Star icon size in px. Default 16.' },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['filled'],
    },
  },
  {
    name: 'add_section_header_v1',
    description:
      'Theme-aware section header (v1). theme="light" (default): byte-parity with ' +
      'add_section_header_v0. theme="dark"/"system": identical (no explicit colors in ' +
      'v0 — text inherits canvas theme color). Accepts theme for API consistency. ' +
      'Big title on the left + optional trailing action ("See all →"). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Section title text. Required.' },
        action: {
          type: 'object',
          properties: {
            label: { type: 'string' },
            icon: { type: 'string' },
          },
          required: ['label'],
          description: 'Optional trailing action (e.g. { label: "See all", icon: "arrow-right" }).',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description: 'Theme variant. Default "light".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
];
