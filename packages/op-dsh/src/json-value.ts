/**
 * Local lossless-JSON value alias.
 *
 * DSH 0.1.5 no longer re-exports `JsonValue` from '@deepseek-ai/dsh-tools';
 * it lives in '@deepseek-ai/dsh-util-values'. That package is not a sensible
 * dependency for this plugin (one recursive alias), and the structural shape
 * is identical to the host's, so tool output and `presentationMeta` contracts
 * type-check against the host's own JsonValue without importing it.
 * @module @zseven-w/dsh-openpencil/json-value
 */

export type JsonValue = null | boolean | number | string | JsonValue[] | {
  [key: string]: JsonValue
}
