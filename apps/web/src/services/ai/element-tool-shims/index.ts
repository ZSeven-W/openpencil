/**
 * Client-side element-tool shims — Phase 2 M2.
 *
 * Each shim is a pure tree-build function matching its pen-mcp
 * `add_*_v0` counterpart. Both sides delegate to the same builders
 * in `@zseven-w/pen-core/element-builders`, so the shape produced
 * by `apps/web` and by external MCP clients (pen-mcp HTTP) is
 * byte-identical — drift is impossible by construction.
 *
 * Meta parameters (`parent_id`, `pageId`) that appear alongside
 * tool-specific args in the `<op_tool>` payload are stripped BEFORE
 * invoking the builder (builder signatures only accept their own
 * params) and re-surfaced on the shim's return shape so the
 * dispatcher can honor them during insert. Without this split the
 * shim would silently drop parent_id / pageId and return `applied`
 * even when the AI explicitly named a target container — that's
 * the 2026-04-21 stop-hook regression that's now guarded by tests.
 */

import {
  assignIdsRecursively,
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
  buildDatePicker,
  buildDivider,
  buildEmptyChart,
  buildEmptyState,
  buildFab,
  buildFaqItem,
  buildFormField,
  buildHeading,
  buildHeadingV1,
  buildIconButton,
  buildIconLabel,
  buildImagePlaceholder,
  buildVideoPlaceholder,
  buildModalShell,
  buildModalShellV1,
  buildUploadDropzone,
  buildOtpInput,
  buildAttachmentRow,
  buildChatBubble,
  buildStatCard,
  buildSocialLoginRow,
  buildPricingCard,
  buildToastV1,
  buildRangeSlider,
  buildEmptyChartV1,
  buildPhoneInput,
  buildInputWithAction,
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
  buildKbd,
  buildLink,
  buildListRow,
  buildMetricComparison,
  buildMetricRow,
  buildNavChipRow,
  buildNotificationRow,
  buildPagination,
  buildPrice,
  buildProgressBar,
  buildQuoteBlock,
  buildRadio,
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
  buildTooltip,
  buildTopNavBar,
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
  buildChartBarsV1,
  buildChartLineV1,
  buildChartPieV1,
  buildChatBubbleV1,
  buildCheckboxV1,
  buildChipInputV1,
  buildCodeBlockV1,
  buildColorSwatchV1,
  buildComboboxV1,
  buildCommentV1,
  buildCookieBannerV1,
  buildDataTableRowV1,
  buildDatePickerV1,
  buildDrawerShellV1,
  buildEmptyStateV1,
  buildEventCardV1,
  buildFabV1,
  buildFaqItemV1,
  buildFilterGroupV1,
  buildFormFieldV1,
  type ActionMenuParams,
  type ActivityRingParams,
  type AlertParams,
  type AvatarParams,
  type BadgeParams,
  type BodyTextParams,
  type BottomNavParams,
  type BreadcrumbParams,
  type CalendarGridParams,
  type CardRowParams,
  type CarouselDotsParams,
  type ChartBarsParams,
  type ChartLineParams,
  type ChartPieParams,
  type CheckboxParams,
  type ChipInputParams,
  type CommentParams,
  type CodeBlockParams,
  type ColorSwatchParams,
  type DatePickerParams,
  type DividerParams,
  type ElementTree,
  type EmptyChartParams,
  type EmptyStateParams,
  type FabParams,
  type FaqItemParams,
  type FormFieldParams,
  type HeadingParams,
  type HeadingV1Params,
  type IconButtonParams,
  type IconLabelParams,
  type ImagePlaceholderParams,
  type VideoPlaceholderParams,
  type ModalShellParams,
  type ModalShellV1Params,
  type UploadDropzoneParams,
  type OtpInputParams,
  type AttachmentRowParams,
  type ChatBubbleParams,
  type StatCardParams,
  type SocialLoginRowParams,
  type PricingCardParams,
  type ToastV1Params,
  type RangeSliderParams,
  type EmptyChartV1Params,
  type PhoneInputParams,
  type InputWithActionParams,
  type CookieBannerParams,
  type SidebarNavParams,
  type AvatarGroupParams,
  type DataTableRowParams,
  type TagParams,
  type UserCardParams,
  type DrawerShellParams,
  type ComboboxParams,
  type ToolbarParams,
  type CalloutParams,
  type ShareRowParams,
  type InlineActionParams,
  type LegendItemParams,
  type InboxMessageParams,
  type ProfileHeaderParams,
  type SettingRowParams,
  type MemberRowParams,
  type FilterGroupParams,
  type InviteRowParams,
  type ActivityLogParams,
  type EventCardParams,
  type StepCardParams,
  type CardRowV1Params,
  type SettingRowV1Params,
  type MemberRowV1Params,
  type ActivityLogV1Params,
  type AvatarV1Params,
  type BadgeV1Params,
  type DividerV1Params,
  type BodyTextV1Params,
  type IconLabelV1Params,
  type AlertV1Params,
  type BottomNavV1Params,
  type BreadcrumbV1Params,
  type ActivityRingV1Params,
  type CarouselDotsV1Params,
  type ActionMenuV1Params,
  type AttachmentRowV1Params,
  type CalendarGridV1Params,
  type AvatarGroupV1Params,
  type CalloutV1Params,
  type ChartBarsV1Params,
  type ChartLineV1Params,
  type ChartPieV1Params,
  type ChatBubbleV1Params,
  type CheckboxV1Params,
  type ChipInputV1Params,
  type CodeBlockV1Params,
  type ColorSwatchV1Params,
  type ComboboxV1Params,
  type CommentV1Params,
  type CookieBannerV1Params,
  type DataTableRowV1Params,
  type DatePickerV1Params,
  type DrawerShellV1Params,
  type EmptyStateV1Params,
  type EventCardV1Params,
  type FabV1Params,
  type FaqItemV1Params,
  type FilterGroupV1Params,
  type FormFieldV1Params,
  type KbdParams,
  type LinkParams,
  type ListRowParams,
  type MetricComparisonParams,
  type MetricRowParams,
  type NavChipRowParams,
  type NotificationRowParams,
  type PaginationParams,
  type PriceParams,
  type ProgressBarParams,
  type QuoteBlockParams,
  type RadioParams,
  type RatingStarsParams,
  type SearchBarParams,
  type SectionHeaderParams,
  type SegmentedControlParams,
  type SelectParams,
  type SkeletonParams,
  type StatGridParams,
  type StatusBadgeParams,
  type SpinnerParams,
  type StepperParams,
  type SwitchParams,
  type TabsParams,
  type TextareaParams,
  type TextButtonParams,
  type TimelineParams,
  type ToastParams,
  type TooltipParams,
  type TopNavBarParams,
} from '@zseven-w/pen-core';
import type { PenNode } from '@/types/pen';

