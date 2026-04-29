/**
 * Parity test fixtures: one canonical { args, build } entry per
 * pen-core element builder. Lives in a sibling .ts file (not .test.ts)
 * to keep the test file under the repo's 800-line ceiling — the test
 * file imports CASES and runs the actual assertions.
 *
 * vi.mock for @/canvas/canvas-text-measure lives in the importing
 * test file; vitest hoists it to module-resolution time, so build
 * calls in this file see the mocked module just like in-test calls.
 */

import {
  buildActionMenu,
  buildActivityRing,
  buildAlert,
  buildAvatar,
  buildBadge,
  buildBodyText,
  buildBottomNav,
  buildBreadcrumb,
  buildCalendarGrid,
  buildCardRow,
  buildCarouselDots,
  buildChartBars,
  buildChartLine,
  buildChartPie,
  buildCheckbox,
  buildChipInput,
  buildComment,
  buildCodeBlock,
  buildColorSwatch,
  buildCookieBanner,
  buildSidebarNav,
  buildAvatarGroup,
  buildDataTableRow,
  buildTag,
  buildUserCard,
  buildDrawerShell,
  buildCombobox,
  buildToolbar,
  buildCallout,
  buildShareRow,
  buildInlineAction,
  buildLegendItem,
  buildInboxMessage,
  buildProfileHeader,
  buildSettingRow,
  buildMemberRow,
  buildFilterGroup,
  buildInviteRow,
  buildActivityLog,
  buildEventCard,
  buildStepCard,
  buildDatePicker,
  buildDivider,
  buildEmptyChart,
  buildEmptyChartV1,
  buildEmptyState,
  buildFab,
  buildFaqItem,
  buildFormField,
  buildHeading,
  buildIconButton,
  buildIconLabel,
  buildInputWithAction,
  buildImagePlaceholder,
  buildVideoPlaceholder,
  buildModalShell,
  buildModalShellV1,
  buildUploadDropzone,
  buildOtpInput,
  buildAttachmentRow,
  buildChatBubble,
  buildSocialLoginRow,
  buildPricingCard,
  buildStatCard,
  buildKbd,
  buildLink,
  buildListRow,
  buildMetricComparison,
  buildMetricRow,
  buildNavChipRow,
  buildNotificationRow,
  buildPagination,
  buildPhoneInput,
  buildPrice,
  buildProgressBar,
  buildQuoteBlock,
  buildRadio,
  buildRangeSlider,
  buildRatingStars,
  buildSearchBar,
  buildSectionHeader,
  buildSegmentedControl,
  buildSelect,
  buildSkeleton,
  buildStatGrid,
  buildStatusBadge,
  buildSpinner,
  buildStepper,
  buildSwitch,
  buildTabs,
  buildTextarea,
  buildTextButton,
  buildTimeline,
  buildToast,
  buildToastV1,
  buildTooltip,
  buildTopNavBar,
  buildHeadingV1,
  buildCardRowV1,
  buildSettingRowV1,
  buildMemberRowV1,
  buildActivityLogV1,
  buildAvatarV1,
  buildBadgeV1,
  buildDividerV1,
  buildBodyTextV1,
  buildIconLabelV1,
  buildAlertV1,
  buildBottomNavV1,
  buildBreadcrumbV1,
  buildActivityRingV1,
  buildCarouselDotsV1,
  buildActionMenuV1,
  buildAttachmentRowV1,
  buildCalendarGridV1,
  buildAvatarGroupV1,
  buildCalloutV1,
} from '@zseven-w/pen-core';

