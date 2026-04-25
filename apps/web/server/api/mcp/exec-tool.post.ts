import { defineEventHandler, readBody, setResponseStatus } from 'h3';
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
  findNodeInTree,
  insertNodeInTree,
  type ElementTree,
} from '@zseven-w/pen-core';
import { runBatchDesignDsl } from '@zseven-w/pen-mcp';
import { getSyncDocument, setSyncDocument } from '../../utils/mcp-sync-state';
import { serverLog } from '../../utils/server-logger';
import type { PenDocument, PenNode } from '../../../src/types/pen';

/**
 * POST /api/mcp/exec-tool — Phase 2 M3 HTTP fallback.
 *
 * Accepts either:
 *   - `{ name: 'add_X_v0', arguments: {...} }` — an element-tool call
 *     the browser couldn't match to a client shim (or for which the
 *     caller wants authoritative server-side state)
 *   - `{ dsl: '...' }` — a batch_design DSL string
 *
 * For known element tools, the endpoint uses the SAME pen-core
 * builders the client shim uses, so the tree shape is byte-identical
 * across paths. The constructed tree is inserted under the
 * explicitly-named parent (parent_id) or the caller-supplied
 * default (default_parent_id) or the first page's root.
 *
 * For batch_design DSL, the endpoint runs pen-mcp's
 * `runBatchDesignDsl` against the in-memory sync-state doc — no
 * file I/O, no post-processing hooks (those live in the pen-mcp
 * server process). This is the fallback path advertised to the
 * embedded orchestrator's AI when no covered `add_*_v0` matches
 * its intent; it must actually execute or the prompt contract is
 * broken.
 *
 * Both paths call `setSyncDocument` to broadcast via SSE and
 * return the updated doc in the response body so the dispatching
 * client can `applyExternalDocument` immediately without waiting
 * for its own SSE to arrive.
 */

type BuilderFn = (args: unknown) => ElementTree;

/**
 * Server-side builder registry. MUST stay in sync with
 * `apps/web/src/services/ai/element-tool-shims/index.ts`
 * (SUPPORTED_EMBEDDED_ELEMENT_TOOLS) — when extending one, extend
 * the other in the same commit. Both dispatch through pen-core
 * `buildX` so the tree shape is drift-free; this registry exists
 * only to gate unknown tool names + own the sync-state mutation.
 *
 * Coverage gap: elements.md names 42 element tools (the full
 * pen-mcp catalog for external MCP clients). Embedded runtime —
 * shim + this file — covers a subset until more builders are
 * extracted to pen-core. Until then, unsupported names return 404
 * with the canonical supported list and the dispatcher short-
 * circuits before calling us.
 */
