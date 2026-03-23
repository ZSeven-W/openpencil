import { handleGetVariables, handleSetVariables, handleSetThemes } from '@/mcp/tools/variables'
import {
  handleSaveThemePreset,
  handleLoadThemePreset,
  handleListThemePresets,
} from '@/mcp/tools/theme-presets'
import { output, outputError, parseJsonArg } from '../output'

interface GlobalFlags {
  file?: string
}

export async function cmdVars(flags: GlobalFlags): Promise<void> {
  const result = await handleGetVariables({ filePath: flags.file })
  output(result)
}

export async function cmdVarsSet(
  args: string[],
  flags: GlobalFlags & { replace?: boolean },
): Promise<void> {
  const data = (await parseJsonArg(args[0])) as Record<string, unknown>
  const result = await handleSetVariables({
    filePath: flags.file,
    variables: data as any,
    replace: flags.replace,
  })
  output(result)
}

export async function cmdThemes(flags: GlobalFlags): Promise<void> {
  const result = await handleGetVariables({ filePath: flags.file })
  output({ themes: result.themes })
}

export async function cmdThemesSet(
  args: string[],
  flags: GlobalFlags & { replace?: boolean },
): Promise<void> {
  const data = (await parseJsonArg(args[0])) as Record<string, unknown>
  const result = await handleSetThemes({
    filePath: flags.file,
    themes: data as any,
    replace: flags.replace,
  })
  output(result)
}

export async function cmdThemeSave(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const presetPath = args[0]
  if (!presetPath) outputError('Usage: op theme:save <file.optheme>')
  const result = await handleSaveThemePreset({
    filePath: flags.file,
    presetPath,
  })
  output(result)
}

export async function cmdThemeLoad(
  args: string[],
  flags: GlobalFlags,
): Promise<void> {
  const presetPath = args[0]
  if (!presetPath) outputError('Usage: op theme:load <file.optheme>')
  const result = await handleLoadThemePreset({
    filePath: flags.file,
    presetPath,
  })
  output(result)
}

export async function cmdThemeList(args: string[]): Promise<void> {
  if (!args[0]) outputError('Usage: op theme:list <directory>')
  const result = await handleListThemePresets({
    directory: args[0],
  })
  output(result)
}