/**
 * Result of running a client shim. The dispatcher owns the insert
 * step so shims stay pure (pen-core builder + id stamping only).
 */
export interface ElementShimResult {
  /** Root node of the subtree — already id-stamped by the shim. */
  node: PenNode;
  /**
   * Target parent node id, from `args.parent_id` if present.
   * `null` means "insert at page root" (matches document-store.addNode
   * convention). Caller validates parent existence before applying.
   */
  parentId: string | null;
  /**
   * Target page id, from `args.pageId` if present. `null` means
   * "use the currently active page". Currently only honored in the
   * HTTP fallback; the client shim path always targets the active
   * page (document-store doesn't expose cross-page insert). Caller
   * validates presence + returns a failed result if the pageId
   * diverges from the active one so we never silently drop it.
   */
  pageId: string | null;
  /**
   * Target .op file path, from `args.filePath` if present. `null`
   * means "live canvas (document-store / mcp-sync-state)". The
   * browser-side path has no file I/O surface — `addNode` only
   * writes to the in-memory document-store. When a caller names
   * a filePath they want pen-mcp's file-aware handler, not the
   * client shim. Caller must route to pen-mcp stdio transport
   * (external MCP clients) or return `failed` with a clear hint
   * — never silently drop it and report `applied`.
   */
  filePath: string | null;
}

