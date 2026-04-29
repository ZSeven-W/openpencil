---
name: elements
description: N-tool element family reference — rows, containers, atoms, text/button primitives, composition, forms, controls (switch/checkbox/radio/tabs/segmented), state/feedback (empty_state/alert/toast/progress_bar), floating/nav (fab/breadcrumb/stepper), ratings & pagination (rating_stars/carousel_dots), inline primitives (link/kbd/price), content blocks (quote_block/code_block), design system (color_swatch). Each tool replaces a documented batch_design failure mode
phase: [generation]
trigger:
  flags: [hasMcpTools]
priority: 14
budget: 2400
category: base
---

<!--
  IMPORTANT: This skill is gated by the `hasMcpTools` flag. It only
  auto-loads into the generation-phase prompt when the caller declares
  the AI has live access to MCP element tools (external clients:
  Claude Code / Codex / Gemini CLI / Cursor). The embedded orchestrator
  in apps/web emits single-shot JSON and cannot call MCP tools — this
  skill would be 1500 tokens of dead weight there, so it stays excluded.

  External MCP clients still retrieve the content explicitly via
  get_design_prompt(section='elements'), which bypasses resolveSkills'
  trigger filter (uses getSkillByName for direct lookup).

  To opt-in auto-loading from a new caller: pass `{ flags: { hasMcpTools: true } }`
  to resolveSkills('generation', prompt, opts).
-->

ELEMENT TOOLS (schema-constrained alternatives to batch_design):

These narrow MCP tools emit well-known structures that batch_design frequently gets wrong on non-Claude models (overflow, wrong role, anti-pattern layout). Each is shape-locked — you pick the tool by matching intent, then supply only content.

**MULTI-TOOL OUTPUT IS THE NORM, NOT THE EXCEPTION.** A brief that names more than one component (a settings panel with 4 toggle rows, a team list with 5 members, a feed with 6 log entries, an onboarding screen with 4 step cards) MUST emit ONE `<op_tool>` block per component. The harness reads every `<op_tool>` tag in your output, so chain as many as the brief implies. Fall back to `batch_design` only when at least one component in the brief truly needs a custom shape no element tool covers — and then use a SINGLE batch_design for the WHOLE response, never as a per-component fallback mixed with element-tool calls (the harness drops the batch_design half whenever element calls share the response).

Multi-tool example — a "Notifications" settings section with a header + 4 toggle rows is **5 tool calls** (1× `add_section_header_v0` + 4× `add_setting_row_v0`), NOT 1 batch_design.

**`parent_id` is REAL or OMITTED — never invented.** Every `add_*_v0` tool's `parent_id` arg must either (1) be omitted entirely so the new node lands at the page root (the safe default for full-page composite briefs), or (2) name a real existing node id you already received from a prior tool call. The cookbook recipes below use placeholders like `<page>` / `<panel>` / `<sidebar>` as DOCUMENTATION shorthand only — when YOU emit the call in your output, OMIT the parent_id field entirely.

❌ WRONG — these all crash with `parent_id "X" not found in document` because every id was made up by the model, not handed to you by a prior tool call:

- `add_activity_log_v0({ parent_id: "entry-1", actor: "Sarah", ... })` — "entry-1" was never created
- `add_setting_row_v0({ parent_id: "root", title: "Email" })` — page root has no id
- `add_member_row_v0({ parent_id: "members-section", name: "Sarah" })` — section-\* placeholders are docs, not real ids
- Same trap for any sequential name you might invent: `card-1` / `item-1` / `row-N` / `<page>` / `panel` / `canvas`

✅ RIGHT — omit `parent_id` entirely; each call lands at the page root, which is what every full-page composite brief wants:

- `add_activity_log_v0({ actor: "Sarah", action: "approved deploy", timestamp: "2h ago" })`
- `add_setting_row_v0({ title: "Email notifications", trailing: { kind: "switch", on: true } })`
- `add_member_row_v0({ name: "Sarah Lee", subtitle: "Designer" })`

## Decision tree (per component — pick first match)

Rows (horizontal, in-card or scrolling):

1. Row of items with title + subtitle + optional icon → `add_card_row_v0` (scroll)
   1b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_card_row_v1`
2. Row of items with small label + big numeric value → `add_metric_row_v0` (scroll)
   2b. Theme-aware variant (no hardcoded colors; all modes identical — accepts theme for API consistency) → `add_metric_row_v1`
3. Row of filter chips / category tabs (label + optional icon, active state) → `add_nav_chip_row_v0` (scroll)
   3b. Theme-aware variant (no hardcoded colors; all modes identical — accepts theme for API consistency) → `add_nav_chip_row_v1`
4. Non-scrolling 2-5 stats inline (auto-share width) → `add_stat_grid_v0`
   4b. Theme-aware variant (no explicit fills in v0; all modes identical — accepts theme for API consistency) → `add_stat_grid_v1`

Containers and single elements:

5. Section header (big title + optional "See all" action) → `add_section_header_v0`
   5b. Theme-aware variant (no hardcoded colors; all modes identical — accepts theme for API consistency) → `add_section_header_v1`
6. Bottom tab bar (inline flow, 3-5 nav items) → `add_bottom_nav_v0`
   6b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_bottom_nav_v1`
7. Mobile top bar (leading icon + centered title + trailing icon) → `add_top_nav_bar_v0`
   7b. Theme-aware variant (accepts `theme` param for API consistency; output identical — no hardcoded surface colors) → `add_top_nav_bar_v1`
8. Icon-only button (44×44, hit-target safe) → `add_icon_button_v0`
   8b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_icon_button_v1`
9. Apple-style progress ring with centered text → `add_activity_ring_v0`
   9b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_activity_ring_v1`

Atoms (1-2 node building blocks):

10. Hairline separator between list rows / sections → `add_divider_v0`
    10b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_divider_v1`
11. Short inline pill / tag / "NEW" / "BETA" / count badge → `add_badge_v0`
    11b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_badge_v1`
12. Circular avatar (with optional initial / empty for later image fill) → `add_avatar_v0`
    12b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_avatar_v1`

Text + button primitives:

13. Padding-based button with text (optional leading icon) → `add_text_button_v0`
    13b. Theme-aware variant (accepts `theme` param for API consistency; output identical — no hardcoded surface colors) → `add_text_button_v1`
14. Heading with enforced fontSize/lineHeight per level + AUTO CJK script detection (SC/JP/KR) → `add_heading_v0`
    14b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_heading_v1`
15. Body text (Inter everywhere — CJK gets lineHeight 1.6 + letterSpacing 0, Latin 1.5) → `add_body_text_v0`
    15b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_body_text_v1`

Composition:

16. Icon + text inline pair (menu items, breadcrumbs, status indicators) → `add_icon_label_v0`
    16b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_icon_label_v1`
17. iOS/Material list row (leading icon + title/subtitle stack + trailing icon) → `add_list_row_v0`
    17b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_list_row_v1`

Forms:

18. Search bar (height=44, cornerRadius=22, leading search icon) → `add_search_bar_v0`
    18b. Theme-aware variant (dark/system adds surface2 fill so bar is visible on dark bg) → `add_search_bar_v1`
19. Form field (label + 48px input with optional affordance icons) → `add_form_field_v0`
    19b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_form_field_v1`
    19c. Multi-line textarea (label + N-row input for notes / bio / feedback) → `add_textarea_v0`
    19c-v1. Theme-aware textarea (accepts `theme` param for API consistency; output identical — no hardcoded surface colors) → `add_textarea_v1`
    19d. Dropdown select closed-state (label + 48px input w/ value text + chevron-down) → `add_select_v0`
    19d. Theme-aware select (placeholder → textSubtle in dark/system) → `add_select_v1`

Controls (toggle / choice / tabs):

