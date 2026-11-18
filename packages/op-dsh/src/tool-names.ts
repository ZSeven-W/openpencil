/** Canonical model-facing OpenPencil tool names. */
export const OPENPENCIL_RENDER_TOOL_NAME = 'openpencil_render' as const
export const OPENPENCIL_SELECTION_TOOL_NAME = 'openpencil_selection' as const
export const OPENPENCIL_NEW_TOOL_NAME = 'openpencil_new' as const
export const OPENPENCIL_CREATE_TOOL_NAME = 'openpencil_create' as const
export const OPENPENCIL_EDIT_TOOL_NAME = 'openpencil_edit' as const
export const OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME = 'openpencil_pipeline_begin' as const
export const OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME = 'openpencil_pipeline_context' as const
export const OPENPENCIL_PIPELINE_BATCH_TOOL_NAME = 'openpencil_pipeline_batch' as const
export const OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME = 'openpencil_pipeline_agent_run' as const
export const OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME = 'openpencil_pipeline_inspect' as const
export const OPENPENCIL_PIPELINE_FINISH_TOOL_NAME = 'openpencil_pipeline_finish' as const
export const OPENPENCIL_PIPELINE_ABORT_TOOL_NAME = 'openpencil_pipeline_abort' as const

export const OPENPENCIL_STYLE_PLAN_TOOL_NAME = 'openpencil_style_plan' as const
export const OPENPENCIL_STYLE_CONFIRM_TOOL_NAME = 'openpencil_style_confirm' as const

/** The ordered model-facing workflow for a private, unpublished design draft. */
export const OPENPENCIL_PIPELINE_TOOL_NAMES = [
  OPENPENCIL_PIPELINE_BEGIN_TOOL_NAME,
  OPENPENCIL_PIPELINE_CONTEXT_TOOL_NAME,
  OPENPENCIL_PIPELINE_BATCH_TOOL_NAME,
  OPENPENCIL_PIPELINE_AGENT_RUN_TOOL_NAME,
  OPENPENCIL_PIPELINE_INSPECT_TOOL_NAME,
  OPENPENCIL_PIPELINE_FINISH_TOOL_NAME,
  OPENPENCIL_PIPELINE_ABORT_TOOL_NAME,
] as const

/**
 * Historical render name retained only by the browser presentation layer so
 * existing conversation cards and details panels remain replayable. The host
 * deliberately does not register this alias as a model-facing tool.
 */
export const LEGACY_DESIGN_RENDER_TOOL_NAME = 'design_render' as const

export const OPENPENCIL_TOOL_NAMES = [
  OPENPENCIL_RENDER_TOOL_NAME,
  OPENPENCIL_SELECTION_TOOL_NAME,
  OPENPENCIL_NEW_TOOL_NAME,
  OPENPENCIL_CREATE_TOOL_NAME,
  OPENPENCIL_EDIT_TOOL_NAME,
  ...OPENPENCIL_PIPELINE_TOOL_NAMES,
  OPENPENCIL_STYLE_PLAN_TOOL_NAME,
  OPENPENCIL_STYLE_CONFIRM_TOOL_NAME,
] as const