export type ElementShim = (args: Record<string, unknown>) => ElementShimResult;

/**
 * pen-mcp's `document-manager.ts::resolveDocPath` treats both
 * `undefined` and the literal string `"live://canvas"` as the live
 * canvas target. The shim must accept the sentinel the same way —
 * rejecting it as "unsupported filePath" would be wrong because it
 * IS the default. Only genuine filesystem paths (anything else that
 * looks like a string) go through the filePath-rejection branch.
 */
const LIVE_CANVAS_SENTINEL = 'live://canvas';

/**
 * Wrap a pen-core builder as an ElementShim. Splits meta parameters
 * (`parent_id`, `pageId`, `filePath`) out of the payload before
 * invoking the builder — the builder only knows tool-specific fields
 * and would reject / misbehave if handed `parent_id` as an unknown
 * param. Meta parameters are re-surfaced on the shim's return shape
 * so the dispatcher can honor them during insert (or reject with a
 * clear diagnostic if it cannot).
 */
function wrap<TParams>(build: (p: TParams) => ElementTree): ElementShim {
  return (args) => {
    const source = (args ?? {}) as Record<string, unknown>;
    const { parent_id, pageId, filePath, ...rest } = source as {
      parent_id?: unknown;
      pageId?: unknown;
      filePath?: unknown;
    };
    const tree = build(rest as TParams);
    assignIdsRecursively(tree);
    // Normalize the live-canvas sentinel to null so the dispatcher's
    // "filePath !== null → fail" branch only fires for real file paths.
    const rawFilePath = typeof filePath === 'string' && filePath.length > 0 ? filePath : null;
    return {
      node: tree as unknown as PenNode,
      parentId: typeof parent_id === 'string' && parent_id.length > 0 ? parent_id : null,
      pageId: typeof pageId === 'string' && pageId.length > 0 ? pageId : null,
      filePath: rawFilePath === LIVE_CANVAS_SENTINEL ? null : rawFilePath,
    };
  };
}

/**
 * Shim registry. Keys match pen-mcp element-tool names verbatim so
 * the dispatcher can look up by the `name` field of a parsed
 * `<op_tool>` tag directly. Covers the 10 most common tools from
 * A/B v1 routing data (openpencil-docs/superpowers/notes/
 * 2026-04-20-ab-v1-results.md); more can be added as builders are
 * extracted into pen-core's element-builders module.
 */