const SERVER_BUILDERS: Record<string, BuilderFn> = {
  add_card_row_v0: (a) => buildCardRow(a as Parameters<typeof buildCardRow>[0]),
  add_metric_row_v0: (a) => buildMetricRow(a as Parameters<typeof buildMetricRow>[0]),
  add_bottom_nav_v0: (a) => buildBottomNav(a as Parameters<typeof buildBottomNav>[0]),
  add_section_header_v0: (a) => buildSectionHeader(a as Parameters<typeof buildSectionHeader>[0]),
  add_top_nav_bar_v0: (a) => buildTopNavBar(a as Parameters<typeof buildTopNavBar>[0]),
  add_heading_v0: (a) => buildHeading(a as Parameters<typeof buildHeading>[0]),
  add_body_text_v0: (a) => buildBodyText(a as Parameters<typeof buildBodyText>[0]),
  add_text_button_v0: (a) => buildTextButton(a as Parameters<typeof buildTextButton>[0]),
  add_search_bar_v0: (a) => buildSearchBar(a as Parameters<typeof buildSearchBar>[0]),
  add_list_row_v0: (a) => buildListRow(a as Parameters<typeof buildListRow>[0]),
  add_divider_v0: (a) => buildDivider(a as Parameters<typeof buildDivider>[0]),
  add_badge_v0: (a) => buildBadge(a as Parameters<typeof buildBadge>[0]),
  add_avatar_v0: (a) => buildAvatar(a as Parameters<typeof buildAvatar>[0]),
  add_icon_button_v0: (a) => buildIconButton(a as Parameters<typeof buildIconButton>[0]),
  add_icon_label_v0: (a) => buildIconLabel(a as Parameters<typeof buildIconLabel>[0]),
  add_stat_grid_v0: (a) => buildStatGrid(a as Parameters<typeof buildStatGrid>[0]),
  add_switch_v0: (a) => buildSwitch(a as Parameters<typeof buildSwitch>[0]),
  add_checkbox_v0: (a) => buildCheckbox(a as Parameters<typeof buildCheckbox>[0]),
  add_radio_v0: (a) => buildRadio(a as Parameters<typeof buildRadio>[0]),
  add_tabs_v0: (a) => buildTabs(a as Parameters<typeof buildTabs>[0]),
  add_segmented_control_v0: (a) =>
    buildSegmentedControl(a as Parameters<typeof buildSegmentedControl>[0]),
  add_empty_state_v0: (a) => buildEmptyState(a as Parameters<typeof buildEmptyState>[0]),
  add_alert_v0: (a) => buildAlert(a as Parameters<typeof buildAlert>[0]),
  add_toast_v0: (a) => buildToast(a as Parameters<typeof buildToast>[0]),
  add_progress_bar_v0: (a) => buildProgressBar(a as Parameters<typeof buildProgressBar>[0]),
  add_fab_v0: (a) => buildFab(a as Parameters<typeof buildFab>[0]),
  add_breadcrumb_v0: (a) => buildBreadcrumb(a as Parameters<typeof buildBreadcrumb>[0]),
  add_stepper_v0: (a) => buildStepper(a as Parameters<typeof buildStepper>[0]),
  add_form_field_v0: (a) => buildFormField(a as Parameters<typeof buildFormField>[0]),
  add_textarea_v0: (a) => buildTextarea(a as Parameters<typeof buildTextarea>[0]),
  add_skeleton_v0: (a) => buildSkeleton(a as Parameters<typeof buildSkeleton>[0]),
  add_select_v0: (a) => buildSelect(a as Parameters<typeof buildSelect>[0]),
  add_chart_line_v0: (a) => buildChartLine(a as Parameters<typeof buildChartLine>[0]),
  add_chart_pie_v0: (a) => buildChartPie(a as Parameters<typeof buildChartPie>[0]),
  add_image_placeholder_v0: (a) =>
    buildImagePlaceholder(a as Parameters<typeof buildImagePlaceholder>[0]),
  add_video_placeholder_v0: (a) =>
    buildVideoPlaceholder(a as Parameters<typeof buildVideoPlaceholder>[0]),
  add_comment_v0: (a) => buildComment(a as Parameters<typeof buildComment>[0]),
  add_modal_shell_v0: (a) => buildModalShell(a as Parameters<typeof buildModalShell>[0]),
  add_status_badge_v0: (a) => buildStatusBadge(a as Parameters<typeof buildStatusBadge>[0]),
  add_spinner_v0: (a) => buildSpinner(a as Parameters<typeof buildSpinner>[0]),
  add_tooltip_v0: (a) => buildTooltip(a as Parameters<typeof buildTooltip>[0]),
  add_metric_comparison_v0: (a) =>
    buildMetricComparison(a as Parameters<typeof buildMetricComparison>[0]),
  add_notification_row_v0: (a) =>
    buildNotificationRow(a as Parameters<typeof buildNotificationRow>[0]),
  add_nav_chip_row_v0: (a) => buildNavChipRow(a as Parameters<typeof buildNavChipRow>[0]),
  add_activity_ring_v0: (a) => buildActivityRing(a as Parameters<typeof buildActivityRing>[0]),
  add_rating_stars_v0: (a) => buildRatingStars(a as Parameters<typeof buildRatingStars>[0]),
  add_carousel_dots_v0: (a) => buildCarouselDots(a as Parameters<typeof buildCarouselDots>[0]),
  add_link_v0: (a) => buildLink(a as Parameters<typeof buildLink>[0]),
  add_kbd_v0: (a) => buildKbd(a as Parameters<typeof buildKbd>[0]),
  add_price_v0: (a) => buildPrice(a as Parameters<typeof buildPrice>[0]),
  add_quote_block_v0: (a) => buildQuoteBlock(a as Parameters<typeof buildQuoteBlock>[0]),
  add_code_block_v0: (a) => buildCodeBlock(a as Parameters<typeof buildCodeBlock>[0]),
  add_color_swatch_v0: (a) => buildColorSwatch(a as Parameters<typeof buildColorSwatch>[0]),
  add_chart_bars_v0: (a) => buildChartBars(a as Parameters<typeof buildChartBars>[0]),
  add_timeline_v0: (a) => buildTimeline(a as Parameters<typeof buildTimeline>[0]),
  add_calendar_grid_v0: (a) => buildCalendarGrid(a as Parameters<typeof buildCalendarGrid>[0]),
  add_pagination_v0: (a) => buildPagination(a as Parameters<typeof buildPagination>[0]),
  add_faq_item_v0: (a) => buildFaqItem(a as Parameters<typeof buildFaqItem>[0]),
  add_chip_input_v0: (a) => buildChipInput(a as Parameters<typeof buildChipInput>[0]),
  add_empty_chart_v0: (a) => buildEmptyChart(a as Parameters<typeof buildEmptyChart>[0]),
  add_action_menu_v0: (a) => buildActionMenu(a as Parameters<typeof buildActionMenu>[0]),
  add_date_picker_v0: (a) => buildDatePicker(a as Parameters<typeof buildDatePicker>[0]),
  add_modal_shell_v1: (a) => buildModalShellV1(a as Parameters<typeof buildModalShellV1>[0]),
  add_upload_dropzone_v0: (a) =>
    buildUploadDropzone(a as Parameters<typeof buildUploadDropzone>[0]),
  add_otp_input_v0: (a) => buildOtpInput(a as Parameters<typeof buildOtpInput>[0]),
  add_attachment_row_v0: (a) => buildAttachmentRow(a as Parameters<typeof buildAttachmentRow>[0]),
  add_chat_bubble_v0: (a) => buildChatBubble(a as Parameters<typeof buildChatBubble>[0]),
  add_stat_card_v0: (a) => buildStatCard(a as Parameters<typeof buildStatCard>[0]),
  add_social_login_row_v0: (a) =>
    buildSocialLoginRow(a as Parameters<typeof buildSocialLoginRow>[0]),
  add_pricing_card_v0: (a) => buildPricingCard(a as Parameters<typeof buildPricingCard>[0]),
  add_toast_v1: (a) => buildToastV1(a as Parameters<typeof buildToastV1>[0]),
  add_range_slider_v0: (a) => buildRangeSlider(a as Parameters<typeof buildRangeSlider>[0]),
  add_empty_chart_v1: (a) => buildEmptyChartV1(a as Parameters<typeof buildEmptyChartV1>[0]),
  add_phone_input_v0: (a) => buildPhoneInput(a as Parameters<typeof buildPhoneInput>[0]),
  add_input_with_action_v0: (a) =>
    buildInputWithAction(a as Parameters<typeof buildInputWithAction>[0]),
};

