import { openDocument, resolveDocPath } from '@/mcp/document-manager'
import {
  generateReactFromDocument,
  generateHTMLFromDocument,
  generateVueFromDocument,
  generateSvelteFromDocument,
  generateFlutterFromDocument,
  generateSwiftUIFromDocument,
  generateComposeFromDocument,
  generateReactNativeFromDocument,
  generateCSSVariables,
} from '@zseven-w/pen-codegen'
import { writeFile } from 'node:fs/promises'
import { output, outputError } from '../output'

type GeneratorResult = string | { html: string; css: string }

const GENERATORS: Record<string, (doc: any) => GeneratorResult> = {
  react: generateReactFromDocument,
  html: generateHTMLFromDocument,
  vue: generateVueFromDocument,
  svelte: generateSvelteFromDocument,
  flutter: generateFlutterFromDocument,
  swiftui: generateSwiftUIFromDocument,
  compose: generateComposeFromDocument,
  rn: generateReactNativeFromDocument,
  'react-native': generateReactNativeFromDocument,
  css: (doc: any) => generateCSSVariables(doc.variables ?? {}),
}

function resultToString(result: GeneratorResult): string {
  if (typeof result === 'string') return result
  // HTML generator returns { html, css }
  const parts: string[] = []
  if (result.css) parts.push(`<style>\n${result.css}\n</style>`)
  parts.push(result.html)
  return parts.join('\n\n')
}

export async function cmdExport(
  args: string[],
  flags: { file?: string; out?: string },
): Promise<void> {
  const format = args[0]
  if (!format) {
    outputError(
      `Usage: op export <format> [--out file]\nFormats: ${Object.keys(GENERATORS).join(', ')}`,
    )
  }
  const generator = GENERATORS[format]
  if (!generator) {
    outputError(`Unknown format: "${format}". Available: ${Object.keys(GENERATORS).join(', ')}`)
  }

  const filePath = resolveDocPath(flags.file)
  const doc = await openDocument(filePath)
  const result = generator(doc)
  const code = resultToString(result)

  if (flags.out) {
    await writeFile(flags.out, code, 'utf-8')
    output({ ok: true, format, file: flags.out, length: code.length })
  } else {
    process.stdout.write(code)
  }
}