export const ELEMENT_SHIMS: Record<string, ElementShim> = {
  add_card_row_v0: wrap<CardRowParams>(buildCardRow),
  add_metric_row_v0: wrap<MetricRowParams>(buildMetricRow),
  add_bottom_nav_v0: wrap<BottomNavParams>(buildBottomNav),
  add_section_header_v0: wrap<SectionHeaderParams>(buildSectionHeader),
  add_top_nav_bar_v0: wrap<TopNavBarParams>(buildTopNavBar),
  add_heading_v0: wrap<HeadingParams>(buildHeading),
  add_heading_v1: wrap<HeadingV1Params>(buildHeadingV1),
  add_body_text_v0: wrap<BodyTextParams>(buildBodyText),
  add_text_button_v0: wrap<TextButtonParams>(buildTextButton),
  add_search_bar_v0: wrap<SearchBarParams>(buildSearchBar),
  add_list_row_v0: wrap<ListRowParams>(buildListRow),
  add_divider_v0: wrap<DividerParams>(buildDivider),
  add_badge_v0: wrap<BadgeParams>(buildBadge),
  add_avatar_v0: wrap<AvatarParams>(buildAvatar),
  add_icon_button_v0: wrap<IconButtonParams>(buildIconButton),
  add_icon_label_v0: wrap<IconLabelParams>(buildIconLabel),
  add_stat_grid_v0: wrap<StatGridParams>(buildStatGrid),
  add_switch_v0: wrap<SwitchParams>(buildSwitch),
  add_checkbox_v0: wrap<CheckboxParams>(buildCheckbox),
  add_radio_v0: wrap<RadioParams>(buildRadio),
  add_tabs_v0: wrap<TabsParams>(buildTabs),
  add_segmented_control_v0: wrap<SegmentedControlParams>(buildSegmentedControl),
  add_empty_state_v0: wrap<EmptyStateParams>(buildEmptyState),
  add_alert_v0: wrap<AlertParams>(buildAlert),
  add_toast_v0: wrap<ToastParams>(buildToast),
  add_progress_bar_v0: wrap<ProgressBarParams>(buildProgressBar),
  add_fab_v0: wrap<FabParams>(buildFab),
  add_breadcrumb_v0: wrap<BreadcrumbParams>(buildBreadcrumb),
  add_stepper_v0: wrap<StepperParams>(buildStepper),
  add_form_field_v0: wrap<FormFieldParams>(buildFormField),
  add_textarea_v0: wrap<TextareaParams>(buildTextarea),
  add_skeleton_v0: wrap<SkeletonParams>(buildSkeleton),
  add_select_v0: wrap<SelectParams>(buildSelect),
  add_chart_line_v0: wrap<ChartLineParams>(buildChartLine),
  add_chart_pie_v0: wrap<ChartPieParams>(buildChartPie),
  add_image_placeholder_v0: wrap<ImagePlaceholderParams>(buildImagePlaceholder),
  add_video_placeholder_v0: wrap<VideoPlaceholderParams>(buildVideoPlaceholder),
  add_comment_v0: wrap<CommentParams>(buildComment),
  add_modal_shell_v0: wrap<ModalShellParams>(buildModalShell),
  add_status_badge_v0: wrap<StatusBadgeParams>(buildStatusBadge),
  add_spinner_v0: wrap<SpinnerParams>(buildSpinner),
  add_tooltip_v0: wrap<TooltipParams>(buildTooltip),
  add_metric_comparison_v0: wrap<MetricComparisonParams>(buildMetricComparison),
  add_notification_row_v0: wrap<NotificationRowParams>(buildNotificationRow),
  add_nav_chip_row_v0: wrap<NavChipRowParams>(buildNavChipRow),
  add_activity_ring_v0: wrap<ActivityRingParams>(buildActivityRing),
  add_rating_stars_v0: wrap<RatingStarsParams>(buildRatingStars),
  add_carousel_dots_v0: wrap<CarouselDotsParams>(buildCarouselDots),
  add_link_v0: wrap<LinkParams>(buildLink),
  add_kbd_v0: wrap<KbdParams>(buildKbd),
  add_price_v0: wrap<PriceParams>(buildPrice),
  add_quote_block_v0: wrap<QuoteBlockParams>(buildQuoteBlock),
  add_code_block_v0: wrap<CodeBlockParams>(buildCodeBlock),
  add_color_swatch_v0: wrap<ColorSwatchParams>(buildColorSwatch),
  add_chart_bars_v0: wrap<ChartBarsParams>(buildChartBars),
  add_timeline_v0: wrap<TimelineParams>(buildTimeline),
  add_calendar_grid_v0: wrap<CalendarGridParams>(buildCalendarGrid),
  add_pagination_v0: wrap<PaginationParams>(buildPagination),
  add_faq_item_v0: wrap<FaqItemParams>(buildFaqItem),
  add_chip_input_v0: wrap<ChipInputParams>(buildChipInput),
  add_empty_chart_v0: wrap<EmptyChartParams>(buildEmptyChart),
  add_action_menu_v0: wrap<ActionMenuParams>(buildActionMenu),
  add_date_picker_v0: wrap<DatePickerParams>(buildDatePicker),
  add_modal_shell_v1: wrap<ModalShellV1Params>(buildModalShellV1),
  add_upload_dropzone_v0: wrap<UploadDropzoneParams>(buildUploadDropzone),
  add_otp_input_v0: wrap<OtpInputParams>(buildOtpInput),
  add_attachment_row_v0: wrap<AttachmentRowParams>(buildAttachmentRow),
  add_chat_bubble_v0: wrap<ChatBubbleParams>(buildChatBubble),
  add_stat_card_v0: wrap<StatCardParams>(buildStatCard),
  add_social_login_row_v0: wrap<SocialLoginRowParams>(buildSocialLoginRow),
  add_pricing_card_v0: wrap<PricingCardParams>(buildPricingCard),
  add_toast_v1: wrap<ToastV1Params>(buildToastV1),
  add_range_slider_v0: wrap<RangeSliderParams>(buildRangeSlider),
  add_empty_chart_v1: wrap<EmptyChartV1Params>(buildEmptyChartV1),
  add_phone_input_v0: wrap<PhoneInputParams>(buildPhoneInput),
  add_input_with_action_v0: wrap<InputWithActionParams>(buildInputWithAction),
  add_cookie_banner_v0: wrap<CookieBannerParams>(buildCookieBanner),
  add_sidebar_nav_v0: wrap<SidebarNavParams>(buildSidebarNav),
  add_avatar_group_v0: wrap<AvatarGroupParams>(buildAvatarGroup),
  add_data_table_row_v0: wrap<DataTableRowParams>(buildDataTableRow),
  add_tag_v0: wrap<TagParams>(buildTag),
  add_user_card_v0: wrap<UserCardParams>(buildUserCard),
  add_drawer_shell_v0: wrap<DrawerShellParams>(buildDrawerShell),
  add_combobox_v0: wrap<ComboboxParams>(buildCombobox),
  add_toolbar_v0: wrap<ToolbarParams>(buildToolbar),
  add_callout_v0: wrap<CalloutParams>(buildCallout),
  add_share_row_v0: wrap<ShareRowParams>(buildShareRow),
  add_inline_action_v0: wrap<InlineActionParams>(buildInlineAction),
  add_legend_item_v0: wrap<LegendItemParams>(buildLegendItem),
  add_inbox_message_v0: wrap<InboxMessageParams>(buildInboxMessage),
  add_profile_header_v0: wrap<ProfileHeaderParams>(buildProfileHeader),
  add_setting_row_v0: wrap<SettingRowParams>(buildSettingRow),
  add_member_row_v0: wrap<MemberRowParams>(buildMemberRow),
  add_filter_group_v0: wrap<FilterGroupParams>(buildFilterGroup),
  add_invite_row_v0: wrap<InviteRowParams>(buildInviteRow),
  add_activity_log_v0: wrap<ActivityLogParams>(buildActivityLog),
  add_event_card_v0: wrap<EventCardParams>(buildEventCard),
  add_step_card_v0: wrap<StepCardParams>(buildStepCard),
  add_card_row_v1: wrap<CardRowV1Params>(buildCardRowV1),
  add_setting_row_v1: wrap<SettingRowV1Params>(buildSettingRowV1),
  add_member_row_v1: wrap<MemberRowV1Params>(buildMemberRowV1),
  add_activity_log_v1: wrap<ActivityLogV1Params>(buildActivityLogV1),
  add_avatar_v1: wrap<AvatarV1Params>(buildAvatarV1),
  add_badge_v1: wrap<BadgeV1Params>(buildBadgeV1),
  add_divider_v1: wrap<DividerV1Params>(buildDividerV1),
  add_body_text_v1: wrap<BodyTextV1Params>(buildBodyTextV1),
  add_icon_label_v1: wrap<IconLabelV1Params>(buildIconLabelV1),
  add_alert_v1: wrap<AlertV1Params>(buildAlertV1),
  add_bottom_nav_v1: wrap<BottomNavV1Params>(buildBottomNavV1),
  add_breadcrumb_v1: wrap<BreadcrumbV1Params>(buildBreadcrumbV1),
  add_activity_ring_v1: wrap<ActivityRingV1Params>(buildActivityRingV1),
  add_carousel_dots_v1: wrap<CarouselDotsV1Params>(buildCarouselDotsV1),
  add_action_menu_v1: wrap<ActionMenuV1Params>(buildActionMenuV1),
  add_attachment_row_v1: wrap<AttachmentRowV1Params>(buildAttachmentRowV1),
  add_calendar_grid_v1: wrap<CalendarGridV1Params>(buildCalendarGridV1),
  add_avatar_group_v1: wrap<AvatarGroupV1Params>(buildAvatarGroupV1),
  add_callout_v1: wrap<CalloutV1Params>(buildCalloutV1),
  add_chart_bars_v1: wrap<ChartBarsV1Params>(buildChartBarsV1),
  add_chart_line_v1: wrap<ChartLineV1Params>(buildChartLineV1),
  add_chart_pie_v1: wrap<ChartPieV1Params>(buildChartPieV1),
  add_chat_bubble_v1: wrap<ChatBubbleV1Params>(buildChatBubbleV1),
  add_checkbox_v1: wrap<CheckboxV1Params>(buildCheckboxV1),
  add_chip_input_v1: wrap<ChipInputV1Params>(buildChipInputV1),
  add_code_block_v1: wrap<CodeBlockV1Params>(buildCodeBlockV1),
  add_color_swatch_v1: wrap<ColorSwatchV1Params>(buildColorSwatchV1),
  add_combobox_v1: wrap<ComboboxV1Params>(buildComboboxV1),
  add_comment_v1: wrap<CommentV1Params>(buildCommentV1),
  add_cookie_banner_v1: wrap<CookieBannerV1Params>(buildCookieBannerV1),
  add_data_table_row_v1: wrap<DataTableRowV1Params>(buildDataTableRowV1),
  add_date_picker_v1: wrap<DatePickerV1Params>(buildDatePickerV1),
  add_drawer_shell_v1: wrap<DrawerShellV1Params>(buildDrawerShellV1),
  add_empty_state_v1: wrap<EmptyStateV1Params>(buildEmptyStateV1),
  add_event_card_v1: wrap<EventCardV1Params>(buildEventCardV1),
  add_fab_v1: wrap<FabV1Params>(buildFabV1),
  add_faq_item_v1: wrap<FaqItemV1Params>(buildFaqItemV1),
  add_filter_group_v1: wrap<FilterGroupV1Params>(buildFilterGroupV1),
  add_form_field_v1: wrap<FormFieldV1Params>(buildFormFieldV1),
};

export function getElementShim(name: string): ElementShim | undefined {
  return ELEMENT_SHIMS[name];
}

export function hasElementShim(name: string): boolean {
  return name in ELEMENT_SHIMS;
}

/**
 * Canonical list of element-tool names the embedded orchestrator
 * can actually execute — shim set + Nitro SERVER_BUILDERS set
 * (which are kept byte-identical by convention; extend this list,
 * the SHIM registry, and `apps/web/server/api/mcp/exec-tool.post.ts`
 * together).
 *
 * Dispatcher uses this to short-circuit on tool names that are in
 * `elements.md`'s 42-tool catalog but NOT wired into embedded
 * runtime, avoiding a wasted HTTP roundtrip and giving the caller
 * a more actionable "not supported yet" message instead of a raw
 * 404 from the Nitro endpoint.
 *
 * Externally-run MCP clients (Claude Code / Codex / Gemini CLI)
 * talk to pen-mcp's full 42-tool handler set via stdio/HTTP
 * transport — the embedded-vs-external coverage asymmetry is
 * why this list exists separately.
 */
export const SUPPORTED_EMBEDDED_ELEMENT_TOOLS: readonly string[] = Object.freeze(
  Object.keys(ELEMENT_SHIMS),
);
