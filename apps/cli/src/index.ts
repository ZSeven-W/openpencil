#!/usr/bin/env node

import pkg from '../package.json'
import { setPretty, output, outputError } from './output'

// --- Arg parsing ---

interface ParsedArgs {
  command: string
  positionals: string[]
  flags: Record<string, string | boolean>
}

function parseArgs(argv: string[]): ParsedArgs {
  const args = argv.slice(2)
  const positionals: string[] = []
  const flags: Record<string, string | boolean> = {}

  for (let i = 0; i < args.length; i++) {
    const arg = args[i]
    if (arg.startsWith('--')) {
      const key = arg.slice(2)
      const next = args[i + 1]
      if (next && !next.startsWith('--')) {
        flags[key] = next
        i++
      } else {
        flags[key] = true
      }
    } else {
      positionals.push(arg)
    }
  }

  return {
    command: positionals[0] ?? '',
    positionals: positionals.slice(1),
    flags,
  }
}

// --- Help ---

const HELP = `OpenPencil CLI v${pkg.version}

Usage: op <command> [options]

App:
  op start [--desktop|--web]    Launch OpenPencil
  op stop                       Stop running instance
  op status                     Check if running

Document:
  op open [file.op]             Open file or connect to live canvas
  op save <file.op>             Save current document to file
  op get [--type X] [--name Y] [--id Z] [--depth N]
  op selection                  Get current canvas selection

Nodes:
  op insert <json> [--parent P] [--index N] [--post-process]
  op update <id> <json> [--post-process]
  op delete <id>
  op move <id> --parent <P> [--index N]
  op copy <id> [--parent P]
  op replace <id> <json> [--post-process]

Design:
  op design <dsl|@file|->
  op design:skeleton <json|@file|->
  op design:content <section-id> <json|@file|->
  op design:refine --root-id <id>

Export:
  op export <format> [--out file]
  Formats: react, html, vue, svelte, flutter, swiftui, compose, rn, css

Variables & Themes:
  op vars                       Get variables
  op vars:set <json>            Set variables
  op themes                     Get themes
  op themes:set <json>          Set themes
  op theme:save <file.optheme>  Save theme preset
  op theme:load <file.optheme>  Load theme preset
  op theme:list [dir]           List theme presets

Pages:
  op page list
  op page add [--name N]
  op page remove <id>
  op page rename <id> <name>
  op page reorder <id> <index>
  op page duplicate <id>

Import:
  op import:svg <file.svg>      Import SVG file
  op import:figma <file.fig>    Import Figma file

Layout:
  op layout [--parent P] [--depth N]
  op find-space [--direction right|bottom|left|top]

Arguments that accept JSON or DSL can be passed as:
  <value>           Inline string
  @filepath         Read from file (e.g. @design.txt)
  -                 Read from stdin (e.g. cat design.txt | op design -)

Global Flags:
  --file <path>     Target .op file (default: live canvas)
  --page <id>       Target page ID
  --pretty          Human-readable JSON output
  --help            Show this help
  --version         Show version
`

// --- Main ---

