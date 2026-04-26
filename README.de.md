<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Das weltweit erste KI-native Open-Source-Vektordesign-Werkzeug.</strong><br />
  <sub>Parallele Agententeams &bull; Design-as-Code &bull; Eingebauter MCP-Server &bull; Multi-Modell-Intelligenz</sub>
</p>

<p align="center">
  <a href="./README.md"><b>English</b></a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — Klicken, um das Demo-Video anzusehen" width="100%" />
  </a>
</p>
<p align="center"><sub>Auf das Bild klicken, um das Demo-Video anzusehen</sub></p>

<br />

> **Hinweis:** Es gibt ein weiteres Open-Source-Projekt mit demselben Namen — [OpenPencil](https://github.com/open-pencil/open-pencil), das sich auf Figma-kompatibles visuelles Design mit Echtzeit-Zusammenarbeit konzentriert. Dieses Projekt konzentriert sich auf AI-native Design-to-Code-Workflows.

## Warum OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

Beschreiben Sie jede UI in natürlicher Sprache. Beobachten Sie, wie sie in Echtzeit mit Streaming-Animation auf der unendlichen Canvas erscheint. Ändern Sie bestehende Designs, indem Sie Elemente auswählen und chatten.

</td>
<td width="50%">

### 🤖 Parallele Agententeams

Der Orchestrierer zerlegt komplexe Seiten in räumliche Teilaufgaben. Mehrere KI-Agenten arbeiten gleichzeitig an verschiedenen Bereichen — Hero, Features, Footer — alle parallel streamend.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Multi-Modell-Intelligenz

Passt sich automatisch an die Fähigkeiten jedes Modells an. Claude erhält vollständige Prompts mit Thinking; GPT-4o/Gemini deaktivieren Thinking; kleinere Modelle (MiniMax, Qwen, Llama) erhalten vereinfachte Prompts für zuverlässige Ausgabe.

</td>
<td width="50%">

### 🔌 MCP-Server

Ein-Klick-Installation in Claude Code, Codex, Gemini, OpenCode, Kiro oder Copilot CLIs. Designen Sie aus Ihrem Terminal — `.op`-Dateien über jeden MCP-kompatiblen Agenten lesen, erstellen und bearbeiten.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

`.op`-Dateien sind JSON — menschenlesbar, Git-freundlich, diff-fähig. Designvariablen generieren CSS Custom Properties. Code-Export nach React + Tailwind oder HTML + CSS.

</td>
<td width="50%">

### 🖥️ Läuft überall

Web-App + native Desktop-Anwendung auf macOS, Windows und Linux über Electron. Auto-Updates über GitHub Releases. `.op`-Dateizuordnung — Doppelklick zum Öffnen.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Steuern Sie das Design-Tool vom Terminal aus. `op design`, `op insert` — Batch-Design-DSL, Knotenmanipulation. Pipe-Eingabe von Dateien oder stdin. Funktioniert mit der Desktop-App oder dem Webserver.

</td>
<td width="50%">

### 🎯 Multiplattform-Code-Export

Export aus einer einzigen `.op`-Datei nach React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Design-Variablen werden zu CSS Custom Properties.

</td>
</tr>
</table>

## Schnellstart

```bash
# Abhängigkeiten installieren
bun install

# Entwicklungsserver auf http://localhost:3000 starten
bun --bun run dev
```

Oder als Desktop-App ausführen:

```bash
bun run electron:dev
```

> **Voraussetzungen:** [Bun](https://bun.sh/) >= 1.0 und [Node.js](https://nodejs.org/) >= 18

### Docker

Mehrere Image-Varianten sind verfügbar — wählen Sie die passende für Ihre Anforderungen:

| Image                        | Größe   | Enthält              |
| ---------------------------- | ------- | -------------------- |
| `openpencil:latest`          | ~226 MB | Nur Web-App          |
| `openpencil-claude:latest`   | —       | + Claude Code CLI    |
| `openpencil-codex:latest`    | —       | + Codex CLI          |
| `openpencil-opencode:latest` | —       | + OpenCode CLI       |
| `openpencil-copilot:latest`  | —       | + GitHub Copilot CLI |
| `openpencil-gemini:latest`   | —       | + Gemini CLI         |
| `openpencil-full:latest`     | ~1 GB   | Alle CLI-Tools       |

**Ausführen (nur Web):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Mit KI-CLI ausführen (z.B. Claude Code):**

Der KI-Chat basiert auf Claude CLI OAuth-Login. Verwenden Sie ein Docker-Volume, um die Login-Sitzung beizubehalten:

```bash
# Schritt 1 — Login (einmalig)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Schritt 2 — Starten
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Lokal bauen:**

```bash
# Basis (nur Web)
docker build --target base -t openpencil .

# Mit einem bestimmten CLI
docker build --target with-claude -t openpencil-claude .

# Vollständig (alle CLIs)
docker build --target full -t openpencil-full .
```

## KI-natives Design

**Vom Prompt zur UI**

- **Text-zu-Design** — eine Seite beschreiben und sie wird in Echtzeit mit Streaming-Animation auf der Canvas generiert
- **Orchestrierer** — zerlegt komplexe Seiten in räumliche Teilaufgaben zur parallelen Generierung
- **Design-Modifikation** — Elemente auswählen und Änderungen in natürlicher Sprache beschreiben
- **Bildeingabe** — Screenshots oder Mockups als Referenz für referenzbasiertes Design anhängen

**Multi-Agenten-Unterstützung**

| Agent                        | Einrichtung                                                                                     |
| ---------------------------- | ----------------------------------------------------------------------------------------------- |
| **Integriert (9+ Anbieter)** | Auswahl aus Anbieter-Presets mit Region-Switcher — Anthropic, OpenAI, Google, DeepSeek und mehr |
| **Claude Code**              | Keine Konfiguration — verwendet Claude Agent SDK mit lokalem OAuth                              |
| **Codex CLI**                | In den Agenteneinstellungen verbinden (`Cmd+,`)                                                 |
| **OpenCode**                 | In den Agenteneinstellungen verbinden (`Cmd+,`)                                                 |
| **GitHub Copilot**           | `copilot login` dann in den Agenteneinstellungen verbinden (`Cmd+,`)                            |
| **Gemini CLI**               | In den Agenteneinstellungen verbinden (`Cmd+,`)                                                 |

**Modell-Fähigkeitsprofile** — passt Prompts, Thinking-Modus und Timeouts automatisch pro Modellstufe an. Modelle der Vollstufe (Claude) erhalten vollständige Prompts; Standardstufe (GPT-4o, Gemini, DeepSeek) deaktiviert Thinking; Basisstufe (MiniMax, Qwen, Llama, Mistral) erhält vereinfachte verschachtelte JSON-Prompts für maximale Zuverlässigkeit.

**i18n** — Vollständige Interface-Lokalisierung in 15 Sprachen: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**MCP-Server**

- Eingebauter MCP-Server — Ein-Klick-Installation in Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Automatische Node.js-Erkennung — falls nicht installiert, automatischer Fallback auf HTTP-Transport und automatischer Start des MCP-HTTP-Servers
- Design-Automatisierung vom Terminal aus: `.op`-Dateien über jeden MCP-kompatiblen Agenten lesen, erstellen und bearbeiten
- **Mehrstufiger Design-Workflow** — `design_skeleton` → `design_content` → `design_refine` für hochwertigere mehrteilige Designs
- **Segmentierter Prompt-Abruf** — laden Sie nur das benötigte Design-Wissen (Schema, Layout, Rollen, Icons, Planung usw.)
- Mehrseitige Unterstützung — Seiten erstellen, umbenennen, neu ordnen und duplizieren über MCP-Tools

**Codegenerierung**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Global installieren und das Design-Tool vom Terminal aus steuern:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Desktop-App starten
op design @landing.txt       # Batch-Design aus Datei
op insert '{"type":"RECT"}'  # Knoten einfügen
op import:figma design.fig   # Figma-Datei importieren
cat design.dsl | op design - # Pipe von stdin
```

Unterstützt drei Eingabemethoden: Inline-String, `@filepath` (aus Datei lesen) oder `-` (von stdin lesen). Funktioniert mit der Desktop-App oder dem Web-Entwicklungsserver. Siehe [CLI README](./apps/cli/README.md) für die vollständige Befehlsreferenz.

**LLM-Skill** — Installieren Sie das [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill)-Plugin, um KI-Agenten (Claude Code, Cursor, Codex, Gemini CLI usw.) das Designen mit `op` beizubringen.

## Funktionen

**Canvas und Zeichnen**

- Unendliche Canvas mit Pan, Zoom, intelligenten Ausrichtungshilfslinien und Einrasten
- Rechteck, Ellipse, Linie, Polygon, Stift (Bezier), Frame, Text
- Boolesche Operationen — Vereinigung, Subtraktion, Schnittmenge mit kontextbezogener Werkzeugleiste
- Icon-Auswahl (Iconify) und Bildimport (PNG/JPEG/SVG/WebP/GIF)
- Auto-Layout — vertikal/horizontal mit Gap, Padding, Justify, Align
- Mehrseitige Dokumente mit Tab-Navigation

**Designsystem**

- Designvariablen — Farb-, Zahl- und Text-Tokens mit `$variable`-Referenzen
- Multi-Theme-Unterstützung — mehrere Achsen, jeweils mit Varianten (Hell/Dunkel, Kompakt/Komfortabel)
- Komponentensystem — wiederverwendbare Komponenten mit Instanzen und Überschreibungen
- CSS-Synchronisierung — automatisch generierte benutzerdefinierte Eigenschaften, `var(--name)` in der Code-Ausgabe
- Wiederverwendbare UIKits — Import/Export von Komponenten-Kits aus `.pen`-Dateien

**KI & Agenten**

- Prompt-zu-Canvas mit Streaming-Generierung und orchestrator-gesteuerter räumlicher Zerlegung
- Parallele Agententeams — mehrere Designer arbeiten parallel an verschiedenen Abschnitten, mit Canvas-Indikatoren pro Mitglied
- Mehrstufiger Workflow — `design_skeleton` → `design_content` → `design_refine` mit fokussierten Prompts pro Phase
- Style Guides — 50+ eingebaute Stile (glassmorphism, brutalist, retro usw.) mit tag-basiertem Fuzzy-Matching, eingebunden in Planung und Generierung
- Multi-Modell-Fähigkeitsprofile — passt Denkmodus, Aufwand und Promptform automatisch an die Modellstufe an
- Integrierte Agent-Laufzeit (`agent-native`, Zig NAPI) + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini-Anbieter
- Anthropic-Format-Passthrough für chinesische LLM-Anbieter — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Git-Integration**

- Clone-Assistent mit SSH-/HTTPS-Authentifizierung und SSH-Schlüsselverwaltung
- Branch-Auswahl — Erstellen, Wechseln, Löschen, Mergen, alles im Git-Panel
- Pull-/Push-Kaskaden mit Auth-Wiederholung und Non-Fast-Forward-Handhabung
- Ordnermodus-Dreiwege-Merge mit Disk-basierter `MERGE_HEAD`-Statusverfolgung
- Konfliktpanel mit Dreiwege-Karten pro Knoten/Feld, Inline-JSON-Editor, Bulk-Aktionen und Inline-Diff-Block
- Remote-Einstellungen und SSH-Schlüssel-UI; 15-Sprachen-i18n für die gesamte Git-Oberfläche

**Export**

- Canvas-Export — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Code-Export — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Inkrementelle MCP-Codegen-Pipeline — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Figma-Import**

- `.fig`-Dateien importieren mit erhaltenem Layout, Füllungen, Konturen, Effekten, Text, Bildern und Vektoren

**Desktop-App**

- Natives macOS, Windows und Linux über Electron
- `.op`-Dateizuordnung — Doppelklick zum Öffnen, Einzelinstanzsperre
- Automatische Aktualisierung über GitHub Releases
- Natives Anwendungsmenü mit „Speichern unter“, „Zuletzt verwendete öffnen“ und einem Dialog zu ungespeicherten Änderungen beim Schließen
- Persistenz der zuletzt verwendeten Dateien

## Technologie-Stack

|                 |                                                                                  |
| --------------- | -------------------------------------------------------------------------------- |
| **Frontend**    | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui · i18next                |
| **Canvas**      | CanvasKit/Skia (WASM, GPU-beschleunigt)                                          |
| **State**       | Zustand v5                                                                       |
| **Server**      | Nitro                                                                            |
| **Desktop**     | Electron 35                                                                      |
| **CLI**         | `op` — Terminal-Steuerung, Batch-Design-DSL                                      |
| **KI**          | Vercel AI SDK v6 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Laufzeit**    | Bun · Vite 7                                                                     |
| **Dateiformat** | `.op` — JSON-basiert, menschenlesbar, Git-freundlich                             |

## Projektstruktur

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start Web-App
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia-Engine — Zeichnen, Sync, Layout
│   │   │   ├── components/  React-UI — Editor, Panels, gemeinsame Dialoge, Icons
│   │   │   ├── services/ai/ KI-Chat, Orchestrierer, Designgenerierung, Streaming
│   │   │   ├── stores/      Zustand — Canvas, Dokument, Seiten, Verlauf, KI
│   │   │   ├── mcp/         MCP-Server-Tools für externe CLI-Integration
│   │   │   ├── hooks/       Tastaturkürzel, Datei-Drop, Figma-Paste
│   │   │   └── uikit/       Wiederverwendbares Komponenten-Kit-System
│   │   └── server/
│   │       ├── api/ai/      Nitro-API — Streaming-Chat, Generierung, Validierung
│   │       └── utils/       Claude CLI, OpenCode, Codex, Copilot-Wrapper
│   ├── desktop/             Electron-Desktop-App
│   │   ├── main.ts          Fenster, Nitro-Fork, natives Menü, Auto-Updater
│   │   ├── ipc-handlers.ts  Native Dateidialoge, Theme-Sync, Einstellungen-IPC
│   │   └── preload.ts       IPC-Brücke
│   └── cli/                 CLI-Tool — `op`-Befehl
│       ├── src/commands/    Design-, Dokument-, Export-, Import-, Knoten-, Seiten-, Variablen-Befehle
│       ├── connection.ts    WebSocket-Verbindung zur laufenden App
│       └── launcher.ts      Automatische Erkennung und Start der Desktop-App oder des Webservers
├── packages/
│   ├── pen-types/           Typdefinitionen für das PenDocument-Modell
│   ├── pen-core/            Dokumentbaum-Operationen, Layout-Engine, Variablen
│   ├── pen-codegen/         Codegeneratoren (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Figma-.fig-Datei-Parser und -Konverter
│   ├── pen-renderer/        Eigenständiger CanvasKit/Skia-Renderer
│   ├── pen-sdk/             Umbrella-SDK (re-exportiert alle Pakete)
│   ├── pen-ai-skills/       KI-Prompt-Skill-Engine (phasengesteuertes Prompt-Laden)
│   └── agent/               KI-Agenten-SDK (Vercel AI SDK, Multi-Anbieter, Agententeams)
└── .githooks/               Pre-Commit-Versionssynchronisierung vom Branch-Namen
```

## Tastaturkürzel

| Taste       | Aktion                 |     | Taste         | Aktion                                     |
| ----------- | ---------------------- | --- | ------------- | ------------------------------------------ |
| `V`         | Auswählen              |     | `Cmd+S`       | Speichern                                  |
| `R`         | Rechteck               |     | `Cmd+Z`       | Rückgängig                                 |
| `O`         | Ellipse                |     | `Cmd+Shift+Z` | Wiederholen                                |
| `L`         | Linie                  |     | `Cmd+C/X/V/D` | Kopieren/Ausschneiden/Einfügen/Duplizieren |
| `T`         | Text                   |     | `Cmd+G`       | Gruppieren                                 |
| `F`         | Frame                  |     | `Cmd+Shift+G` | Gruppierung aufheben                       |
| `P`         | Stiftwerkzeug          |     | `Cmd+Shift+P` | Export (PNG/JPG/WEBP/PDF)                  |
| `H`         | Hand (Pan)             |     | `Cmd+Shift+C` | Code-Panel                                 |
| `Del`       | Löschen                |     | `Cmd+Shift+V` | Variablen-Panel                            |
| `[ / ]`     | Reihenfolge ändern     |     | `Cmd+J`       | KI-Chat                                    |
| Pfeiltasten | 1px verschieben        |     | `Cmd+,`       | Agenteneinstellungen                       |
| `Cmd+Alt+U` | Boolesche Vereinigung  |     | `Cmd+Alt+S`   | Boolesche Subtraktion                      |
| `Cmd+Alt+I` | Boolesche Schnittmenge |     | `Cmd+Shift+S` | Speichern unter                            |

## Skripte

```bash
bun --bun run dev          # Entwicklungsserver (Port 3000)
bun --bun run build        # Produktions-Build
bun --bun run test         # Tests ausführen (Vitest)
npx tsc --noEmit           # Typprüfung
bun run bump <version>     # Version über alle package.json synchronisieren
bun run electron:dev       # Electron-Entwicklung
bun run electron:build     # Electron-Paketierung
bun run cli:dev            # CLI aus Quellcode ausführen
bun run cli:compile        # CLI nach dist kompilieren
```

## Mitwirken

Beiträge sind willkommen! Siehe [CLAUDE.md](./CLAUDE.md) für Architekturdetails und Code-Stil.

1. Forken und klonen
2. Versionssynchronisierung einrichten: `git config core.hooksPath .githooks`
3. Branch erstellen: `git checkout -b feat/my-feature`
4. Prüfungen ausführen: `npx tsc --noEmit && bun --bun run test`
5. Mit [Conventional Commits](https://www.conventionalcommits.org/) committen: `feat(canvas): add rotation snapping`
6. Pull Request gegen `main` öffnen

## Roadmap

- [x] Designvariablen & Tokens mit CSS-Synchronisierung
- [x] Komponentensystem (Instanzen & Überschreibungen)
- [x] KI-Designgenerierung mit Orchestrierer
- [x] MCP-Server-Integration mit mehrstufigem Design-Workflow
- [x] Mehrseitige Unterstützung
- [x] Figma-`.fig`-Import
- [x] Boolesche Operationen (Vereinigung, Subtraktion, Schnittmenge)
- [x] Multi-Modell-Fähigkeitsprofile
- [x] Monorepo-Umstrukturierung mit wiederverwendbaren Paketen
- [x] CLI-Tool (`op`) für Terminal-Steuerung
- [x] Integriertes KI-Agenten-SDK mit Multi-Anbieter-Unterstützung
- [x] i18n — 15 Sprachen
- [x] Git-Integration (Klonen, Branches, Push/Pull, Ordnermodus-Dreiwege-Merge)
- [x] Canvas-Rasterexport (PNG / JPEG / WEBP / PDF)
- [ ] Kollaboratives Bearbeiten
- [ ] Plugin-System

## Mitwirkende

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Sponsoren

OpenPencil ist kostenlos und Open Source. Die Entwicklung wird von Menschen finanziert, die es nützlich finden — danke, dass ihr die Leinwand offen haltet.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Danke an **[MrQyun](https://github.com/mrqyun)** — soll dein Name auch hier stehen? **[Sponsor werden →](https://github.com/sponsors/ZSeven-W)**

## Community

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Unserem Discord beitreten</strong>
</a>
— Fragen stellen, Designs teilen, Funktionen vorschlagen.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Lizenz

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
