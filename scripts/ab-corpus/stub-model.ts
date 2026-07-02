/**
 * Offline model stub for --dry-run mode. Returns hardcoded outputs that
 * cover all four kinds the scorer can see (tool_call, batch_design,
 * garbage, plus a "mostly right" batch_design that passes shape checks
 * too). Used to verify the full pipeline without burning API credits.
 *
 * Keyed by (promptId, variant) so individual prompts can be told to
 * return specific shapes. Fallback response when a prompt isn't
 * explicitly fixtured: treatment variant returns a generic
 * batch_design (simulates "weak model didn't route to element tool"),
 * baseline returns a minimal valid design.
 *
 * When the real model adapter lands, swap this out with a thin
 * wrapper around apps/web/src/services/ai/ai-service.ts::streamChat
 * (or a provider-native fetch). The return-shape contract — a single
 * raw string the scorer parses — stays the same.
 */

import type { CorpusPrompt } from '@zseven-w/pen-ai-skills';
import type { ChatCallResult } from './clients/openai-compat';

export interface ModelCall {
  model: string;
  prompt: CorpusPrompt;
  variant: 'B' | 'T';
  /** Full resolved system+user prompt text sent to the model. Ignored
   *  by the stub, kept so the real adapter has the same signature. */
  systemPrompt: string;
  userPrompt: string;
}

export async function stubModelCall(call: ModelCall): Promise<ChatCallResult> {
  const key = `${call.prompt.id}|${call.variant}`;
  const fixture = FIXTURES[key];
  // Fallback: treatment-arm on unfixtured prompts simulates a weak
  // model that knew about the tools but picked batch_design anyway
  // (interesting M5 signal); baseline always emits batch_design DSL.
  const content = fixture ?? DEFAULT_BATCH_DESIGN;
  // Synthesize plausible token counts so dry-run reports exercise the
  // token-cost code path. ~4 chars per token is the canonical OpenAI
  // tokenizer rule of thumb; we use length/4 for both prompt and
  // completion. The harness consumer doesn't know it's a stub at this
  // layer, so producing zeros would skip token aggregation entirely.
  const promptChars = call.systemPrompt.length + call.userPrompt.length;
  return {
    content,
    usage: {
      promptTokens: Math.max(1, Math.round(promptChars / 4)),
      completionTokens: Math.max(1, Math.round(content.length / 4)),
    },
  };
}

// batch_design's DSL parser expects one operation per line. Keep this
// on a single physical line or it fails at apply time — the point of
// a fallback stub is to produce a *legal* design so M1 passes.
const DEFAULT_BATCH_DESIGN =
  'root=I(null, {"type":"frame","name":"Root","role":"section","width":375,"height":812,"layout":"vertical","children":[]})';

/**
 * A few hand-crafted fixtures so the dry-run report has variety in it
 * (some M1 passes, some fails, some tool_calls, some batch_design). The
 * shapes here aren't meant to be realistic model outputs — they're just
 * enough to exercise the pipeline's branches.
 */
