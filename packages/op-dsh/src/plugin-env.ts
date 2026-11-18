/**
 * Plugin-owned environment variables live under `DSHPLUGIN_`, never `DSH_`.
 *
 * DSH reserves the ENTIRE `DSH_` prefix for host bootstrap settings and
 * refuses to load any `.env` file that sets one: `@deepseek-ai/dsh-app-boot`
 * (`BOOTSTRAP_PREFIXES`) throws on the first match and rejects the whole file,
 * so `dsh web` aborts at startup. A user who configured this plugin the way
 * DSH teaches — `~/.dsh/.env`, or the working directory's `.env` — therefore
 * took the host down with an error naming a variable this plugin owns
 * (dsh-openpencil issue #6).
 *
 * Legacy `DSH_`-prefixed names are still READ so existing shell exports keep
 * working; they are removed in the first 1.0.0 prerelease. Reading them cannot
 * make them work in a `.env` file — the host rejects that file before any
 * plugin loads — which is why migration means REPLACING the old assignment,
 * never adding the new name beside it.
 */

export const PLUGIN_ENV_PREFIX = 'DSHPLUGIN_'
export const LEGACY_ENV_PREFIX = 'DSH_'

/** Name of the plugin, used to attribute the deprecation warning. */
const PLUGIN_LABEL = 'dsh-openpencil'

/** One warning per legacy variable per process, not one per lookup. */
const warned = new Set<string>()

/** Reset the once-per-process warning ledger (tests only). */
export function resetPluginEnvWarnings(): void {
  warned.clear()
}

/** Current name of a plugin variable, given the suffix after the prefix. */
export function pluginEnvName(suffix: string): string {
  return `${PLUGIN_ENV_PREFIX}${suffix}`
}

/** Legacy (pre-rename) name of the same variable. */
export function legacyEnvName(suffix: string): string {
  return `${LEGACY_ENV_PREFIX}${suffix}`
}

export interface PluginEnvOptions {
  env?: Readonly<Record<string, string | undefined>>
  warn?: (message: string) => void
}

/**
 * Read one plugin variable by its suffix (e.g. `OPENPENCIL_BINARY`).
 *
 * Resolution is by PRESENCE, not truthiness: a new variable set to the empty
 * string is a deliberate "no override" and must not silently resurrect a stale
 * legacy value. Validation of the value itself stays with the caller.
 */
export function pluginEnv(suffix: string, options: PluginEnvOptions = {}): string | undefined {
  const env = options.env ?? process.env
  const warn = options.warn ?? ((message: string) => { process.stderr.write(`${message}\n`) })
  const current = pluginEnvName(suffix)
  const legacy = legacyEnvName(suffix)
  const hasCurrent = env[current] !== undefined
  const hasLegacy = env[legacy] !== undefined
  if (hasLegacy && !warned.has(legacy)) {
    warned.add(legacy)
    // Names and precedence only — never the value, which is often a path.
    warn(
      `${PLUGIN_LABEL}: ${legacy} is deprecated; rename it to ${current}`
      + `${hasCurrent ? ` (${current} is also set and takes precedence)` : ''}. `
      + `DSH reserves the ${LEGACY_ENV_PREFIX} prefix for itself: a .env file that sets ${legacy} makes the `
      + 'host abort at startup, so REPLACE the old assignment — adding the new name beside it does not help.',
    )
  }
  return hasCurrent ? env[current] : env[legacy]
}