export interface BuilderCase {
  toolName: string;
  args: Record<string, unknown>;
  build: (args: Record<string, unknown>) => unknown;
}
export const CASES: BuilderCase[] = [
  {
    toolName: 'add_card_row_v0',
    args: { items: [{ title: 'A' }, { title: 'B' }] },
    build: (a) => buildCardRow(a as unknown as Parameters<typeof buildCardRow>[0]),
  },
  {
    toolName: 'add_metric_row_v0',
    args: { items: [{ label: 'X', value: '1' }] },
    build: (a) => buildMetricRow(a as unknown as Parameters<typeof buildMetricRow>[0]),
  },
  {
    toolName: 'add_bottom_nav_v0',
    args: { items: [{ title: 'Home', icon: 'home' }] },
    build: (a) => buildBottomNav(a as unknown as Parameters<typeof buildBottomNav>[0]),
  },
  {
    toolName: 'add_section_header_v0',
    args: { title: 'Recent' },
    build: (a) => buildSectionHeader(a as unknown as Parameters<typeof buildSectionHeader>[0]),
  },
  {
    toolName: 'add_top_nav_bar_v0',
    args: { title: 'Home' },
    build: (a) => buildTopNavBar(a as unknown as Parameters<typeof buildTopNavBar>[0]),
  },
  {
    toolName: 'add_heading_v0',
    args: { content: 'Title' },
    build: (a) => buildHeading(a as unknown as Parameters<typeof buildHeading>[0]),
  },
  {
    toolName: 'add_body_text_v0',
    args: { content: 'Body text' },
    build: (a) => buildBodyText(a as unknown as Parameters<typeof buildBodyText>[0]),
  },
  {
    toolName: 'add_text_button_v0',
    args: { label: 'Go' },
    build: (a) => buildTextButton(a as unknown as Parameters<typeof buildTextButton>[0]),
  },
  {
    toolName: 'add_search_bar_v0',
    args: {},
    build: (a) => buildSearchBar(a as unknown as Parameters<typeof buildSearchBar>[0]),
  },
  {
    toolName: 'add_list_row_v0',
    args: { title: 'Item' },
    build: (a) => buildListRow(a as unknown as Parameters<typeof buildListRow>[0]),
  },
  {
    toolName: 'add_divider_v0',
    args: {},
    build: (a) => buildDivider(a as unknown as Parameters<typeof buildDivider>[0]),
  },
  {
    toolName: 'add_badge_v0',
    args: { label: 'NEW' },
    build: (a) => buildBadge(a as unknown as Parameters<typeof buildBadge>[0]),
  },
  {
    toolName: 'add_avatar_v0',
    args: { initial: 'JD' },
    build: (a) => buildAvatar(a as unknown as Parameters<typeof buildAvatar>[0]),
  },
  {
    toolName: 'add_icon_button_v0',
    args: { icon: 'search' },
    build: (a) => buildIconButton(a as unknown as Parameters<typeof buildIconButton>[0]),
  },
  {
    toolName: 'add_icon_label_v0',
    args: { icon: 'info', label: 'Info' },
    build: (a) => buildIconLabel(a as unknown as Parameters<typeof buildIconLabel>[0]),
  },
  {
    toolName: 'add_stat_grid_v0',
    args: { items: [{ value: '1', label: 'A' }] },
    build: (a) => buildStatGrid(a as unknown as Parameters<typeof buildStatGrid>[0]),
  },
  {
    toolName: 'add_switch_v0',
    args: { active: true },
    build: (a) => buildSwitch(a as unknown as Parameters<typeof buildSwitch>[0]),
  },
  {
    toolName: 'add_checkbox_v0',
    args: { label: 'A', checked: true },
    build: (a) => buildCheckbox(a as unknown as Parameters<typeof buildCheckbox>[0]),
  },
  {
    toolName: 'add_radio_v0',
    args: { label: 'A', selected: true },
    build: (a) => buildRadio(a as unknown as Parameters<typeof buildRadio>[0]),
  },
  {
    toolName: 'add_tabs_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) => buildTabs(a as unknown as Parameters<typeof buildTabs>[0]),
  },
  {
    toolName: 'add_segmented_control_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) =>
      buildSegmentedControl(a as unknown as Parameters<typeof buildSegmentedControl>[0]),
  },
  {
    toolName: 'add_empty_state_v0',
    args: { title: 'Empty' },
    build: (a) => buildEmptyState(a as unknown as Parameters<typeof buildEmptyState>[0]),
  },
  {
    toolName: 'add_alert_v0',
    args: { message: 'Saved' },
    build: (a) => buildAlert(a as unknown as Parameters<typeof buildAlert>[0]),
  },
  {
    toolName: 'add_toast_v0',
    args: { message: 'Copied' },
    build: (a) => buildToast(a as unknown as Parameters<typeof buildToast>[0]),
  },
  {
    toolName: 'add_progress_bar_v0',
    args: { value: 60 },
    build: (a) => buildProgressBar(a as unknown as Parameters<typeof buildProgressBar>[0]),
  },
  {
    toolName: 'add_fab_v0',
    args: { icon: 'plus' },
    build: (a) => buildFab(a as unknown as Parameters<typeof buildFab>[0]),
  },
  {
    toolName: 'add_breadcrumb_v0',
    args: { items: [{ label: 'A' }, { label: 'B' }] },
    build: (a) => buildBreadcrumb(a as unknown as Parameters<typeof buildBreadcrumb>[0]),
  },
  {
    toolName: 'add_stepper_v0',
    args: { total: 3, current: 1 },
    build: (a) => buildStepper(a as unknown as Parameters<typeof buildStepper>[0]),
  },
  {
    toolName: 'add_textarea_v0',
    args: { label: 'Bio', placeholder: 'Tell us about yourself', rows: 5 },
    build: (a) => buildTextarea(a as unknown as Parameters<typeof buildTextarea>[0]),
  },
  {
    toolName: 'add_skeleton_v0',
    args: { rows: 3 },
    build: (a) => buildSkeleton(a as unknown as Parameters<typeof buildSkeleton>[0]),
  },
  {
    toolName: 'add_select_v0',
    args: { label: 'Country', value: 'US' },
    build: (a) => buildSelect(a as unknown as Parameters<typeof buildSelect>[0]),
  },
  {
    toolName: 'add_chart_line_v0',
    args: { values: [1, 3, 2, 5, 4, 6] },
    build: (a) => buildChartLine(a as unknown as Parameters<typeof buildChartLine>[0]),
  },
  {
    toolName: 'add_chart_pie_v0',
    args: { values: [40, 30, 20, 10] },
    build: (a) => buildChartPie(a as unknown as Parameters<typeof buildChartPie>[0]),
  },
  {
    toolName: 'add_image_placeholder_v0',
    args: { width: 200, height: 140, label: 'Upload' },
    build: (a) =>
      buildImagePlaceholder(a as unknown as Parameters<typeof buildImagePlaceholder>[0]),
  },
  {
    toolName: 'add_video_placeholder_v0',
    args: { width: 320, height: 180, label: 'Coming soon' },
    build: (a) =>
      buildVideoPlaceholder(a as unknown as Parameters<typeof buildVideoPlaceholder>[0]),
  },
  {
    toolName: 'add_comment_v0',
    args: { author: 'Alice', body: 'Nice!', avatar_initial: 'A', timestamp: '2h ago' },
    build: (a) => buildComment(a as unknown as Parameters<typeof buildComment>[0]),
  },
  {
    toolName: 'add_modal_shell_v0',
    args: { title: 'Confirm', subtitle: 'Are you sure?' },
    build: (a) => buildModalShell(a as unknown as Parameters<typeof buildModalShell>[0]),
  },
  {
    toolName: 'add_status_badge_v0',
    args: { label: 'Online', tone: 'success' },
    build: (a) => buildStatusBadge(a as unknown as Parameters<typeof buildStatusBadge>[0]),
  },
  {
    toolName: 'add_spinner_v0',
    args: {},
    build: (a) => buildSpinner(a as unknown as Parameters<typeof buildSpinner>[0]),
  },
  {
    toolName: 'add_tooltip_v0',
    args: { text: 'Help' },
    build: (a) => buildTooltip(a as unknown as Parameters<typeof buildTooltip>[0]),
  },
  {
    toolName: 'add_metric_comparison_v0',
    args: { label: 'Revenue', value: '$12k', change: '8%', trend: 'up' },
    build: (a) =>
      buildMetricComparison(a as unknown as Parameters<typeof buildMetricComparison>[0]),
  },
  {
    toolName: 'add_notification_row_v0',
    args: { title: 'New follower', body: 'Alice is following you.', timestamp: '2m', unread: true },
    build: (a) => buildNotificationRow(a as unknown as Parameters<typeof buildNotificationRow>[0]),
  },
  {
    toolName: 'add_form_field_v0',
    args: { label: 'Email' },
    build: (a) => buildFormField(a as unknown as Parameters<typeof buildFormField>[0]),
  },
  {
    toolName: 'add_nav_chip_row_v0',
    args: { items: [{ label: 'A' }] },
    build: (a) => buildNavChipRow(a as unknown as Parameters<typeof buildNavChipRow>[0]),
  },
  {
    toolName: 'add_activity_ring_v0',
    args: { center_text: '50%' },
    build: (a) => buildActivityRing(a as unknown as Parameters<typeof buildActivityRing>[0]),
  },
  {
    toolName: 'add_rating_stars_v0',
    args: { filled: 4 },
    build: (a) => buildRatingStars(a as unknown as Parameters<typeof buildRatingStars>[0]),
  },
  {
    toolName: 'add_carousel_dots_v0',
    args: { total: 5, current: 2 },
    build: (a) => buildCarouselDots(a as unknown as Parameters<typeof buildCarouselDots>[0]),
  },
  {
    toolName: 'add_link_v0',
    args: { label: 'Read more' },
    build: (a) => buildLink(a as unknown as Parameters<typeof buildLink>[0]),
  },
  {
    toolName: 'add_kbd_v0',
    args: { keys: ['⌘', 'K'] },
    build: (a) => buildKbd(a as unknown as Parameters<typeof buildKbd>[0]),
  },
  {
    toolName: 'add_price_v0',
    args: { amount: '29' },
    build: (a) => buildPrice(a as unknown as Parameters<typeof buildPrice>[0]),
  },
  {
    toolName: 'add_quote_block_v0',
    args: { quote: 'Stay hungry.' },
    build: (a) => buildQuoteBlock(a as unknown as Parameters<typeof buildQuoteBlock>[0]),
  },
  {
    toolName: 'add_code_block_v0',
    args: { code: 'const x = 1;' },
    build: (a) => buildCodeBlock(a as unknown as Parameters<typeof buildCodeBlock>[0]),
  },
  {
    toolName: 'add_color_swatch_v0',
    args: { color: '#2563EB' },
    build: (a) => buildColorSwatch(a as unknown as Parameters<typeof buildColorSwatch>[0]),
  },
  {
    toolName: 'add_chart_bars_v0',
    args: { values: [1, 2, 3] },
    build: (a) => buildChartBars(a as unknown as Parameters<typeof buildChartBars>[0]),
  },
  {
    toolName: 'add_timeline_v0',
    args: { items: [{ title: 'A' }, { title: 'B' }] },
    build: (a) => buildTimeline(a as unknown as Parameters<typeof buildTimeline>[0]),
  },
  {
    toolName: 'add_calendar_grid_v0',
    args: {},
    build: (a) => buildCalendarGrid(a as unknown as Parameters<typeof buildCalendarGrid>[0]),
  },
  {
    toolName: 'add_pagination_v0',
    args: { total: 10, current: 5 },
    build: (a) => buildPagination(a as unknown as Parameters<typeof buildPagination>[0]),
  },
  {
    toolName: 'add_faq_item_v0',
    args: { question: 'How do I cancel?', answer: 'Email support.', expanded: true },
    build: (a) => buildFaqItem(a as unknown as Parameters<typeof buildFaqItem>[0]),
  },
  {
    toolName: 'add_chip_input_v0',
    args: { label: 'Tags', chips: ['design', 'mobile'], placeholder: 'Add tag…' },
    build: (a) => buildChipInput(a as unknown as Parameters<typeof buildChipInput>[0]),
  },
  {
    toolName: 'add_empty_chart_v0',
    args: { width: 320, height: 200, icon: 'line-chart', title: 'No data yet' },
    build: (a) => buildEmptyChart(a as unknown as Parameters<typeof buildEmptyChart>[0]),
  },
  {
    toolName: 'add_action_menu_v0',
    args: {
      items: [
        { label: 'Edit', icon: 'pencil' },
        { label: 'Share', icon: 'share' },
        { label: 'Delete', icon: 'trash', destructive: true, divider_before: true },
      ],
    },
    build: (a) => buildActionMenu(a as unknown as Parameters<typeof buildActionMenu>[0]),
  },
  {
    toolName: 'add_date_picker_v0',
    args: { label: 'Due date', value: 'Jan 15, 2026', clearable: true },
    build: (a) => buildDatePicker(a as unknown as Parameters<typeof buildDatePicker>[0]),
  },
  {
    toolName: 'add_modal_shell_v1',
    args: { title: 'Delete item?', subtitle: 'This cannot be undone.', theme: 'dark' },
    build: (a) => buildModalShellV1(a as unknown as Parameters<typeof buildModalShellV1>[0]),
  },
  {
    toolName: 'add_upload_dropzone_v0',
    args: { width: 480, title: 'Drop PDFs here', subtitle: 'Max 10 MB', icon: 'upload' },
    build: (a) => buildUploadDropzone(a as unknown as Parameters<typeof buildUploadDropzone>[0]),
  },
  {
    toolName: 'add_otp_input_v0',
    args: { length: 6, digits: ['1', '2', '3'], focused_index: 3 },
    build: (a) => buildOtpInput(a as unknown as Parameters<typeof buildOtpInput>[0]),
  },
  {
    toolName: 'add_attachment_row_v0',
    args: { filename: 'report.pdf', size: '1.2 MB', icon: 'file-text' },
    build: (a) => buildAttachmentRow(a as unknown as Parameters<typeof buildAttachmentRow>[0]),
  },
  {
    toolName: 'add_chat_bubble_v0',
    args: { message: 'Hello!', side: 'left', author: 'Sarah', timestamp: '2m' },
    build: (a) => buildChatBubble(a as unknown as Parameters<typeof buildChatBubble>[0]),
  },
  {
    toolName: 'add_stat_card_v0',
    args: {
      label: 'Monthly revenue',
      value: '$12.4k',
      icon: 'trending-up',
      delta: '+8%',
      trend: 'up',
    },
    build: (a) => buildStatCard(a as unknown as Parameters<typeof buildStatCard>[0]),
  },
  {
    toolName: 'add_social_login_row_v0',
    args: { providers: [{ name: 'google' }, { name: 'apple' }] },
    build: (a) => buildSocialLoginRow(a as unknown as Parameters<typeof buildSocialLoginRow>[0]),
  },
  {
    toolName: 'add_pricing_card_v0',
    args: {
      tier: 'Pro',
      price: '29',
      period: '/month',
      features: ['Unlimited projects', 'Priority support'],
      emphasis: 'featured',
    },
    build: (a) => buildPricingCard(a as unknown as Parameters<typeof buildPricingCard>[0]),
  },
  {
    toolName: 'add_toast_v1',
    args: { message: 'Copied to clipboard', icon: 'check', theme: 'dark' },
    build: (a) => buildToastV1(a as unknown as Parameters<typeof buildToastV1>[0]),
  },
  {
    toolName: 'add_range_slider_v0',
    args: { value: 40, label: 'Volume', show_value: true, value_suffix: '%' },
    build: (a) => buildRangeSlider(a as unknown as Parameters<typeof buildRangeSlider>[0]),
  },
  {
    toolName: 'add_empty_chart_v1',
    args: { width: 320, height: 200, icon: 'line-chart', title: 'No data yet', theme: 'dark' },
    build: (a) => buildEmptyChartV1(a as unknown as Parameters<typeof buildEmptyChartV1>[0]),
  },
  {
    toolName: 'add_phone_input_v0',
    args: { label: 'Phone number', country_code: '+1', country_flag: '🇺🇸', value: '555 123 4567' },
    build: (a) => buildPhoneInput(a as unknown as Parameters<typeof buildPhoneInput>[0]),
  },
  {
    toolName: 'add_input_with_action_v0',
    args: { placeholder: 'Enter your email', action_label: 'Subscribe', leading_icon: 'mail' },
    build: (a) => buildInputWithAction(a as unknown as Parameters<typeof buildInputWithAction>[0]),
  },
  {
    toolName: 'add_cookie_banner_v0',
    args: { title: 'We use cookies', show_settings_link: true },
    build: (a) => buildCookieBanner(a as unknown as Parameters<typeof buildCookieBanner>[0]),
  },
  {
    toolName: 'add_sidebar_nav_v0',
    args: {
      title: 'Acme',
      items: [
        { label: 'Dashboard', icon: 'home', active: true },
        { label: 'Settings', icon: 'settings' },
      ],
    },
    build: (a) => buildSidebarNav(a as unknown as Parameters<typeof buildSidebarNav>[0]),
  },
  {
    toolName: 'add_avatar_group_v0',
    args: {
      max_visible: 3,
      items: [
        { initial: 'JD' },
        { initial: 'SK' },
        { initial: 'MN' },
        { initial: 'AL' },
        { initial: 'BR' },
      ],
    },
    build: (a) => buildAvatarGroup(a as unknown as Parameters<typeof buildAvatarGroup>[0]),
  },
  {
    toolName: 'add_data_table_row_v0',
    args: {
      columns: [
        { content: 'Sarah' },
        { content: 'sarah@acme.com' },
        { content: 'Active' },
        { content: '$1,240' },
      ],
      selected: true,
    },
    build: (a) => buildDataTableRow(a as unknown as Parameters<typeof buildDataTableRow>[0]),
  },
  {
    toolName: 'add_tag_v0',
    args: { label: 'Status: Active', tone: 'accent' },
    build: (a) => buildTag(a as unknown as Parameters<typeof buildTag>[0]),
  },
  {
    toolName: 'add_user_card_v0',
    args: { name: 'Sarah Lee', role: 'Senior Engineer', initial: 'SL' },
    build: (a) => buildUserCard(a as unknown as Parameters<typeof buildUserCard>[0]),
  },
  {
    toolName: 'add_drawer_shell_v0',
    args: { title: 'Edit project', side: 'right' as const, width: 400 },
    build: (a) => buildDrawerShell(a as unknown as Parameters<typeof buildDrawerShell>[0]),
  },
  {
    toolName: 'add_combobox_v0',
    args: {
      label: 'Country',
      value: 'Sing',
      options: [{ label: 'Singapore', highlighted: true }, { label: 'Sweden' }],
    },
    build: (a) => buildCombobox(a as unknown as Parameters<typeof buildCombobox>[0]),
  },
  {
    toolName: 'add_toolbar_v0',
    args: {
      items: [
        { icon: 'bold', active: true },
        { icon: 'italic', divider_after: true },
        { icon: 'list' },
      ],
    },
    build: (a) => buildToolbar(a as unknown as Parameters<typeof buildToolbar>[0]),
  },
  {
    toolName: 'add_callout_v0',
    args: { tone: 'info' as const, title: 'Heads up', body: 'Action affects every team member.' },
    build: (a) => buildCallout(a as unknown as Parameters<typeof buildCallout>[0]),
  },
  {
    toolName: 'add_share_row_v0',
    args: {
      targets: [
        { label: 'Twitter', icon: 'twitter' },
        { label: 'Email', icon: 'mail' },
      ],
    },
    build: (a) => buildShareRow(a as unknown as Parameters<typeof buildShareRow>[0]),
  },
  {
    toolName: 'add_inline_action_v0',
    args: { message: 'Comment deleted', action_label: 'Undo', icon: 'info' },
    build: (a) => buildInlineAction(a as unknown as Parameters<typeof buildInlineAction>[0]),
  },
  {
    toolName: 'add_legend_item_v0',
    args: { label: 'Revenue', color: '#2563EB', value: '$12,480' },
    build: (a) => buildLegendItem(a as unknown as Parameters<typeof buildLegendItem>[0]),
  },
  {
    toolName: 'add_inbox_message_v0',
    args: {
      from: 'Stripe',
      subject: 'Your weekly summary',
      preview: 'Volume rose 8.4% week-over-week.',
      timestamp: '10:42 AM',
      unread: true,
    },
    build: (a) => buildInboxMessage(a as unknown as Parameters<typeof buildInboxMessage>[0]),
  },
  {
    toolName: 'add_profile_header_v0',
    args: { name: 'Sarah Lee', handle: '@sarah', bio: 'Designer at Acme.', initial: 'SL' },
    build: (a) => buildProfileHeader(a as unknown as Parameters<typeof buildProfileHeader>[0]),
  },
  {
    toolName: 'add_setting_row_v0',
    args: {
      title: 'Notifications',
      subtitle: 'Push, email, in-app',
      leading_icon: 'bell',
      trailing: { kind: 'switch', on: true },
    },
    build: (a) => buildSettingRow(a as unknown as Parameters<typeof buildSettingRow>[0]),
  },
  {
    toolName: 'add_member_row_v0',
    args: {
      name: 'Sarah Lee',
      subtitle: 'sarah@acme.com',
      initial: 'SL',
      trailing: { kind: 'role_badge', value: 'Owner' },
    },
    build: (a) => buildMemberRow(a as unknown as Parameters<typeof buildMemberRow>[0]),
  },
  {
    toolName: 'add_filter_group_v0',
    args: {
      title: 'Category',
      options: [
        { label: 'Books', count: 42, selected: true },
        { label: 'Music', count: 17 },
      ],
    },
    build: (a) => buildFilterGroup(a as unknown as Parameters<typeof buildFilterGroup>[0]),
  },
  {
    toolName: 'add_invite_row_v0',
    args: {
      email: 'sarah@acme.com',
      role: 'Editor',
      status: 'pending',
      action_label: 'Resend',
    },
    build: (a) => buildInviteRow(a as unknown as Parameters<typeof buildInviteRow>[0]),
  },
  {
    toolName: 'add_activity_log_v0',
    args: {
      actor: 'Sarah Lee',
      action: 'merged pull request #142',
      timestamp: '2h ago',
      icon: 'git-merge',
      tone: 'success',
    },
    build: (a) => buildActivityLog(a as unknown as Parameters<typeof buildActivityLog>[0]),
  },
  {
    toolName: 'add_event_card_v0',
    args: {
      month: 'OCT',
      day: 15,
      title: 'Design review',
      time: '2:00 PM – 3:00 PM',
      location: 'Conference Room B',
    },
    build: (a) => buildEventCard(a as unknown as Parameters<typeof buildEventCard>[0]),
  },
  {
    toolName: 'add_step_card_v0',
    args: {
      number: 1,
      title: 'Connect your repo',
      description: 'Authorize GitHub so we can read your commits.',
      completed: true,
    },
    build: (a) => buildStepCard(a as unknown as Parameters<typeof buildStepCard>[0]),
  },
  {
    toolName: 'add_heading_v1',
    args: { content: 'Welcome', level: 'h1' },
    build: (a) => buildHeadingV1(a as unknown as Parameters<typeof buildHeadingV1>[0]),
  },
  {
    toolName: 'add_card_row_v1',
    args: { items: [{ title: 'Card A', subtitle: 'Sub' }], theme: 'dark' },
    build: (a) => buildCardRowV1(a as unknown as Parameters<typeof buildCardRowV1>[0]),
  },
  {
    toolName: 'add_setting_row_v1',
    args: { title: 'Notifications', leading_icon: 'bell', theme: 'dark' },
    build: (a) => buildSettingRowV1(a as unknown as Parameters<typeof buildSettingRowV1>[0]),
  },
  {
    toolName: 'add_member_row_v1',
    args: {
      name: 'Sarah Lee',
      subtitle: 'sarah@acme.com',
      initial: 'SL',
      trailing: { kind: 'role_badge', value: 'Owner' },
      theme: 'dark',
    },
    build: (a) => buildMemberRowV1(a as unknown as Parameters<typeof buildMemberRowV1>[0]),
  },
  {
    toolName: 'add_activity_log_v1',
    args: {
      actor: 'Sarah Lee',
      action: 'merged pull request #142',
      timestamp: '2h ago',
      icon: 'git-merge',
      tone: 'success',
      theme: 'dark',
    },
    build: (a) => buildActivityLogV1(a as unknown as Parameters<typeof buildActivityLogV1>[0]),
  },
  {
    toolName: 'add_avatar_v1',
    args: { initial: 'SL', size: 48, theme: 'light' },
    build: (a) => buildAvatarV1(a as unknown as Parameters<typeof buildAvatarV1>[0]),
  },
  {
    toolName: 'add_badge_v1',
    args: { label: 'NEW', theme: 'light' },
    build: (a) => buildBadgeV1(a as unknown as Parameters<typeof buildBadgeV1>[0]),
  },
  {
    toolName: 'add_divider_v1',
    args: { orientation: 'horizontal', thickness: 1, theme: 'light' },
    build: (a) => buildDividerV1(a as unknown as Parameters<typeof buildDividerV1>[0]),
  },
  {
    toolName: 'add_body_text_v1',
    args: { content: 'The quick brown fox jumps over the lazy dog.', theme: 'light' },
    build: (a) => buildBodyTextV1(a as unknown as Parameters<typeof buildBodyTextV1>[0]),
  },
  {
    toolName: 'add_icon_label_v1',
    args: { icon: 'star', label: 'Favorites', theme: 'light' },
    build: (a) => buildIconLabelV1(a as unknown as Parameters<typeof buildIconLabelV1>[0]),
  },
  {
    toolName: 'add_alert_v1',
    args: { message: 'This is an alert', theme: 'light' },
    build: (a) => buildAlertV1(a as unknown as Parameters<typeof buildAlertV1>[0]),
  },
  {
    toolName: 'add_bottom_nav_v1',
    args: {
      items: [
        { title: 'Home', icon: 'home', active: true },
        { title: 'Search', icon: 'search' },
      ],
      theme: 'light',
    },
    build: (a) => buildBottomNavV1(a as unknown as Parameters<typeof buildBottomNavV1>[0]),
  },
  {
    toolName: 'add_breadcrumb_v1',
    args: { items: [{ label: 'Home' }, { label: 'Settings' }], theme: 'light' },
    build: (a) => buildBreadcrumbV1(a as unknown as Parameters<typeof buildBreadcrumbV1>[0]),
  },
  {
    toolName: 'add_activity_ring_v1',
    args: { center_text: '72%', theme: 'light' },
    build: (a) => buildActivityRingV1(a as unknown as Parameters<typeof buildActivityRingV1>[0]),
  },
  {
    toolName: 'add_carousel_dots_v1',
    args: { total: 5, current: 2, theme: 'light' },
    build: (a) => buildCarouselDotsV1(a as unknown as Parameters<typeof buildCarouselDotsV1>[0]),
  },
  {
    toolName: 'add_action_menu_v1',
    args: { items: [{ label: 'Edit' }, { label: 'Delete', destructive: true }], theme: 'light' },
    build: (a) => buildActionMenuV1(a as unknown as Parameters<typeof buildActionMenuV1>[0]),
  },
  {
    toolName: 'add_attachment_row_v1',
    args: { filename: 'document.pdf', size: '1.2 MB', theme: 'light' },
    build: (a) => buildAttachmentRowV1(a as unknown as Parameters<typeof buildAttachmentRowV1>[0]),
  },
  {
    toolName: 'add_calendar_grid_v1',
    args: { days_in_month: 30, today: 15, theme: 'light' },
    build: (a) => buildCalendarGridV1(a as unknown as Parameters<typeof buildCalendarGridV1>[0]),
  },
  {
    toolName: 'add_avatar_group_v1',
    args: { items: [{ initial: 'A' }, { initial: 'B' }], theme: 'light' },
    build: (a) => buildAvatarGroupV1(a as unknown as Parameters<typeof buildAvatarGroupV1>[0]),
  },
  {
    toolName: 'add_callout_v1',
    args: { body: 'This is a callout note.', tone: 'info', theme: 'light' },
    build: (a) => buildCalloutV1(a as unknown as Parameters<typeof buildCalloutV1>[0]),
  },
];
