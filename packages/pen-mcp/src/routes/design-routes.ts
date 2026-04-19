import { handleBatchDesign } from '../tools/batch-design';
import { buildDesignPrompt, listPromptSections } from '../tools/design-prompt';
import { handleDesignSkeleton } from '../tools/design-skeleton';
import { handleDesignContent } from '../tools/design-content';
import { handleDesignRefine } from '../tools/design-refine';
import { LAYERED_DESIGN_TOOLS } from '../tools/layered-design-defs';
import { handleAddSectionV0 } from '../tools/add-section-v0';
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

export const DESIGN_TOOL_DEFINITIONS = [
  {
    name: 'get_design_prompt',
    description:
      'Get design knowledge prompt. Use "section" to retrieve a focused subset instead of the full prompt. ' +
      'Sections: schema (PenNode types), layout (flexbox rules), roles (semantic roles), text (typography/CJK/copywriting), ' +
      'style (visual style policy), icons (icon names), examples (design examples), guidelines (design tips), ' +
      'planning (layered workflow guide), elements (N-tool element-tool family reference — decision tree, ' +
      'PREFER/FALLBACK rules, composition pattern; the section itself enumerates the current tools). ' +
      'Omit section for the full prompt.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        section: {
          type: 'string',
          enum: [
            'all',
            'schema',
            'layout',
            'roles',
            'text',
            'style',
            'icons',
            'examples',
            'guidelines',
            'planning',
            'elements',
          ],
          description:
            'Which section of design knowledge to retrieve. Default: all. Use "planning" for layered generation workflow; "elements" for N-tool element tool reference.',
        },
      },
      required: [],
    },
  },
  {
    name: 'batch_design',
    description:
      'Execute batch design operations in a compact DSL. Each line is one operation:\n' +
      '  binding=I(parent, { ...nodeData })  — Insert node (binding captures new ID)\n' +
      '  U(path, { ...updates })             — Update node properties\n' +
      '  binding=C(sourceId, parent, { overrides })  — Copy node\n' +
      '  binding=R(path, { ...newNodeData }) — Replace node\n' +
      '  M(nodeId, parent, index?)           — Move node\n' +
      '  D(nodeId)                           — Delete node (use batch_get to find IDs first)\n' +
      'Use null for root-level parent. Reference previous bindings by name. ' +
      'Path expressions support binding+"/ childId" for nested access. ' +
      'Always set postProcess=true when generating designs for best visual quality.',
    inputSchema: {
      type: 'object' as const,
      properties: {
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        operations: {
          type: 'string',
          description:
            'DSL operations, one per line. Example:\nroot=I(null, { "type": "frame", "name": "Page", "width": 1200, "height": 0, "layout": "vertical", "children": [...] })',
        },
        postProcess: {
          type: 'boolean',
          description:
            'Apply post-processing (role defaults, icon resolution, layout sanitization). Always true for design generation.',
        },
        canvasWidth: {
          type: 'number',
          description: 'Canvas width for post-processing (default 1200, use 375 for mobile).',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['operations'],
    },
  },
  ...LAYERED_DESIGN_TOOLS,
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
      'Typographic heading with Pencil-demo-derived fontSize / fontWeight / lineHeight / ' +
      'letterSpacing presets per level. display=48/700/1.0/-0.5, h1=32/700/1.1, h2=24/600/1.2 ' +
      '(DEFAULT), h3=20/600/1.25. Encodes the preset so non-Claude models cannot forget the ' +
      'correct lineHeight (1.5 default stacks multi-word headings too tight, per memory Pencil ' +
      'demo data). Emits a single text node with role=heading. schemaVersion 1.0',
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
      'Body / description text with AUTO CJK detection: Chinese/Japanese/Korean content gets ' +
      'fontFamily=Noto Sans SC + lineHeight=1.6 + letterSpacing=0 (per memory: NEVER use Space ' +
      'Grotesk / Manrope for CJK — no CJK glyphs); Latin gets Inter + 1.5. Always sets ' +
      'width=fill_container + textGrowth=fixed-width so long text wraps (overflow.md: fixed-width ' +
      'is required or >15-char text does not wrap). Intended for use in VERTICAL-layout parents — ' +
      'the only context where fill_container + fixed-width is respected by the layout engine. ' +
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
];

export const DESIGN_TOOL_NAMES = new Set([
  'get_design_prompt',
  'batch_design',
  'design_skeleton',
  'design_content',
  'design_refine',
  'add_card_row_v0',
  'add_metric_row_v0',
  'add_nav_chip_row_v0',
  'add_bottom_nav_v0',
  'add_activity_ring_v0',
  'add_stat_grid_v0',
  'add_section_header_v0',
  'add_top_nav_bar_v0',
  'add_icon_button_v0',
  'add_divider_v0',
  'add_badge_v0',
  'add_avatar_v0',
  'add_text_button_v0',
  'add_heading_v0',
  'add_body_text_v0',
]);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleDesignToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  const a = args as any; // eslint-disable-line @typescript-eslint/no-explicit-any
  switch (name) {
    case 'get_design_prompt':
      return JSON.stringify(
        {
          section: (a.section as string | undefined) ?? 'all',
          availableSections: listPromptSections(),
          designPrompt: buildDesignPrompt(a.section as string | undefined),
        },
        null,
        2,
      );
    case 'batch_design':
      return JSON.stringify(await handleBatchDesign(a), null, 2);
    case 'design_skeleton':
      return JSON.stringify(await handleDesignSkeleton(a), null, 2);
    case 'design_content':
      return JSON.stringify(await handleDesignContent(a), null, 2);
    case 'design_refine':
      return JSON.stringify(await handleDesignRefine(a), null, 2);
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
    default:
      return '';
  }
}

// --- D0 parity spike tool (gated behind OPENPENCIL_D0_SPIKE=1) ---
//
// add_section_v0 是一次性 parity 验证工具，不进生产工具集。与 debug tools
// 同样走环境变量条件暴露模式（见 server.ts 里对 DEBUG_TOOL_DEFINITIONS 的处理）。
// 默认不出现在 ListTools 响应里，避免外部 MCP 客户端误用。
// Spec: openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0

export const D0_SPIKE_TOOL_DEFINITIONS = [
  {
    name: 'add_section_v0',
    description:
      'D0 parity spike tool (not for production, gated by OPENPENCIL_D0_SPIKE=1). ' +
      'Insert one section frame with minimal schema: title (becomes frame.name) + ' +
      'layout (horizontal|vertical). Validates N-tool additive-only hypothesis. ' +
      'See spec openpencil-docs/superpowers/specs/2026-04-19-element-tools-v0.md §D0. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: {
          type: 'string',
          enum: ['1.0'],
          description:
            'Schema version this tool was authored against (v0-MUST §4.2). Clients MAY omit.',
        },
        filePath: {
          type: 'string',
          description: 'Path to .op file, or omit to use the live canvas (default)',
        },
        title: { type: 'string', description: 'Section name (becomes frame.name)' },
        layout: {
          type: 'string',
          enum: ['horizontal', 'vertical'],
          description: 'Flex direction',
        },
        pageId: { type: 'string', description: 'Target page ID (defaults to first page)' },
      },
      required: ['title', 'layout'],
    },
  },
];

export const D0_SPIKE_TOOL_NAMES = new Set(['add_section_v0']);

export async function handleD0SpikeToolCall(
  name: string,
  args: Record<string, unknown>,
): Promise<string> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const a = args as any;
  switch (name) {
    case 'add_section_v0':
      return JSON.stringify(await handleAddSectionV0(a), null, 2);
    default:
      return '';
  }
}
