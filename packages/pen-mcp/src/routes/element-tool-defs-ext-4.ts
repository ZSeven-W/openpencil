// Extension tool definitions — shard 4 of 4 (siblings: base / ext /
// ext-2 / ext-3). Houses tools shipped 2026-04-27 to push the family
// past 80. Each shard caps at the repo's 800-line ceiling.
//
// When adding a new tool: pick whichever shard has the fewest tools
// to keep the split balanced. Run `wc -l` on all four files before
// committing.

import {
  schemaVersionProp,
  filePathProp,
  parentIdProp,
  pageIdProp,
} from './element-tool-def-props';

export const ELEMENT_TOOL_DEFINITIONS_EXT_4 = [
  {
    name: 'add_user_card_v0',
    description:
      'Profile / contact card — circular avatar + (name + optional role) text stack laid out ' +
      'horizontally. Distinct from add_comment_v0 (carries a message body + timestamp) and ' +
      'add_avatar_v0 (just the disk). Use for "team member tile", "contact card", "people picker ' +
      'row", "用户卡片", "联系人卡片". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        name: { type: 'string', description: 'Display name (e.g. "Sarah Lee").' },
        role: {
          type: 'string',
          description: 'Optional secondary line (e.g. "Senior Engineer", "@sarah").',
        },
        initial: { type: 'string', description: 'Optional avatar initial (1-2 chars).' },
        avatar_size: {
          type: 'number',
          description: 'Avatar diameter. Default 48. Clamped 32..96.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['name'],
    },
  },
  {
    name: 'add_drawer_shell_v0',
    description:
      'Slide-in drawer shell — full-height side panel for edit / detail surfaces. Emits the ' +
      'drawer card with header (title + close ×); body content composed via subsequent calls ' +
      'under the drawer id. Distinct from add_modal_shell_v0 (centered card with scrim) and ' +
      'add_action_menu_v0 (compact dropdown). Side encoded in role hint (drawer-shell-right / ' +
      'drawer-shell-left). Use for "side panel", "edit drawer", "detail panel", "抽屉", "侧滑面板". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Drawer header title.' },
        side: {
          type: 'string',
          enum: ['right', 'left'],
          description: 'Side the drawer slides from. Default "right".',
        },
        width: {
          type: 'number',
          description: 'Drawer width. Default 400. Clamped 280..640.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_combobox_v0',
    description:
      'Combobox / autocomplete in the OPEN state — input field with a dropdown of suggestion ' +
      'rows below. Distinct from add_select_v0 (CLOSED state — value + chevron only) and ' +
      'add_search_bar_v0 (no suggestion list). Use for "autocomplete", "command palette", ' +
      '"open country picker", "open state dropdown", "自动补全". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        placeholder: {
          type: 'string',
          description: 'Placeholder shown when value is empty. Default "Search...".',
        },
        value: { type: 'string', description: 'Pre-filled query text.' },
        options: {
          type: 'array',
          description: 'Suggestion rows shown in the open dropdown.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Suggestion label.' },
              highlighted: {
                type: 'boolean',
                description: 'Mark as the currently highlighted row (slate-100 bg).',
              },
            },
            required: ['label'],
          },
        },
        label: { type: 'string', description: 'Optional label above the field.' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['options'],
    },
  },
  {
    name: 'add_toolbar_v0',
    description:
      'Desktop toolbar — horizontal row of 36×36 icon buttons with optional vertical dividers ' +
      'between groups. Distinct from add_top_nav_bar_v0 (mobile single-row header with title) ' +
      'and add_icon_button_v0 (single 44×44 button, no row). Use for "editor toolbar", ' +
      '"formatting controls", "kanban actions", "工具栏". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        items: {
          type: 'array',
          description: 'Toolbar buttons in left-to-right order.',
          items: {
            type: 'object',
            properties: {
              icon: { type: 'string', description: 'Lucide icon slug.' },
              active: {
                type: 'boolean',
                description: 'Mark this button as the active state (slate-100 bg).',
              },
              divider_after: {
                type: 'boolean',
                description: 'Render a vertical divider AFTER this item (group separator).',
              },
            },
            required: ['icon'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['items'],
    },
  },
  {
    name: 'add_callout_v0',
    description:
      'Inline doc callout — standalone tinted block carrying a heading + body paragraph. ' +
      'Distinct from add_alert_v0 (close-able banner with × icon, single-line message) and ' +
      'add_tooltip_v0 (small floating dark pill). tone enum picks the bg + fg + leading icon: ' +
      'info / success / warning / danger / note (default). Use for "doc callout", "tip block", ' +
      '"onboarding note", "did you know", "提示框", "信息块". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        body: { type: 'string', description: 'Body text (paragraph).' },
        title: { type: 'string', description: 'Optional bold heading line above the body.' },
        tone: {
          type: 'string',
          enum: ['info', 'success', 'warning', 'danger', 'note'],
          description: 'Color tone. Default "note" (slate).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['body'],
    },
  },
  {
    name: 'add_share_row_v0',
    description:
      'Social-share button row — horizontal list of 40×40 circular icon buttons each labeled ' +
      'below. Distinct from add_social_login_row_v0 (sign-in CTAs with provider names + ' +
      '"Continue with X"). Use for "share to social", "send via", "post share", "分享按钮组". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        targets: {
          type: 'array',
          description: 'Share destinations in left-to-right order.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string', description: 'Display label (e.g. "Twitter").' },
              icon: { type: 'string', description: 'Lucide icon slug.' },
            },
            required: ['label', 'icon'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['targets'],
    },
  },
  {
    name: 'add_inline_action_v0',
    description:
      'Inline status + action row — message text on the left, blue text button on the right ' +
      '(e.g. "Comment deleted • Undo"). Distinct from add_toast_v0 (floating pill, NOT inline) ' +
      'and add_alert_v0 (full-width banner with dismiss ×). Use for "Comment deleted • Undo", ' +
      '"Saved offline • Retry", inline micro-feedback. schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        message: { type: 'string', description: 'Status / context message.' },
        action_label: { type: 'string', description: 'Action label (e.g. "Undo").' },
        icon: { type: 'string', description: 'Optional leading lucide icon slug.' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['message', 'action_label'],
    },
  },
  {
    name: 'add_legend_item_v0',
    description:
      'Chart legend entry — colored marker + label + optional right-aligned value. Pair multiple ' +
      'under a vertical or horizontal parent for a full legend. Distinct from add_status_badge_v0 ' +
      '(semantic "● Online" with fixed tone enum) and add_color_swatch_v0 (big square + token ' +
      'label, design-system context). Use for "chart legend", "数据图例". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        label: { type: 'string', description: 'Series label (e.g. "Revenue").' },
        color: { type: 'string', description: 'Marker fill hex (e.g. "#2563EB").' },
        value: { type: 'string', description: 'Optional right-aligned value (e.g. "$12,480").' },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['label', 'color'],
    },
  },
  {
    name: 'add_inbox_message_v0',
    description:
      'Inbox / email list row — sender + (subject inline with timestamp) stacked over an ' +
      'optional preview line, with optional unread blue dot. Distinct from add_notification_row_v0 ' +
      '(single title + body, no separate sender field) and add_list_row_v0 (title/subtitle + ' +
      'chevron). Use for "email row", "inbox item", "message list cell", "邮件条目", "收件箱". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        from: { type: 'string', description: 'Sender name (e.g. "Stripe").' },
        subject: { type: 'string', description: 'Email subject line.' },
        preview: { type: 'string', description: 'Optional body preview (one line).' },
        timestamp: { type: 'string', description: 'Time / date label (e.g. "10:42 AM").' },
        unread: {
          type: 'boolean',
          description: 'When true, bolder typography + leading blue unread dot.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['from', 'subject'],
    },
  },
  {
    name: 'add_profile_header_v0',
    description:
      'Large profile header — centered avatar + display name + optional handle / bio. Distinct ' +
      'from add_user_card_v0 (compact horizontal row for lists / tiles) and add_avatar_v0 (just ' +
      'the disk). Use for "profile page hero", "About me block", "account header", "个人主页头部". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        name: { type: 'string', description: 'Display name.' },
        handle: { type: 'string', description: 'Optional handle (e.g. "@sarah").' },
        bio: { type: 'string', description: 'Optional bio / role line.' },
        initial: { type: 'string', description: 'Optional avatar initial (1-2 chars).' },
        avatar_size: {
          type: 'number',
          description: 'Avatar diameter. Default 96. Clamped 64..160.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['name'],
    },
  },
  {
    name: 'add_setting_row_v0',
    description:
      'Settings menu row — leading icon + (title over optional subtitle) + trailing control ' +
      '(chevron / value text / switch / badge). Distinct from add_list_row_v0 (generic, trailing ' +
      'is always an icon — no switch/value/badge variants) and add_form_field_v0 (label-above-' +
      'input pattern for forms, not menus). Use for "settings row", "preferences row", "menu ' +
      'item with toggle", "设置项", "偏好项". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Title (left, 15/500).' },
        subtitle: { type: 'string', description: 'Optional second-line text (13/400, muted).' },
        leading_icon: { type: 'string', description: 'Optional leading lucide icon slug (24×24).' },
        trailing: {
          type: 'object',
          description:
            'Trailing control. Default: { kind: "chevron" }. Variants: ' +
            '{ kind: "chevron" } | { kind: "value", value: string } | ' +
            '{ kind: "switch", on: boolean } | { kind: "badge", value: string }.',
          properties: {
            kind: {
              type: 'string',
              enum: ['chevron', 'value', 'switch', 'badge'],
              description: 'Trailing variant.',
            },
            value: {
              type: 'string',
              description: 'Required for kind="value" and kind="badge".',
            },
            on: {
              type: 'boolean',
              description: 'Required for kind="switch".',
            },
          },
          required: ['kind'],
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title'],
    },
  },
  {
    name: 'add_member_row_v0',
    description:
      'Team / member list row — circular avatar + (name over optional subtitle) + optional ' +
      'trailing (role badge / kebab menu / status dot). Distinct from add_user_card_v0 (compact ' +
      'fit_content tile, no trailing slot — used in lobbies / chip-style picker lists) and ' +
      'add_list_row_v0 (no avatar slot). Use for "members list", "team page row", "people list", ' +
      '"sharing dialog row", "团队成员", "成员列表行". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        name: { type: 'string', description: 'Display name (e.g. "Sarah Lee").' },
        subtitle: {
          type: 'string',
          description:
            'Optional second line — caller picks email / role / status text (e.g. "sarah@acme.com", "Designer • Owner").',
        },
        initial: {
          type: 'string',
          description: 'Optional avatar initial (1-2 chars). Defaults to first char of name.',
        },
        avatar_color: {
          type: 'string',
          description: 'Avatar background hex (e.g. "#3B82F6"). Default "#3B82F6".',
        },
        trailing: {
          type: 'object',
          description:
            'Trailing slot. Default: none. Variants: ' +
            '{ kind: "role_badge", value: string } | { kind: "menu" } | ' +
            '{ kind: "status_dot", tone?: "online"|"busy"|"away"|"offline" }.',
          properties: {
            kind: {
              type: 'string',
              enum: ['role_badge', 'menu', 'status_dot'],
              description: 'Trailing variant.',
            },
            value: { type: 'string', description: 'Required for kind="role_badge".' },
            tone: {
              type: 'string',
              enum: ['online', 'busy', 'away', 'offline'],
              description: 'Optional for kind="status_dot". Default "online".',
            },
          },
          required: ['kind'],
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['name'],
    },
  },
  {
    name: 'add_filter_group_v0',
    description:
      'Sidebar filter group / facet — heading + vertical list of checkbox-style options ' +
      '(label + optional count). Distinct from add_nav_chip_row_v0 (horizontal scrolling chips), ' +
      'add_tag_v0 (single applied chip with × close), and add_segmented_control_v0 (mutex pill ' +
      'tabs). Use for "facet panel", "filter sidebar", "category checklist", "brand filter", ' +
      '"搜索筛选侧栏". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        title: { type: 'string', description: 'Heading text (e.g. "Category", "Brand").' },
        options: {
          type: 'array',
          description: 'Option rows. Each: { label: string, count?: number, selected?: boolean }.',
          items: {
            type: 'object',
            properties: {
              label: { type: 'string' },
              count: { type: 'number' },
              selected: { type: 'boolean' },
            },
            required: ['label'],
          },
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['title', 'options'],
    },
  },
  {
    name: 'add_invite_row_v0',
    description:
      'Pending invite list row — initial avatar + (email over optional role) + status pill ' +
      '(pending / expired / accepted) + trailing action label (e.g. "Resend"). Distinct from ' +
      'add_member_row_v0 (a JOINED member with full name + role badge) and add_list_row_v0 (no ' +
      'avatar / status / action slots). Use for "pending invites list", "invitations table row", ' +
      '"team invite row", "邀请列表行", "待接受邀请". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        email: { type: 'string', description: 'Invitee email.' },
        role: { type: 'string', description: 'Optional invited role (e.g. "Editor").' },
        status: {
          type: 'string',
          enum: ['pending', 'expired', 'accepted'],
          description: 'Status pill. Default "pending".',
        },
        action_label: {
          type: 'string',
          description: 'Trailing action label (e.g. "Resend", "Revoke"). Default "Resend".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['email'],
    },
  },
  {
    name: 'add_activity_log_v0',
    description:
      'Audit / activity feed entry — optional small tinted icon dot + "<actor> <action>" line ' +
      '(actor bold) + right-aligned relative timestamp. Stack vertically under a heading to form ' +
      'a feed. Distinct from add_timeline_v0 (multi-event vertical with connecting line + per-' +
      'event title + body) and add_notification_row_v0 (title + body, no actor focus). Use for ' +
      '"audit log row", "activity feed entry", "recent activity item", "审计日志条目", "活动记录". ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        actor: { type: 'string', description: 'Actor name (rendered bold).' },
        action: { type: 'string', description: 'Action verb-phrase (plain).' },
        timestamp: { type: 'string', description: 'Relative time (e.g. "2h ago").' },
        icon: { type: 'string', description: 'Optional lucide icon slug for the leading dot.' },
        tone: {
          type: 'string',
          enum: ['info', 'success', 'warning', 'danger', 'neutral'],
          description: 'Icon dot tone. Default "info".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['actor', 'action', 'timestamp'],
    },
  },
  {
    name: 'add_event_card_v0',
    description:
      'Calendar event tile — left date column (month band + day number) + right text stack ' +
      '(title + optional time + optional location). Distinct from add_calendar_grid_v0 (full ' +
      'month grid) and add_card_row_v0 (horizontally scrolling cards with title + subtitle + ' +
      'image, no date column). Use for "upcoming event", "agenda item", "meeting tile", ' +
      '"日程卡片", "会议条目". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        month: { type: 'string', description: 'Short month code (e.g. "OCT", "10月").' },
        day: { type: ['string', 'number'], description: 'Day-of-month (e.g. "15" or 15).' },
        title: { type: 'string', description: 'Event title.' },
        time: { type: 'string', description: 'Optional time string (e.g. "2:00 PM").' },
        location: { type: 'string', description: 'Optional location string.' },
        accent: {
          type: 'string',
          description: 'Accent color hex for the month band. Default "#2563EB".',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['month', 'day', 'title'],
    },
  },
  {
    name: 'add_step_card_v0',
    description:
      'Onboarding / how-it-works step card — numbered circle (or check when completed) + ' +
      '(title over description) body. Stack vertically to form an onboarding column. Distinct ' +
      'from add_stepper_v0 (horizontal progress nav with connectors) and add_faq_item_v0 ' +
      '(collapsible Q&A). Use for "onboarding step", "how-it-works step", "tutorial step card", ' +
      '"setup checklist item", "操作步骤卡片", "教程步骤". schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        number: {
          type: ['string', 'number'],
          description:
            'Short step index rendered inside the 36px circle marker. Keep it 1–3 characters ' +
            '(e.g. 1, "01", "1.1"). Long prose like "Step 1" overflows the marker — put that ' +
            'in `title` instead.',
        },
        title: { type: 'string', description: 'Step title (16/600).' },
        description: { type: 'string', description: 'Description body (14/400, muted).' },
        completed: {
          type: 'boolean',
          description: 'Filled circle + check icon when true. Default false (numbered ring).',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['number', 'title', 'description'],
    },
  },
  {
    name: 'add_heading_v1',
    description:
      'Theme-aware typographic heading (v1) — same typography presets as add_heading_v0 with an ' +
      'additional `theme` param. theme="light" (default): byte-parity with v0 ' +
      '(display=48/700, h1=32/700, h2=24/600, h3=20/600 + CJK detection). ' +
      'theme="dark": dark text fill (#F1F5F9) with palette-scale sizes ' +
      '(display=64, h1=24, h2=20, h3=16 from semantic palette tokens). ' +
      'theme="system": emits $type-* size/weight/lineHeight refs + $color-text-primary fill ref; ' +
      'resolves at paint time via doc.variables (requires applySemanticPalette). ' +
      'CJK detection applies in all modes — script-specific fontFamily is always a concrete string. ' +
      'Prefer v1 with theme="system" for design-system-aware designs; use v0 for byte-frozen output. ' +
      'schemaVersion 1.0',
    inputSchema: {
      type: 'object' as const,
      properties: {
        schemaVersion: schemaVersionProp,
        filePath: filePathProp,
        content: { type: 'string', description: 'Heading text' },
        level: {
          type: 'string',
          enum: ['display', 'h1', 'h2', 'h3'],
          description: 'Typography level. Default: h2.',
        },
        theme: {
          type: 'string',
          enum: ['light', 'dark', 'system'],
          description:
            'Theme variant. "light" (default) = v0 byte-parity. "dark" = dark hex fill. ' +
            '"system" = emits $type-*/​$color-* refs; requires applySemanticPalette(doc) seeded.',
        },
        parent_id: parentIdProp,
        pageId: pageIdProp,
      },
      required: ['content'],
    },
  },
];
