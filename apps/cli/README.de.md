# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [**Deutsch**](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI fuer [OpenPencil](https://github.com/ZSeven-W/openpencil) — steuere das Design-Tool von deinem Terminal aus.

## Installation

```bash
npm install -g @zseven-w/openpencil
```

## Plattformunterstuetzung

Das CLI erkennt und startet die OpenPencil-Desktop-App automatisch auf allen Plattformen:

| Plattform   | Erkannte Installationspfade                                                                         |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS pro Benutzer (`%LOCALAPPDATA%`), systemweit (`%PROGRAMFILES%`), portabel                        |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Verwendung

```bash
op <Befehl> [Optionen]
```

### Eingabemethoden

Argumente, die JSON oder DSL akzeptieren, koennen auf drei Arten uebergeben werden:

```bash
op design '...'              # Inline-Zeichenkette (kleine Nutzlasten)
op design @design.txt        # Aus Datei lesen (empfohlen fuer grosse Designs)
cat design.txt | op design - # Von stdin lesen (Piping)
```

### App-Steuerung

```bash
op start [--desktop|--web]   # OpenPencil starten (standardmaessig Desktop)
op stop                      # Laufende Instanz beenden
op status                    # Pruefen, ob die App laeuft
```

### Design (Batch-DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Dokumentoperationen

```bash
op open [file.op]            # Datei oeffnen oder mit aktivem Canvas verbinden
op save <file.op>            # Aktuelles Dokument speichern
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Aktuelle Canvas-Auswahl abrufen
```

### Knotenmanipulation

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Code-Export

```bash
op export <format> [--out file]
# Formate: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variablen und Themes

```bash
op vars                      # Variablen abrufen
op vars:set <json>           # Variablen setzen
op themes                    # Themes abrufen
op themes:set <json>         # Themes setzen
op theme:save <file.optheme> # Theme-Preset speichern
op theme:load <file.optheme> # Theme-Preset laden
op theme:list [dir]          # Theme-Presets auflisten
```

### Seiten

```bash
op page list                 # Seiten auflisten
op page add [--name N]       # Eine Seite hinzufuegen
op page remove <id>          # Eine Seite entfernen
op page rename <id> <name>   # Eine Seite umbenennen
op page reorder <id> <index> # Eine Seite neu anordnen
op page duplicate <id>       # Eine Seite duplizieren
```

### Import

```bash
op import:svg <file.svg>     # SVG-Datei importieren
op import:figma <file.fig>   # Figma-.fig-Datei importieren
```

### Layout

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Globale Optionen

```text
--file <path>     Ziel-.op-Datei (Standard: aktives Canvas)
--page <id>       Zielseiten-ID
--pretty          Menschenlesbare JSON-Ausgabe
--help            Hilfe anzeigen
--version         Version anzeigen
```

## Lizenz

MIT
