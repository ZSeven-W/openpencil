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
import { recordElementToolCall } from '../metrics/element-tool-metrics';
import { ELEMENT_TOOL_DEFINITIONS_BASE } from './element-tool-defs-base';
import { ELEMENT_TOOL_DEFINITIONS_EXT } from './element-tool-defs-ext';
import { ELEMENT_TOOL_DEFINITIONS_EXT_2 } from './element-tool-defs-ext-2';
import { ELEMENT_TOOL_DEFINITIONS_EXT_3 } from './element-tool-defs-ext-3';

export const ELEMENT_TOOL_DEFINITIONS = [
  ...ELEMENT_TOOL_DEFINITIONS_BASE,
  ...ELEMENT_TOOL_DEFINITIONS_EXT,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_2,
  ...ELEMENT_TOOL_DEFINITIONS_EXT_3,
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
    default:
      return '';
  }
}