20. iOS/Material toggle switch (51×31, thumb floats) → `add_switch_v0`
    20b. Theme-aware variant (iOS HIG values #34C759/#E5E5EA are builder-private per §3.4; all modes identical) → `add_switch_v1`
21. Checkbox + inline label (20×20 box, `check` icon inside when checked) → `add_checkbox_v0`
    21b. Theme-aware variant (accent fill checked, border token unchecked) → `add_checkbox_v1`
22. Radio button + inline label (20×20 ring, dot inside when selected) → `add_radio_v0`
    22b. Theme-aware variant (accent brand-invariant; unselected ring → border in dark/system) → `add_radio_v1`
23. Horizontal top tabs with underline on active (fontWeight 600 + 2px sibling rectangle underline) → `add_tabs_v0`
    23b. Theme-aware variant (#2563EB underline is brand accent per §3.4; all modes identical) → `add_tabs_v1`
24. iOS pill-style segmented control (equal-width segments, active floats white) → `add_segmented_control_v0`
    24b. Theme-aware variant (track → surface2, active seg → surface, labels → textPrimary/textMuted) → `add_segmented_control_v1`

State / feedback:

25. Empty state (icon + title + optional subtitle + optional CTA button, centered) → `add_empty_state_v0`
    25b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_empty_state_v1`
26. Inline banner / callout (icon + message + optional close x, fill_container) → `add_alert_v0`
    26b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_alert_v1`
27. Floating pill notification (dark fit_content pill) → `add_toast_v0`
28. Linear progress bar (fixed bar_width + value 0-100) → `add_progress_bar_v0`
    28b. Theme-aware variant (track → surface2 in dark/system; accent fill brand-invariant) → `add_progress_bar_v1`

Floating / nav / wizard:

29. Floating action button (circular 56×56, icon centered) → `add_fab_v0`
    29b. Theme-aware variant (bg → accent in dark/system; icon always white) → `add_fab_v1`
30. Breadcrumb trail with chevron separators (last crumb auto-active) → `add_breadcrumb_v0`
    30b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_breadcrumb_v1`
31. Horizontal numbered stepper (circles + fill_container connectors) → `add_stepper_v0`
    31b. Theme-aware variant (pending fill → border; pending number → textMuted; accent + done-white stay hardcoded) → `add_stepper_v1`

Ratings & pagination:

32. N-of-M star rating (filled + empty stars using lucide `star`) → `add_rating_stars_v0`
    32b. Theme-aware variant (no explicit colors; all modes identical — accepts theme for API consistency) → `add_rating_stars_v1`
33. Carousel / slide dots (active becomes elongated pill, inactive circles) → `add_carousel_dots_v0`
    33b. Theme-aware variant (active=text-primary, inactive=border in dark/system modes) → `add_carousel_dots_v1`

Inline text primitives:

34. Text link with optional trailing icon ("Learn more →") → `add_link_v0`
    34b. Theme-aware variant (accepts `theme` param for API consistency; output identical across themes) → `add_link_v1`
35. Keyboard shortcut glyph ("⌘ K" / "Ctrl + Shift + P") → `add_kbd_v0`
    35b. Theme-aware variant (key bg → surface2, stroke → border in dark/system) → `add_kbd_v1`
36. Pricing typography ("$29/month": currency 20/500 + amount 40/700 + period 14/500) → `add_price_v0`
    36b. Theme-aware variant (no hardcoded colors; all modes identical — accepts theme for API consistency) → `add_price_v1`

Content blocks:

37. Quoted passage with optional author attribution → `add_quote_block_v0`
    37b. Theme-aware variant (container bg → surface in dark/system) → `add_quote_block_v1`
38. Preformatted code block (fill_container, wraps, gray-50 bg) → `add_code_block_v0`
    38b. Theme-aware variant (bg → surface2 in dark/system) → `add_code_block_v1`

Design-system:

39. Color swatch (colored square + optional token label) → `add_color_swatch_v0`
    39b. Theme-aware variant (same tree; theme param for API consistency) → `add_color_swatch_v1`

Charts / data visualization:

40. Bar-chart skeleton (one rectangle per value, bottom-aligned) → `add_chart_bars_v0`
    40b. Line-chart skeleton (polyline + optional dots) → `add_chart_line_v0`
    40c. Pie-chart skeleton (colored ellipse slices via arc angles) → `add_chart_pie_v0`
    40d. Theme-aware bar chart (chart-1 token in dark/system) → `add_chart_bars_v1`
    40e. Theme-aware line chart (chart-1 token in dark/system) → `add_chart_line_v1`
    40f. Theme-aware pie chart (chart-1..6 tokens in dark/system) → `add_chart_pie_v1`

Media / placeholder:

44. Image placeholder (gray box + centered icon + optional caption — future image slot) → `add_image_placeholder_v0`
    44c. Theme-aware image placeholder (bg → bgDeep, icon/label → textMuted in dark/system) → `add_image_placeholder_v1`
    44b. Video placeholder (dark box + play icon + optional caption — future video embed) → `add_video_placeholder_v0`
    44b-v1. Theme-aware video placeholder (dark bg/icon/caption are builder-private per §3.4; all modes identical) → `add_video_placeholder_v1`

Social / UGC:

45. Comment row (circular avatar + author/timestamp header + body) → `add_comment_v0`
    45b. Theme-aware variant (avatar bg → surface2, timestamp → textMuted in dark/system) → `add_comment_v1`

Chrome / modals:

46. Modal dialog shell (dimmed backdrop + centered card + title — body composed separately) → `add_modal_shell_v0`
    46b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_modal_shell_v1`

Status / presence:

47. Status badge (small colored dot + short label, "● Online" pattern, tone-enum'd) → `add_status_badge_v0`
    47b. Theme-aware variant (tone colors are status semantics kept hardcoded; all modes identical) → `add_status_badge_v1`

Feedback / loading:

48. Loading spinner (static ring + 3/4 arc) → `add_spinner_v0`
    48b. Theme-aware variant (track_color/active_color are caller params; accepts theme for API consistency; all modes identical) → `add_spinner_v1`
49. Tooltip pill (dark pill + white text, hover-hint appearance) → `add_tooltip_v0`
    49b. Theme-aware variant (dark body #111827 is intentional inverted-contrast per §3.4; all modes identical) → `add_tooltip_v1`

Analytics / KPIs:

50. KPI cell with trend arrow + change ("$12k ↑ 8%") → `add_metric_comparison_v0`
    50b. Theme-aware variant (label → textMuted; trend up → success, down → destructive, flat → textMuted) → `add_metric_comparison_v1`

Notifications:

51. Notification list row (icon + title/timestamp header + optional body preview + optional unread dot) → `add_notification_row_v0`
    51b. Theme-aware variant (timestamp → textSubtle, body → textBody, unread dot → destructive) → `add_notification_row_v1`

Activity / history:

41. Vertical timeline (dots + fixed 24px connectors + content; no padding/gap) → `add_timeline_v0`
    41b. Theme-aware variant (#2563EB active dot hardcoded per §3.4; inactive dot/connector → border, subtitle → textMuted) → `add_timeline_v1`

Calendars:

42. Month calendar grid (weekday header + 7-col day rows, today/selected tint) → `add_calendar_grid_v0`
    42b. Theme-aware variant (header text, day numbers, selected/today fills respond to theme) → `add_calendar_grid_v1`

Loading / placeholder:

43. Loading skeleton (N gray rectangles, last row ~60% width) → `add_skeleton_v0`
    43b. Theme-aware variant (row fill → surface2 in dark/system so bars are visible on dark bg) → `add_skeleton_v1`

Pagination:

52. Pagination bar (numbered pills + prev/next arrows, Google-style ellipses for big ranges) → `add_pagination_v0`
    52b. Theme-aware variant (arrow/inactive → textBody, ellipsis → textMuted; active pill bg invariant, active text stays white) → `add_pagination_v1`

Collapsible content:

53. FAQ / accordion item (question + chevron; expanded variant shows answer paragraph) → `add_faq_item_v0`
    53b. Theme-aware variant (chevron/answer → textMuted, divider → border in dark/system) → `add_faq_item_v1`

Tag / multi-value inputs:

54. Chip input / tag input (pills + removable × + inline caret, wrap layout) → `add_chip_input_v0`
    54b. Theme-aware variant (chip bg → surface2, field bg → surface, placeholder → textSubtle) → `add_chip_input_v1`

Chart empty state:

55. Empty chart placeholder (dashed tile in chart footprint; "No data yet" message) → `add_empty_chart_v0`

Menus / floating panels:

56. Action / context menu panel (dropdown list of icon+label rows, destructive variant supported) → `add_action_menu_v0`
    56b. Theme-aware variant (surface, border, icon, label colors respond to theme) → `add_action_menu_v1`

Dates:

57. Date picker CLOSED state (labeled input + "Jan 15, 2026" + trailing calendar icon) → `add_date_picker_v0`
    57b. Theme-aware variant (input bg → surface, stroke → border in dark/system) → `add_date_picker_v1`

Upload / file intake:

58. File upload dropzone (dashed tile + cloud icon + "Drop files / click to browse") → `add_upload_dropzone_v0`
    58b. Theme-aware variant (bg → bgDeep, dashed border → border, icon → textMuted, title → textBody, subtitle → textMuted) → `add_upload_dropzone_v1`

Auth / verification:

59. OTP / PIN code input (row of N square slots, 4..8 digits; blank / partial / full states) → `add_otp_input_v0`
    59b. Theme-aware variant (slot bg → surface, borders → border/borderStrong, digit → textPrimary; focused border = accent_color invariant) → `add_otp_input_v1`

Attachments:

60. File attachment row (type-icon + filename + optional size + remove ×) → `add_attachment_row_v0`
    60b. Theme-aware variant (surface, text hierarchy, icon colors respond to theme) → `add_attachment_row_v1`

Messaging:

61. Chat message bubble (left=from-others slate bg / right=from-self accent bg; optional author + timestamp) → `add_chat_bubble_v0`
    61b. Theme-aware variant (surface2 bg for left, accent bg for right, textMuted for author/timestamp) → `add_chat_bubble_v1`

Dashboard KPIs:

62. Big-number stat card (standalone metric tile — label + huge value + optional delta/icon) → `add_stat_card_v0`
    62b. Theme-aware variant (bg → surface; border → border; label → textMuted; icon → textSubtle; value → textPrimary; delta tones stay hardcoded) → `add_stat_card_v1`

Auth / login:

63. Social auth provider buttons ("Continue with Google / Apple / Microsoft", OAuth/SSO row, third-party sign-in) → `add_social_login_row_v0`
    63b. Theme-aware variant (button bg → surface; border → border; icon → textMuted; label → textPrimary in dark/system) → `add_social_login_row_v1`

Pricing / monetization:

64. Pricing plan tier card (SaaS pricing table column: tier name + big price + feature list + CTA; emphasize the recommended tier with `emphasis="featured"`) → `add_pricing_card_v0`
    64b. Theme-aware variant (card bg → surface, border → border; featured accent/CTA stay brand-invariant; text → textPrimary/textMuted/textBody) → `add_pricing_card_v1`

Input / forms:

65. Range slider (single-thumb horizontal slider showing current value: volume, opacity, brightness, price range) → `add_range_slider_v0`
    65b. Theme-aware variant (accent brand-invariant; thumb bg → surface; remaining → border; label → textPrimary; value → textMuted) → `add_range_slider_v1`

66. International phone number input with country-code prefix selector → `add_phone_input_v0`
    66b. Theme-aware variant (field bg → surface, stroke/divider → border, label → textBody, code → textPrimary, chevron → textMuted) → `add_phone_input_v1`

67. Input field with inline action button (newsletter signup, apply discount code, send chat message) → `add_input_with_action_v0`
    67b. Theme-aware variant (input bg → surface, stroke → border, text → textPrimary; button bg → accent (brand-invariant), button text → white in all modes) → `add_input_with_action_v1`

Compliance / disclosure:

68. Cookie consent / GDPR / privacy banner (sticky bottom-of-page disclosure card with accept / decline / settings) → `add_cookie_banner_v0`
    68b. Theme-aware variant (card bg → surface, title → textPrimary, accept → accent in dark/system) → `add_cookie_banner_v1`

Desktop / dashboard rails:

69. Persistent vertical sidebar (left rail with icon+label rows, optional brand title, active item gets pill bg) → `add_sidebar_nav_v0`
    69b. Theme-aware variant (bg → surface; title + active label → textPrimary; inactive → textMuted; active item bg → surface2) → `add_sidebar_nav_v1`

Presence / collaboration:

70. Stacked avatar group (team / online users / "+N more" tile, white-ringed circles in a tight horizontal row) → `add_avatar_group_v0`
    70b. Theme-aware variant (ring, overflow bg/text respond to theme; brand avatar palette stays hardcoded) → `add_avatar_group_v1`

Tabular data:

71. Desktop data-table row (N column-aligned cells in a single horizontal row, header / body / selected variants) → `add_data_table_row_v0`
    71b. Theme-aware variant (header text → textMuted, body → textPrimary, selected → bgDeep in dark/system) → `add_data_table_row_v1`

Filter / selection chips:

72. Single closable tag chip (filter / applied criterion / category, optional × close icon, tone enum) → `add_tag_v0`
    72b. Theme-aware variant (tone bg/fg are status semantic colors per §3.4; all modes identical) → `add_tag_v1`

People / profile:

73. Compact user card (avatar + name + optional role line, horizontal row) → `add_user_card_v0`
    73b. Theme-aware variant (name → textPrimary, role → textMuted; avatar bg #3B82F6 + initial white stay brand-invariant per §3.4) → `add_user_card_v1`
74. Large profile header (centered avatar + name + optional handle/bio) → `add_profile_header_v0`
    74b. Theme-aware variant (name → textPrimary, handle → textMuted, bio → textBody; avatar bg #3B82F6 + initial white stay brand-invariant) → `add_profile_header_v1`

Side panels & docking:

75. Slide-in drawer shell (full-height side panel with header) → `add_drawer_shell_v0`
    75b. Theme-aware variant (bg → surface, border → border token, title/icon → semantic tokens in dark/system) → `add_drawer_shell_v1`

Forms (open state) & toolbars:

76. Open-state combobox / autocomplete (input + visible dropdown rows) → `add_combobox_v0`
    76b. Theme-aware variant (surface/border/surface2 tokens in dark/system) → `add_combobox_v1`
77. Desktop toolbar (icon button row with optional dividers) → `add_toolbar_v0`
    77b. Theme-aware variant (surface → surface, border → border, active-bg → surface2, icon fills → textPrimary/textMuted) → `add_toolbar_v1`

Doc / inline feedback:

78. Inline doc callout (tinted block with title + body, tone enum) → `add_callout_v0`
    78b. Theme-aware variant (tone-keyed bg/fg → semantic alert palette in dark/system modes) → `add_callout_v1`
79. Inline status + action ("Comment deleted • Undo") → `add_inline_action_v0`
    79b. Theme-aware variant (icon/message → textBody, CTA → accent in dark/system) → `add_inline_action_v1`

Sharing / chart annotations:

80. Social share button row (circular icon buttons + labels) → `add_share_row_v0`
    80b. Theme-aware variant (icon btn bg → surface2; icon/label → textMuted in dark/system) → `add_share_row_v1`
81. Chart legend entry (color marker + label + optional value) → `add_legend_item_v0`
    81b. Theme-aware variant (label → textBody, value → textPrimary; marker color kept as-is) → `add_legend_item_v1`

Mail / inbox:

82. Inbox / email list row (sender + subject + preview + unread dot) → `add_inbox_message_v0`
    82b. Theme-aware variant (primary → textPrimary, timestamp → textSubtle, preview → textMuted, unread dot → accent in dark/system) → `add_inbox_message_v1`

Settings / preferences:

83. Settings menu row (icon + title + subtitle + trailing chevron / value / switch / badge) → `add_setting_row_v0`
    83b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_setting_row_v1`

People / lists:

84. Members / team list row (avatar + name + subtitle + role badge / kebab menu / status dot) → `add_member_row_v0`
    84b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_member_row_v1`
85. Pending invite list row (initial avatar + email/role + status pill + Resend action) → `add_invite_row_v0`
    85b. Theme-aware variant (avatar bg → surface2, text → textPrimary/textMuted, action → accent; status pills use alertColors in dark/system) → `add_invite_row_v1`

Faceted search / filter sidebar:

86. Sidebar filter group (heading + checkbox-style option list with counts) → `add_filter_group_v0`
    86b. Theme-aware variant (title → textPrimary, label → textBody, box bg/stroke → surface/border in dark/system) → `add_filter_group_v1`

Audit / activity feed:

87. Activity log entry (actor in bold + action + timestamp + tinted icon dot) → `add_activity_log_v0`
    87b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_activity_log_v1`

Calendar:

88. Single event card (month/day column + title + time + location) → `add_event_card_v0`
    88b. Theme-aware variant (card bg/stroke/date column → semantic tokens; accent strip + white text stay brand-invariant) → `add_event_card_v1`

Onboarding:

89. Numbered step card (circle/check + title + description) → `add_step_card_v0`
    89b. Theme-aware variant (title → textPrimary; desc → textMuted; incomplete ring bg → surface; accent stays hardcoded) → `add_step_card_v1`

90. None match → fall through to `batch_design`

**Disambiguation**: if you need a ROW of 3 metrics that should NOT scroll (e.g. a stats strip inside a card), use `add_stat_grid_v0`, NOT `add_metric_row_v0`. The grid uses `fill_container` per cell so it never overflows; the metric row uses fixed-px cells + scroll wrapper.

## When to use vs batch_design

PREFER an element tool when the spec says any of:

- "horizontal scrolling cards", "swipeable row", "chip row", "pills" → `add_card_row_v0`
- "metric tiles", "KPI cards", "dashboard stats" (SCROLLING row) → `add_metric_row_v0`
- "stats row", "3 metrics side by side", "summary bar" (NON-scrolling grid) → `add_stat_grid_v0`
- "dark stats grid", "dark-mode stat grid", "theme-aware stat grid", "暗色指标网格" → `add_stat_grid_v1` (no explicit fills; all modes identical — accepts theme for API consistency).
- "category filter chips", "quick-access shortcuts" → `add_nav_chip_row_v0`
- "section title with See all / View more" → `add_section_header_v0`
- "dark section header", "dark-mode section title", "theme-aware section header", "暗色版块标题" → `add_section_header_v1` (no color change; accepts theme for API consistency)
- "bottom nav", "tab bar", "tabbar", "底部导航" → `add_bottom_nav_v0`
- "top bar", "app bar", "header with back button", "页面标题栏" → `add_top_nav_bar_v0`
- "dark top bar", "dark-mode app bar", "theme-aware top nav", "暗色顶部导航栏" → `add_top_nav_bar_v1` (no hardcoded colors; all modes identical — accepts theme for API consistency).
- "icon-only button", "close button", "menu button" (toolbar-style) → `add_icon_button_v0`
- "activity ring", "progress ring", "circular progress", "Apple health ring" → `add_activity_ring_v0`
- "hairline divider", "separator", "row divider", "section separator" → `add_divider_v0`
- "badge", "pill", "tag", "NEW label", "count bubble" (≤16 Latin / ≤8 CJK chars) → `add_badge_v0`
- "avatar", "profile picture", "user circle", "initial bubble" → `add_avatar_v0`
- "primary button", "secondary button", "CTA", "submit button" (short label) → `add_text_button_v0`
- "dark text button", "dark-mode CTA", "theme-aware text button", "暗色按钮" → `add_text_button_v1` (no hardcoded colors; all modes identical — accepts theme for API consistency).
- "hero headline", "section title", "card title" / 特定字号标题 → `add_heading_v0`
- "dark heading", "dark-mode title", "theme-aware heading", "system theme heading", "暗色标题", "主题感知标题" → `add_heading_v1` (accepts `theme: 'light' | 'dark' | 'system'`; use `"system"` when `applySemanticPalette(doc)` is seeded)
- "dark card row", "dark-mode scrollable cards", "theme-aware card scroll", "暗色卡片行" → `add_card_row_v1` (accepts `theme`; dark surface fill on cards + dark text fills)
- "dark setting row", "dark-mode settings item", "theme-aware settings row", "暗色设置项" → `add_setting_row_v1` (accepts `theme`; all fills from semantic palette in dark/system)
- "dark member row", "dark-mode team row", "theme-aware member row", "暗色成员行" → `add_member_row_v1` (accepts `theme`; avatar stays user-supplied color; badge/text use palette tokens)
- "dark activity log", "dark-mode audit entry", "theme-aware activity feed", "暗色活动记录" → `add_activity_log_v1` (accepts `theme`; tone dot uses alertColors in dark/system)
- "body paragraph", "description text", "intro copy" (包含 CJK 时尤其推荐) → `add_body_text_v0`
- "icon with label", "menu item (inline)", "breadcrumb segment", "status indicator text" → `add_icon_label_v0`
- "list item", "iOS list cell", "table row with chevron" → `add_list_row_v0` (settings rows specifically — `add_setting_row_v0`, which has switch / value / badge trailing variants)
- "search bar", "search input", "filter search", "搜索栏" → `add_search_bar_v0`
- "dark search bar", "dark-mode search", "theme-aware search bar", "暗色搜索栏" → `add_search_bar_v1` (adds surface2 fill in dark/system so bar is visible on dark bg)
- "form field", "email input", "password field", "labeled input", "required field" → `add_form_field_v0`
- "textarea", "multi-line input", "notes field", "description box", "bio input", "feedback box", "多行输入", "备注" → `add_textarea_v0`
- "dark textarea", "dark-mode notes field", "theme-aware textarea", "暗色多行输入" → `add_textarea_v1` (no hardcoded colors; all modes identical — accepts theme for API consistency).
- "dropdown", "select", "picker", "combo box", "下拉选择", "选择器" → `add_select_v0`
- "dark select", "dark-mode dropdown", "theme-aware select", "暗色下拉" → `add_select_v1` (placeholder → textSubtle in dark/system)
- "skeleton", "loading placeholder", "shimmer", "loading state", "placeholder lines", "骨架屏", "加载中占位" → `add_skeleton_v0`
- "dark skeleton", "dark-mode skeleton", "theme-aware skeleton", "暗色骨架屏" → `add_skeleton_v1` (row fill → surface2 so bars are visible on dark bg).
- "line chart", "trend chart", "折线图" → `add_chart_line_v0`
- "pie chart", "donut chart", "饼图" → `add_chart_pie_v0`
- "image placeholder", "photo slot", "upload zone", "hero image area", "cover placeholder", "图片占位" → `add_image_placeholder_v0`
- "video placeholder", "video slot", "play placeholder", "upcoming video", "视频占位" → `add_video_placeholder_v0`
- "dark video placeholder", "dark-mode video slot", "theme-aware video placeholder", "暗色视频占位" → `add_video_placeholder_v1` (dark bg/icon/caption are builder-private per §3.4; all modes identical).
- "comment", "reply", "feedback row", "review row", "评论" → `add_comment_v0`
- "modal", "dialog", "popup", "confirm dialog", "模态框", "弹窗" → `add_modal_shell_v0`
- "dark modal", "dark-mode dialog", "theme-aware modal", "system theme modal", "暗色弹窗", "主题感知弹窗" → `add_modal_shell_v1` (accepts `theme` param; use `"system"` when the document has `applySemanticPalette(doc)` seeded)
- "dark toast", "dark-mode snackbar", "theme-aware toast", "system theme toast", "暗色 toast", "暗色浮层通知" → `add_toast_v1` (accepts `theme` param; toasts use INVERTED contrast — `"dark"` gives a light pill with dark fg. Use `"system"` when `applySemanticPalette(doc)` is seeded)
- "dark empty chart", "dark-mode no-data placeholder", "theme-aware chart empty state", "暗色空图表", "暗色无数据占位" → `add_empty_chart_v1` (accepts `theme` param; use inside dark-theme dashboards so the empty slot doesn't punch a light rectangle into dark surfaces. Use `"system"` when `applySemanticPalette(doc)` is seeded)
- "status", "online indicator", "presence dot", "health status", "busy indicator", "状态", "在线" → `add_status_badge_v0`
- "dark status badge", "dark-mode status indicator", "theme-aware status badge", "暗色状态徽标" → `add_status_badge_v1` (tone colors are status semantics kept hardcoded; all modes identical).
- "spinner", "loading spinner", "progress circle", "loader", "加载圈" → `add_spinner_v0`
- "dark spinner", "dark-mode spinner", "theme-aware spinner", "暗色加载圈" → `add_spinner_v1` (track_color/active_color are caller params; accepts theme for API consistency).
- "tooltip", "hover hint", "help tip", "提示浮层" → `add_tooltip_v0`
- "dark tooltip", "dark-mode tooltip", "theme-aware tooltip", "暗色提示浮层" → `add_tooltip_v1` (dark body #111827 is intentional inverted-contrast; all modes identical).
- "toggle", "switch", "on/off", "开关" → `add_switch_v0`
- "dark toggle", "dark switch", "theme-aware switch", "暗色开关" → `add_switch_v1` (iOS HIG values kept hardcoded; all modes identical — accepts theme for API consistency).
- "checkbox", "agreement", "select option", "复选框" → `add_checkbox_v0`
- "radio", "single choice", "单选" → `add_radio_v0` (stack multiple in a vertical parent)
- "dark radio", "dark-mode radio button", "theme-aware radio", "暗色单选" → `add_radio_v1` (accent stays brand-invariant; unselected ring → border token)
- "top tabs", "underline tabs", "secondary nav", "下划线 tab" → `add_tabs_v0`
- "dark tabs", "dark-mode tabs", "theme-aware tabs", "暗色 tab" → `add_tabs_v1` (#2563EB underline is brand accent per §3.4; all modes identical).
- "segmented control", "iOS pill tabs", "filter toggle group", "iOS 分段控制" → `add_segmented_control_v0`
- "dark segmented control", "dark-mode pill tabs", "theme-aware segmented control", "暗色分段控制" → `add_segmented_control_v1` (track → surface2; active seg → surface; labels tokenized)
- "empty state", "no results", "nothing here yet", "first-run state", "空状态" → `add_empty_state_v0`
- "alert", "callout", "banner", "notification bar", "告知条", "warning banner" → `add_alert_v0`
- "toast", "snackbar", "popup notification", "轻提示" → `add_toast_v0`
- "progress bar", "loading bar", "线性进度条", "linear progress" → `add_progress_bar_v0`
- "dark progress bar", "dark-mode progress", "theme-aware progress bar", "暗色进度条" → `add_progress_bar_v1` (track → surface2; accent brand-invariant)
- "FAB", "floating action button", "compose button", "新建按钮" → `add_fab_v0`
- "breadcrumb", "nav path", "面包屑" → `add_breadcrumb_v0`
- "stepper", "progress steps", "wizard nav", "步骤条" → `add_stepper_v0`
- "dark stepper", "dark-mode stepper", "theme-aware stepper", "暗色步骤条" → `add_stepper_v1` (pending fill → border; pending number → textMuted in dark/system).
- "rating", "review stars", "评分" → `add_rating_stars_v0`
- "dark rating", "dark-mode stars", "theme-aware rating", "暗色评分" → `add_rating_stars_v1` (no color change; accepts theme for API consistency)
- "carousel dots", "slide indicator", "轮播指示" → `add_carousel_dots_v0`
- "text link", "learn more", "inline link" → `add_link_v0`
- "keyboard shortcut", "hotkey", "⌘K", "快捷键" → `add_kbd_v0`
- "price", "plan cost", "$29/month", "定价" → `add_price_v0`
- "quote", "testimonial quote", "引言" → `add_quote_block_v0`
- "dark quote", "dark-mode testimonial", "theme-aware quote block", "暗色引言" → `add_quote_block_v1` (container bg → surface in dark/system)
- "code snippet", "code block", "代码块" → `add_code_block_v0`
- "color swatch", "palette", "token", "色板" → `add_color_swatch_v0`
- "bar chart", "histogram skeleton", "weekly steps", "柱状图" → `add_chart_bars_v0`
- "timeline", "activity history", "vertical stepper", "时间线", "动态" → `add_timeline_v0`
- "dark timeline", "dark-mode activity history", "theme-aware timeline", "暗色时间线" → `add_timeline_v1` (#2563EB active dot hardcoded per §3.4; inactive dot/connector → border, subtitle → textMuted).
- "calendar", "date picker grid", "month view", "日历" → `add_calendar_grid_v0`
- "pagination", "page nav", "page numbers", "prev/next pages", "分页", "分页条" → `add_pagination_v0`
- "FAQ", "accordion", "collapsible item", "Q&A", "expandable row", "常见问题", "折叠面板" → `add_faq_item_v0`
- "chip input", "tag input", "multi-select field", "recipient list", "email chips", "标签输入", "多选标签" → `add_chip_input_v0`
- "empty chart", "no data chart", "chart placeholder", "empty analytics tile", "暂无数据", "空图表" → `add_empty_chart_v0`
- "action menu", "context menu", "dropdown menu", "more menu", "kebab menu", "action sheet", "下拉菜单", "操作菜单" → `add_action_menu_v0`
- "date picker", "date input", "date field", "due date", "picker closed", "日期选择器", "日期输入" → `add_date_picker_v0` (for the calendar grid shown after clicking, use `add_calendar_grid_v0`)
- "upload", "drop files here", "drag and drop", "file picker", "dropzone", "upload area", "上传区", "文件拖放" → `add_upload_dropzone_v0` (visually similar to empty-chart but semantically different — pick by intent)
- "dark upload dropzone", "dark-mode file upload", "theme-aware dropzone", "暗色上传区" → `add_upload_dropzone_v1` (bg → bgDeep, dashed border → border, icon → textMuted, title → textBody, subtitle → textMuted).
- "OTP", "PIN code", "verification code", "2FA code", "6-digit code", "enter code", "验证码", "PIN 码" → `add_otp_input_v0`
- "attachment", "attached file", "uploaded file", "file item", "file list row", "附件", "已上传文件" → `add_attachment_row_v0` (for upload-in-progress state, compose `add_progress_bar_v0` below)
- "chat message", "message bubble", "conversation row", "iMessage bubble", "chat UI", "聊天气泡", "消息气泡" → `add_chat_bubble_v0` (side="left" for from-others, side="right" for from-self)
- "KPI card", "big number card", "metric tile", "stat widget", "featured metric", "关键指标卡", "数据大屏卡片" → `add_stat_card_v0` (distinct from `add_stat_grid_v0` which is multi-cell side-by-side)
- "dark stat card", "dark KPI card", "dark-mode metric tile", "暗色指标卡" → `add_stat_card_v1` (bg → surface; border → border; label → textMuted; icon → textSubtle; value → textPrimary in dark/system).
- "Continue with Google", "Sign in with Apple", "social login", "OAuth buttons", "SSO providers", "third-party login", "第三方登录", "社交登录", "OAuth 登录" → `add_social_login_row_v0` (orientation="vertical" for stacked full-width on mobile; orientation="horizontal" for the compact "or sign in with..." icon-only row)
- "dark social login", "dark-mode social auth buttons", "theme-aware social login", "暗色第三方登录" → `add_social_login_row_v1` (button bg → surface; border → border; icon → textMuted; label → textPrimary in dark/system).
- "pricing card", "plan card", "SaaS tier", "subscription plan", "pricing tier", "billing card", "价格卡", "套餐卡", "定价卡片" → `add_pricing_card_v0` (set one tile's `emphasis: "featured"` to visually recommend it — auto-gets "Most popular" badge unless `badge` overrides). For a 3-tier pricing section, call this 3× under the same parent section.
- "slider", "range input", "volume control", "opacity slider", "brightness slider", "filter slider", "滑块", "滑动条", "音量条" → `add_range_slider_v0` (single-handle; set `show_value=true` + `value_suffix="%"` to render the readout). For a dual-handle range (min+max), still fall through to batch_design.
- "dark slider", "dark-mode range input", "theme-aware slider", "暗色滑块" → `add_range_slider_v1` (accent invariant; thumb bg → surface; remaining → border; label/value tokenized)
- "phone input", "phone field", "international phone", "country code input", "+1 (555) ...", "电话号码", "手机号输入", "国际电话" → `add_phone_input_v0` (renders country dial code button + digits input in a 44px row; pass `country_flag` for emoji prefix). For a plain single-line text input without the country prefix, use `add_form_field_v0`.
- "newsletter signup", "subscribe form", "subscribe to newsletter", "promo code input", "apply discount", "send message input", "chat composer", "search with submit", "订阅", "应用优惠码", "发送消息" → `add_input_with_action_v0` (action_kind="text" for "Subscribe" pill button, action_kind="icon" for chat send arrow).
- "cookie banner", "cookie consent", "GDPR banner", "CCPA banner", "privacy notice", "cookie disclosure", "cookie 提示", "隐私同意条" → `add_cookie_banner_v0` (set `show_settings_link: true` for fine-grained GDPR consent UX). Caller positions sticky-bottom; the tool emits the banner card itself.
- "sidebar", "side nav", "sidebar nav", "left rail", "dashboard nav rail", "admin sidebar", "settings sidebar", "docs sidebar", "vertical nav", "侧边栏", "侧边导航", "左侧导航" → `add_sidebar_nav_v0` (desktop persistent rail; pass `title` for a brand row above the items, mark current page with `active: true` on the item).
- "dark sidebar", "dark-mode sidebar nav", "theme-aware sidebar", "暗色侧边栏" → `add_sidebar_nav_v1` (bg → surface; active label → textPrimary; inactive → textMuted; active item bg → surface2 in dark/system).
- "stacked avatars", "avatar group", "avatar stack", "team avatars", "5 contributors", "online users", "+N more", "viewers row", "presence indicator", "成员头像", "团队头像", "在线用户", "头像组" → `add_avatar_group_v0` (renders up to `max_visible` ringed avatar circles + a "+N" overflow tile; pen-core flex doesn't allow negative gap so the white ring + 4px gap is the affordance, not literal overlap).
- "data table row", "table row", "table header row", "customer row", "order row", "report row", "transaction row", "users table", "数据表行", "表格行", "表头行" → `add_data_table_row_v0` (desktop pattern; pass `header: true` for the column-header row, `selected: true` to tint a hover/selected body row). For row separators stack `add_divider_v0` between rows.
- "filter chip", "filter tag", "applied filter", "selected criterion", "category pill", "removable tag", "Status: Active ×", "可移除标签", "筛选标签" → `add_tag_v0` (single chip with optional × close icon, default removable=true; pass `tone` for accent / success / warning / error palettes).
- "dark tag", "dark-mode filter chip", "theme-aware tag", "暗色标签" → `add_tag_v1` (tone colors are status semantics per §3.4; all modes identical).
- "user card", "contact card", "people picker row", "user mini card", "用户卡片", "联系人卡片" → `add_user_card_v0` (compact fit_content tile, no trailing slot — used in lobbies / chip-style lists).
- "dark user card", "dark-mode contact card", "theme-aware user card", "暗色用户卡片" → `add_user_card_v1` (name → textPrimary, role → textMuted; avatar bg #3B82F6 + initial white stay brand-invariant per §3.4).
- "members list", "team page row", "people row", "sharing dialog row", "user list with role", "team member row", "团队成员", "成员列表行" → `add_member_row_v0` (avatar + name + optional subtitle + trailing role badge / kebab menu / status dot).
- "profile header", "profile hero", "about me block", "account header", "user profile page", "个人主页头部" → `add_profile_header_v0` (large centered avatar + display name + optional handle / bio).
- "dark profile header", "theme-aware profile page", "暗色个人主页头部" → `add_profile_header_v1` (name → textPrimary, handle → textMuted, bio → textBody; avatar bg #3B82F6 + initial white stay brand-invariant).
- "dark pagination", "dark-mode page nav", "theme-aware pagination", "暗色分页" → `add_pagination_v1` (arrow/inactive → textBody, ellipsis → textMuted; active pill bg = accent_color invariant, active text stays white).
- "dark OTP input", "dark 2FA code", "dark PIN input", "theme-aware OTP", "暗色验证码输入" → `add_otp_input_v1` (slot bg → surface, borders → border/borderStrong, digit → textPrimary; focused border = accent_color invariant).
- "dark phone input", "dark phone field", "theme-aware phone number input", "暗色电话号码输入" → `add_phone_input_v1` (field bg → surface, stroke/divider → border, label → textBody, code → textPrimary, chevron → textMuted).
- "dark pricing card", "dark SaaS tier", "theme-aware pricing card", "暗色价格卡", "暗色套餐卡" → `add_pricing_card_v1` (card bg → surface, border → border; featured accent border/CTA bg stay brand-invariant; text hierarchy → textPrimary/textMuted/textBody).
- "dark notification row", "theme-aware notification", "暗色通知行" → `add_notification_row_v1` (timestamp → textSubtle, body → textBody, unread dot → destructive).
- "dark KPI cell", "dark metric comparison", "theme-aware trend metric", "暗色 KPI 趋势" → `add_metric_comparison_v1` (label → textMuted; up → success, down → destructive, flat → textMuted).
- "dark metric row", "theme-aware metric tiles", "暗色指标行" → `add_metric_row_v1` (no hardcoded colors in v0; all modes identical — accepts theme for API consistency).
- "dark nav chips", "theme-aware nav chip row", "暗色导航芯片行" → `add_nav_chip_row_v1` (no hardcoded colors in v0; all modes identical — accepts theme for API consistency).
- "dark price", "theme-aware price display", "暗色定价" → `add_price_v1` (no hardcoded colors in v0; all modes identical — accepts theme for API consistency).
- "drawer", "side panel", "slide-in panel", "edit drawer", "detail panel", "抽屉", "侧滑面板" → `add_drawer_shell_v0` (full-height drawer with title + close ×; body composed via subsequent calls under its id).
- "combobox", "autocomplete", "open dropdown", "command palette", "open select", "search with results", "自动补全", "下拉补全" → `add_combobox_v0` (OPEN-state input + dropdown with N suggestion rows; one row optionally `highlighted: true`).
- "toolbar", "editor toolbar", "formatting toolbar", "kanban actions", "icon button row", "工具栏" → `add_toolbar_v0` (horizontal 36×36 icon buttons + optional vertical dividers).
- "dark toolbar", "dark-mode editor toolbar", "theme-aware toolbar", "暗色工具栏" → `add_toolbar_v1` (surface, border, active-bg, icon fills tokenized for dark/system).
- "callout", "tip block", "doc note", "did you know", "info box", "提示框", "信息块" → `add_callout_v0` (tinted block with body + optional title + tone-driven leading icon: info / success / warning / danger / note).
- "inline action", "Undo button inline", "Saved • Retry", "comment deleted undo", "inline feedback", "inline action row" → `add_inline_action_v0` (left message + right blue action label, NO floating).
- "share row", "share to social", "share buttons", "post share", "send via", "分享按钮组" → `add_share_row_v0` (horizontal circular icon buttons each labeled below).
- "dark share row", "dark-mode share buttons", "theme-aware share row", "暗色分享按钮" → `add_share_row_v1` (icon bg → surface2; icon/label → textMuted in dark/system).
- "chart legend", "legend item", "legend entry", "数据图例", "图例条目" → `add_legend_item_v0` (marker + label + optional value).
- "inbox row", "email row", "message list cell", "mail item", "email preview", "邮件条目", "收件箱条目" → `add_inbox_message_v0` (sender + subject + preview + timestamp + unread dot).
- "settings row", "preference row", "menu item with toggle", "settings list item", "设置项", "偏好项", "开关行" → `add_setting_row_v0` (icon + title/subtitle + trailing chevron/value/switch/badge).
- "facet panel", "filter sidebar", "category checklist", "brand filter", "applied filters group", "搜索筛选侧栏", "分面筛选", "类目筛选" → `add_filter_group_v0` (heading + vertical checkbox-style option list with optional counts).
- "pending invite", "invitation row", "team invite", "邀请列表行", "待接受邀请", "invitations table row" → `add_invite_row_v0` (initial avatar + email/role + status pill + trailing action).
- "audit log row", "activity feed entry", "recent activity item", "audit entry", "审计日志条目", "活动记录", "操作日志行" → `add_activity_log_v0` (single line: optional tinted icon dot + "<actor> <action>" + right-aligned timestamp).
- "event card", "agenda item", "meeting tile", "upcoming event", "calendar event row", "日程卡片", "会议条目", "活动卡片" → `add_event_card_v0` (date column with month band + day number, then title + time + location).
- "onboarding step", "how-it-works step", "tutorial step card", "setup checklist item", "操作步骤卡片", "教程步骤", "新手引导步骤" → `add_step_card_v0` (numbered circle / check + title + description, stacks vertically).
- "dark step card", "dark-mode onboarding step", "theme-aware step card", "暗色步骤卡片" → `add_step_card_v1` (title → textPrimary; desc → textMuted; ring bg → surface in dark/system).

STILL use batch_design when (emit a SINGLE batch_design for the whole response — do NOT mix with element-tool calls):

- The brief's items are structurally heterogeneous (can't be uniformly described by a single items[] shape)
- The brief needs a custom container or layout that no element tool covers (e.g. a horizontal row of pricing cards with a specific gap, a scroll row + sibling content)
- The brief asks for unusual styling (custom fills, typography, theme variables) on top of a known structure

## Common compositions (cookbook)

When the user asks for a recognizable screen pattern (login, signup,
settings, paywall, dashboard tile row, support chat), STACK existing
element tools under one parent rather than reaching for batch_design.
The recipes below are by-frequency-of-real-use; each one fits in 4-7
tool calls.

### Login screen (phone + password + social)

```
add_heading_v0({ parent_id: "<page>", content: "Welcome back" })
add_body_text_v0({ parent_id: "<page>", content: "Sign in to continue" })
add_phone_input_v0({ parent_id: "<page>", label: "Phone number", country_code: "+1", country_flag: "🇺🇸", required: true })
add_form_field_v0({ parent_id: "<page>", label: "Password", required: true })
add_text_button_v0({ parent_id: "<page>", label: "Sign in" })
add_link_v0({ parent_id: "<page>", label: "Forgot password?" })
add_divider_v0({ parent_id: "<page>" })
add_social_login_row_v0({ parent_id: "<page>", providers: [{ name: "Google" }, { name: "Apple" }] })
```

### Signup form (email + password + agreement)

```
add_heading_v0({ parent_id: "<form>", content: "Create your account" })
add_form_field_v0({ parent_id: "<form>", label: "Email", required: true })
add_form_field_v0({ parent_id: "<form>", label: "Password", required: true })
add_form_field_v0({ parent_id: "<form>", label: "Confirm password", required: true })
add_checkbox_v0({ parent_id: "<form>", label: "I agree to the Terms of Service", checked: false })
add_text_button_v0({ parent_id: "<form>", label: "Sign up" })
```

### Settings page (groups of setting rows)

```
add_section_header_v0({ parent_id: "<page>", title: "Account" })
add_setting_row_v0({ parent_id: "<page>", title: "Profile", leading_icon: "user" })
add_setting_row_v0({ parent_id: "<page>", title: "Email", leading_icon: "mail", trailing: { kind: "value", value: "you@acme.com" } })
add_divider_v0({ parent_id: "<page>" })
add_section_header_v0({ parent_id: "<page>", title: "Notifications" })
add_setting_row_v0({ parent_id: "<page>", title: "Push notifications", leading_icon: "bell", trailing: { kind: "switch", on: true } })
add_setting_row_v0({ parent_id: "<page>", title: "Email digest", leading_icon: "mail", trailing: { kind: "switch", on: false } })
```

<!-- @domain:dashboard -->

### Team / members list (avatars + pending invitations)

```
add_section_header_v0({ parent_id: "<page>", title: "Members" })
add_body_text_v0({ parent_id: "<page>", content: "5 people" })
add_member_row_v0({ parent_id: "<page>", name: "Sarah Lee", subtitle: "sarah@acme.com", initial: "S", trailing: { kind: "role_badge", value: "Owner" } })
add_member_row_v0({ parent_id: "<page>", name: "Marcus Chen", subtitle: "marcus@acme.com", initial: "M", trailing: { kind: "role_badge", value: "Admin" } })
add_member_row_v0({ parent_id: "<page>", name: "Aiko Tanaka", subtitle: "aiko@acme.com", initial: "A", trailing: { kind: "role_badge", value: "Editor" } })
add_member_row_v0({ parent_id: "<page>", name: "Raj Patel", subtitle: "raj@acme.com", initial: "R", trailing: { kind: "role_badge", value: "Editor" } })
add_member_row_v0({ parent_id: "<page>", name: "Jordan Kim", subtitle: "jordan@acme.com", initial: "J", trailing: { kind: "role_badge", value: "Viewer" } })
add_divider_v0({ parent_id: "<page>" })
add_section_header_v0({ parent_id: "<page>", title: "Pending invitations" })
add_invite_row_v0({ parent_id: "<page>", email: "leon@acme.com", role: "Editor", status: "pending", action_label: "Resend" })
```

<!-- /@domain -->

<!-- @domain:dashboard -->

### Audit / activity feed (N log entries under a header)

```
add_section_header_v0({ parent_id: "<panel>", title: "Recent activity" })
add_body_text_v0({ parent_id: "<panel>", content: "Last 24 hours" })
add_activity_log_v0({ parent_id: "<panel>", actor: "Sarah Lee", action: "approved the production deploy", timestamp: "2h ago", icon: "check", tone: "success" })
add_activity_log_v0({ parent_id: "<panel>", actor: "Marcus Chen", action: "uploaded final-mockups.zip", timestamp: "3h ago", icon: "upload", tone: "info" })
add_activity_log_v0({ parent_id: "<panel>", actor: "Aiko Tanaka", action: "invited jordan@acme.com to the workspace", timestamp: "5h ago", icon: "user-plus", tone: "info" })
add_activity_log_v0({ parent_id: "<panel>", actor: "System", action: "rate-limited an IP after 50 failed sign-ins", timestamp: "8h ago", icon: "alert-triangle", tone: "warning" })
add_activity_log_v0({ parent_id: "<panel>", actor: "Raj Patel", action: "updated billing details", timestamp: "yesterday", icon: "settings", tone: "neutral" })
add_activity_log_v0({ parent_id: "<panel>", actor: "Sarah Lee", action: "deleted archive-2024.zip", timestamp: "yesterday", icon: "trash", tone: "danger" })
```

<!-- /@domain -->

<!-- @domain:dashboard -->

### Faceted search filter sidebar (N filter groups stacked)

```
add_filter_group_v0({ parent_id: "<sidebar>", title: "Category", options: [{ label: "Apparel", count: 124, selected: true }, { label: "Footwear", count: 86 }, { label: "Bags", count: 41 }, { label: "Accessories", count: 67 }] })
add_filter_group_v0({ parent_id: "<sidebar>", title: "Brand", options: [{ label: "Nike", count: 32 }, { label: "Adidas", count: 28 }, { label: "Patagonia", count: 15, selected: true }, { label: "Arc'teryx", count: 9 }] })
```

<!-- /@domain -->

<!-- @domain:mobile -->

### Onboarding "How it works" (N step cards)

```
add_heading_v0({ parent_id: "<page>", content: "Get started in minutes" })
add_body_text_v0({ parent_id: "<page>", content: "Three steps to a smarter wallet." })
add_step_card_v0({ parent_id: "<page>", number: 1, title: "Create your account", description: "Use your email and a strong password to sign up. No credit card required." })
add_step_card_v0({ parent_id: "<page>", number: 2, title: "Connect your bank", description: "Link your account in seconds. We use 256-bit encryption to keep your data safe." })
add_step_card_v0({ parent_id: "<page>", number: 3, title: "Set your goals", description: "Tell us what you want to save for — we'll do the rest." })
add_step_card_v0({ parent_id: "<page>", number: 4, title: "You're all set", description: "You're ready to use the app. Tap below to continue.", completed: true })
```

<!-- /@domain -->

<!-- @domain:landing -->

### Pricing section (3 tiers, middle one featured)

```
add_pricing_card_v0({ parent_id: "<page>", tier: "Starter", price: "0", period: "/month", features: ["3 projects", "Community support"] })
add_pricing_card_v0({ parent_id: "<page>", tier: "Pro", price: "29", period: "/month", features: ["Unlimited projects", "Priority support", "Advanced analytics"], emphasis: "featured" })
add_pricing_card_v0({ parent_id: "<page>", tier: "Enterprise", price: "Custom", features: ["Dedicated support", "SSO", "SLA"], cta: "Contact sales" })
```

<!-- /@domain -->

<!-- @domain:dashboard -->

### Dashboard KPI strip (4 stat cards)

```
add_stat_card_v0({ parent_id: "<page>", label: "Revenue", value: "$12.4k", icon: "trending-up", delta: "+8%", trend: "up" })
add_stat_card_v0({ parent_id: "<page>", label: "Active users", value: "1,284", icon: "users", delta: "+3%", trend: "up" })
add_stat_card_v0({ parent_id: "<page>", label: "Churn", value: "3.2%", icon: "user-minus", delta: "-0.4%", trend: "down" })
add_stat_card_v0({ parent_id: "<page>", label: "Sessions", value: "5,471", icon: "activity" })
```

<!-- /@domain -->

### OTP / 2FA verification screen

```
add_heading_v0({ parent_id: "<page>", content: "Enter verification code" })
add_body_text_v0({ parent_id: "<page>", content: "We sent a 6-digit code to +1 (555) 123-4567" })
add_otp_input_v0({ parent_id: "<page>", length: 6, digits: ["1", "2", "3"], focused_index: 3 })
add_text_button_v0({ parent_id: "<page>", label: "Verify" })
add_link_v0({ parent_id: "<page>", label: "Resend code" })
```

<!-- @domain:mobile -->

### Support chat thread

```
add_chat_bubble_v0({ parent_id: "<thread>", message: "Hi! How can I help today?", side: "left", author: "Sarah", timestamp: "Just now" })
add_chat_bubble_v0({ parent_id: "<thread>", message: "My order hasn't arrived.", side: "right", timestamp: "2m" })
add_chat_bubble_v0({ parent_id: "<thread>", message: "Sorry to hear! Let me check on that.", side: "left", author: "Sarah", timestamp: "1m" })
add_attachment_row_v0({ parent_id: "<thread>", filename: "receipt.pdf", size: "240 KB", icon: "file-text" })
```

<!-- /@domain -->

### Empty inbox / first-run onboarding

```
add_empty_state_v0({ parent_id: "<page>", title: "No messages yet", subtitle: "When someone messages you, it'll show up here.", icon: "inbox", cta_label: "Find friends" })
```

### Composition rules of thumb

- **One parent for one row of siblings.** Don't pass `parent_id` of an unrelated container.
- **Order matters.** Tools insert as the LAST child of `parent_id`, so call sequence is render order top-to-bottom (vertical) or left-to-right (horizontal).
- **Don't mix element tools and `batch_design` in the same output.** Pick one strategy: chain `add_*_v0` calls (preferred when every component fits an element tool) OR a single `batch_design` covering the whole brief (when at least one component needs a custom shape). The corpus harness drops `batch_design` tags whenever element-tool calls share the response, so a mixed output silently loses its scaffolding.

## Invariants you don't need to think about

The tool guarantees — you cannot break them from the input side:

- Wrapper structure (`scroll-row-wrapper` + `scroll-row` + fixed-width children) for row tools — overflow-safe
- `bottom-tab-bar` is inline (no empty spacer sibling needed, do NOT add one)
- Activity ring is frame+cornerRadius=size/2+stroke+centered text — NEVER emit ellipse+sibling text for rings
- Every emitted node has a unique id (you can reference it later)
- Roles are set (`card` / `metric-tile` / `nav-chip` / `nav-chip-active` / `bottom-tab-bar` / `nav-item` / `nav-item-active` / `activity-ring` / `stat-grid` / `stat-cell` / `section-header` / `section-header-title` / `section-header-action` / `top-nav-bar` / `nav-spacer` / `icon-button` / `divider` / `badge` / `avatar` / `button` / `heading` / `body` / `label` / `icon-label` / `list-row` / `list-row-text` / `search-bar` / `form-field` / `form-input` / `switch` / `switch-thumb` / `checkbox` / `checkbox-checked` / `checkbox-row` / `radio` / `radio-selected` / `radio-row` / `radio-dot` / `tabs` / `tab` / `tab-active` / `tab-underline` / `segmented-control` / `segment` / `segment-active` / `empty-state` / `empty-state-icon` / `empty-state-title` / `empty-state-subtitle` / `alert` / `alert-message` / `alert-close` / `toast` / `toast-message` / `progress-bar` / `progress-bar-fill` / `fab` / `breadcrumb` / `breadcrumb-item` / `breadcrumb-item-active` / `breadcrumb-separator` / `stepper` / `step` / `step-active` / `step-connector` / `step-connector-active` / `rating-stars` / `star-filled` / `star-empty` / `link` / `link-label` / `link-icon` / `kbd` / `kbd-key` / `kbd-glyph` / `kbd-separator` / `carousel-dots` / `dot` / `dot-active` / `price` / `price-currency` / `price-amount` / `price-period` / `quote-block` / `quote-text` / `quote-author` / `code-block` / `code` / `color-swatch` / `color-swatch-square` / `color-swatch-label` / `chart-bars` / `chart-bar` / `timeline` / `timeline-item` / `timeline-icon-column` / `timeline-dot` / `timeline-dot-active` / `timeline-connector` / `timeline-content` / `timeline-title` / `timeline-subtitle` / `calendar-grid` / `calendar-header-row` / `calendar-header` / `calendar-week` / `calendar-day` / `calendar-day-today` / `calendar-day-selected` / `calendar-day-empty`)

## Failure mode

If the tool throws, **do NOT retry with the same arguments** — the tool has already verified the failure is real (pre-check rejected the parent_id, or post-check detected a silent DSL no-op). Re-throwing from your side wastes tokens. Inspect the error message and either:

- Fix `parent_id` (ensure the referenced node exists and has no `"` or `\` in its id)
- Switch to `batch_design` with the structure taught in `overflow.md` / `layout.md`