async function main(): Promise<void> {
  const { command, positionals, flags } = parseArgs(process.argv)

  if (flags.pretty) setPretty(true)
  if (flags.help || command === 'help') {
    process.stdout.write(HELP)
    return
  }
  if (flags.version || command === 'version') {
    output({ version: pkg.version })
    return
  }
  if (!command) {
    process.stdout.write(HELP)
    return
  }

  const globalFlags = {
    file: flags.file as string | undefined,
    page: flags.page as string | undefined,
  }

  switch (command) {
    // --- App ---
    case 'start': {
      const { cmdStart } = await import('./commands/app')
      await cmdStart({ desktop: !!flags.desktop, web: !!flags.web })
      break
    }
    case 'stop': {
      const { cmdStop } = await import('./commands/app')
      await cmdStop()
      break
    }
    case 'status': {
      const { cmdStatus } = await import('./commands/app')
      await cmdStatus()
      break
    }

    // --- Document ---
    case 'open': {
      const { cmdOpen } = await import('./commands/document')
      await cmdOpen(positionals, globalFlags)
      break
    }
    case 'save': {
      const { cmdSave } = await import('./commands/document')
      await cmdSave(positionals, globalFlags)
      break
    }
    case 'get': {
      const { cmdGet } = await import('./commands/document')
      await cmdGet(positionals, {
        ...globalFlags,
        type: flags.type as string | undefined,
        name: flags.name as string | undefined,
        id: flags.id as string | undefined,
        depth: flags.depth as string | undefined,
        parent: flags.parent as string | undefined,
      })
      break
    }
    case 'selection': {
      const { cmdSelection } = await import('./commands/document')
      await cmdSelection({ ...globalFlags, depth: flags.depth as string | undefined })
      break
    }

    // --- Nodes ---
    case 'insert': {
      const { cmdInsert } = await import('./commands/nodes')
      await cmdInsert(positionals, {
        ...globalFlags,
        parent: flags.parent as string | undefined,
        index: flags.index as string | undefined,
        postProcess: !!flags['post-process'],
      })
      break
    }
    case 'update': {
      const { cmdUpdate } = await import('./commands/nodes')
      await cmdUpdate(positionals, {
        ...globalFlags,
        postProcess: !!flags['post-process'],
      })
      break
    }
    case 'delete': {
      const { cmdDelete } = await import('./commands/nodes')
      await cmdDelete(positionals, globalFlags)
      break
    }
    case 'move': {
      const { cmdMove } = await import('./commands/nodes')
      await cmdMove(positionals, {
        ...globalFlags,
        parent: flags.parent as string | undefined,
        index: flags.index as string | undefined,
      })
      break
    }
    case 'copy': {
      const { cmdCopy } = await import('./commands/nodes')
      await cmdCopy(positionals, {
        ...globalFlags,
        parent: flags.parent as string | undefined,
      })
      break
    }
    case 'replace': {
      const { cmdReplace } = await import('./commands/nodes')
      await cmdReplace(positionals, {
        ...globalFlags,
        postProcess: !!flags['post-process'],
      })
      break
    }

    // --- Design ---
    case 'design': {
      const { cmdDesign } = await import('./commands/design')
      await cmdDesign(positionals, {
        ...globalFlags,
        postProcess: flags['post-process'] !== false ? true : undefined,
        canvasWidth: flags['canvas-width'] as string | undefined,
      })
      break
    }
    case 'design:skeleton': {
      const { cmdDesignSkeleton } = await import('./commands/design')
      await cmdDesignSkeleton(positionals, globalFlags)
      break
    }
    case 'design:content': {
      const { cmdDesignContent } = await import('./commands/design')
      await cmdDesignContent(positionals, {
        ...globalFlags,
        canvasWidth: flags['canvas-width'] as string | undefined,
      })
      break
    }
    case 'design:refine': {
      const { cmdDesignRefine } = await import('./commands/design')
      await cmdDesignRefine(positionals, {
        ...globalFlags,
        rootId: flags['root-id'] as string | undefined,
        canvasWidth: flags['canvas-width'] as string | undefined,
      })
      break
    }

    // --- Export ---
    case 'export': {
      const { cmdExport } = await import('./commands/export')
      await cmdExport(positionals, {
        file: globalFlags.file,
        out: flags.out as string | undefined,
      })
      break
    }

    // --- Variables & Themes ---
    case 'vars': {
      const { cmdVars } = await import('./commands/variables')
      await cmdVars(globalFlags)
      break
    }
    case 'vars:set': {
      const { cmdVarsSet } = await import('./commands/variables')
      await cmdVarsSet(positionals, { ...globalFlags, replace: !!flags.replace })
      break
    }
    case 'themes': {
      const { cmdThemes } = await import('./commands/variables')
      await cmdThemes(globalFlags)
      break
    }
    case 'themes:set': {
      const { cmdThemesSet } = await import('./commands/variables')
      await cmdThemesSet(positionals, { ...globalFlags, replace: !!flags.replace })
      break
    }
    case 'theme:save': {
      const { cmdThemeSave } = await import('./commands/variables')
      await cmdThemeSave(positionals, globalFlags)
      break
    }
    case 'theme:load': {
      const { cmdThemeLoad } = await import('./commands/variables')
      await cmdThemeLoad(positionals, globalFlags)
      break
    }
    case 'theme:list': {
      const { cmdThemeList } = await import('./commands/variables')
      await cmdThemeList(positionals)
      break
    }

    // --- Pages ---
    case 'page': {
      const subCmd = positionals[0]
      const subArgs = positionals.slice(1)
      switch (subCmd) {
        case 'list': {
          const { cmdPageList } = await import('./commands/pages')
          await cmdPageList(globalFlags)
          break
        }
        case 'add': {
          const { cmdPageAdd } = await import('./commands/pages')
          await cmdPageAdd(subArgs, { ...globalFlags, name: flags.name as string | undefined })
          break
        }
        case 'remove': {
          const { cmdPageRemove } = await import('./commands/pages')
          await cmdPageRemove(subArgs, globalFlags)
          break
        }
        case 'rename': {
          const { cmdPageRename } = await import('./commands/pages')
          await cmdPageRename(subArgs, globalFlags)
          break
        }
        case 'reorder': {
          const { cmdPageReorder } = await import('./commands/pages')
          await cmdPageReorder(subArgs, globalFlags)
          break
        }
        case 'duplicate': {
          const { cmdPageDuplicate } = await import('./commands/pages')
          await cmdPageDuplicate(subArgs, globalFlags)
          break
        }
        default:
          outputError(`Unknown page subcommand: "${subCmd}". Use: list, add, remove, rename, reorder, duplicate`)
      }
      break
    }

    // --- Import ---
    case 'import:svg': {
      const { cmdImportSvg } = await import('./commands/import')
      await cmdImportSvg(positionals, {
        ...globalFlags,
        parent: flags.parent as string | undefined,
      })
      break
    }
    case 'import:figma': {
      const { cmdImportFigma } = await import('./commands/import')
      await cmdImportFigma(positionals, {
        ...globalFlags,
        out: flags.out as string | undefined,
      })
      break
    }

    // --- Layout ---
    case 'layout': {
      const { cmdLayout } = await import('./commands/layout')
      await cmdLayout({
        ...globalFlags,
        parent: flags.parent as string | undefined,
        depth: flags.depth as string | undefined,
      })
      break
    }
    case 'find-space': {
      const { cmdFindSpace } = await import('./commands/layout')
      await cmdFindSpace({
        ...globalFlags,
        direction: flags.direction as string | undefined,
        width: flags.width as string | undefined,
        height: flags.height as string | undefined,
      })
      break
    }

    default:
      outputError(`Unknown command: "${command}". Run "op --help" for usage.`)
  }
}

main().catch((err) => {
  outputError(err instanceof Error ? err.message : String(err))
})
