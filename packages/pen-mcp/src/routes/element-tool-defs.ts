// Element-tool JSON schema definitions + names + dispatcher.
// Split out from design-routes.ts (2026-04-19) so that file stays under
// the repo's 800-line limit as the element-tool family continues to grow.
// Base (19) + ext (6) = 25 element tools → aggregated here.

import { handleAddCardRowV0 } from '../tools/add-card-row-v0';
import { handleAddMetricRowV0 } from '../tools/add-metric-row-v0';
import { handleAddNavChipRowV0 } from '../tools/add-nav-chip-row-v0';
import { handleAddBottomNavV0 } from '../tools/add-bottom-nav-v0';
import { handleAddActivityRingV0 } from '../tools/add-activity-ring-v0';
import { handleAddStatGridV0 } from '../tools/add-stat-grid-v0';
import { handleAddSectionHeaderV0 } from '../tools/add-section-header-v0';
import { handleAddTopNavBarV0 } from '../tools/add-top-nav-bar-v0';
import { handleAddIconButtonV0 } from '../tools/add-icon-button-v0';
import { handleAddDividerV0 } from '../tools/add-divider-v0';
import { handleAddBadgeV0 } from '../tools/add-badge-v0';
import { handleAddAvatarV0 } from '../tools/add-avatar-v0';
import { handleAddTextButtonV0 } from '../tools/add-text-button-v0';
import { handleAddHeadingV0 } from '../tools/add-heading-v0';
import { handleAddBodyTextV0 } from '../tools/add-body-text-v0';
import { handleAddIconLabelV0 } from '../tools/add-icon-label-v0';
import { handleAddListRowV0 } from '../tools/add-list-row-v0';
import { handleAddSearchBarV0 } from '../tools/add-search-bar-v0';
import { handleAddFormFieldV0 } from '../tools/add-form-field-v0';
import { handleAddSwitchV0 } from '../tools/add-switch-v0';
import { handleAddCheckboxV0 } from '../tools/add-checkbox-v0';
import { handleAddRadioV0 } from '../tools/add-radio-v0';
import { handleAddTabsV0 } from '../tools/add-tabs-v0';
import { handleAddSegmentedControlV0 } from '../tools/add-segmented-control-v0';
import { handleAddEmptyStateV0 } from '../tools/add-empty-state-v0';
import { ELEMENT_TOOL_DEFINITIONS_EXT } from './element-tool-defs-ext';

