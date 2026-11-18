<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>Das DeepSeek-Harness-Plugin für OpenPencil – echte <code>.op</code>-Dokumente direkt in einer Unterhaltung anzeigen, untersuchen und bearbeiten.</strong><br />
  <sub>Exakte Multi-Frame-Vorschauen &bull; Interaktive Leinwand &bull; Verwalteter Editor &bull; Agent-native Design-Tools</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · Aktuelle Plugin-Version: <code>0.1.0-rc.7</code> · Bis einschließlich DSH <code>0.1.1-rc.2</code> getestet</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md"><b>Deutsch</b></a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil – Multi-Frame-Vorschau und Seitenleisten-Editor" width="100%" />
</p>
<p align="center"><sub>Exakte Multi-Frame-<code>.op</code>-Vorschauen mit einer interaktiven Leinwand und dem Arbeitsbereich des verwalteten Editors</sub></p>

## Warum DSH OpenPencil

DSH OpenPencil verbindet [DeepSeek Harness](https://github.com/deepseek-ai/DSH) mit [OpenPencil](https://github.com/ZSeven-W/openpencil), sodass ein Agent eine echte, bearbeitbare, interaktive Design-Leinwand steuert, statt ein generiertes Bild zurückzugeben.

<table>
<tr>
<td width="50%">

### 🖼️ Exakte Multi-Frame-Vorschauen

Der installierte Headless-Exporter von OpenPencil rendert designgetreue Vorschauen: den ersten Frame der obersten Ebene als großes, replaysicheres PNG sowie eine horizontal scrollbare Miniaturbildleiste mit Auswählen per Klick und Zurück-/Weiter-Navigation für Dokumente mit mehreren Frames.

</td>
<td width="50%">

### 🗺️ Interaktive Leinwand

„Interaktive Leinwand öffnen“ lädt die schreibgeschützte OpenPencil-Web-SDK bei Bedarf nach und bietet Schwenken, Zoomen und Einpassen – beliebige Seiten, verschachtelte Knoten oder inaktive Seiten lassen sich untersuchen, ohne die Unterhaltung zu verlassen.

</td>
</tr>
<tr>
<td width="50%">

### ✏️ Verwalteter Editor

Mit `editable: true` öffnet die Bearbeiten-Aktion den verwalteten OpenPencil-Editor – Auswahl, Ebenen, Eigenschaften, Zeichenwerkzeuge, Rückgängig/Wiederholen und explizite Speichersemantik – in einem in der Größe veränderbaren Arbeitsbereich auf der rechten Seite mit Vollbild-Option.

</td>
<td width="50%">

### 🤖 Agent-native Design-Tools

Fünf direkte Canvas-Tools plus sechs `openpencil_pipeline_*`-Tools ermöglichen es dem Agenten, über verwaltete OpenPencil-Laufzeiten eine echte Leinwand zu erstellen, zu prüfen, zu verfeinern, zu veröffentlichen, zu ändern und zu lesen.

</td>
</tr>
<tr>
<td width="50%">

### 🔐 Durch Capabilities geschützte Grants

Bild- und Dokument-Grants sind signierte, an Hashes gebundene Capabilities. Browser-Metadaten legen niemals einen beliebigen Host-Pfad offen, und signierte Vorschau-/Editor-Capabilities gelangen nie in das kanonische Tool-Ergebnis oder den Modellkontext.

</td>
<td width="50%">

### ⚡ Transaktionale Sicherheit

Ein Dokument der vollständigen Pipeline bleibt ein privater, unveröffentlichter Entwurf, bis alle nativen und DSH-Qualitäts-Gates bestanden sind. Die Veröffentlichung überschreibt keinen vorhandenen Pfad; Abbruch oder fehlerhafte Batches hinterlassen kein leeres Ziel.

</td>
</tr>
<tr>
<td width="50%">

### 🌍 Übernimmt das Look & Feel von DSH

Die Tool-Karte und der verwaltete Editor übernehmen das Chinesisch-/Englisch-Locale und das Hell-/Dunkel-Design von DSH, ohne die Bearbeitungssitzung neu zu laden.

</td>
<td width="50%">

### 🎯 Ein vollständiger Workflow

„Anforderung → privater Entwurf → semantische Batches → Live-Vorschauen für den Benutzer → deterministische Struktur-/Layout-/Qualitätsprüfung → atomare Veröffentlichung mit Integritätsnachweis des finalen PNGs“ – ein vollständiger Ablauf in DSH.

</td>
</tr>
</table>

## Installation in DSH

DSH ist ein eigenes Paket. Installiere es einmalig, falls noch nicht vorhanden:

```sh
npm install -g @deepseek-ai/dsh@latest
```

Füge dann das Plugin einem Profil hinzu und starte die Web-App:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

Für die lokale Entwicklung bauen Sie diesen Checkout, verlinken seinen absoluten Pfad mit dem Web-Profil und starten DSH anschließend vollständig neu:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

Die `link:`-Abhängigkeit macht spätere Neubauten direkt aus diesem Checkout sichtbar. Nach dem Ersetzen der Profilabhängigkeit muss DSH jedoch vollständig neu gestartet werden, da das mitgelieferte Web-Profil Host-Bundles standardmäßig nicht per Hot Reload aktualisiert.

Lieber ohne globale DSH-Installation? Führe dieselben zwei Schritte über `pnpm dlx` aus:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> Das OpenPencil-Plugin ist öffentlich und benötigt kein npm-Token. Wenn die DSH-Prerelease selbst eine Registry-Authentifizierung erfordert, bewahren Sie diese Anmeldedaten in einer npm-Konfiguration auf Benutzer- oder temporärer Ebene außerhalb des Checkouts auf. Dieses Repository enthält bewusst keine Registry-Anmeldedaten.

## Design-Tools

Die optionale Auswahl folgt `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` mit ursprünglichem Brief und `style_selection`. Katalog und exakte Auswahl benötigen keine Empfehlung; der Anbieter ist standardmäßig deaktiviert. Diese Übergabe unterstützt script, nicht app-agent oder Commerce. Normale app-agent-Generierung bleibt verfügbar.

| Tool | Funktion |
| --- | --- |
| `openpencil_new` | Kompatibler Schnellpfad für einfache Aufgaben: führt ein transaktionales QuickJS-`batch_design`-Skript aus, veröffentlicht nur bei noch nicht vorhandenem Ziel und gibt eine bearbeitbare Darstellung zurück. Für produktionsreife Designs ist die vollständige Pipeline vorzuziehen. |
| `openpencil_pipeline_begin` | Startet einen sitzungseigenen privaten Entwurf, legt intern dessen einzigen Root an, gibt den kompakten laufzeitgleichen Canvas-/Build-Vertrag zurück und öffnet denselben Live-Entwurf sofort in der Seitenleiste; die Ziel-`.op` bleibt unveröffentlicht. |
| `openpencil_pipeline_context` | Lädt nur ein gezieltes Detail zu Richtlinie, Stil, Theme oder UI-Kit, das im Begin-Vertrag tatsächlich fehlt; dies ist keine feste Startphase. |
| `openpencil_pipeline_batch` | Führt höchstens zwei direkte native `I(...)`-/`K(...)`-Generierungsskripte aus: zuerst den von `begin.next` begrenzten ersten sichtbaren Viewport, dann alle restlichen Bereiche in einem zweiten und letzten Batch. Jede erfolgreiche Transaktion versucht eine exakte PNG-Vorschau für den Benutzer. |
| `openpencil_pipeline_inspect` | Dient nur der manuellen Diagnose, wenn der Benutzer sie ausdrücklich verlangt; die normale Generierung verwendet es weder als Vorschau noch als visuelle Modellprüfung. |
| `openpencil_pipeline_finish` | Finalisiert und validiert den Entwurf in einem gesunden Aufruf, rendert automatisch die exakte finale Root-PNG-Vorschau und veröffentlicht atomar mit `createIfAbsent`. Genau eine reine `U(...)`-Reparatur und ein letzter Finish-Aufruf sind nur bei `needs_correction`, `canContinue: true`, einem vollständigen nicht leeren `repairTargets` und `omitted: 0` erlaubt; jedes andere nicht veröffentlichte Ergebnis ist terminal. |
| `openpencil_pipeline_abort` | Verwirft den unveröffentlichten Entwurf, ohne die Zieldatei anzulegen. |
| `openpencil_create` | Wendet ein transaktionales `batch_design`-Programm an, um Knoten auf einer vorhandenen Live-Leinwand zu erzeugen oder umzustrukturieren. |
| `openpencil_edit` | Modifiziert einen expliziten Knoten oder den einzelnen vom Benutzer ausgewählten Knoten. |
| `openpencil_render` | Erstellt einen unveränderlichen, inhaltsadressierten `.op`-Snapshot und rendert jeden Frame der obersten Ebene auf der aktiven Seite – optional mit `scale` und `editable`. |
| `openpencil_selection` | Liest die exakt auf der Leinwand des Live-Editors ausgewählten Knoten. |

## Agent-Design-Workflow

Rufen Sie für eine gewöhnliche kurze Anforderung sofort `openpencil_pipeline_begin` auf und folgen Sie dem festen Schnellpfad: der von `begin.next` begrenzte erste direkte JS-Batch → ein zweiter und letzter direkter JS-Batch mit allen verbleibenden Bereichen → ein `openpencil_pipeline_finish`. Begin legt den einzigen Root an und öffnet die private Live-Leinwand sofort in der Seitenleiste; der angeforderte Arbeitsbereichspfad bleibt bis zur atomaren Veröffentlichung abwesend. Nach jedem erfolgreichen Begin- oder Batch-Aufruf folgt der nächste erforderliche Aufruf sofort und ohne Erläuterung, Planung, Vergleich, Inspektion oder fremden Tool-Aufruf. Standardmäßig verwendet das gesamte Design genau ein Bild; jeder Hero-/Produkt-/Art-/Media-Frame hat genau ein primäres Visual, niemals zugleich ein Bild und einen Platzhalter-Icon.

Nach jedem erfolgreich eingespielten Batch versucht das Tool lediglich, dem Benutzer eine exakte PNG-Vorschau anzuzeigen. Sobald der Aufruf zurückkehrt, folgen Sie sofort seinem `next`; warten Sie nicht auf eine visuelle Selbstprüfung des Modells. Meldet `next` `previewUnavailable`, ist das Skript bereits auf der Live-Leinwand festgeschrieben: den Batch nicht erneut ausführen und weder `openpencil_pipeline_inspect` noch `read_image` aufrufen. `openpencil_pipeline_inspect` bleibt ausschließlich einer vom Benutzer ausdrücklich verlangten Diagnose vorbehalten.

Finish führt die deterministische native Finalisierung, Lint-, Kontrast- und Layoutprüfung sowie die harten DSH-Qualitäts-Gates aus. Ein gesunder Aufruf rendert das exakte finale Root-PNG und veröffentlicht im selben Schritt. Eine Reparatur ist nur erlaubt, wenn das Ergebnis zugleich `stage: "needs_correction"`, `canContinue: true`, ein vollständiges nicht leeres `repairTargets` mit `omitted: 0` und für jedes Ziel `operation: "U"`, eine exakte nicht leere `nodeId` sowie einen nicht leeren `patch` enthält. Alle Ziele werden zusammen in genau einem reinen `U(...)`-Batch angewendet; danach folgt ohne Erläuterung genau ein letzter Finish-Aufruf. Jeder andere nicht veröffentlichte Rückgabewert, ein Fehler oder `canContinue: false` ist terminal: einmal melden und nicht erneut versuchen, inspizieren, Bild/Kontext lesen, abbrechen oder einen Ersatzentwurf starten. Bei `published: true` sind finales PNG und Live-Editor bereits angehängt: Ergebnis zurückgeben und sofort beenden. Ein fehlgeschlagenes Gate oder `openpencil_pipeline_abort` lässt das Ziel abwesend. Die dokumentgebundene Bearbeitungsberechtigung öffnet die Seitenleiste nur im Leerlauf automatisch und behält **Leinwand bearbeiten** für einen ausdrücklichen Wechsel.

Innerhalb desselben laufenden DSH-Dienstes lässt sich nach einem Browserwechsel oder Neuladen eine strikt geparste dauerhafte Veröffentlichung von `openpencil_new` oder `openpencil_pipeline_finish` als exaktes PNG mit ausdrücklicher Aktion **Leinwand bearbeiten** wiederherstellen. Eine Verlaufskarte öffnet die Seitenleiste nie automatisch; der Benutzer muss diese Aktion anklicken. Ein gewöhnlicher historischer `openpencil_render` bleibt schreibgeschützt, und Nicht-Loopback-Verbindungen erhalten niemals einen Editor-Grant.

Der gebündelte `openpencil-design`-Skill bleibt eine optionale schlanke Referenz; normale Generierung beginnt direkt mit dem vollständigen kompakten Begin-Vertrag. Die aktuelle native Tool-Spezifikation ist maßgeblich, und die verwaltete Laufzeit benötigt keine Desktop-Binärdatei. `openpencil_new` bleibt ein kompatibler Einzel-Batch-Schnellpfad.

Verwenden Sie `openpencil_create` und `openpencil_edit` nur für eine vorhandene Live-Leinwand. Ihre Änderungen bleiben bis zur Aktion „Speichern“ im Editor ungespeichert.

## Web-Viewer-Assets

DSH liefert für ein Client-Plugin nur `client.js` aus; daher werden die OpenPencil-ESM-SDK, ihr WASM und CanvasKit als explizite Same-Origin-Assets bereitgestellt:

```sh
pnpm run sync:viewer-assets
```

Der Sync-Befehl verwendet standardmäßig das umschließende OpenPencil-Monorepo (`../..`) und fällt bei Bedarf auf ein benachbartes `../openpencil`-Checkout zurück. Überschreiben Sie es mit `OPENPENCIL_ROOT` oder `--openpencil-root`. Ein vollständiges vorgefertigtes Asset-Verzeichnis kann mit `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` ausgewählt werden. Die Suche zur Laufzeit kann mit `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR` überschrieben werden.

Viewer-Assets werden erst nachgeladen, nachdem der Benutzer die Leinwand geöffnet hat. Sind sie nicht vorhanden oder ungültig, bleibt die PNG-Vorschau verfügbar und es wird kein Leinwand-Button angezeigt.

## Verwalteter Editor

Bearbeitbare Sitzungen nutzen den verwalteten Web-Host von OpenPencil – dieselbe Architektur wie bei `op-vscode`. Das Plugin startet den Host erst nach einer autorisierten Benutzeraktion, hält das Daemon-Token im Arbeitsspeicher, validiert iframe-Quelle und -Origin und beendet den Prozess, wenn die Editor-Sitzung endet. Die Editor-Oberfläche wird schrittweise ausgewählt: native Tool-Details, wenn der Host diese Schnittstelle deklariert, andernfalls der rechtsseitige Arbeitsbereich des Plugins mit Größenänderung und Vollbild-Steuerung.

Der Start verwendet einen für langsame Mounts sicheren listening handshake: Bereitschaftsprüfungen beginnen erst, nachdem der mitgelieferte Host seine gebundene Adresse gemeldet hat. Eine Desktop-Installation von OpenPencil ist nicht erforderlich.

Veröffentlichte Installationen unterstützen sechs native Ziele: `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64` und `win32-x64`; die Linux-Pakete setzen glibc voraus. Das Root-Paket wählt über exakt versionierte `optionalDependencies` anhand von Betriebssystem und CPU das passende Plattformpaket aus (zum Beispiel `@zseven-w/dsh-openpencil-darwin-arm64`). Dieses Paket stellt `op-host-web-server`, das Web-Bundle des Editors und CanvasKit als eine zusammengehörige Laufzeit bereit. Der verwaltete Editor hängt daher weder von `/Applications/OpenPencil.app` noch von `openpencil-desktop` im `PATH` oder einem OpenPencil-Quell-Checkout ab.

Wenn DSH das Plugin neu lädt oder entlädt, während die Leinwand ungespeicherte Änderungen enthält, bewahrt der Host bis zu sieben Tage lang einen undurchsichtigen lokalen Wiederherstellungsentwurf auf. Beim erneuten Öffnen derselben Quelle wird nachgefragt, bevor sie in die Live-Leinwand wiederhergestellt wird; die Wiederherstellung überschreibt die `.op`-Datei nie, bis der Benutzer explizit speichert.

Die offiziellen Pakete für alle sechs Plattformen erhalten ihre Bootstrap-Endpunkte für die Zusammenarbeit in China und weltweit während des geschützten Release-Builds; die injizierten Endpunkte werden vor der Veröffentlichung validiert. Bei einem lokalen Eigenbau ohne diese Injektion kann der Bootstrap vor dem Start von DSH mit `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` überschrieben werden. Der Wert muss `https` verwenden und exakt den Pfad `/api/v1/collaboration/bootstrap` besitzen.

Für die geräteübergreifende Leinwandsynchronisierung müssen sowohl die native PC/DSH-Laufzeit als auch die mobile App auf dieselbe OpenPencil-Release-Linie aktualisiert werden, die die aktuelle Korrektur der Kollaborationswarteschlange enthält. Wird eine ältere mobile App mit einer neueren PC-Laufzeit kombiniert, können weiterhin entfernte Cursor sichtbar sein, obwohl keine Leinwand-Commits empfangen werden.

Bei der Entwicklung aus diesem Repository müssen vor dem Start von DSH zuerst das Web-Bundle des Editors und dann der native Host gebaut werden; anschließend wird die zusammengehörige Laufzeit bereitgestellt.

`pnpm run build:editor-web` führt das von OpenPencil unterstützte WASM-Bundle-Gate aus. Erforderlich sind Bash, Cargo/Rust mit dem Target `wasm32-unknown-unknown`, die `wasm-bindgen`-CLI, `wasm-opt` aus Binaryen, Node.js und `gzip`; CanvasKit benötigt kein EMSDK. Der Web-Build verwendet die Build-Variablen für den Collaboration-Bootstrap nicht. Vor `pnpm run build:editor-runtime` müssen sowohl `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` als auch `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL` gesetzt sein. Sie werden ausschließlich vom nativen Cargo-Build verwendet, der beim Fehlen einer Variablen nach dem Fail-closed-Prinzip abbricht. Nach beiden erfolgreichen Builds wird die Laufzeit mit dem letzten Befehl bereitgestellt.

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

Explizite Laufzeit-Überschreibungen werden nur als vollständiger, zusammengehöriger Satz akzeptiert:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` für `op-host-web-server`;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` für das gebaute Editor-Web-Bundle;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` für die CanvasKit-Assets.

Nur einen Teil dieses Satzes anzugeben ist ungültig; das Plugin kombiniert keine benutzerdefinierten Pfade mit den Laufzeit-Assets des Pakets.

Speichervorgänge nutzen einen optimistischen Quell-Hash, ein atomares Ersetzen und eine Nachfolger-Capability. Wenn sich die Quelle außerhalb des Editors ändert, meldet das Plugin einen Konflikt, statt sie zu überschreiben.

## Ergebnis-Metadaten

Das für das Modell sichtbare Ergebnis bleibt einfaches JSON. Das nur im Browser verfügbare `presentationMeta.$dshOpenPencil` enthält additive Grants für:

- `image`: PNG-Pfad, Vorschau-/Download-URLs und echte Breite/Höhe;
- `frames`: jeder exakt gerenderte Frame der obersten Ebene in der Reihenfolge der aktiven Seite, einschließlich Knoten-ID/-Name/-Index und signierter PNG-URLs;
- `document`: Quell-Aktionspfad plus unveränderliche Snapshot-URL, Bytes und SHA-256;
- `viewer`: versionierte SDK-/WASM-/CanvasKit-URLs, wenn die Asset-Route angehängt ist;
- `editor`: eingeschränkte Start-/Aktualisierungs-Capabilities, wenn `editable: true` autorisiert ist.

Das Ergebnis erfasst außerdem `renderer`, `rendererBinary`, `fidelity` und etwaige Warnungen. Vorhandene reine PNG-Schema-v1-Nachrichten bleiben renderbar.

DSH `0.1.1-rc.2` speichert Browser-Präsentationsmetadaten für Tools, die unter PTC/Code Mode verschachtelt sind, nicht dauerhaft. Das Plugin stellt diese reine UI-Projektion über einen Same-Origin-, sitzungsgebundenen Endpunkt wieder her: Der Browser sendet nur die Sitzungs-ID, die Call-ID und den unveränderlichen Dokument-SHA-256, während der Host das maßgebliche Ergebnis aus dem dauerhaften DSH-Sitzungsprotokoll auflöst und einen kurzlebigen In-Process-Marker ausschließlich zur Autorisierung aktueller Live-Bearbeitungen verwendet. Signierte Vorschau-/Editor-Capabilities gelangen nie in das kanonische Tool-Ergebnis oder den Modellkontext. Der dauerhafte Verlauf eines gewöhnlichen `openpencil_render` bleibt schreibgeschützt. Eine strikt geparste dauerhafte Veröffentlichung von `openpencil_new` oder `openpencil_pipeline_finish` kann nur über Loopback und nach ausdrücklichem Klick des Benutzers einen Editor-Grant erhalten; das automatische Öffnen der Seitenleiste bleibt aktuellen, vertrauenswürdigen Live-Ergebnissen vorbehalten.

Für eine begrenzte Wiedergabe akzeptiert die verschachtelte Metadaten-Wiederherstellung bis zu 128 Frames der obersten Ebene; größere Code-Mode-Ergebnisse bleiben über ihren kanonischen JSON-Fallback verfügbar.

## Aktuelle Einschränkungen

- Folgeänderungen an einer vorhandenen Leinwand setzen einen bereits geöffneten verwalteten Editor voraus. Änderungen bleiben ungespeichert, bis der Benutzer dessen Aktion „Speichern“ ausführt.
- Die leichtgewichtige Leinwand der Web-SDK ist schreibgeschützt; die vollständige Bearbeitung nutzt die separate Oberfläche des verwalteten Editors. Auf DSH `0.1.1-rc.2` verwendet das Plugin den in der Größe veränderbaren rechten Arbeitsbereich mit Vollbild-Option.
- Die exakte Galerie umfasst Frames der obersten Ebene auf der aktiven Seite; die interaktive Leinwand bleibt der Weg, um inaktive Seiten und verschachtelte Knoten zu untersuchen.
- Render- und Snapshot-Caches benötigen weiterhin eine produktweite Aufbewahrungsrichtlinie.

## Projektstruktur

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## Build und Verifizieren

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

Builds erfordern Node 24.11 oder neuer und pnpm. DSH-Host-/Client-Pakete sind Peer-Abhängigkeiten, die vom Ziel-DSH-Profil bereitgestellt werden. Build-Tools werden aus lokalen Dev-Abhängigkeiten, dem aktiven verlinkten DSH-Checkout oder einem installierten DSH-Quellbundle aufgelöst; mit `DSH_SOURCE_ROOT` kann ein Quell-Checkout explizit ausgewählt werden. Die Lockfile pinnt eigenständige öffentliche Build-Tools, wenn diese Umgebung separat bereitgestellt wird.

Für eine private DSH-Prerelease bewahren Sie die ausgegebene npm-Anmeldeinformation außerhalb dieses Repositorys auf (zum Beispiel in einer `.npmrc` auf Benutzer- oder temporärer Ebene) und führen Sie die angeforderte Version direkt aus:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

Committen Sie niemals `.npmrc`, `NPM_TOKEN` oder kopierte Registry-Anmeldedaten. Dieses Repository ignoriert die lokale npm-Konfiguration standardmäßig.

`test:host` führt einen echten exakten Render durch, validiert PNG-IHDR-Geometrie und SHA-256, testet unveränderliche Bild-/Dokument-Capabilities über HTTP und prüft, ob Viewer-Assets gewährbar sind. Die erwarteten Abmessungen sind fixture-spezifisch.

## Ökosystem

DSH OpenPencil ist das DeepSeek-Harness-Plugin für **[OpenPencil](https://github.com/ZSeven-W/openpencil)** – das weltweit erste quelloffene, AI-native Vektor-Design-Tool – und Teil der **[ZSeven-W](https://github.com/ZSeven-W)**-Familie reiner Rust- und AI-nativer Tools.

| Projekt | Was es ist |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | Das Design-Tool, das dieses Plugin steuert – Prompt-zu-Leinwand-Generierung, parallele Agent-Teams, Design-als-Code-`.op`-Dateien und ein integrierter MCP-Server. Die exakten Vorschauen, die interaktive Leinwand und der verwaltete Editor hier werden von OpenPencil selbst betrieben. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Eine Async-Runtime aus reinem Rust für die Auslieferung von LLM-Agenten – Multi-Provider, durchgängig tool-fähig, strukturierte Berechtigungen, echtes MCP, null `unsafe`. Treibt die integrierte Agent-Runtime von OpenPencil an. |
| **[jian](https://github.com/ZSeven-W/jian)** | Ein UI-Framework aus reinem Rust mit GPU-Skia – Widgets, Layout, Ereignisse und Hot Reload in einem Stack. OpenPencils UI-Framework und die Quelle des Fallback-Renderers dieses Plugins. |
| **[Zode](https://github.com/ZSeven-W/zode)** | Ein quelloffener, AI-nativer Coding-Assistent für Ihr Terminal – liest Ihren Code, führt Befehle aus und steuert OpenPencil über MCP. |
| **[noema](https://github.com/ZSeven-W/noema)** | Ein Local-first-, nicht-vektorbasiertes Speichersystem für Coding-Agenten – dauerhafter Speicher als einsehbare Dateien, funktioniert über Laufzeiten hinweg. |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | Das LLM-Skill-Plugin, das KI-Agenten beibringt, wie sie mit `op` designen – ein Begleiter zu diesem DSH-Plugin. |

Weitere DSH-Plugins:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — ein Live-Android-Emulator oder USB-Gerät in der Konversation, vollständig über adb gesteuert
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — Arbeit aus Claude Code / Codex an DSH-Agenten delegieren
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — ein lebender iOS-Simulator — und ein iPhone per USB — in der Konversation
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — Langzeitgedächtnis für DSH

## Mitwirken

Beiträge sind willkommen! Forken und klonen Sie das Repository, erstellen Sie einen Branch, führen Sie `pnpm run build` und die Test-Suites aus, committen Sie mit [Conventional Commits](https://www.conventionalcommits.org/) und eröffnen Sie einen PR gegen `main`.

## Community

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Treten Sie unserem Discord bei</strong>
</a>
– Stellen Sie Fragen, teilen Sie Designs, schlagen Sie Funktionen vor.

**Anerkannte Community: [LINUX DO](https://linux.do/)**

## Lizenz

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W

Komponenten von Drittanbietern sind in [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md) aufgeführt.
