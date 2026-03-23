# @zseven-w/openpencil

[**English**](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI for [OpenPencil](https://github.com/ZSeven-W/openpencil) — control the design tool from your terminal.

## Install

```bash
npm install -g @zseven-w/openpencil
```

## Platform Support

The CLI automatically detects and launches the OpenPencil desktop app on all platforms:

| Platform    | Installation paths detected                                                                        |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS per-user (`%LOCALAPPDATA%`), per-machine (`%PROGRAMFILES%`), portable                           |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Usage

```bash
op <command> [options]
```

### Input Methods

Arguments that accept JSON or DSL can be passed in three ways:

```bash
op design '...'              # Inline string (small payloads)
op design @design.txt        # Read from file (recommended for large designs)
cat design.txt | op design - # Read from stdin (piping)
```

### App Control

```bash
op start [--desktop|--web]   # Launch OpenPencil (desktop by default)
op stop                      # Stop running instance
op status                    # Check if running
```

### Design (Batch DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Document Operations

```bash
op open [file.op]            # Open file or connect to live canvas
op save <file.op>            # Save current document
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Get current canvas selection
```

### Node Manipulation

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Code Export

```bash
op export <format> [--out file]
# Formats: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variables & Themes

```bash
op vars                      # Get variables
op vars:set <json>           # Set variables
op themes                    # Get themes
op themes:set <json>         # Set themes
op theme:save <file.optheme> # Save theme preset
op theme:load <file.optheme> # Load theme preset
op theme:list [dir]          # List theme presets
```

### Pages

```bash
op page list                 # List pages
op page add [--name N]       # Add a page
op page remove <id>          # Remove a page
op page rename <id> <name>   # Rename a page
op page reorder <id> <index> # Reorder a page
op page duplicate <id>       # Duplicate a page
```

### Import

```bash
op import:svg <file.svg>     # Import SVG file
op import:figma <file.fig>   # Import Figma .fig file
```

### Layout

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Global Flags

```text
--file <path>     Target .op file (default: live canvas)
--page <id>       Target page ID
--pretty          Human-readable JSON output
--help            Show help
--version         Show version
```

## License

MIT