export const ELEMENT_TOOL_DEFINITIONS = [
  {
    name: 'add_card_row_v0',
    description:
      'Create a horizontal scroll row of CARDS (title + subtitle + optional icon). Each card is ' +
      '140×160, cornerRadius=20. Forces the overflow-safe wrapper+clipContent+fit_content structure ' +
      'taught in packages/pen-ai-skills/skills/phases/generation/overflow.md §HORIZONTAL SCROLL ROWS. ' +
      'Use when spec mentions "workout cards", "feature cards", "swipeable content cards", "pills", ' +
      'or any row where each item has a prominent title plus descriptive subtext. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        items: {
          type: 'array',
          description: 'Card items. Each needs title; subtitle and icon are optional.',
          items: {
            type: 'object',
            properties: {
              title: { type: 'string' },
              subtitle: { type: 'string' },
              icon: { type: 'string', description: 'lucide icon name' },
            },
            required: ['title'],
          },
        },
        card_width: { type: 'number', description: 'Fixed width per card (default 140)' },
        gap: { type: 'number', description: 'Inner-row gap in px (default 12)' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist in the document). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['items'],
    },
  },
  {
    name: 'add_metric_row_v0',
    description:
      'Create a horizontal scroll row of METRIC TILES (small label + big value + optional icon). ' +
      'Each tile is 120×100, cornerRadius=16, value rendered at 28/700 heading. Forces the ' +
      'overflow-safe wrapper+clipContent+fit_content structure. Use when spec mentions "dashboard ' +
      'stats", "KPI cards", "metric tiles", or shows Steps/Kcal/Sleep/Revenue-style rows. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        items: {
          type: 'array',
          description: 'Metric items. Each needs label + value; icon is optional.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Small descriptive label (e.g. "Steps")' },
              value: { type: 'string', description: 'Big number / formatted value (e.g. "8,432")' },
              icon: { type: 'string', description: 'lucide icon name' },
            },
            required: ['label', 'value'],
          },
        },
        tile_width: { type: 'number', description: 'Fixed width per tile (default 120)' },
        gap: { type: 'number', description: 'Inner-row gap in px (default 12)' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist in the document). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['items'],
    },
  },
  {
    name: 'add_nav_chip_row_v0',
    description:
      'Create a horizontal scroll row of NAV CHIPS (icon + small label, each with optional active ' +
      'state). Each chip is 72×fit_content, cornerRadius=12. Forces the overflow-safe ' +
      'wrapper+clipContent+fit_content structure. Use when spec mentions "category filter chips", ' +
      '"quick access shortcuts", "horizontal tab chips", "swipeable nav items". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        items: {
          type: 'array',
          description:
            'Chip items. Each needs label; icon and active are optional. Label-only chips are supported (text-only filter tags).',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              icon: { type: 'string', description: 'lucide icon name (optional)' },
              active: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        chip_width: { type: 'number', description: 'Fixed width per chip (default 72)' },
        gap: { type: 'number', description: 'Inner-row gap in px (default 12)' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist in the document). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['items'],
    },
  },
  {
    name: 'add_bottom_nav_v0',
    description:
      'Create a bottom tab bar (inline, not fixed-position). Forces the pattern taught in ' +
      'packages/pen-ai-skills/skills/phases/generation/layout.md §NO FIXED-POSITION LAYOUT: ' +
      'bottom-tab-bar is an inline child of the page (no empty spacer sibling needed, no ' +
      'position:fixed since the engine does not support it). Always prefer this over batch_design ' +
      'when the spec mentions "bottom nav", "tab bar", "tabbar", "底部导航", "tab bar with icons". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        items: {
          type: 'array',
          description:
            'Tab items. Each needs title + icon (lucide name). active marks current tab.',
          items: {
            type: 'object',
            properties: {
              title: { type: 'string' },
              icon: { type: 'string', description: 'lucide icon name (e.g. "home")' },
              active: { type: 'boolean' },
            },
            required: ['title', 'icon'],
          },
        },
        height: { type: 'number', description: 'Bar height in px (default 62)' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist in the document; validated before insertion). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['items'],
    },
  },
  {
    name: 'add_activity_ring_v0',
    description:
      'Create an Apple-style activity ring (progress ring) with centered text. Forces the ' +
      'frame+cornerRadius+stroke+centered-text pattern taught in ' +
      'packages/pen-ai-skills/skills/phases/generation/layout.md §RING / CIRCLE WITH CENTER CONTENT. ' +
      'NEVER emits the documented anti-patterns (ellipse+sibling text, layout=none+absolute). ' +
      'Ships colorless with fixed typography (Style Guide orthogonal) — override ring color / text ' +
      'size / weight via a follow-up batch_design U-op. Use when the spec mentions "activity ring", ' +
      '"progress ring", "circular progress", "ring with number", "Apple health ring". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit. Breaking schema changes ship as a new tool with _v1 suffix; old tools are kept one stage before being removed from ListTools.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        size: { type: 'number', description: 'Ring outer diameter in px (default 80)' },
        thickness: { type: 'number', description: 'Stroke thickness in px (default 8)' },
        center_text: { type: 'string', description: 'Text displayed in the ring center' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist in the document). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['center_text'],
    },
  },
  {
    name: 'add_stat_grid_v0',
    description:
      'Create a NON-scrolling stat grid (2-5 items share the row via fill_container). ' +
      'Different from add_metric_row_v0: this emits an inline grid that auto-distributes ' +
      'available width, solving the documented activity-rings overflow bug in ' +
      'packages/pen-ai-skills/skills/phases/generation/layout.md (three fixed 100px items in ' +
      'a 279px inner card silently clip the third item). Each cell is width=fill_container; ' +
      'renderer does the division. Use when spec mentions "stats row", "3 metrics side by side", ' +
      '"summary bar", or an inline (non-scrollable) row of KPIs. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        items: {
          type: 'array',
          description:
            'Stat cells. Each needs value + label; icon is optional. 2-5 items work best.',
          items: {
            type: 'object',
            properties: {
              value: { type: 'string', description: 'Big numeric value (e.g. "8,432")' },
              label: { type: 'string', description: 'Small descriptive label (e.g. "Steps")' },
              icon: { type: 'string', description: 'lucide icon name (optional)' },
            },
            required: ['value', 'label'],
          },
        },
        gap: { type: 'number', description: 'Gap between cells in px (default 16)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['items'],
    },
  },
  {
    name: 'add_section_header_v0',
    description:
      'Section header with big title on left + optional trailing action (e.g. "See all", ' +
      '"View more"). Forces horizontal space_between alignItems=center layout so the action ' +
      'always sits flush-right. Use when spec shows "Section Title" with a "See all" or "→" link, ' +
      'or any heading + secondary action pair. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        title: { type: 'string', description: 'Section title (big heading)' },
        action: {
          type: 'object',
          description: 'Optional trailing action (e.g. { label: "See all", icon: "arrow-right" })',
          properties: {
            label: { type: 'string' },
            icon: { type: 'string', description: 'lucide icon name (optional)' },
          },
          required: ['label'],
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['title'],
    },
  },
  {
    name: 'add_top_nav_bar_v0',
    description:
      'Mobile top navigation bar: optional leading icon (back/menu) + centered title + ' +
      'optional trailing icon (search/more). Dual of add_bottom_nav_v0. Title always centered; ' +
      'empty slots become 44×44 spacers so the title visually stays centered. Use when spec ' +
      'mentions "top bar", "app bar", "header with back button", "页面标题栏". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        title: { type: 'string', description: 'Centered title text' },
        leading_icon: {
          type: 'string',
          description: 'lucide icon name for the left slot (e.g. "chevron-left", "menu")',
        },
        trailing_icon: {
          type: 'string',
          description: 'lucide icon name for the right slot (e.g. "search", "more-vertical")',
        },
        height: { type: 'number', description: 'Bar height in px (default 56)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['title'],
    },
  },
  {
    name: 'add_icon_button_v0',
    description:
      'Icon-only button. Forces 44×44 minimum hit target (Apple HIG + Material) with ' +
      'flex-centered icon — NEVER emits the layout=none + absolute-positioned icon anti-pattern ' +
      'documented in pen-ai-skills memory (layout=none + nested children renders unreliably). ' +
      'Use when the spec shows "icon-only" buttons, search/close/menu buttons, or iconic actions ' +
      'in toolbars. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        icon: { type: 'string', description: 'lucide icon name' },
        size: { type: 'number', description: 'Button size (hit-target) in px (default 44)' },
        icon_size: { type: 'number', description: 'Icon glyph size in px (default 24)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['icon'],
    },
  },
  {
    name: 'add_divider_v0',
    description:
      'Hairline divider (rectangle, not stroke). Forces the pattern documented in pen-ai-skills memory: ' +
      'rectangle with height=1 + width=fill_container (horizontal) or width=1 + height=fill_container ' +
      '(vertical). Use between list rows, between form sections, between a card content and its footer. ' +
      'Ships colorless — override fill via a follow-up batch_design U-op. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        orientation: {
          type: 'string',
          enum: ['horizontal', 'vertical'],
          description: 'Default horizontal',
        },
        thickness: { type: 'number', description: 'Divider thickness in px (default 1)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: [],
    },
  },
  {
    name: 'add_badge_v0',
    description:
      'Short inline badge / pill / tag ("NEW", "BETA", "42", "Sale"). Forces the standard pill ' +
      'layout (cornerRadius=999, padding=[4,10], font 11/600). Documented constraint (overflow.md): ' +
      'CJK ≤8 chars, Latin ≤16 chars — longer labels are NOT a badge, use batch_design. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        label: {
          type: 'string',
          description: 'Short label (Latin ≤16 chars / CJK ≤8 chars)',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['label'],
    },
  },
  {
    name: 'add_avatar_v0',
    description:
      'Circular avatar with optional centered initial. Forces the same frame+cornerRadius=size/2+flex-' +
      'centering pattern as add_activity_ring_v0 — NEVER the ellipse+sibling text anti-pattern ' +
      'documented in layout.md. Default size 40 (inline). For larger profile avatars pass size: 56 ' +
      'or 96. Without `initial`, emits an empty circle ready for an image child via batch_design. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        initial: {
          type: 'string',
          description: 'Single character / short initial (e.g. "A", "JD"). Omit for empty circle.',
        },
        size: { type: 'number', description: 'Diameter in px (default 40)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: [],
    },
  },
  {
    name: 'add_text_button_v0',
    description:
      'Padding-based text button matching the Pencil demo pattern (documented in memory): ' +
      'frame(padding=[12,20], horizontal, alignItems=center, justifyContent=center, cornerRadius=8) ' +
      '+ optional leading icon + label text. Height auto-derives from padding + text; NO explicit ' +
      'fixed height. Use for primary / secondary buttons with a short text label. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        label: { type: 'string', description: 'Button text (1-3 words recommended)' },
        leading_icon: {
          type: 'string',
          description: 'Optional leading lucide icon name (e.g. "plus", "arrow-right")',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['label'],
    },
  },
  {
    name: 'add_heading_v0',
    description:
      'Typographic heading with Pencil-demo-derived presets + AUTO CJK script handling. ' +
      'Latin (default) per-level: display=48/700/1.0/letterSpacing=-0.5, h1=32/700/1.1, ' +
      'h2=24/600/1.2 (DEFAULT), h3=20/600/1.25. CJK content switches to: display=48/700/1.3 + ' +
      'NO negative letterSpacing, h1=32/700/1.3, h2=24/600/1.35, h3=20/600/1.4 + script-specific ' +
      'fontFamily (Chinese → Noto Sans SC / Japanese → Noto Sans JP / Korean → Noto Sans KR — ' +
      'text-rules.md: NEVER use SC for JP or KR). Encodes the preset so non-Claude models cannot ' +
      'forget lineHeight (default 1.5 stacks multi-word headings tight) or pick the wrong font for ' +
      'JP/KR content. Emits a single text node with role=heading. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        content: { type: 'string', description: 'Heading text' },
        level: {
          type: 'string',
          enum: ['display', 'h1', 'h2', 'h3'],
          description: 'Typography level. Default: h2.',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['content'],
    },
  },
  {
    name: 'add_body_text_v0',
    description:
      'Body / description text. fontFamily ALWAYS Inter (per text-rules.md rule "body=Inter" — ' +
      'Inter uses system CJK fallback, so script-specific Noto faces apply ONLY to headings, ' +
      'not body). Script is auto-detected just to set the right lineHeight + letterSpacing: ' +
      'CJK (Chinese / Japanese / Korean) body gets lineHeight=1.6 + letterSpacing=0 (NEVER ' +
      'negative — causes CJK character overlap); Latin body gets lineHeight=1.5 + no letterSpacing ' +
      'override. Always sets width=fill_container + textGrowth=fixed-width so long text wraps ' +
      '(overflow.md: fixed-width required for >15-char text to wrap). Intended for VERTICAL-layout ' +
      'parents only. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        content: { type: 'string', description: 'Body text content (single paragraph)' },
        parent_id: {
          type: 'string',
          description:
            'Target parent node id (must exist; MUST be a vertical-layout frame). Omit for root insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['content'],
    },
  },
  {
    name: 'add_icon_label_v0',
    description:
      'Atomic icon + label pair (horizontal, alignItems=center, gap=8). Common building block ' +
      'for menu items, breadcrumbs, status indicators, any "icon with text" inline composition. ' +
      'Narrow schema: icons always lead; size/weight fixed (icon 16×16, text 14/500). ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        icon: { type: 'string', description: 'lucide icon name' },
        label: { type: 'string', description: 'Text label' },
        gap: { type: 'number', description: 'Gap between icon and label (default 8)' },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['icon', 'label'],
    },
  },
  {
    name: 'add_search_bar_v0',
    description:
      'Rounded search bar (height=44, cornerRadius=22, width=fill_container) matching the ' +
      'search-bar role spec in ROLE_GUIDE. Leading icon default "search" — override for ' +
      'custom affordance (e.g. "filter", "map-pin"). Placeholder text default "Search...". ' +
      'Use when spec shows a standalone search input, header search, or list filter bar. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        placeholder: {
          type: 'string',
          description: 'Placeholder text (default "Search...")',
        },
        leading_icon: {
          type: 'string',
          description: 'lucide icon name (default "search")',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: [],
    },
  },
  {
    name: 'add_form_field_v0',
    description:
      'Label + input vertical pair. Enforces FORMS rule from DESIGN_GUIDELINES ("ALL inputs ' +
      'MUST use width=fill_container, vertical layout, gap=16-20") and ROLE_GUIDE input specs ' +
      '(height=48, padding=[12,16]). Affordance icons: leading_icon for email/search leads, ' +
      'trailing_icon for password-toggle. required=true appends " *" to the label. Intended ' +
      'for use inside a vertical form container with its siblings at gap=16-20. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        label: { type: 'string', description: 'Field label text' },
        placeholder: { type: 'string', description: 'Optional input placeholder' },
        leading_icon: {
          type: 'string',
          description: 'Optional leading icon (e.g. "mail" for email, "search" for search)',
        },
        trailing_icon: {
          type: 'string',
          description: 'Optional trailing icon (e.g. "eye" for password toggle)',
        },
        required: {
          type: 'boolean',
          description: 'When true, appends " *" to the label text',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['label'],
    },
  },
  {
    name: 'add_list_row_v0',
    description:
      'iOS / Material-style list row: optional leading icon + center text stack (title + ' +
      'optional subtitle) + optional trailing icon (typically chevron-right). The middle text ' +
      'stack is wrapped in a VERTICAL container with width=fill_container — long titles wrap ' +
      'vertically and the row grows height-wise WITHOUT pushing the trailing icon out of frame ' +
      '(same pattern as add_section_header_v0; per overflow.md: text with fill_container + ' +
      'fixed-width only works inside vertical-layout parents). schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description: 'Schema version (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: { type: 'string', description: 'Path to .op file, or omit for live canvas' },
        title: { type: 'string', description: 'Main row title (bold, 15/500)' },
        subtitle: {
          type: 'string',
          description: 'Optional secondary text (smaller, 13/400) shown below title',
        },
        leading_icon: {
          type: 'string',
          description: 'Optional 24×24 lucide icon before the text stack',
        },
        trailing_icon: {
          type: 'string',
          description: 'Optional 16×16 lucide icon after the text stack (e.g. "chevron-right")',
        },
        parent_id: {
          type: 'string',
          description: 'Target parent node id (must exist). Omit for root-level insertion.',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['title'],
    },
  },
  ...ELEMENT_TOOL_DEFINITIONS_EXT,
];