interface ExecToolBody {
  name?: string;
  arguments?: unknown;
  dsl?: string;
  /**
   * Fallback parent id when `arguments.parent_id` is not provided.
   * Forwarded by the client dispatcher from the orchestrator's subtask
   * context (`subtask.parentFrameId ?? plan.rootFrame.id`). Without it
   * rootless payloads would land at page root outside the generation's
   * scope — mirrors the client-shim `ctx.defaultParentId` contract.
   */
  default_parent_id?: string;
}

interface ExecToolError {
  ok: false;
  error: string;
}

interface ExecToolSuccess {
  ok: true;
  document: PenDocument;
  /**
   * Single inserted root id (element-tool path) or the first node
   * produced by a batch_design run. Kept for legacy callers that
   * only read one id.
   */
  insertedNodeId: string;
  /**
   * All inserted root ids. Element-tool path populates exactly one;
   * batch_design DSL can populate many (one per top-level I / C / R
   * binding). Orchestrator uses this to update progress tallies.
   */
  insertedNodeIds: string[];
}

export default defineEventHandler(async (event): Promise<ExecToolSuccess | ExecToolError> => {
  const body = (await readBody(event).catch(() => null)) as ExecToolBody | null;
  if (!body) {
    setResponseStatus(event, 400);
    return { ok: false, error: 'Missing or malformed JSON body' };
  }

  if (body.dsl) {
    const { doc: currentDsl } = getSyncDocument();
    if (!currentDsl) {
      setResponseStatus(event, 409);
      return {
        ok: false,
        error:
          'No live canvas document is currently synced. Open a document before invoking ' +
          '/api/mcp/exec-tool with a DSL payload.',
      };
    }

    // Resolve pageId for the DSL executor. `live://canvas` sentinel +
    // real filePaths are already rejected above; DSL writes always
    // target the in-memory sync-state (which represents the live
    // canvas). Reject real filePaths here too for parity with the
    // element-tool branch.
    const LIVE_CANVAS_SENTINEL_DSL = 'live://canvas';
    const rawDslFilePath =
      typeof (body as Record<string, unknown>).filePath === 'string'
        ? ((body as Record<string, unknown>).filePath as string)
        : null;
    if (
      rawDslFilePath !== null &&
      rawDslFilePath.length > 0 &&
      rawDslFilePath !== LIVE_CANVAS_SENTINEL_DSL
    ) {
      setResponseStatus(event, 501);
      return {
        ok: false,
        error:
          `filePath="${rawDslFilePath}" is not supported by /api/mcp/exec-tool's DSL path — ` +
          `this endpoint mutates the in-memory live canvas only. Route through pen-mcp's ` +
          `stdio / HTTP transport for file-backed DSL.`,
      };
    }
    const dslPageId =
      typeof (body as Record<string, unknown>).pageId === 'string' &&
      ((body as Record<string, unknown>).pageId as string).length > 0
        ? ((body as Record<string, unknown>).pageId as string)
        : undefined;

    // runBatchDesignDsl mutates the cloned doc in place; bindings +
    // per-line errors are collected and surfaced in the response so
    // the dispatcher / user sees exactly which ops failed.
    const nextDsl = structuredClone(currentDsl) as PenDocument;
    const { results: dslResults, errors: dslErrors } = await runBatchDesignDsl(nextDsl, body.dsl, {
      pageId: dslPageId,
    });
    if (dslErrors.length > 0) {
      setResponseStatus(event, 400);
      return {
        ok: false,
        error:
          `batch_design DSL had ${dslErrors.length} failing operation(s): ` +
          dslErrors
            .map((e: { line: string; error: string }) => `${e.line.slice(0, 80)}: ${e.error}`)
            .join('; '),
      };
    }

    setSyncDocument(nextDsl);
    const insertedIds = dslResults
      .map((r: { nodeId: string }) => r.nodeId)
      .filter((id: string) => typeof id === 'string' && id.length > 0);
    const firstId = insertedIds[0] ?? '';
    serverLog.info(
      `[exec-tool] applied batch_design DSL → ${insertedIds.length} op result(s)` +
        (dslPageId ? ` on page ${dslPageId}` : ''),
    );
    return {
      ok: true,
      document: nextDsl,
      insertedNodeId: firstId,
      insertedNodeIds: insertedIds,
    };
  }

  if (!body.name) {
    setResponseStatus(event, 400);
    return { ok: false, error: 'Missing "name" (or "dsl") in request body' };
  }

  const builder = SERVER_BUILDERS[body.name];
  if (!builder) {
    setResponseStatus(event, 404);
    return {
      ok: false,
      error:
        `Element tool "${body.name}" has no server-side builder. Phase 2 M3 currently ` +
        `covers: ${Object.keys(SERVER_BUILDERS).join(', ')}. Extend SERVER_BUILDERS in ` +
        `apps/web/server/api/mcp/exec-tool.post.ts (and the matching shim in ` +
        `apps/web/src/services/ai/element-tool-shims/index.ts) to add more.`,
    };
  }

  // Split meta fields out of the payload before the builder sees them.
  // Builder signatures only accept tool-specific params; leaving
  // parent_id/pageId in would either be silently ignored or rejected
  // as unknown. Mirrors the client shim wrap() contract.
  const rawArgs = (body.arguments ?? {}) as Record<string, unknown>;
  const parentId =
    typeof rawArgs.parent_id === 'string' && rawArgs.parent_id.length > 0
      ? rawArgs.parent_id
      : null;
  const targetPageId =
    typeof rawArgs.pageId === 'string' && rawArgs.pageId.length > 0 ? rawArgs.pageId : null;
  // `live://canvas` is pen-mcp's explicit sentinel for "the live
  // canvas" (document-manager.ts::resolveDocPath treats it the same
  // as an undefined filePath). Normalize to null so the non-live-
  // canvas rejection below only triggers for real file paths.
  const LIVE_CANVAS_SENTINEL = 'live://canvas';
  const rawFilePath =
    typeof rawArgs.filePath === 'string' && rawArgs.filePath.length > 0 ? rawArgs.filePath : null;
  const targetFilePath = rawFilePath === LIVE_CANVAS_SENTINEL ? null : rawFilePath;
  const builderArgs = (() => {
    const { parent_id: _pid, pageId: _pg, filePath: _fp, ...rest } = rawArgs;
    return rest;
  })();

  // filePath is only meaningful for pen-mcp's file-backed handlers
  // (they read/write .op files via document-manager). This endpoint
  // targets the in-memory live document (mcp-sync-state). Silently
  // dropping filePath would report success while the named file stays
  // untouched — surface the mismatch with 501 so the caller routes
  // through pen-mcp's stdio/HTTP transport instead.
  if (targetFilePath !== null) {
    setResponseStatus(event, 501);
    return {
      ok: false,
      error:
        `filePath="${targetFilePath}" is not supported by /api/mcp/exec-tool — this ` +
        `endpoint mutates the in-memory live canvas only. Route through pen-mcp's ` +
        `stdio / HTTP transport (which owns .op file I/O via document-manager), or ` +
        `omit filePath to target the currently-synced live canvas.`,
    };
  }

  let tree: ElementTree;
  try {
    tree = builder(builderArgs);
  } catch (err) {
    setResponseStatus(event, 400);
    return {
      ok: false,
      error: `Builder "${body.name}" rejected arguments: ${err instanceof Error ? err.message : String(err)}`,
    };
  }
  assignIdsRecursively(tree);
  const insertedNodeId = String(tree.id);

  const { doc: current } = getSyncDocument();
  if (!current) {
    setResponseStatus(event, 409);
    return {
      ok: false,
      error:
        'No live canvas document is currently synced. Open a document before invoking ' +
        '/api/mcp/exec-tool.',
    };
  }

  const next = structuredClone(current) as PenDocument;

  // Resolve target page: explicit pageId > first page > legacy children[]
  const pageList = Array.isArray(next.pages) ? next.pages : [];
  let targetPage: { id?: string; children: PenNode[] } | null = null;
  if (targetPageId !== null) {
    const match = pageList.find((p) => p.id === targetPageId);
    if (!match) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error: `pageId "${targetPageId}" not found in the live document.`,
      };
    }
    targetPage = match;
  } else if (pageList.length > 0) {
    targetPage = pageList[0];
  }

  // Resolve target parent: explicit parent_id > caller-supplied
  // default_parent_id > page root. Explicit parent_id must exist on
  // the chosen page (not silently inserted at root). default_parent_id
  // also must exist (caller is committing to a real target frame).
  const pageChildren: PenNode[] = targetPage
    ? targetPage.children
    : ((next.children ?? []) as PenNode[]);
  if (parentId !== null) {
    const found = findNodeInTree(pageChildren, parentId);
    if (!found) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error: `parent_id "${parentId}" not found in ${
          targetPageId !== null ? `page "${targetPageId}"` : 'the live document'
        }.`,
      };
    }
  }
  const defaultParentId =
    typeof body.default_parent_id === 'string' && body.default_parent_id.length > 0
      ? body.default_parent_id
      : null;
  if (parentId === null && defaultParentId !== null) {
    const defaultParentNode = findNodeInTree(pageChildren, defaultParentId);
    if (!defaultParentNode) {
      setResponseStatus(event, 404);
      return {
        ok: false,
        error:
          `default_parent_id "${defaultParentId}" not found in ${
            targetPageId !== null ? `page "${targetPageId}"` : 'the live document'
          }. Fix the orchestrator's subtask parent binding or omit default_parent_id ` +
          `to fall through to page-root insertion.`,
      };
    }
  }
  const effectiveParent = parentId ?? defaultParentId;

  // Append (index=Infinity) to match the streaming path's generation-
  // order semantics. Default document-store.addNode prepends (index=0)
  // because new user-created nodes want to be on top of the layer
  // panel; AI-generation wants each element to stack after earlier
  // siblings so multi-call output renders top-to-bottom as emitted.
  const updatedChildren = insertNodeInTree(
    pageChildren,
    effectiveParent,
    tree as unknown as PenNode,
    Infinity,
  );
  if (targetPage) {
    targetPage.children = updatedChildren;
  } else {
    next.children = updatedChildren;
  }

  setSyncDocument(next);
  serverLog.info(
    `[exec-tool] applied ${body.name} → node ${insertedNodeId}` +
      (effectiveParent !== null
        ? ` under ${parentId !== null ? 'parent' : 'default parent'} ${effectiveParent}`
        : '') +
      (targetPageId !== null ? ` on page ${targetPageId}` : ''),
  );

  return {
    ok: true,
    document: next,
    insertedNodeId,
    insertedNodeIds: [insertedNodeId],
  };
});
