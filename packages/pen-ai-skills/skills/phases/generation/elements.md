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

These narrow MCP tools emit well-known structures that batch_design frequently gets wrong on non-Claude models (overflow, wrong role, anti-pattern layout). Each is shape-locked — you pick the tool by matching intent, then supply only content. Visual styling (color, font) stays orthogonal: override via a follow-up batch_design U-op if needed.

## Decision tree (pick first match)

Rows (horizontal, in-card or scrolling):

1. Row of items with title + subtitle + optional icon → `add_card_row_v0` (scroll)
2. Row of items with small label + big numeric value → `add_metric_row_v0` (scroll)
3. Row of filter chips / category tabs (label + optional icon, active state) → `add_nav_chip_row_v0` (scroll)
4. Non-scrolling 2-5 stats inline (auto-share width) → `add_stat_grid_v0`

Containers and single elements:

5. Section header (big title + optional "See all" action) → `add_section_header_v0`
6. Bottom tab bar (inline flow, 3-5 nav items) → `add_bottom_nav_v0`
7. Mobile top bar (leading icon + centered title + trailing icon) → `add_top_nav_bar_v0`
8. Icon-only button (44×44, hit-target safe) → `add_icon_button_v0`
9. Apple-style progress ring with centered text → `add_activity_ring_v0`

Atoms (1-2 node building blocks):

10. Hairline separator between list rows / sections → `add_divider_v0`
11. Short inline pill / tag / "NEW" / "BETA" / count badge → `add_badge_v0`
12. Circular avatar (with optional initial / empty for later image fill) → `add_avatar_v0`

Text + button primitives:

13. Padding-based button with text (optional leading icon) → `add_text_button_v0`
14. Heading with enforced fontSize/lineHeight per level + AUTO CJK script detection (SC/JP/KR) → `add_heading_v0`
15. Body text (Inter everywhere — CJK gets lineHeight 1.6 + letterSpacing 0, Latin 1.5) → `add_body_text_v0`

Composition:

16. Icon + text inline pair (menu items, breadcrumbs, status indicators) → `add_icon_label_v0`
17. iOS/Material list row (leading icon + title/subtitle stack + trailing icon) → `add_list_row_v0`

Forms:

18. Search bar (height=44, cornerRadius=22, leading search icon) → `add_search_bar_v0`
19. Form field (label + 48px input with optional affordance icons) → `add_form_field_v0`
    19b. Multi-line textarea (label + N-row input for notes / bio / feedback) → `add_textarea_v0`
    19c. Dropdown select closed-state (label + 48px input w/ value text + chevron-down) → `add_select_v0`

Controls (toggle / choice / tabs):

20. iOS/Material toggle switch (51×31, thumb floats) → `add_switch_v0`
21. Checkbox + inline label (20×20 box, `check` icon inside when checked) → `add_checkbox_v0`
22. Radio button + inline label (20×20 ring, dot inside when selected) → `add_radio_v0`
23. Horizontal top tabs with underline on active (fontWeight 600 + 2px sibling rectangle underline) → `add_tabs_v0`
24. iOS pill-style segmented control (equal-width segments, active floats white) → `add_segmented_control_v0`

State / feedback:

25. Empty state (icon + title + optional subtitle + optional CTA button, centered) → `add_empty_state_v0`
26. Inline banner / callout (icon + message + optional close x, fill_container) → `add_alert_v0`
27. Floating pill notification (dark fit_content pill) → `add_toast_v0`
28. Linear progress bar (fixed bar_width + value 0-100) → `add_progress_bar_v0`

Floating / nav / wizard:

29. Floating action button (circular 56×56, icon centered) → `add_fab_v0`
30. Breadcrumb trail with chevron separators (last crumb auto-active) → `add_breadcrumb_v0`
31. Horizontal numbered stepper (circles + fill_container connectors) → `add_stepper_v0`

Ratings & pagination:

32. N-of-M star rating (filled + empty stars using lucide `star`) → `add_rating_stars_v0`
33. Carousel / slide dots (active becomes elongated pill, inactive circles) → `add_carousel_dots_v0`

Inline text primitives:

34. Text link with optional trailing icon ("Learn more →") → `add_link_v0`
35. Keyboard shortcut glyph ("⌘ K" / "Ctrl + Shift + P") → `add_kbd_v0`
36. Pricing typography ("$29/month": currency 20/500 + amount 40/700 + period 14/500) → `add_price_v0`

