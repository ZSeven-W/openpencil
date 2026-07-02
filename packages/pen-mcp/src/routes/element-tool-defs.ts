// Element-tool aggregator + NAMES set + runtime dispatcher.
//
// JSON schema definitions live in two sibling files to keep each file
// under the repo's 800-line ceiling:
//
//   element-tool-defs-base.ts — 19 initial tools (the 2026-04-19 batch)
//   element-tool-defs-ext.ts  — 20 extension tools (later batches through
//                                2026-04-20's 8-tool atom batch)
//
// This file does NOT define tool schemas inline; it only:
//   - imports the two registries, concatenates into ELEMENT_TOOL_DEFINITIONS
//   - imports every element-tool handler and wires them into a switch
//     that `design-routes.ts` dispatches to
// When growing the family, add the schema to ext-registry and extend
// this file's handler imports + switch case — schema and dispatch stay
// one edit each, file stays short.

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
import { handleAddTextareaV0 } from '../tools/add-textarea-v0';
import { handleAddSkeletonV0 } from '../tools/add-skeleton-v0';
import { handleAddSelectV0 } from '../tools/add-select-v0';
import { handleAddChartLineV0 } from '../tools/add-chart-line-v0';
import { handleAddChartPieV0 } from '../tools/add-chart-pie-v0';
import { handleAddImagePlaceholderV0 } from '../tools/add-image-placeholder-v0';
import { handleAddVideoPlaceholderV0 } from '../tools/add-video-placeholder-v0';
import { handleAddCommentV0 } from '../tools/add-comment-v0';
import { handleAddModalShellV0 } from '../tools/add-modal-shell-v0';
import { handleAddStatusBadgeV0 } from '../tools/add-status-badge-v0';
import { handleAddSpinnerV0 } from '../tools/add-spinner-v0';
import { handleAddTooltipV0 } from '../tools/add-tooltip-v0';
import { handleAddMetricComparisonV0 } from '../tools/add-metric-comparison-v0';
import { handleAddNotificationRowV0 } from '../tools/add-notification-row-v0';
import { handleAddSwitchV0 } from '../tools/add-switch-v0';
import { handleAddCheckboxV0 } from '../tools/add-checkbox-v0';
import { handleAddRadioV0 } from '../tools/add-radio-v0';
import { handleAddTabsV0 } from '../tools/add-tabs-v0';
import { handleAddSegmentedControlV0 } from '../tools/add-segmented-control-v0';
import { handleAddEmptyStateV0 } from '../tools/add-empty-state-v0';
import { handleAddAlertV0 } from '../tools/add-alert-v0';
import { handleAddToastV0 } from '../tools/add-toast-v0';
import { handleAddProgressBarV0 } from '../tools/add-progress-bar-v0';
import { handleAddFabV0 } from '../tools/add-fab-v0';
import { handleAddBreadcrumbV0 } from '../tools/add-breadcrumb-v0';
import { handleAddStepperV0 } from '../tools/add-stepper-v0';
import { handleAddRatingStarsV0 } from '../tools/add-rating-stars-v0';
import { handleAddLinkV0 } from '../tools/add-link-v0';
import { handleAddKbdV0 } from '../tools/add-kbd-v0';
import { handleAddCarouselDotsV0 } from '../tools/add-carousel-dots-v0';
import { handleAddPriceV0 } from '../tools/add-price-v0';
import { handleAddQuoteBlockV0 } from '../tools/add-quote-block-v0';
import { handleAddCodeBlockV0 } from '../tools/add-code-block-v0';
import { handleAddColorSwatchV0 } from '../tools/add-color-swatch-v0';
import { handleAddChartBarsV0 } from '../tools/add-chart-bars-v0';
import { handleAddTimelineV0 } from '../tools/add-timeline-v0';
import { handleAddCalendarGridV0 } from '../tools/add-calendar-grid-v0';
import { handleAddPaginationV0 } from '../tools/add-pagination-v0';
import { handleAddFaqItemV0 } from '../tools/add-faq-item-v0';
import { handleAddChipInputV0 } from '../tools/add-chip-input-v0';
import { handleAddEmptyChartV0 } from '../tools/add-empty-chart-v0';
import { handleAddActionMenuV0 } from '../tools/add-action-menu-v0';
import { handleAddDatePickerV0 } from '../tools/add-date-picker-v0';
import { handleAddModalShellV1 } from '../tools/add-modal-shell-v1';
import { handleAddUploadDropzoneV0 } from '../tools/add-upload-dropzone-v0';
import { handleAddOtpInputV0 } from '../tools/add-otp-input-v0';
import { handleAddAttachmentRowV0 } from '../tools/add-attachment-row-v0';
import { handleAddChatBubbleV0 } from '../tools/add-chat-bubble-v0';
import { handleAddStatCardV0 } from '../tools/add-stat-card-v0';
import { handleAddSocialLoginRowV0 } from '../tools/add-social-login-row-v0';
import { handleAddPricingCardV0 } from '../tools/add-pricing-card-v0';
import { handleAddToastV1 } from '../tools/add-toast-v1';
import { handleAddRangeSliderV0 } from '../tools/add-range-slider-v0';
import { handleAddEmptyChartV1 } from '../tools/add-empty-chart-v1';
import { handleAddPhoneInputV0 } from '../tools/add-phone-input-v0';
import { handleAddInputWithActionV0 } from '../tools/add-input-with-action-v0';
import { handleAddCookieBannerV0 } from '../tools/add-cookie-banner-v0';
import { handleAddSidebarNavV0 } from '../tools/add-sidebar-nav-v0';
import { handleAddAvatarGroupV0 } from '../tools/add-avatar-group-v0';
import { handleAddDataTableRowV0 } from '../tools/add-data-table-row-v0';
import { handleAddTagV0 } from '../tools/add-tag-v0';
import { handleAddUserCardV0 } from '../tools/add-user-card-v0';
import { handleAddDrawerShellV0 } from '../tools/add-drawer-shell-v0';
import { handleAddComboboxV0 } from '../tools/add-combobox-v0';
import { handleAddToolbarV0 } from '../tools/add-toolbar-v0';
import { handleAddCalloutV0 } from '../tools/add-callout-v0';
import { handleAddShareRowV0 } from '../tools/add-share-row-v0';
import { handleAddInlineActionV0 } from '../tools/add-inline-action-v0';
import { handleAddLegendItemV0 } from '../tools/add-legend-item-v0';
import { handleAddInboxMessageV0 } from '../tools/add-inbox-message-v0';
import { handleAddProfileHeaderV0 } from '../tools/add-profile-header-v0';
import { handleAddSettingRowV0 } from '../tools/add-setting-row-v0';
import { handleAddMemberRowV0 } from '../tools/add-member-row-v0';
import { handleAddFilterGroupV0 } from '../tools/add-filter-group-v0';
import { handleAddInviteRowV0 } from '../tools/add-invite-row-v0';
import { handleAddActivityLogV0 } from '../tools/add-activity-log-v0';
import { handleAddEventCardV0 } from '../tools/add-event-card-v0';
import { handleAddStepCardV0 } from '../tools/add-step-card-v0';
import { handleAddHeadingV1 } from '../tools/add-heading-v1';
import { handleAddCardRowV1 } from '../tools/add-card-row-v1';
import { handleAddSettingRowV1 } from '../tools/add-setting-row-v1';
import { handleAddMemberRowV1 } from '../tools/add-member-row-v1';
import { handleAddActivityLogV1 } from '../tools/add-activity-log-v1';
import { handleAddAvatarV1 } from '../tools/add-avatar-v1';
import { handleAddBadgeV1 } from '../tools/add-badge-v1';
import { handleAddDividerV1 } from '../tools/add-divider-v1';
import { handleAddBodyTextV1 } from '../tools/add-body-text-v1';
import { handleAddIconLabelV1 } from '../tools/add-icon-label-v1';
import { handleAddAlertV1 } from '../tools/add-alert-v1';
import { handleAddBottomNavV1 } from '../tools/add-bottom-nav-v1';
import { handleAddBreadcrumbV1 } from '../tools/add-breadcrumb-v1';
import { handleAddActivityRingV1 } from '../tools/add-activity-ring-v1';
import { handleAddCarouselDotsV1 } from '../tools/add-carousel-dots-v1';
import { handleAddActionMenuV1 } from '../tools/add-action-menu-v1';
import { handleAddAttachmentRowV1 } from '../tools/add-attachment-row-v1';
import { handleAddCalendarGridV1 } from '../tools/add-calendar-grid-v1';
import { handleAddAvatarGroupV1 } from '../tools/add-avatar-group-v1';
import { handleAddCalloutV1 } from '../tools/add-callout-v1';
import { handleAddChartBarsV1 } from '../tools/add-chart-bars-v1';
import { handleAddChartLineV1 } from '../tools/add-chart-line-v1';
import { handleAddChartPieV1 } from '../tools/add-chart-pie-v1';
import { handleAddChatBubbleV1 } from '../tools/add-chat-bubble-v1';
import { handleAddCheckboxV1 } from '../tools/add-checkbox-v1';
import { handleAddChipInputV1 } from '../tools/add-chip-input-v1';
import { handleAddCodeBlockV1 } from '../tools/add-code-block-v1';
import { handleAddColorSwatchV1 } from '../tools/add-color-swatch-v1';
import { handleAddComboboxV1 } from '../tools/add-combobox-v1';
import { handleAddCommentV1 } from '../tools/add-comment-v1';
import { handleAddCookieBannerV1 } from '../tools/add-cookie-banner-v1';
import { handleAddDataTableRowV1 } from '../tools/add-data-table-row-v1';
import { handleAddDatePickerV1 } from '../tools/add-date-picker-v1';
import { handleAddDrawerShellV1 } from '../tools/add-drawer-shell-v1';
import { handleAddEmptyStateV1 } from '../tools/add-empty-state-v1';
import { handleAddEventCardV1 } from '../tools/add-event-card-v1';
import { handleAddFabV1 } from '../tools/add-fab-v1';
import { handleAddFaqItemV1 } from '../tools/add-faq-item-v1';
import { handleAddFilterGroupV1 } from '../tools/add-filter-group-v1';
import { handleAddFormFieldV1 } from '../tools/add-form-field-v1';
import { handleAddIconButtonV1 } from '../tools/add-icon-button-v1';
import { handleAddImagePlaceholderV1 } from '../tools/add-image-placeholder-v1';
import { handleAddInboxMessageV1 } from '../tools/add-inbox-message-v1';
import { handleAddInlineActionV1 } from '../tools/add-inline-action-v1';
import { handleAddInputWithActionV1 } from '../tools/add-input-with-action-v1';
import { handleAddInviteRowV1 } from '../tools/add-invite-row-v1';
import { handleAddKbdV1 } from '../tools/add-kbd-v1';
import { handleAddLegendItemV1 } from '../tools/add-legend-item-v1';
import { handleAddLinkV1 } from '../tools/add-link-v1';
import { handleAddListRowV1 } from '../tools/add-list-row-v1';
import { handleAddMetricComparisonV1 } from '../tools/add-metric-comparison-v1';
import { handleAddMetricRowV1 } from '../tools/add-metric-row-v1';
import { handleAddNavChipRowV1 } from '../tools/add-nav-chip-row-v1';
import { handleAddNotificationRowV1 } from '../tools/add-notification-row-v1';
import { handleAddOtpInputV1 } from '../tools/add-otp-input-v1';
import { handleAddPaginationV1 } from '../tools/add-pagination-v1';
import { handleAddPhoneInputV1 } from '../tools/add-phone-input-v1';
import { handleAddPriceV1 } from '../tools/add-price-v1';
import { handleAddPricingCardV1 } from '../tools/add-pricing-card-v1';
import { handleAddProfileHeaderV1 } from '../tools/add-profile-header-v1';
import { handleAddProgressBarV1 } from '../tools/add-progress-bar-v1';
import { handleAddQuoteBlockV1 } from '../tools/add-quote-block-v1';
import { handleAddRadioV1 } from '../tools/add-radio-v1';
import { handleAddRangeSliderV1 } from '../tools/add-range-slider-v1';
import { handleAddRatingStarsV1 } from '../tools/add-rating-stars-v1';
import { handleAddSearchBarV1 } from '../tools/add-search-bar-v1';
import { handleAddSectionHeaderV1 } from '../tools/add-section-header-v1';
import { handleAddSegmentedControlV1 } from '../tools/add-segmented-control-v1';
import { handleAddSelectV1 } from '../tools/add-select-v1';
import { handleAddShareRowV1 } from '../tools/add-share-row-v1';
import { handleAddSidebarNavV1 } from '../tools/add-sidebar-nav-v1';
import { handleAddSkeletonV1 } from '../tools/add-skeleton-v1';
import { handleAddSocialLoginRowV1 } from '../tools/add-social-login-row-v1';
import { handleAddSpinnerV1 } from '../tools/add-spinner-v1';
import { handleAddStatCardV1 } from '../tools/add-stat-card-v1';
import { handleAddStatGridV1 } from '../tools/add-stat-grid-v1';
import { handleAddStatusBadgeV1 } from '../tools/add-status-badge-v1';
import { handleAddStepCardV1 } from '../tools/add-step-card-v1';
import { handleAddStepperV1 } from '../tools/add-stepper-v1';
import { handleAddSwitchV1 } from '../tools/add-switch-v1';
import { handleAddTabsV1 } from '../tools/add-tabs-v1';
import { handleAddTagV1 } from '../tools/add-tag-v1';
import { handleAddTextButtonV1 } from '../tools/add-text-button-v1';
import { handleAddTextareaV1 } from '../tools/add-textarea-v1';
import { handleAddTimelineV1 } from '../tools/add-timeline-v1';
import { handleAddToolbarV1 } from '../tools/add-toolbar-v1';
import { handleAddTooltipV1 } from '../tools/add-tooltip-v1';
import { handleAddTopNavBarV1 } from '../tools/add-top-nav-bar-v1';
import { handleAddUploadDropzoneV1 } from '../tools/add-upload-dropzone-v1';
import { handleAddUserCardV1 } from '../tools/add-user-card-v1';
import { handleAddVideoPlaceholderV1 } from '../tools/add-video-placeholder-v1';
import { recordElementToolCall } from '../metrics/element-tool-metrics';
import { ELEMENT_TOOL_DEFINITIONS_BASE } from './element-tool-defs-base';
import { ELEMENT_TOOL_DEFINITIONS_EXT } from './element-tool-defs-ext';
import { ELEMENT_TOOL_DEFINITIONS_EXT_2 } from './element-tool-defs-ext-2';
import { ELEMENT_TOOL_DEFINITIONS_EXT_3 } from './element-tool-defs-ext-3';
import { ELEMENT_TOOL_DEFINITIONS_EXT_4 } from './element-tool-defs-ext-4';
import { ELEMENT_TOOL_DEFINITIONS_EXT_5 } from './element-tool-defs-ext-5';
import { ELEMENT_TOOL_DEFINITIONS_EXT_6 } from './element-tool-defs-ext-6';
import { ELEMENT_TOOL_DEFINITIONS_EXT_7 } from './element-tool-defs-ext-7';
import { ELEMENT_TOOL_DEFINITIONS_EXT_8 } from './element-tool-defs-ext-8';
import { ELEMENT_TOOL_DEFINITIONS_EXT_9 } from './element-tool-defs-ext-9';