const FIXTURES: Record<string, string> = {
  // mobile-filter-chips (obvious) in treatment: correct tool_call routing
  'mobile-filter-chips|T': [
    '<op_tool>',
    '{"name": "add_nav_chip_row_v0", "arguments": {',
    '  "items": [',
    '    {"label": "All", "active": true},',
    '    {"label": "Videos", "icon": "video"},',
    '    {"label": "Photos", "icon": "image"},',
    '    {"label": "Music", "icon": "music"},',
    '    {"label": "Articles", "icon": "file-text"},',
    '    {"label": "Podcasts", "icon": "headphones"},',
    '    {"label": "Events", "icon": "calendar"},',
    '    {"label": "Books", "icon": "book"}',
    '  ]',
    '}}',
    '</op_tool>',
  ].join('\n'),
  // mobile-empty-inbox (obvious) in treatment: correct tool routing
  'mobile-empty-inbox|T': [
    '<op_tool>',
    '{"name": "add_empty_state_v0", "arguments": {',
    '  "title": "No messages yet",',
    '  "subtitle": "New conversations will appear here",',
    '  "icon": "inbox",',
    '  "cta_label": "Start a conversation"',
    '}}',
    '</op_tool>',
  ].join('\n'),
  // dashboard-alert-banner (obvious) in treatment: correct routing
  'dashboard-alert-banner|T': [
    '<op_tool>',
    '{"name": "add_alert_v0", "arguments": {',
    '  "message": "Your subscription expires in 7 days.",',
    '  "icon": "triangle-alert",',
    '  "dismissible": true',
    '}}',
    '</op_tool>',
  ].join('\n'),
  // landing-testimonial (obvious) in treatment: correct routing
  'landing-testimonial|T': [
    '<op_tool>',
    '{"name": "add_quote_block_v0", "arguments": {',
    '  "quote": "OpenPencil replaced two tools in our stack.",',
    '  "author": "Alex Chen"',
    '}}',
    '</op_tool>',
  ].join('\n'),
  // landing-product-ratings (obvious) in treatment: WRONG tool routing.
  // Prompt asks for star rating (expected_tool_if_any=add_rating_stars_v0)
  // but stub simulates a weak model routing to add_badge_v0 — the
  // badge IS a schema-valid tool call, just wrong intent. Exercises
  // the wrong-tool routing classification.
  'landing-product-ratings|T': [
    '<op_tool>',
    '{"name": "add_badge_v0", "arguments": {',
    '  "label": "4.8"',
    '}}',
    '</op_tool>',
  ].join('\n'),
  // Composite multi-tool fixtures — exercise the new tool_calls list
  // path end-to-end without burning live API credits. Each value is a
  // multi-tag raw string the parser will split into multiple element-
  // tool calls (apply.ts loops over them, byTool tallies per-name).
  // These are the only stubs that produce >1 op_tool block; obvious
  // prompts still emit 1 by default.
  'dashboard-settings-page-composite|T': [
    '<op_tool>{"name":"add_section_header_v0","arguments":{"title":"Notifications"}}</op_tool>',
    '<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Choose what you want to be alerted about."}}</op_tool>',
    '<op_tool>{"name":"add_setting_row_v0","arguments":{"title":"Email digest","subtitle":"Daily summary at 9am","leading_icon":"bell","trailing":{"kind":"switch","on":true}}}</op_tool>',
    '<op_tool>{"name":"add_setting_row_v0","arguments":{"title":"Push notifications","subtitle":"All channels","leading_icon":"phone","trailing":{"kind":"switch","on":true}}}</op_tool>',
    '<op_tool>{"name":"add_setting_row_v0","arguments":{"title":"Direct messages","subtitle":"Mentions only","leading_icon":"message-circle","trailing":{"kind":"switch","on":false}}}</op_tool>',
    '<op_tool>{"name":"add_setting_row_v0","arguments":{"title":"Critical alerts","subtitle":"Always on","leading_icon":"alert-triangle","trailing":{"kind":"switch","on":true}}}</op_tool>',
  ].join('\n'),
  'dashboard-team-people-page-composite|T': [
    '<op_tool>{"name":"add_section_header_v0","arguments":{"title":"Members"}}</op_tool>',
    '<op_tool>{"name":"add_body_text_v0","arguments":{"content":"5 people"}}</op_tool>',
    '<op_tool>{"name":"add_member_row_v0","arguments":{"name":"Sarah Lee","subtitle":"sarah@acme.com","initial":"S","trailing":{"kind":"role_badge","value":"Owner"}}}</op_tool>',
    '<op_tool>{"name":"add_member_row_v0","arguments":{"name":"Marcus Chen","subtitle":"marcus@acme.com","initial":"M","trailing":{"kind":"role_badge","value":"Admin"}}}</op_tool>',
    '<op_tool>{"name":"add_member_row_v0","arguments":{"name":"Aiko Tanaka","subtitle":"aiko@acme.com","initial":"A","trailing":{"kind":"role_badge","value":"Editor"}}}</op_tool>',
    '<op_tool>{"name":"add_member_row_v0","arguments":{"name":"Raj Patel","subtitle":"raj@acme.com","initial":"R","trailing":{"kind":"role_badge","value":"Editor"}}}</op_tool>',
    '<op_tool>{"name":"add_member_row_v0","arguments":{"name":"Jordan Kim","subtitle":"jordan@acme.com","initial":"J","trailing":{"kind":"role_badge","value":"Viewer"}}}</op_tool>',
    '<op_tool>{"name":"add_invite_row_v0","arguments":{"email":"leon@acme.com","role":"Editor","status":"pending","action_label":"Resend"}}</op_tool>',
  ].join('\n'),
  'dashboard-search-filters-composite|T': [
    '<op_tool>{"name":"add_filter_group_v0","arguments":{"title":"Category","options":[{"label":"Apparel","count":124,"selected":true},{"label":"Footwear","count":86},{"label":"Bags","count":41},{"label":"Accessories","count":67}]}}</op_tool>',
    '<op_tool>{"name":"add_filter_group_v0","arguments":{"title":"Brand","options":[{"label":"Nike","count":32},{"label":"Adidas","count":28},{"label":"Patagonia","count":15,"selected":true},{"label":"Arc Teryx","count":9}]}}</op_tool>',
  ].join('\n'),
  'dashboard-audit-feed-composite|T': [
    '<op_tool>{"name":"add_section_header_v0","arguments":{"title":"Recent activity"}}</op_tool>',
    '<op_tool>{"name":"add_body_text_v0","arguments":{"content":"Last 24 hours"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"Sarah Lee","action":"approved the production deploy","timestamp":"2h ago","icon":"check","tone":"success"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"Marcus Chen","action":"uploaded final-mockups.zip","timestamp":"3h ago","icon":"upload","tone":"info"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"Aiko Tanaka","action":"invited jordan@acme.com to the workspace","timestamp":"5h ago","icon":"user-plus","tone":"info"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"System","action":"rate-limited an IP after 50 failed sign-ins","timestamp":"8h ago","icon":"alert-triangle","tone":"warning"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"Raj Patel","action":"updated billing details","timestamp":"yesterday","icon":"settings","tone":"neutral"}}</op_tool>',
    '<op_tool>{"name":"add_activity_log_v0","arguments":{"actor":"Sarah Lee","action":"deleted archive-2024.zip","timestamp":"yesterday","icon":"trash","tone":"danger"}}</op_tool>',
  ].join('\n'),
  'mobile-onboarding-flow-composite|T': [
    '<op_tool>{"name":"add_step_card_v0","arguments":{"number":1,"title":"Create your account","description":"Use your email and a strong password to sign up. No credit card required."}}</op_tool>',
    '<op_tool>{"name":"add_step_card_v0","arguments":{"number":2,"title":"Connect your bank","description":"Link your account in seconds. We use 256-bit encryption to keep your data safe."}}</op_tool>',
    '<op_tool>{"name":"add_step_card_v0","arguments":{"number":3,"title":"Set your goals","description":"Tell us what you want to save for — we will do the rest."}}</op_tool>',
    '<op_tool>{"name":"add_step_card_v0","arguments":{"number":"4","title":"You are all set","description":"You are ready to use the app. Tap below to continue.","completed":true}}</op_tool>',
  ].join('\n'),
  // Baseline garbage response (simulates a weak model producing prose)
  'mobile-tabs-demo|B':
    'I can help you design that! Let me create a tabs demo for you with three underline tabs.',
  // Treatment garbage on an OBVIOUS prompt — exercises the Garbage
  // column in the routing breakdown. Codex stop-hook 2026-04-20 fix:
  // if garbage isn't counted in the M5 denominator, right-tool rates
  // look rosy even when most outputs fail to parse.
  'dashboard-wizard-stepper|T':
    'Sure! Here is a stepper wizard header: <thinking about design> ... actually let me just describe it.',
};
