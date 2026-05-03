---
name: elements-cookbook
description: Minimal-usage examples for every add_*_v0 element tool — arg-shape templates the A/B treatment prompt feeds to LLMs that don't have MCP tools/list (text-only callers). Pair with the elements skill for the decision tree + PREFER mappings; this file just shows what each tool's arguments look like end-to-end
phase: [generation]
trigger:
  flags: [hasMcpTools]
priority: 15
budget: 2400
category: base
---

<!--
  This skill is the arg-shape companion to `elements.md`. The decision
  tree + PREFER list in `elements.md` teaches WHICH tool to pick;
  this file shows WHAT the resulting `<op_tool>` payload looks like
  for every element tool in the family.

  Why a separate file: the combined content used to live in
  `elements.md` and it grew past the repo's 800-line per-file ceiling
  after the 81-90 batch shipped. Splitting keeps each file under the
  cap while preserving the LLM-facing context contract: callers that
  already auto-load `elements` via `hasMcpTools` will also auto-load
  this file (same flag, slightly later priority).

  External MCP clients can fetch each section explicitly via
  `get_design_prompt(section='elements-cookbook')`.
-->

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

add_stat_card_v0({ label: "Monthly revenue", value: "$12.4k", icon: "trending-up", delta: "+8% vs last week", trend: "up" })
add_stat_card_v0({ label: "Active users", value: "1,284", icon: "users" })                        // no delta = static snapshot

add_social_login_row_v0({ providers: [{ name: "Google" }, { name: "Apple" }, { name: "Microsoft" }] })  // stacked "Continue with X" buttons
add_social_login_row_v0({ providers: [{ name: "Google" }, { name: "GitHub" }, { name: "Slack" }], orientation: "horizontal" })  // compact icon-only row

add_pricing_card_v0({ tier: "Starter", price: "0", period: "/month", features: ["3 projects", "Community support"], cta: "Get started" })
add_pricing_card_v0({ tier: "Pro", price: "29", period: "/month", features: ["Unlimited projects", "Priority support", "Advanced analytics"], emphasis: "featured" })  // highlighted recommended tier
add_pricing_card_v0({ tier: "Enterprise", price: "Custom", features: ["Dedicated support", "SSO", "SLA"], cta: "Contact sales" })

add_toast_v1({ message: "Changes saved", icon: "check" })                     // default light = v0 parity (dark pill)
add_toast_v1({ message: "Changes saved", icon: "check", theme: "dark" })      // inverted light pill for dark-surface screens
add_toast_v1({ message: "Changes saved", theme: "system" })                   // $color-* refs — requires applySemanticPalette(doc) seeded

add_range_slider_v0({ value: 60, label: "Volume", show_value: true, value_suffix: "%" })
add_range_slider_v0({ value: 128, min: 0, max: 255, label: "Brightness", show_value: true })

add_empty_chart_v1({ icon: "line-chart" })                           // default light = v0 parity
add_empty_chart_v1({ icon: "pie-chart", theme: "dark" })              // dashboard dark-mode "no data" slot
add_empty_chart_v1({ icon: "bar-chart-2", theme: "system" })          // $color-* refs — requires applySemanticPalette(doc) seeded

add_phone_input_v0({ label: "Phone number", country_code: "+1", country_flag: "🇺🇸", required: true })
add_phone_input_v0({ country_code: "+86", country_flag: "🇨🇳", value: "138 0000 0000" })  // populated state

add_input_with_action_v0({ placeholder: "Enter your email", action_label: "Subscribe", leading_icon: "mail" })  // newsletter signup
add_input_with_action_v0({ placeholder: "Apply discount code", action_label: "Apply" })                          // checkout discount
add_input_with_action_v0({ placeholder: "Type a message…", action_kind: "icon", action_icon: "send" })          // chat composer

add_cookie_banner_v0({})                                                                                          // default GDPR banner
add_cookie_banner_v0({ show_settings_link: true })                                                                // with fine-grained consent link
add_cookie_banner_v0({ title: "Privacy choices", body: "We use cookies for analytics.", accept_label: "Allow", decline_label: "Decline" })

add_sidebar_nav_v0({
  title: "Acme",
  items: [
    { label: "Dashboard", icon: "layout-dashboard", active: true },
    { label: "Customers", icon: "users" },
    { label: "Orders",    icon: "shopping-cart" },
    { label: "Reports",   icon: "bar-chart-3" },
    { label: "Settings",  icon: "settings" },
  ],
})
add_sidebar_nav_v0({ items: [{ label: "Home", icon: "home", active: true }, { label: "Profile", icon: "user" }] })  // titleless minimal

add_avatar_group_v0({
  items: [
    { initial: "JD" },
    { initial: "SK" },
    { initial: "MN" },
    { initial: "AL" },
    { initial: "BR" },
    { initial: "CT" },
    { initial: "EF" },
  ],
  max_visible: 4,
})  // renders 4 ringed circles + "+3" overflow tile
add_avatar_group_v0({ items: [{ initial: "A" }, { initial: "B" }, { initial: "C" }], size: 24 })  // compact 3-up