Content blocks:

37. Quoted passage with optional author attribution → `add_quote_block_v0`
38. Preformatted code block (fill_container, wraps, gray-50 bg) → `add_code_block_v0`

Design-system:

39. Color swatch (colored square + optional token label) → `add_color_swatch_v0`

Charts / data visualization:

40. Bar-chart skeleton (one rectangle per value, bottom-aligned) → `add_chart_bars_v0`
    40b. Line-chart skeleton (polyline + optional dots) → `add_chart_line_v0`
    40c. Pie-chart skeleton (colored ellipse slices via arc angles) → `add_chart_pie_v0`

Media / placeholder:

44. Image placeholder (gray box + centered icon + optional caption — future image slot) → `add_image_placeholder_v0`
    44b. Video placeholder (dark box + play icon + optional caption — future video embed) → `add_video_placeholder_v0`

Social / UGC:

45. Comment row (circular avatar + author/timestamp header + body) → `add_comment_v0`

Chrome / modals:

46. Modal dialog shell (dimmed backdrop + centered card + title — body composed separately) → `add_modal_shell_v0`
    46b. Theme-aware variant (supports `theme: 'light' | 'dark' | 'system'`) → `add_modal_shell_v1`

Status / presence:

47. Status badge (small colored dot + short label, "● Online" pattern, tone-enum'd) → `add_status_badge_v0`

Feedback / loading:

48. Loading spinner (static ring + 3/4 arc) → `add_spinner_v0`
49. Tooltip pill (dark pill + white text, hover-hint appearance) → `add_tooltip_v0`

Analytics / KPIs:

50. KPI cell with trend arrow + change ("$12k ↑ 8%") → `add_metric_comparison_v0`

Notifications:

51. Notification list row (icon + title/timestamp header + optional body preview + optional unread dot) → `add_notification_row_v0`

Activity / history:

41. Vertical timeline (dots + fixed 24px connectors + content; no padding/gap) → `add_timeline_v0`

Calendars:

42. Month calendar grid (weekday header + 7-col day rows, today/selected tint) → `add_calendar_grid_v0`

Loading / placeholder:

43. Loading skeleton (N gray rectangles, last row ~60% width) → `add_skeleton_v0`

Pagination:

52. Pagination bar (numbered pills + prev/next arrows, Google-style ellipses for big ranges) → `add_pagination_v0`

Collapsible content:

53. FAQ / accordion item (question + chevron; expanded variant shows answer paragraph) → `add_faq_item_v0`

Tag / multi-value inputs:

54. Chip input / tag input (pills + removable × + inline caret, wrap layout) → `add_chip_input_v0`

Chart empty state:

55. Empty chart placeholder (dashed tile in chart footprint; "No data yet" message) → `add_empty_chart_v0`

Menus / floating panels:

56. Action / context menu panel (dropdown list of icon+label rows, destructive variant supported) → `add_action_menu_v0`

Dates:

57. Date picker CLOSED state (labeled input + "Jan 15, 2026" + trailing calendar icon) → `add_date_picker_v0`

Upload / file intake:

58. File upload dropzone (dashed tile + cloud icon + "Drop files / click to browse") → `add_upload_dropzone_v0`

Auth / verification:

59. OTP / PIN code input (row of N square slots, 4..8 digits; blank / partial / full states) → `add_otp_input_v0`

Attachments:

60. File attachment row (type-icon + filename + optional size + remove ×) → `add_attachment_row_v0`

Messaging:

61. Chat message bubble (left=from-others slate bg / right=from-self accent bg; optional author + timestamp) → `add_chat_bubble_v0`

62. None match → fall through to `batch_design`

**Disambiguation**: if you need a ROW of 3 metrics that should NOT scroll (e.g. a stats strip inside a card), use `add_stat_grid_v0`, NOT `add_metric_row_v0`. The grid uses `fill_container` per cell so it never overflows; the metric row uses fixed-px cells + scroll wrapper.

## When to use vs batch_design

PREFER an element tool when the spec says any of:

- "horizontal scrolling cards", "swipeable row", "chip row", "pills" → `add_card_row_v0`
- "metric tiles", "KPI cards", "dashboard stats" (SCROLLING row) → `add_metric_row_v0`
- "stats row", "3 metrics side by side", "summary bar" (NON-scrolling grid) → `add_stat_grid_v0`
- "category filter chips", "quick-access shortcuts" → `add_nav_chip_row_v0`
- "section title with See all / View more" → `add_section_header_v0`
- "bottom nav", "tab bar", "tabbar", "底部导航" → `add_bottom_nav_v0`
- "top bar", "app bar", "header with back button", "页面标题栏" → `add_top_nav_bar_v0`
- "icon-only button", "close button", "menu button" (toolbar-style) → `add_icon_button_v0`
- "activity ring", "progress ring", "circular progress", "Apple health ring" → `add_activity_ring_v0`
- "hairline divider", "separator", "row divider", "section separator" → `add_divider_v0`
- "badge", "pill", "tag", "NEW label", "count bubble" (≤16 Latin / ≤8 CJK chars) → `add_badge_v0`
- "avatar", "profile picture", "user circle", "initial bubble" → `add_avatar_v0`
- "primary button", "secondary button", "CTA", "submit button" (short label) → `add_text_button_v0`
- "hero headline", "section title", "card title" / 特定字号标题 → `add_heading_v0`
- "body paragraph", "description text", "intro copy" (包含 CJK 时尤其推荐) → `add_body_text_v0`
- "icon with label", "menu item (inline)", "breadcrumb segment", "status indicator text" → `add_icon_label_v0`
- "settings row", "list item", "iOS list cell", "table row with chevron" → `add_list_row_v0`
- "search bar", "search input", "filter search", "搜索栏" → `add_search_bar_v0`
- "form field", "email input", "password field", "labeled input", "required field" → `add_form_field_v0`
- "textarea", "multi-line input", "notes field", "description box", "bio input", "feedback box", "多行输入", "备注" → `add_textarea_v0`
- "dropdown", "select", "picker", "combo box", "下拉选择", "选择器" → `add_select_v0`
- "skeleton", "loading placeholder", "shimmer", "loading state", "placeholder lines", "骨架屏", "加载中占位" → `add_skeleton_v0`
- "line chart", "trend chart", "折线图" → `add_chart_line_v0`
- "pie chart", "donut chart", "饼图" → `add_chart_pie_v0`
- "image placeholder", "photo slot", "upload zone", "hero image area", "cover placeholder", "图片占位" → `add_image_placeholder_v0`
- "video placeholder", "video slot", "play placeholder", "upcoming video", "视频占位" → `add_video_placeholder_v0`
- "comment", "reply", "feedback row", "review row", "评论" → `add_comment_v0`
- "modal", "dialog", "popup", "confirm dialog", "模态框", "弹窗" → `add_modal_shell_v0`
- "dark modal", "dark-mode dialog", "theme-aware modal", "system theme modal", "暗色弹窗", "主题感知弹窗" → `add_modal_shell_v1` (accepts `theme` param; use `"system"` when the document has `applySemanticPalette(doc)` seeded)
- "status", "online indicator", "presence dot", "health status", "busy indicator", "状态", "在线" → `add_status_badge_v0`
- "spinner", "loading spinner", "progress circle", "loader", "加载圈" → `add_spinner_v0`
- "tooltip", "hover hint", "help tip", "提示浮层" → `add_tooltip_v0`
- "toggle", "switch", "on/off", "开关" → `add_switch_v0`
- "checkbox", "agreement", "select option", "复选框" → `add_checkbox_v0`
- "radio", "single choice", "单选" → `add_radio_v0` (stack multiple in a vertical parent)
- "top tabs", "underline tabs", "secondary nav", "下划线 tab" → `add_tabs_v0`
- "segmented control", "iOS pill tabs", "filter toggle group", "iOS 分段控制" → `add_segmented_control_v0`
- "empty state", "no results", "nothing here yet", "first-run state", "空状态" → `add_empty_state_v0`
- "alert", "callout", "banner", "notification bar", "告知条", "warning banner" → `add_alert_v0`
- "toast", "snackbar", "popup notification", "轻提示" → `add_toast_v0`
- "progress bar", "loading bar", "线性进度条", "linear progress" → `add_progress_bar_v0`
- "FAB", "floating action button", "compose button", "新建按钮" → `add_fab_v0`
- "breadcrumb", "nav path", "面包屑" → `add_breadcrumb_v0`
- "stepper", "progress steps", "wizard nav", "步骤条" → `add_stepper_v0`
- "rating", "review stars", "评分" → `add_rating_stars_v0`
- "carousel dots", "slide indicator", "轮播指示" → `add_carousel_dots_v0`
- "text link", "learn more", "inline link" → `add_link_v0`
- "keyboard shortcut", "hotkey", "⌘K", "快捷键" → `add_kbd_v0`
- "price", "plan cost", "$29/month", "定价" → `add_price_v0`
- "quote", "testimonial quote", "引言" → `add_quote_block_v0`
- "code snippet", "code block", "代码块" → `add_code_block_v0`
- "color swatch", "palette", "token", "色板" → `add_color_swatch_v0`
- "bar chart", "histogram skeleton", "weekly steps", "柱状图" → `add_chart_bars_v0`
- "timeline", "activity history", "vertical stepper", "时间线", "动态" → `add_timeline_v0`
- "calendar", "date picker grid", "month view", "日历" → `add_calendar_grid_v0`
- "pagination", "page nav", "page numbers", "prev/next pages", "分页", "分页条" → `add_pagination_v0`
- "FAQ", "accordion", "collapsible item", "Q&A", "expandable row", "常见问题", "折叠面板" → `add_faq_item_v0`
- "chip input", "tag input", "multi-select field", "recipient list", "email chips", "标签输入", "多选标签" → `add_chip_input_v0`
- "empty chart", "no data chart", "chart placeholder", "empty analytics tile", "暂无数据", "空图表" → `add_empty_chart_v0`
- "action menu", "context menu", "dropdown menu", "more menu", "kebab menu", "action sheet", "下拉菜单", "操作菜单" → `add_action_menu_v0`
- "date picker", "date input", "date field", "due date", "picker closed", "日期选择器", "日期输入" → `add_date_picker_v0` (for the calendar grid shown after clicking, use `add_calendar_grid_v0`)
- "upload", "drop files here", "drag and drop", "file picker", "dropzone", "upload area", "上传区", "文件拖放" → `add_upload_dropzone_v0` (visually similar to empty-chart but semantically different — pick by intent)
- "OTP", "PIN code", "verification code", "2FA code", "6-digit code", "enter code", "验证码", "PIN 码" → `add_otp_input_v0`
- "attachment", "attached file", "uploaded file", "file item", "file list row", "附件", "已上传文件" → `add_attachment_row_v0` (for upload-in-progress state, compose `add_progress_bar_v0` below)
- "chat message", "message bubble", "conversation row", "iMessage bubble", "chat UI", "聊天气泡", "消息气泡" → `add_chat_bubble_v0` (side="left" for from-others, side="right" for from-self)

STILL use batch_design when:

- The row's items are structurally heterogeneous (can't be uniformly described by a single items[] shape)
- You need to build a larger composite (e.g. a whole section containing a scroll row + other content — build the section via batch_design, then insert the row via element tool with parent_id)
- Post-hoc styling: once the element tool has laid the structure, use batch_design U-ops to apply fills, typography, or theme variables