export const ELEMENT_TOOL_DEFINITIONS = [
  ...ELEMENT_TOOL_DEFINITIONS_BASE,
  ...ELEMENT_TOOL_DEFINITIONS_EXT,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_2,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_3,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_4,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_5,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_6,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_7,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_8,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_9,
];

export const ELEMENT_TOOL_NAMES: ReadonlySet<string> = new Set(
  ELEMENT_TOOL_DEFINITIONS.map((t) => t.name),
);

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export async function handleElementToolCall(name: string, a: any): Promise<string> {
  try {
    const out = await dispatchElementToolCall(name, a);
    recordElementToolCall(name, true);
    return out;
  } catch (err) {
    recordElementToolCall(name, false, err instanceof Error ? err.message : String(err));
    throw err;
  }
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
async function dispatchElementToolCall(name: string, a: any): Promise<string> {
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
    case 'add_textarea_v0':
      return JSON.stringify(await handleAddTextareaV0(a), null, 2);
    case 'add_skeleton_v0':
      return JSON.stringify(await handleAddSkeletonV0(a), null, 2);
    case 'add_select_v0':
      return JSON.stringify(await handleAddSelectV0(a), null, 2);
    case 'add_chart_line_v0':
      return JSON.stringify(await handleAddChartLineV0(a), null, 2);
    case 'add_chart_pie_v0':
      return JSON.stringify(await handleAddChartPieV0(a), null, 2);
    case 'add_image_placeholder_v0':
      return JSON.stringify(await handleAddImagePlaceholderV0(a), null, 2);
    case 'add_video_placeholder_v0':
      return JSON.stringify(await handleAddVideoPlaceholderV0(a), null, 2);
    case 'add_comment_v0':
      return JSON.stringify(await handleAddCommentV0(a), null, 2);
    case 'add_modal_shell_v0':
      return JSON.stringify(await handleAddModalShellV0(a), null, 2);
    case 'add_status_badge_v0':
      return JSON.stringify(await handleAddStatusBadgeV0(a), null, 2);
    case 'add_spinner_v0':
      return JSON.stringify(await handleAddSpinnerV0(a), null, 2);
    case 'add_tooltip_v0':
      return JSON.stringify(await handleAddTooltipV0(a), null, 2);
    case 'add_metric_comparison_v0':
      return JSON.stringify(await handleAddMetricComparisonV0(a), null, 2);
    case 'add_notification_row_v0':
      return JSON.stringify(await handleAddNotificationRowV0(a), null, 2);
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
    case 'add_alert_v0':
      return JSON.stringify(await handleAddAlertV0(a), null, 2);
    case 'add_toast_v0':
      return JSON.stringify(await handleAddToastV0(a), null, 2);
    case 'add_progress_bar_v0':
      return JSON.stringify(await handleAddProgressBarV0(a), null, 2);
    case 'add_fab_v0':
      return JSON.stringify(await handleAddFabV0(a), null, 2);
    case 'add_breadcrumb_v0':
      return JSON.stringify(await handleAddBreadcrumbV0(a), null, 2);
    case 'add_stepper_v0':
      return JSON.stringify(await handleAddStepperV0(a), null, 2);
    case 'add_rating_stars_v0':
      return JSON.stringify(await handleAddRatingStarsV0(a), null, 2);
    case 'add_link_v0':
      return JSON.stringify(await handleAddLinkV0(a), null, 2);
    case 'add_kbd_v0':
      return JSON.stringify(await handleAddKbdV0(a), null, 2);
    case 'add_carousel_dots_v0':
      return JSON.stringify(await handleAddCarouselDotsV0(a), null, 2);
    case 'add_price_v0':
      return JSON.stringify(await handleAddPriceV0(a), null, 2);
    case 'add_quote_block_v0':
      return JSON.stringify(await handleAddQuoteBlockV0(a), null, 2);
    case 'add_code_block_v0':
      return JSON.stringify(await handleAddCodeBlockV0(a), null, 2);
    case 'add_color_swatch_v0':
      return JSON.stringify(await handleAddColorSwatchV0(a), null, 2);
    case 'add_chart_bars_v0':
      return JSON.stringify(await handleAddChartBarsV0(a), null, 2);
    case 'add_timeline_v0':
      return JSON.stringify(await handleAddTimelineV0(a), null, 2);
    case 'add_calendar_grid_v0':
      return JSON.stringify(await handleAddCalendarGridV0(a), null, 2);
    case 'add_pagination_v0':
      return JSON.stringify(await handleAddPaginationV0(a), null, 2);
    case 'add_faq_item_v0':
      return JSON.stringify(await handleAddFaqItemV0(a), null, 2);
    case 'add_chip_input_v0':
      return JSON.stringify(await handleAddChipInputV0(a), null, 2);
    case 'add_empty_chart_v0':
      return JSON.stringify(await handleAddEmptyChartV0(a), null, 2);
    case 'add_action_menu_v0':
      return JSON.stringify(await handleAddActionMenuV0(a), null, 2);
    case 'add_date_picker_v0':
      return JSON.stringify(await handleAddDatePickerV0(a), null, 2);
    case 'add_modal_shell_v1':
      return JSON.stringify(await handleAddModalShellV1(a), null, 2);
    case 'add_upload_dropzone_v0':
      return JSON.stringify(await handleAddUploadDropzoneV0(a), null, 2);
    case 'add_otp_input_v0':
      return JSON.stringify(await handleAddOtpInputV0(a), null, 2);
    case 'add_attachment_row_v0':
      return JSON.stringify(await handleAddAttachmentRowV0(a), null, 2);
    case 'add_chat_bubble_v0':
      return JSON.stringify(await handleAddChatBubbleV0(a), null, 2);
    case 'add_stat_card_v0':
      return JSON.stringify(await handleAddStatCardV0(a), null, 2);
    case 'add_social_login_row_v0':
      return JSON.stringify(await handleAddSocialLoginRowV0(a), null, 2);
    case 'add_pricing_card_v0':
      return JSON.stringify(await handleAddPricingCardV0(a), null, 2);
    case 'add_toast_v1':
      return JSON.stringify(await handleAddToastV1(a), null, 2);
    case 'add_range_slider_v0':
      return JSON.stringify(await handleAddRangeSliderV0(a), null, 2);
    case 'add_empty_chart_v1':
      return JSON.stringify(await handleAddEmptyChartV1(a), null, 2);
    case 'add_phone_input_v0':
      return JSON.stringify(await handleAddPhoneInputV0(a), null, 2);
    case 'add_input_with_action_v0':
      return JSON.stringify(await handleAddInputWithActionV0(a), null, 2);
    case 'add_cookie_banner_v0':
      return JSON.stringify(await handleAddCookieBannerV0(a), null, 2);
    case 'add_sidebar_nav_v0':
      return JSON.stringify(await handleAddSidebarNavV0(a), null, 2);
    case 'add_avatar_group_v0':
      return JSON.stringify(await handleAddAvatarGroupV0(a), null, 2);
    case 'add_data_table_row_v0':
      return JSON.stringify(await handleAddDataTableRowV0(a), null, 2);
    case 'add_tag_v0':
      return JSON.stringify(await handleAddTagV0(a), null, 2);
    case 'add_user_card_v0':
      return JSON.stringify(await handleAddUserCardV0(a), null, 2);
    case 'add_drawer_shell_v0':
      return JSON.stringify(await handleAddDrawerShellV0(a), null, 2);
    case 'add_combobox_v0':
      return JSON.stringify(await handleAddComboboxV0(a), null, 2);
    case 'add_toolbar_v0':
      return JSON.stringify(await handleAddToolbarV0(a), null, 2);
    case 'add_callout_v0':
      return JSON.stringify(await handleAddCalloutV0(a), null, 2);
    case 'add_share_row_v0':
      return JSON.stringify(await handleAddShareRowV0(a), null, 2);
    case 'add_inline_action_v0':
      return JSON.stringify(await handleAddInlineActionV0(a), null, 2);
    case 'add_legend_item_v0':
      return JSON.stringify(await handleAddLegendItemV0(a), null, 2);
    case 'add_inbox_message_v0':
      return JSON.stringify(await handleAddInboxMessageV0(a), null, 2);
    case 'add_profile_header_v0':
      return JSON.stringify(await handleAddProfileHeaderV0(a), null, 2);
    case 'add_setting_row_v0':
      return JSON.stringify(await handleAddSettingRowV0(a), null, 2);
    case 'add_member_row_v0':
      return JSON.stringify(await handleAddMemberRowV0(a), null, 2);
    case 'add_filter_group_v0':
      return JSON.stringify(await handleAddFilterGroupV0(a), null, 2);
    case 'add_invite_row_v0':
      return JSON.stringify(await handleAddInviteRowV0(a), null, 2);
    case 'add_activity_log_v0':
      return JSON.stringify(await handleAddActivityLogV0(a), null, 2);
    case 'add_event_card_v0':
      return JSON.stringify(await handleAddEventCardV0(a), null, 2);
    case 'add_step_card_v0':
      return JSON.stringify(await handleAddStepCardV0(a), null, 2);
    case 'add_heading_v1':
      return JSON.stringify(await handleAddHeadingV1(a), null, 2);
    case 'add_card_row_v1':
      return JSON.stringify(await handleAddCardRowV1(a), null, 2);
    case 'add_setting_row_v1':
      return JSON.stringify(await handleAddSettingRowV1(a), null, 2);
    case 'add_member_row_v1':
      return JSON.stringify(await handleAddMemberRowV1(a), null, 2);
    case 'add_activity_log_v1':
      return JSON.stringify(await handleAddActivityLogV1(a), null, 2);
    case 'add_avatar_v1':
      return JSON.stringify(await handleAddAvatarV1(a), null, 2);
    case 'add_badge_v1':
      return JSON.stringify(await handleAddBadgeV1(a), null, 2);
    case 'add_divider_v1':
      return JSON.stringify(await handleAddDividerV1(a), null, 2);
    case 'add_body_text_v1':
      return JSON.stringify(await handleAddBodyTextV1(a), null, 2);
    case 'add_icon_label_v1':
      return JSON.stringify(await handleAddIconLabelV1(a), null, 2);
    case 'add_alert_v1':
      return JSON.stringify(await handleAddAlertV1(a), null, 2);
    case 'add_bottom_nav_v1':
      return JSON.stringify(await handleAddBottomNavV1(a), null, 2);
    case 'add_breadcrumb_v1':
      return JSON.stringify(await handleAddBreadcrumbV1(a), null, 2);
    case 'add_activity_ring_v1':
      return JSON.stringify(await handleAddActivityRingV1(a), null, 2);
    case 'add_carousel_dots_v1':
      return JSON.stringify(await handleAddCarouselDotsV1(a), null, 2);
    case 'add_action_menu_v1':
      return JSON.stringify(await handleAddActionMenuV1(a), null, 2);
    case 'add_attachment_row_v1':
      return JSON.stringify(await handleAddAttachmentRowV1(a), null, 2);
    case 'add_calendar_grid_v1':
      return JSON.stringify(await handleAddCalendarGridV1(a), null, 2);
    case 'add_avatar_group_v1':
      return JSON.stringify(await handleAddAvatarGroupV1(a), null, 2);
    case 'add_callout_v1':
      return JSON.stringify(await handleAddCalloutV1(a), null, 2);
    case 'add_chart_bars_v1':
      return JSON.stringify(await handleAddChartBarsV1(a), null, 2);
    case 'add_chart_line_v1':
      return JSON.stringify(await handleAddChartLineV1(a), null, 2);
    case 'add_chart_pie_v1':
      return JSON.stringify(await handleAddChartPieV1(a), null, 2);
    case 'add_chat_bubble_v1':
      return JSON.stringify(await handleAddChatBubbleV1(a), null, 2);
    case 'add_checkbox_v1':
      return JSON.stringify(await handleAddCheckboxV1(a), null, 2);
    case 'add_chip_input_v1':
      return JSON.stringify(await handleAddChipInputV1(a), null, 2);
    case 'add_code_block_v1':
      return JSON.stringify(await handleAddCodeBlockV1(a), null, 2);
    case 'add_color_swatch_v1':
      return JSON.stringify(await handleAddColorSwatchV1(a), null, 2);
    case 'add_combobox_v1':
      return JSON.stringify(await handleAddComboboxV1(a), null, 2);
    case 'add_comment_v1':
      return JSON.stringify(await handleAddCommentV1(a), null, 2);
    case 'add_cookie_banner_v1':
      return JSON.stringify(await handleAddCookieBannerV1(a), null, 2);
    case 'add_data_table_row_v1':
      return JSON.stringify(await handleAddDataTableRowV1(a), null, 2);
    case 'add_date_picker_v1':
      return JSON.stringify(await handleAddDatePickerV1(a), null, 2);
    case 'add_drawer_shell_v1':
      return JSON.stringify(await handleAddDrawerShellV1(a), null, 2);
    case 'add_empty_state_v1':
      return JSON.stringify(await handleAddEmptyStateV1(a), null, 2);
    case 'add_event_card_v1':
      return JSON.stringify(await handleAddEventCardV1(a), null, 2);
    case 'add_fab_v1':
      return JSON.stringify(await handleAddFabV1(a), null, 2);
    case 'add_faq_item_v1':
      return JSON.stringify(await handleAddFaqItemV1(a), null, 2);
    case 'add_filter_group_v1':
      return JSON.stringify(await handleAddFilterGroupV1(a), null, 2);
    case 'add_form_field_v1':
      return JSON.stringify(await handleAddFormFieldV1(a), null, 2);
    case 'add_icon_button_v1':
      return JSON.stringify(await handleAddIconButtonV1(a), null, 2);
    case 'add_image_placeholder_v1':
      return JSON.stringify(await handleAddImagePlaceholderV1(a), null, 2);
    case 'add_inbox_message_v1':
      return JSON.stringify(await handleAddInboxMessageV1(a), null, 2);
    case 'add_inline_action_v1':
      return JSON.stringify(await handleAddInlineActionV1(a), null, 2);
    case 'add_input_with_action_v1':
      return JSON.stringify(await handleAddInputWithActionV1(a), null, 2);
    case 'add_invite_row_v1':
      return JSON.stringify(await handleAddInviteRowV1(a), null, 2);
    case 'add_kbd_v1':
      return JSON.stringify(await handleAddKbdV1(a), null, 2);
    case 'add_legend_item_v1':
      return JSON.stringify(await handleAddLegendItemV1(a), null, 2);
    case 'add_link_v1':
      return JSON.stringify(await handleAddLinkV1(a), null, 2);
    case 'add_list_row_v1':
      return JSON.stringify(await handleAddListRowV1(a), null, 2);
    case 'add_metric_comparison_v1':
      return JSON.stringify(await handleAddMetricComparisonV1(a), null, 2);
    case 'add_metric_row_v1':
      return JSON.stringify(await handleAddMetricRowV1(a), null, 2);
    case 'add_nav_chip_row_v1':
      return JSON.stringify(await handleAddNavChipRowV1(a), null, 2);
    case 'add_notification_row_v1':
      return JSON.stringify(await handleAddNotificationRowV1(a), null, 2);
    case 'add_otp_input_v1':
      return JSON.stringify(await handleAddOtpInputV1(a), null, 2);
    case 'add_pagination_v1':
      return JSON.stringify(await handleAddPaginationV1(a), null, 2);
    case 'add_phone_input_v1':
      return JSON.stringify(await handleAddPhoneInputV1(a), null, 2);
    case 'add_price_v1':
      return JSON.stringify(await handleAddPriceV1(a), null, 2);
    case 'add_pricing_card_v1':
      return JSON.stringify(await handleAddPricingCardV1(a), null, 2);
    case 'add_profile_header_v1':
      return JSON.stringify(await handleAddProfileHeaderV1(a), null, 2);
    case 'add_progress_bar_v1':
      return JSON.stringify(await handleAddProgressBarV1(a), null, 2);
    case 'add_quote_block_v1':
      return JSON.stringify(await handleAddQuoteBlockV1(a), null, 2);
    case 'add_radio_v1':
      return JSON.stringify(await handleAddRadioV1(a), null, 2);
    case 'add_range_slider_v1':
      return JSON.stringify(await handleAddRangeSliderV1(a), null, 2);
    case 'add_rating_stars_v1':
      return JSON.stringify(await handleAddRatingStarsV1(a), null, 2);
    case 'add_search_bar_v1':
      return JSON.stringify(await handleAddSearchBarV1(a), null, 2);
    case 'add_section_header_v1':
      return JSON.stringify(await handleAddSectionHeaderV1(a), null, 2);
    case 'add_segmented_control_v1':
      return JSON.stringify(await handleAddSegmentedControlV1(a), null, 2);
    case 'add_select_v1':
      return JSON.stringify(await handleAddSelectV1(a), null, 2);
    case 'add_share_row_v1':
      return JSON.stringify(await handleAddShareRowV1(a), null, 2);
    case 'add_sidebar_nav_v1':
      return JSON.stringify(await handleAddSidebarNavV1(a), null, 2);
    case 'add_skeleton_v1':
      return JSON.stringify(await handleAddSkeletonV1(a), null, 2);
    case 'add_social_login_row_v1':
      return JSON.stringify(await handleAddSocialLoginRowV1(a), null, 2);
    case 'add_spinner_v1':
      return JSON.stringify(await handleAddSpinnerV1(a), null, 2);
    case 'add_stat_card_v1':
      return JSON.stringify(await handleAddStatCardV1(a), null, 2);
    case 'add_stat_grid_v1':
      return JSON.stringify(await handleAddStatGridV1(a), null, 2);
    case 'add_status_badge_v1':
      return JSON.stringify(await handleAddStatusBadgeV1(a), null, 2);
    case 'add_step_card_v1':
      return JSON.stringify(await handleAddStepCardV1(a), null, 2);
    case 'add_stepper_v1':
      return JSON.stringify(await handleAddStepperV1(a), null, 2);
    case 'add_switch_v1':
      return JSON.stringify(await handleAddSwitchV1(a), null, 2);
    case 'add_tabs_v1':
      return JSON.stringify(await handleAddTabsV1(a), null, 2);
    case 'add_tag_v1':
      return JSON.stringify(await handleAddTagV1(a), null, 2);
    case 'add_text_button_v1':
      return JSON.stringify(await handleAddTextButtonV1(a), null, 2);
    case 'add_textarea_v1':
      return JSON.stringify(await handleAddTextareaV1(a), null, 2);
    case 'add_timeline_v1':
      return JSON.stringify(await handleAddTimelineV1(a), null, 2);
    case 'add_toolbar_v1':
      return JSON.stringify(await handleAddToolbarV1(a), null, 2);
    case 'add_tooltip_v1':
      return JSON.stringify(await handleAddTooltipV1(a), null, 2);
    case 'add_top_nav_bar_v1':
      return JSON.stringify(await handleAddTopNavBarV1(a), null, 2);
    case 'add_upload_dropzone_v1':
      return JSON.stringify(await handleAddUploadDropzoneV1(a), null, 2);
    case 'add_user_card_v1':
      return JSON.stringify(await handleAddUserCardV1(a), null, 2);
    case 'add_video_placeholder_v1':
      return JSON.stringify(await handleAddVideoPlaceholderV1(a), null, 2);
    default:
      return '';
  }
}
