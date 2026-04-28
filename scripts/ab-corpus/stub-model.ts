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