## Minimal usage

```
add_card_row_v0({
  items: [
    { title: "Hiit",     subtitle: "30 min", icon: "flame" },
    { title: "Strength", subtitle: "45 min", icon: "dumbbell" },
    { title: "Yoga",     subtitle: "25 min", icon: "leaf" },
  ],
})

add_metric_row_v0({
  items: [
    { label: "Steps",  value: "8,432",  icon: "activity" },
    { label: "Kcal",   value: "512",    icon: "flame" },
    { label: "Sleep",  value: "7h 24m", icon: "moon" },
  ],
})

add_nav_chip_row_v0({
  items: [
    { label: "All",     active: true },           // label-only chips OK
    { label: "Videos",  icon: "video" },
    { label: "Photos",  icon: "image" },
  ],
})

add_bottom_nav_v0({
  items: [
    { title: "Home",    icon: "home",    active: true },
    { title: "Search",  icon: "search" },
    { title: "Profile", icon: "user" },
  ],
})

add_activity_ring_v0({
  center_text: "8,432",
  size: 80,
  thickness: 8,
})

add_stat_grid_v0({
  items: [
    { value: "8,432", label: "Steps",  icon: "activity" },
    { value: "512",   label: "Kcal",   icon: "flame" },
    { value: "7h",    label: "Sleep",  icon: "moon" },
  ],
})

add_section_header_v0({
  title: "Recent Workouts",
  action: { label: "See all", icon: "arrow-right" },
})

add_top_nav_bar_v0({
  title: "Settings",
  leading_icon: "chevron-left",
  trailing_icon: "more-vertical",
})

add_icon_button_v0({
  icon: "search",
})

add_divider_v0({})                           // horizontal hairline (h=1 fill_container)
add_divider_v0({ orientation: "vertical" })  // vertical hairline

add_badge_v0({ label: "NEW" })

add_avatar_v0({ initial: "JD", size: 56 })   // with initial
add_avatar_v0({ size: 40 })                  // empty circle (fill via batch_design image later)

add_text_button_v0({ label: "Get Started" })
add_text_button_v0({ label: "Add item", leading_icon: "plus" })

add_heading_v0({ content: "Welcome back" })                   // defaults to h2 (24/600/1.2)
add_heading_v0({ content: "Hero Headline", level: "display" }) // 48/700/1.0/-0.5

add_body_text_v0({ content: "Lorem ipsum dolor sit amet…" })  // Inter + 1.5
add_body_text_v0({ content: "你好世界，这是一段中文正文。" })  // CJK: Inter + 1.6 + letterSpacing 0
// body ALWAYS Inter; only HEADINGS dispatch to Noto Sans SC/JP/KR.

add_icon_label_v0({ icon: "info", label: "Learn more" })

add_list_row_v0({
  title: "Notifications",
  subtitle: "Push, email, and in-app",
  leading_icon: "bell",
  trailing_icon: "chevron-right",
})

add_search_bar_v0({ placeholder: "Search workouts..." })

add_form_field_v0({ label: "Email", placeholder: "you@example.com", leading_icon: "mail", required: true })
add_form_field_v0({ label: "Password", leading_icon: "lock", trailing_icon: "eye", required: true })
add_textarea_v0({ label: "Bio", placeholder: "Tell us about yourself", rows: 5 })
add_textarea_v0({ label: "Feedback", rows: 4, required: true })
add_skeleton_v0({})                                  // default 3 rows, last short
add_skeleton_v0({ rows: 5, row_height: 20, row_gap: 8 })
add_select_v0({ label: "Country", value: "United States" })
add_select_v0({ label: "Currency", placeholder: "Choose currency", required: true })
add_chart_line_v0({ values: [2, 5, 3, 7, 4, 8, 6] })
add_chart_pie_v0({ values: [40, 30, 20, 10], diameter: 200 })
add_chart_pie_v0({ values: [1, 1, 1, 1], inner_radius_ratio: 0.5 })  // donut
add_image_placeholder_v0({ width: 320, height: 200, label: "Upload cover" })
add_video_placeholder_v0({ width: 320, height: 180, label: "Coming soon" })
add_comment_v0({ author: "Sarah", timestamp: "2h ago", body: "Looks great!", avatar_initial: "S" })
add_modal_shell_v0({ title: "Confirm delete", subtitle: "This cannot be undone." })
add_modal_shell_v1({ title: "Confirm delete", subtitle: "This cannot be undone.", theme: "dark" })
add_modal_shell_v1({ title: "Confirm delete", theme: "system" })      // $color-* refs; requires seeded palette
add_status_badge_v0({ label: "Online", tone: "success" })
add_status_badge_v0({ label: "Degraded", tone: "warning" })
add_spinner_v0({ size: 40 })
add_tooltip_v0({ text: "Click to delete" })
add_metric_comparison_v0({ label: "Revenue", value: "$12,480", change: "8%", trend: "up" })
add_notification_row_v0({ title: "New follower", body: "Alice started following you.", timestamp: "2m", unread: true, icon: "user-plus" })

add_switch_v0({})                          // off (default)
add_switch_v0({ active: true })             // on — iOS green

add_checkbox_v0({ label: "Accept terms" })
add_checkbox_v0({ label: "Done", checked: true })

add_radio_v0({ label: "Small" })
add_radio_v0({ label: "Medium", selected: true })

add_tabs_v0({
  items: [
    { label: "Overview", active: true },
    { label: "Details" },
    { label: "Reviews" },
  ],
})

add_segmented_control_v0({
  items: [
    { label: "Day" },
    { label: "Week", active: true },
    { label: "Month" },
  ],
})

add_empty_state_v0({
  title: "No items yet",
  subtitle: "Add one to get started",
  icon: "inbox",
  cta_label: "Create new",
})

add_alert_v0({ message: "Your changes are saved.", icon: "check", dismissible: true })

add_toast_v0({ message: "Copied to clipboard", icon: "check" })

add_progress_bar_v0({ value: 60 })               // default bar_width=240 → fill=144
add_progress_bar_v0({ value: 25, bar_width: 400 })

add_fab_v0({ icon: "plus" })                      // default 56×56
add_fab_v0({ icon: "edit", size: 40 })

add_breadcrumb_v0({
  items: [{ label: "Home" }, { label: "Settings" }, { label: "Billing" }],
})

add_stepper_v0({ total: 4, current: 1 })          // steps 1+2 done, 3+4 pending

add_rating_stars_v0({ filled: 4 })                           // 4/5 (default total)
add_carousel_dots_v0({ total: 5, current: 2 })
add_link_v0({ label: "Learn more", trailing_icon: "arrow-right" })
add_kbd_v0({ keys: ["Ctrl", "Shift", "P"] })                 // default "+" separator
add_price_v0({ amount: "29", period: "/month" })             // currency defaults to "$"
add_quote_block_v0({ quote: "Stay hungry.", author: "Steve Jobs" })
add_code_block_v0({ code: "const x = 1;", language: "typescript" })
add_color_swatch_v0({ color: "#2563EB", label: "Primary" })  // hex OR $ref both accepted

add_chart_bars_v0({ values: [4, 7, 3, 9, 5, 8, 6] })          // weekly steps skeleton
add_chart_bars_v0({ values: [10, 20], bar_width: 40, chart_height: 200 })

add_timeline_v0({
  items: [
    { title: "Order placed",   subtitle: "10:42 AM", active: true },
    { title: "Preparing" },
    { title: "Out for delivery" },
    { title: "Delivered" },
  ],
})

add_calendar_grid_v0({})                                       // vanilla 30-day month, Sun-start
add_calendar_grid_v0({ days_in_month: 31, start_day_offset: 2, today: 15, selected_day: 22 })

add_pagination_v0({ total: 10, current: 5 })                   // 1 … 4 [5] 6 … 10
add_pagination_v0({ total: 3, current: 1, show_arrows: false })  // no prev/next

add_faq_item_v0({ question: "Can I cancel anytime?" })                                 // collapsed
add_faq_item_v0({ question: "How do refunds work?", answer: "Email billing@…", expanded: true })

add_chip_input_v0({ label: "Tags", chips: ["design", "mobile", "a11y"], placeholder: "Add tag…" })
add_chip_input_v0({ label: "Send to", chips: [], placeholder: "Enter emails" })         // empty

add_empty_chart_v0({})                                                     // default 320×200 bar-chart-2 icon
add_empty_chart_v0({ icon: "line-chart", title: "No trends yet", subtitle: "Come back after 7 days" })

add_action_menu_v0({
  items: [
    { label: "Edit",   icon: "pencil" },
    { label: "Share",  icon: "share" },
    { label: "Report", icon: "flag",  divider_before: true },
    { label: "Delete", icon: "trash", destructive: true },
  ],
})

add_date_picker_v0({ label: "Due date" })                                               // placeholder state
add_date_picker_v0({ label: "Due date", value: "Jan 15, 2026", clearable: true })      // populated

add_upload_dropzone_v0({})                                                              // default 480×200 cloud icon
add_upload_dropzone_v0({ icon: "file-up", title: "Drop resume here", subtitle: "PDF or DOCX, max 5 MB" })

add_otp_input_v0({})                                                                    // 6 blank slots, first focused
add_otp_input_v0({ length: 6, digits: ["1","2","3"], focused_index: 3 })               // partial state, 4th slot focused
add_otp_input_v0({ length: 4, digits: ["1","2","3","4"] })                             // 4-digit PIN, all filled

add_attachment_row_v0({ filename: "report.pdf", size: "1.2 MB", icon: "file-text" })
add_attachment_row_v0({ filename: "sealed.zip", icon: "file-archive", removable: false })

add_chat_bubble_v0({ message: "Hi! How can I help?", author: "Support", timestamp: "Just now" })  // left (from-others)
add_chat_bubble_v0({ message: "My order hasn't arrived.", side: "right", timestamp: "2m" })      // right (from-self)
```

## Composition pattern

For a dashboard that needs a metric row inside a page:

1. Build the page structure via `batch_design` (root frame + section container) — note the section's id
2. Call `add_metric_row_v0({ parent_id: "<section-id>", items: [...] })` to insert the row under that section
3. Optional: a second `batch_design` U-op to style (fill, theme variables)

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
