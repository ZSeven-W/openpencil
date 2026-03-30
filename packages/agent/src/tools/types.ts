import type { z } from 'zod';
import type { FlexibleSchema } from 'ai';

export type AuthLevel = 'read' | 'create' | 'modify' | 'delete' | 'orchestrate';

export interface AgentTool<TArgs = any, TResult = any> {
  name: string;
  description: string;
  /** Zod schema or AI SDK jsonSchema() — used as inputSchema for the LLM. */
  schema: z.ZodType<TArgs> | FlexibleSchema<TArgs>;
  level: AuthLevel;
  pluginName?: string;
  execute?: (args: TArgs) => Promise<TResult>;
}

export interface ToolResult {
  success: boolean;
  data?: unknown;
  error?: string;
}

export interface ToolCallInfo {
  id: string;
  name: string;
  args: unknown;
  level: AuthLevel;
}

export interface FallbackStrategy {
  /** Appended to system prompt when tool calling is unavailable. */
  systemSuffix: string;
  /**
   * Parse the model's text response in fallback mode.
   * Called by the consumer on accumulated text events, not by the SDK internally.
   * Allows the strategy to co-locate parsing logic with the prompt it generates.
   */
  parseResponse: (text: string) => unknown;
}