export const ELEMENT_TOOL_NAMES: ReadonlySet<string> = new Set(
  ELEMENT_TOOL_DEFINITIONS.map((t) => t.name),
);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleElementToolCall(name: string, a: any): Promise<string> {
  switch (name) {
    case 'add_card_row_v0':
      return JSON.stringify(await handleAddCardRowV0(a), null, 2);
    case 'add_metric_row_v0':
      return JSON.stringify(await handleAddMetricRowV0(a), null, 2);
    case 'add_nav_chip_row_v0':
      return JSON.stringify(await handleAddNavChipRowV0(a), null, 2);
    case 'add_bottom_nav_v0':
      return JSON.stringify(await handleAddBottomNavV0(a), null, 2);
    case 'add_activity_ring_v0':
      return JSON.stringify(await handleAddActivityRingV0(a), null, 2);
    case 'add_stat_grid_v0':
      return JSON.stringify(await handleAddStatGridV0(a), null, 2);
    case 'add_section_header_v0':
      return JSON.stringify(await handleAddSectionHeaderV0(a), null, 2);
    case 'add_top_nav_bar_v0':
      return JSON.stringify(await handleAddTopNavBarV0(a), null, 2);
    case 'add_icon_button_v0':
      return JSON.stringify(await handleAddIconButtonV0(a), null, 2);
    case 'add_divider_v0':
      return JSON.stringify(await handleAddDividerV0(a), null, 2);
    case 'add_badge_v0':
      return JSON.stringify(await handleAddBadgeV0(a), null, 2);
    case 'add_avatar_v0':
      return JSON.stringify(await handleAddAvatarV0(a), null, 2);
    case 'add_text_button_v0':
      return JSON.stringify(await handleAddTextButtonV0(a), null, 2);
    case 'add_heading_v0':
      return JSON.stringify(await handleAddHeadingV0(a), null, 2);
    case 'add_body_text_v0':
      return JSON.stringify(await handleAddBodyTextV0(a), null, 2);
    case 'add_icon_label_v0':
      return JSON.stringify(await handleAddIconLabelV0(a), null, 2);
    case 'add_list_row_v0':
      return JSON.stringify(await handleAddListRowV0(a), null, 2);
    case 'add_search_bar_v0':
      return JSON.stringify(await handleAddSearchBarV0(a), null, 2);
    case 'add_form_field_v0':
      return JSON.stringify(await handleAddFormFieldV0(a), null, 2);
    case 'add_switch_v0':
      return JSON.stringify(await handleAddSwitchV0(a), null, 2);
    case 'add_checkbox_v0':
      return JSON.stringify(await handleAddCheckboxV0(a), null, 2);
    case 'add_radio_v0':
      return JSON.stringify(await handleAddRadioV0(a), null, 2);
    case 'add_tabs_v0':
      return JSON.stringify(await handleAddTabsV0(a), null, 2);
    case 'add_segmented_control_v0':
      return JSON.stringify(await handleAddSegmentedControlV0(a), null, 2);
    case 'add_empty_state_v0':
      return JSON.stringify(await handleAddEmptyStateV0(a), null, 2);
    default:
      return '';
  }
}