add_data_table_row_v0({
  header: true,
  columns: [
    { content: "Customer" },
    { content: "Email" },
    { content: "Status" },
    { content: "Total" },
  ],
})
add_data_table_row_v0({
  columns: [
    { content: "Sarah Lee" },
    { content: "sarah@acme.com" },
    { content: "Active" },
    { content: "$1,240" },
  ],
})
add_data_table_row_v0({
  selected: true,
  columns: [
    { content: "Alex Park" },
    { content: "alex@acme.com" },
    { content: "Pending" },
    { content: "$680" },
  ],
})  // tinted hover/selected row

add_tag_v0({ label: "Status: Active" })                          // default tone, × visible
add_tag_v0({ label: "Plan: Pro", tone: "accent" })               // accent (blue) tone
add_tag_v0({ label: "Verified", tone: "success", removable: false })  // read-only success chip

add_user_card_v0({ name: "Sarah Lee", role: "Senior Engineer", initial: "SL" })

add_profile_header_v0({ name: "Sarah Lee", handle: "@sarah", bio: "Designer at Acme. Cat-mom of two.", initial: "SL" })

add_drawer_shell_v0({ title: "Edit project", side: "right", width: 480 })

add_combobox_v0({
  label: "Country",
  value: "Sing",
  options: [
    { label: "Singapore", highlighted: true },
    { label: "Sweden" },
    { label: "Switzerland" },
  ],
})

add_toolbar_v0({
  items: [
    { icon: "bold", active: true },
    { icon: "italic" },
    { icon: "underline", divider_after: true },
    { icon: "list" },
    { icon: "list-ordered" },
  ],
})

add_callout_v0({ tone: "info", title: "Heads up", body: "This action affects every team member." })
add_callout_v0({ tone: "warning", body: "Saving will overwrite the current draft." })

add_inline_action_v0({ message: "Comment deleted", action_label: "Undo", icon: "info" })

add_share_row_v0({
  targets: [
    { label: "Twitter", icon: "twitter" },
    { label: "Facebook", icon: "facebook" },
    { label: "Email", icon: "mail" },
    { label: "Copy", icon: "link" },
  ],
})

add_legend_item_v0({ label: "Revenue", color: "#2563EB", value: "$12,480" })

add_inbox_message_v0({
  from: "Stripe",
  subject: "Your weekly summary",
  preview: "Total volume rose 8.4% week-over-week.",
  timestamp: "10:42 AM",
  unread: true,
})

add_setting_row_v0({
  title: "Notifications",
  subtitle: "Push, email, in-app",
  leading_icon: "bell",
  trailing: { kind: "switch", on: true },
})

add_setting_row_v0({
  title: "Language",
  leading_icon: "globe",
  trailing: { kind: "value", value: "English" },
})

add_setting_row_v0({
  title: "What's new",
  trailing: { kind: "badge", value: "New" },
})

add_member_row_v0({
  name: "Sarah Lee",
  subtitle: "sarah@acme.com",
  initial: "SL",
  trailing: { kind: "role_badge", value: "Owner" },
})

add_member_row_v0({
  name: "Marcus Chen",
  subtitle: "Designer",
  initial: "MC",
  avatar_color: "#10B981",
  trailing: { kind: "status_dot", tone: "online" },
})

add_member_row_v0({
  name: "Priya Patel",
  subtitle: "priya@acme.com",
  initial: "PP",
  trailing: { kind: "menu" },
})

add_filter_group_v0({
  title: "Category",
  options: [
    { label: "Books", count: 42, selected: true },
    { label: "Music", count: 17 },
    { label: "Movies", count: 9 },
  ],
})

add_invite_row_v0({
  email: "sarah@acme.com",
  role: "Editor",
  status: "pending",
  action_label: "Resend",
})

add_invite_row_v0({
  email: "alex@acme.com",
  role: "Viewer",
  status: "expired",
  action_label: "Revoke",
})

add_activity_log_v0({
  actor: "Sarah Lee",
  action: "merged pull request #142",
  timestamp: "2h ago",
  icon: "git-merge",
  tone: "success",
})

add_activity_log_v0({
  actor: "Marcus Chen",
  action: "deleted the staging environment",
  timestamp: "Yesterday",
  icon: "trash-2",
  tone: "danger",
})

add_event_card_v0({
  month: "OCT",
  day: 15,
  title: "Design review",
  time: "2:00 PM – 3:00 PM",
  location: "Conference Room B",
})

add_step_card_v0({
  number: 1,
  title: "Connect your repo",
  description: "Authorize GitHub so we can read your commits and surface them in your dashboard.",
  completed: true,
})

add_step_card_v0({
  number: 2,
  title: "Invite teammates",
  description: "Add your team so they can collaborate on changes.",
})
```
