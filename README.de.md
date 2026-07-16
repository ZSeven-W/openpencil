<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
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

Web-App + native Desktop-Anwendung auf macOS, Windows und Linux — ein Rust-Kern, eine einzelne eigenständige Binärdatei, keine Browser-Engine. `.op`-Dateizuordnung — Doppelklick zum Öffnen.

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
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Oder als Desktop-App ausführen:

```bash
cargo run -p op-host-desktop
```

> **Voraussetzungen:** [Rust](https://www.rust-lang.org/) (stable) zum Bauen des Produkts. [Bun](https://bun.sh/) >= 1.0 und [Node.js](https://nodejs.org/) >= 18 werden nur für das Web-SDK unter `packages/` benötigt.

### Docker

Getaggte Rust-Releases veröffentlichen ein einzelnes Web-Host-Image. Die früheren TypeScript-Images mit gebündelten KI-CLIs werden nicht mehr veröffentlicht.

| Image | Enthält |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust-Web-Host, wasm bundle und CanvasKit-Assets |

Die Web-UI zeigt nur integrierte Agent-Profile; Claude/Codex/OpenCode/Copilot/Gemini-CLI-Tools sind nicht in Docker-Images enthalten.

**Ausführen:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

Öffnen Sie dann `http://localhost:3100/`.

**Lokal bauen:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
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

- Eingebauter MCP-Server (`op-mcp`-Crate) — Ein-Klick-Installation in Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Kein Node.js erforderlich — stdio-Transport über die Desktop-Binärdatei (`--mcp <path>`), plus ein Live-HTTP-Endpunkt (`127.0.0.1:<port>/mcp`) der laufenden App
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # Desktop-App starten
op start --headless --file design.op # Headless-Server starten
op design @landing.txt       # Batch-Design aus Datei
op design @ui.js             # Sandbox-JavaScript mit Schleifen
op insert '{"type":"rectangle"}' # Knoten einfügen
op import:figma design.fig   # Figma-Datei importieren
cat design.dsl | op design - # Pipe von stdin
```

Unterstützt Inline-Strings, `@filepath` und stdin (`-`). Funktioniert mit Desktop-App, Webserver oder dateibasiertem Headless-Server. Alle Befehle stehen in der [CLI-Befehlsreferenz](./crates/op-cli/src/usage.txt).

**LLM-Skill** — Installieren Sie das [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill)-Plugin, um KI-Agenten das Designen mit `op` beizubringen. Verwenden Sie `op install` für erkannte Agenten oder `op install --target codex` für ein bestimmtes Ziel.

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
- Integrierte Agent-Laufzeit (Rust) + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini-Anbieter
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

- Natives macOS, Windows und Linux — eine einzelne eigenständige Binärdatei (winit + GPU Skia, kein Electron)
- `.op`-Dateizuordnung — Doppelklick zum Öffnen, Einzelinstanzsperre
- Update-Prüfung im Hintergrund über GitHub Releases
- Natives Anwendungsmenü mit „Speichern unter“, „Zuletzt verwendete öffnen“ und einem Dialog zu ungespeicherten Änderungen beim Schließen
- Persistenz der zuletzt verwendeten Dateien

## Technologie-Stack

|                  |                                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------- |
| **Kern**         | Rust-Workspace (`crates/`) — Editor-Zustand, Widgets, Hosts, MCP, KI, Codegen                |
| **Rendering**    | GPU Skia überall — `skia-safe` (GL) nativ, CanvasKit (WASM/WebGL2) im Browser                |
| **UI-Framework** | jian — vendorisiertes reines Rust-GPU-Skia-UI-Framework: Widgets, Layout, Events, Hot Reload (`vendor/jian`) |
| **Fenstersystem**| winit (vendorisierter `casement`-Fork)                                                       |
| **Desktop**      | Native Binärdatei `openpencil-desktop` — keine Browser-Engine                                |
| **Web-SDK**      | `op-web-sdk` + React-19-/Vue-3-Adapter — schreibgeschützter `.op`-Viewer (TypeScript)         |
| **CLI**          | `op` — Terminal-Steuerung, Batch-Design-DSL                                                  |
| **KI**           | Integrierte Rust-Agentenlaufzeit · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**         | clippy · rustfmt (Rust) · oxlint · oxfmt (Web-SDK)                                            |
| **Dateiformat**  | `.op` — JSON-basiert, menschenlesbar, Git-freundlich                                         |

## Ökosystem

OpenPencil ist Teil einer Familie von reinen Rust-, KI-nativen Werkzeugen von **[ZSeven-W](https://github.com/ZSeven-W)**. Sie greifen ineinander: `jian` rendert OpenPencil, `agent-rs` betreibt seine Agenten, `noema` erinnert sich, und `zode` designt vom Terminal aus.

| Projekt | Was es ist |
| ------- | ---------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Open-Source-, KI-nativer Coding-Assistent für Ihr Terminal — eine schnelle Rust-TUI (`ratatui`), die Ihren Code liest, Befehle ausführt, Dateien durchsucht und Git verwaltet. Steuert OpenPencil über MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Eine reine Rust-Async-Laufzeit zum Ausliefern von LLM-Agenten — Multi-Anbieter, durchgängig werkzeugfähig, strukturierte Berechtigungen, echtes MCP, null `unsafe`. Treibt OpenPencils integrierte Agentenlaufzeit (`vendor/agent`) und Zode an. |
| **[jian](https://github.com/ZSeven-W/jian)** | Reines Rust-, GPU-Skia-UI-Framework — Widgets, Layout, Events und Hot Reload in einem Stack. Verwandelt ein deklaratives `.op`-Dokument in eine native, KI-steuerbare App ohne JS-Laufzeit, ohne DOM, ohne Electron. OpenPencils UI-Framework (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Local-first-, nicht-vektorbasiertes Speichersystem für Coding-Agenten. Dauerhafter Speicher als inspizierbare Dateien, eine Prüfungswarteschlange für neue Einträge und lexikalischer (einbettungsfreier) Abruf — funktioniert über Zode, Codex, Claude Code und MCP-Laufzeiten hinweg. |

## Warum Rust

OpenPencil wurde von Grund auf in **Rust** neu geschrieben ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). Die Neufassung ist abgeschlossen — der TypeScript + Electron-Editor wurde mit `v0.7.5` eingestellt, und der Rust-Workspace in diesem Repository ist das Produkt: ein einzelner nativer Kern, der dramatisch kleiner und schneller ist und aus einer einzigen Codebasis auf mehr Plattformen läuft.

|                            | TypeScript + Electron (eingestellt, `v0.7.5`)            | Rust (heute)                                                                  |
| -------------------------- | ------------------------------------------------------- | ----------------------------------------------------------------------------- |
| **Desktop-Laufzeit**       | Electron — bündelt Chromium + Node.js                   | Natives Fenster (`winit` + GPU Skia), keine Browser-Engine                    |
| **Desktop-Größe**          | Vollständige Chromium-Laufzeit pro Installation         | Einzelne eigenständige Binärdatei — **55.5 MB**                               |
| **Web-Nutzlast**           | JS + WASM-Bundle                                        | **8.2 MB** wasm / **2.18 MB** gzip über die Leitung                           |
| **Rendering**              | CanvasKit/Skia im Web                                   | Ein GPU-beschleunigtes Skia-Backend auf **jedem** Ziel                        |
| **Speicher**               | JavaScript GC-Pausen                                    | Kein GC — Rust-Ownership, vorhersagbare Latenz                                |
| **Codebasis**              | Web-Stack + Electron                   | Ein Rust-Workspace: Editor · CLI · MCP · AI · Codegen · Figma · Git           |
| **Zielplattformen**        | Web + Desktop, zwei separate Stacks                     | Desktop (macOS/Win/Linux) · Mobile (iOS/Android) · Browser — ein Kern         |

**Gemessene Verbesserungen**

- **Geringer Speicherbedarf** — die gesamte Desktop-App ist eine einzelne native Binärdatei von **55.5 MB** anstelle einer gebündelten Browser-Engine plus einer Node-Laufzeit. Der Web-Build ist **8.2 MB** roh / **2.18 MB** gzip nach dem Aufteilen des Icon-Katalogs (−48% über die Leitung).
- **Skaliert auf große Dokumente** — eine Live-Canvas mit **10,000-node** (verschachteltes Auto-Layout, vier Ebenen tief) schreibt, liest und erstellt Layout-Snapshots **ohne Panics und mit ~0% CPU-Leerlast**; ein vollständiger Layout-Snapshot aller 10k Knoten wird in **~0.68 s** zurückgegeben.
- **Schnelle Interaktion** — Pan/Zoom serialisiert das Dokument nicht mehr bei jedem Frame neu (ein einziger Hot-Path-Fix senkte den CPU-Verbrauch beim Rollen von **~69% auf ~0%**); Drags aktualisieren die Szene inkrementell, die Textmessung wird gecacht, und Neuzeichnungen werden auf einen pro Frame zusammengeführt.
- **Ein Kern, jeder Bildschirm** — derselbe Editor-Zustand und dasselbe Render-Backend werden zu nativem Desktop, Mobile und dem Browser via WASM kompiliert — keine parallelen Neuimplementierungen, die synchron gehalten werden müssen.
- **GPU Skia überall** — native Ausgabe über `skia-safe` in einem GL-Kontext; der Browser rendert über CanvasKit auf WebGL2 — derselbe Zeichencode, dieselbe Ausgabe.
- **Native Barrierefreiheit** — AccessKit auf macOS, Windows und Linux sowie ein DOM-Spiegel im Web, anstatt sich auf den a11y-Baum eines Browsers zu stützen.
- **Ein typgeprüfter Workspace** — der MCP-Host, CLI, AI-Anbieter, Codegenerierung, Figma-Import und Git-Integration leben alle in einem einzigen Rust-Workspace, mit `cargo-deny`-Supply-Chain-Kontrolle in der CI.

> **Status:** der TypeScript-Editor wurde mit `v0.7.5` eingestellt und existiert nur noch in der Git-Historie; dieses Repository ist der Rust-Workspace. Das Rust-Produkt wird aktiv weiterentwickelt (siehe Roadmap unten).

## Projektstruktur

```text
openpencil/
├── crates/                   Rust-Workspace — das Produkt
│   ├── op-editor-core/       Kanonischer `.op`-Zustand (PenDocument) + EditorCommand + Designvariablen
│   ├── op-editor-ui/         Plattformfreie Widgets + RenderBackend-Fassade (wasm32-clean)
│   ├── op-editor-host-core/  Transportfreie Host-Zustandsmaschinen, gemeinsam für alle Hosts
│   ├── op-host-native/       Native Host-Bibliothek — winit + skia-safe GL (Desktop + Mobile)
│   ├── op-host-web/          Browser-Bundle — wasm32-cdylib, CanvasKit-Renderer
│   ├── op-host-desktop/      Desktop-Binärdatei `openpencil-desktop`; auch der `--serve-web`-Daemon
│   ├── op-host-services/     Headless serve-web-/MCP-Daemon-Bibliothek
│   ├── op-host-web-server/   Schlanke GL-freie Webserver-Binärdatei
│   ├── op-cli/               CLI-Tool — `op`-Befehl
│   ├── op-mcp/               MCP-Server — Tools, Batch-Design, mehrstufiger Workflow
│   ├── op-ai/                KI-Anbieter, Chat-Laufzeit, Streaming
│   ├── op-ai-skills/         KI-Prompt-Skill-Engine (phasengesteuertes Prompt-Laden)
│   ├── op-orchestrator/      Orchestrierung paralleler Agententeams
│   ├── op-codegen/           Codegeneratoren (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma-.fig-Datei-Parser und -Konverter
│   ├── op-git/               Git-Integration — Klonen, Branches, Push/Pull, Merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web-SDK-Workspace (Bun)
│   ├── op-web-sdk/           Schreibgeschützte `.op`-Web-Viewer-SDK (umschließt das wasm-Bundle)
│   ├── op-web-sdk-react/     React-19-Adapter
│   └── op-web-sdk-vue/       Vue-3-Adapter
├── vendor/                   Vendorisierte Subsysteme (Git-Submodule)
│   ├── jian/                 GPU-Skia UI-Framework — Widgets/Render/Events
│   ├── casement/             winit-Fork
│   └── agent/                Produktübergreifende Rust-Agentenlaufzeit (agent-rs)
└── .githooks/                Pre-Commit-Prüfung auf Versionsabweichungen
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
# Product (Rust — run from the repo root)
cargo build --workspace              # Build all crates (add --release for prod)
cargo test --workspace               # Run all tests
cargo check --workspace              # Type check
cargo clippy --workspace --all-targets -- -D warnings   # Lint
cargo fmt --all                      # Format
bash scripts/start-web-rust.sh       # Web dev server (wasm bundle + headless host)
cargo run -p op-host-desktop         # Desktop app (binary: openpencil-desktop)
cargo run -p op-cli -- <args>        # CLI (binary: op)

# Web SDK / JS tooling (run from packages/)
cd packages && bun run lint          # Lint the web SDK (oxlint); also: bun run format
cd packages && bun run generate-iconify-catalog   # Regenerate the Rust icon catalog assets

# Versionssynchronisierung (im Repository-Stammverzeichnis ausführen)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## Mitwirken

Beiträge sind willkommen! Siehe [CLAUDE.md](./CLAUDE.md) für Architekturdetails und Code-Stil.

1. Forken und klonen
2. Versionsabweichungsprüfung aktivieren: `git config core.hooksPath .githooks`
3. Branch erstellen: `git checkout -b feat/my-feature`
4. Prüfungen ausführen: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Cargo-Workspace mit wiederverwendbaren Rust-Crates und Web-SDK-Paketen
- [x] Rust-Editor für Desktop und Web
- [x] CLI-Tool (`op`) für Terminal-Steuerung
- [x] Integrierte Rust-Agenten-Laufzeit mit Multi-Anbieter-Unterstützung
- [x] i18n — 15 Sprachen
- [x] Wasm-gestützte Viewer-SDKs für JavaScript, React und Vue
- [x] Style Guides mit tag-basierter Zuordnung und MCP-Tools
- [x] Gleichzeitige Agent Teams mit Delegation und Canvas-Indikatoren
- [x] Git-Integration (Klonen, Branches, Push/Pull, Ordnermodus-Dreiwege-Merge)
- [x] Canvas-Export (SVG / PNG / JPEG / WEBP / PDF)
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
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Unserem Discord beitreten</strong>
</a>
— Fragen stellen, Designs teilen, Funktionen vorschlagen.

**Anerkannte Community: [LINUX DO](https://linux.do/)**

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Bewertungen

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Lizenz

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
