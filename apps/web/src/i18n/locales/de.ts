import type { TranslationKeys } from './en'

const de: TranslationKeys = {
  // ── Common ──
  'common.rename': 'Umbenennen',
  'common.duplicate': 'Duplizieren',
  'common.delete': 'Löschen',
  'common.cancel': 'Abbrechen',
  'common.save': 'Speichern',
  'common.close': 'Schließen',
  'common.connect': 'Verbinden',
  'common.disconnect': 'Trennen',
  'common.import': 'Importieren',
  'common.export': 'Exportieren',
  'common.name': 'Name',
  'common.untitled': 'Unbenannt',
  'common.best': 'Beste',
  'common.selected': '{{count}} ausgewählt',

  // ── Toolbar ──
  'toolbar.select': 'Auswählen',
  'toolbar.text': 'Text',
  'toolbar.frame': 'Rahmen',
  'toolbar.hand': 'Hand',
  'toolbar.undo': 'Rückgängig',
  'toolbar.redo': 'Wiederherstellen',
  'toolbar.variables': 'Variablen',
  'toolbar.uikitBrowser': 'UIKit-Browser',

  // ── Shapes ──
  'shapes.rectangle': 'Rechteck',
  'shapes.ellipse': 'Ellipse',
  'shapes.polygon': 'Polygon',
  'shapes.line': 'Linie',
  'shapes.icon': 'Symbol',
  'shapes.importImageSvg': 'Bild oder SVG importieren\u2026',
  'shapes.pen': 'Stift',
  'shapes.shapeTools': 'Formwerkzeuge',
  'shapes.moreShapeTools': 'Weitere Formwerkzeuge',

  // ── Top Bar ──
  'topbar.hideLayers': 'Ebenen ausblenden',
  'topbar.showLayers': 'Ebenen einblenden',
  'topbar.new': 'Neu',
  'topbar.open': 'Öffnen',
  'topbar.save': 'Speichern',
  'topbar.importFigma': 'Figma importieren',
  'topbar.codePanel': 'Code',
  'topbar.lightMode': 'Heller Modus',
  'topbar.darkMode': 'Dunkler Modus',
  'topbar.fullscreen': 'Vollbild',
  'topbar.exitFullscreen': 'Vollbild beenden',
  'topbar.edited': '— Bearbeitet',
  'topbar.closeConfirmMessage': 'Möchten Sie die Änderungen vor dem Schließen speichern?',
  'topbar.closeConfirmDetail': 'Ihre Änderungen gehen verloren, wenn Sie sie nicht speichern.',
  'topbar.dontSave': 'Nicht speichern',
  'topbar.agentsAndMcp': 'Agenten & MCP',
  'topbar.setupAgentsMcp': 'Agenten & MCP einrichten',
  'topbar.connected': 'verbunden',
  'topbar.agentStatus': '{{agents}} Agent{{agentSuffix}} · {{mcp}} MCP',

  // ── Right Panel ──
  'rightPanel.design': 'Design',
  'rightPanel.code': 'Code',
  'rightPanel.noSelection': 'Element auswählen',

  // ── Pages ──
  'pages.title': 'Seiten',
  'pages.addPage': 'Seite hinzufügen',
  'pages.moveUp': 'Nach oben',
  'pages.moveDown': 'Nach unten',

  // ── Status Bar ──
  'statusbar.zoomOut': 'Herauszoomen',
  'statusbar.zoomIn': 'Hineinzoomen',
  'statusbar.resetZoom': 'Zoom zurücksetzen',

  // ── Updater ──
  'updater.softwareUpdate': 'Software-Update',
  'updater.dismiss': 'Schließen',
  'updater.current': 'Aktuell',
  'updater.latest': 'Neueste',
  'updater.unknown': 'Unbekannt',
  'updater.checking': 'Prüfe...',
  'updater.downloadProgress': 'Downloadfortschritt',
  'updater.checkAgain': 'Erneut prüfen',
  'updater.restartInstall': 'Neustart & Installieren',
  'updater.installing': 'Installiere...',
  'updater.releaseDate': 'Veröffentlichungsdatum: {{date}}',
  'updater.restartHint':
    'Neustart zum Anwenden des Updates. Der Neustart dauert in der Regel 10–15 Sekunden.',
  'updater.unknownError': 'Unbekannter Updater-Fehler.',
  'updater.title.checking': 'Suche nach Updates',
  'updater.title.available': 'Update gefunden',
  'updater.title.downloading': 'Update wird heruntergeladen',
  'updater.title.downloaded': 'Bereit zur Installation',
  'updater.title.error': 'Update fehlgeschlagen',
  'updater.subtitle.checking': 'Suche nach der neuesten Version...',
  'updater.subtitle.available': 'Version {{version}} ist verfügbar.',
  'updater.subtitle.availableGeneric': 'Eine neue Version ist verfügbar.',
  'updater.subtitle.downloading':
    'Version {{version}} wird im Hintergrund heruntergeladen.',
  'updater.subtitle.downloadingGeneric':
    'Update-Paket wird im Hintergrund heruntergeladen.',
  'updater.subtitle.downloaded': 'Version {{version}} wurde heruntergeladen.',
  'updater.subtitle.downloadedGeneric': 'Das Update wurde heruntergeladen.',
  'updater.subtitle.error':
    'Das Update konnte nicht geprüft oder heruntergeladen werden.',

  // ── Layers ──
  'layers.title': 'Ebenen',
  'layers.empty':
    'Noch keine Ebenen. Verwende die Werkzeugleiste, um Formen zu zeichnen.',

  // ── Layer Context Menu ──
  'layerMenu.groupSelection': 'Auswahl gruppieren',
  'layerMenu.createComponent': 'Komponente erstellen',
  'layerMenu.detachComponent': 'Komponente lösen',
  'layerMenu.detachInstance': 'Instanz lösen',
  'layerMenu.booleanUnion': 'Vereinigung',
  'layerMenu.booleanSubtract': 'Subtraktion',
  'layerMenu.booleanIntersect': 'Schnittmenge',
  'layerMenu.toggleLock': 'Sperren umschalten',
  'layerMenu.toggleVisibility': 'Sichtbarkeit umschalten',

  // ── Property Panel ──
  'property.createComponent': 'Komponente erstellen',
  'property.detachComponent': 'Komponente lösen',
  'property.goToComponent': 'Zur Komponente gehen',
  'property.detachInstance': 'Instanz lösen',

  // ── Fill ──
  'fill.title': 'Füllung',
  'fill.solid': 'Vollton',
  'fill.linear': 'Linear',
  'fill.radial': 'Radial',
  'fill.image': 'Bild',
  'fill.stops': 'Stops',
  'fill.angle': 'Winkel',

  // ── Image ──
  'image.title': 'Bild',
  'image.fit': 'Anpassungsmodus',
  'image.fill': 'Füllen',
  'image.fitMode': 'Anpassen',
  'image.crop': 'Zuschneiden',
  'image.tile': 'Kacheln',
  'image.clickToUpload': 'Zum Hochladen klicken',
  'image.changeImage': 'Bild ändern',
  'image.adjustments': 'Anpassungen',
  'image.exposure': 'Belichtung',
  'image.contrast': 'Kontrast',
  'image.saturation': 'Sättigung',
  'image.temperature': 'Temperatur',
  'image.tint': 'Farbton',
  'image.highlights': 'Lichter',
  'image.shadows': 'Schatten',
  'image.reset': 'Zurücksetzen',

  // ── Stroke ──
  'stroke.title': 'Kontur',

  // ── Appearance ──
  'appearance.layer': 'Ebene',
  'appearance.opacity': 'Deckkraft',

  // ── Layout ──
  'layout.flexLayout': 'Flex-Layout',
  'layout.freedom': 'Frei (kein Layout)',
  'layout.vertical': 'Vertikales Layout',
  'layout.horizontal': 'Horizontales Layout',
  'layout.alignment': 'Ausrichtung',
  'layout.gap': 'Abstand',
  'layout.spaceBetween': 'Zwischenraum',
  'layout.spaceAround': 'Umgebungsraum',
  'layout.dimensions': 'Abmessungen',
  'layout.fillWidth': 'Breite füllen',
  'layout.fillHeight': 'Höhe füllen',
  'layout.hugWidth': 'Breite anpassen',
  'layout.hugHeight': 'Höhe anpassen',
  'layout.clipContent': 'Inhalt beschneiden',

  // ── Padding ──
  'padding.title': 'Innenabstand',
  'padding.paddingMode': 'Innenabstandmodus',
  'padding.paddingValues': 'Innenabstandwerte',
  'padding.oneValue': 'Ein Wert für alle Seiten',
  'padding.horizontalVertical': 'Horizontal/Vertikal',
  'padding.topRightBottomLeft': 'Oben/Rechts/Unten/Links',

  // ── Typography ──
  'text.typography': 'Typografie',
  'text.lineHeight': 'Zeilenhöhe',
  'text.letterSpacing': 'Zeichenabstand',
  'text.horizontal': 'Horizontal',
  'text.vertical': 'Vertikal',
  'text.alignLeft': 'Linksbündig',
  'text.alignCenter': 'Zentriert',
  'text.alignRight': 'Rechtsbündig',
  'text.justify': 'Blocksatz',
  'text.top': 'Oben',
  'text.middle': 'Mitte',
  'text.bottom': 'Unten',
  'text.weight.thin': 'Dünn',
  'text.weight.light': 'Leicht',
  'text.weight.regular': 'Normal',
  'text.weight.medium': 'Mittel',
  'text.weight.semibold': 'Halbfett',
  'text.weight.bold': 'Fett',
  'text.weight.black': 'Schwarz',
  'text.font.search': 'Schriften suchen\u2026',
  'text.font.bundled': 'Mitgeliefert',
  'text.font.system': 'System',
  'text.font.loading': 'Schriften werden geladen\u2026',
  'text.font.noResults': 'Keine Schriften gefunden',

  // ── Text Layout ──
  'textLayout.title': 'Layout',
  'textLayout.dimensions': 'Abmessungen',
  'textLayout.resizing': 'Größenanpassung',
  'textLayout.autoWidth': 'Auto B',
  'textLayout.autoWidthDesc': 'Auto Breite — Text dehnt sich horizontal aus',
  'textLayout.autoHeight': 'Auto H',
  'textLayout.autoHeightDesc':
    'Auto Höhe — feste Breite, Höhe passt sich an',
  'textLayout.fixed': 'Fest',
  'textLayout.fixedDesc':
    'Feste Größe — Breite und Höhe sind festgelegt',
  'textLayout.fillWidth': 'Breite füllen',
  'textLayout.fillHeight': 'Höhe füllen',

  // ── Effects ──
  'effects.title': 'Effekte',
  'effects.dropShadow': 'Schlagschatten',
  'effects.blur': 'Unschärfe',
  'effects.spread': 'Ausdehnung',
  'effects.color': 'Farbe',

  // ── Export ──
  'export.title': 'Exportieren',
  'export.format': 'Format',
  'export.scale': 'Skalierung',
  'export.selectedOnly': 'Nur Auswahl exportieren',
  'export.exportFormat': '{{format}} exportieren',
  'export.exportLayer': 'Ebene exportieren',

  // ── Polygon ──
  'polygon.sides': 'Seiten',

  // ── Ellipse ──
  'ellipse.start': 'Start',
  'ellipse.sweep': 'Bogen',
  'ellipse.innerRadius': 'Innen',

  // ── Corner Radius ──
  'cornerRadius.title': 'Eckenradius',

  // ── Size / Position ──
  'size.position': 'Position',

  // ── Icon ──
  'icon.title': 'Symbol',
  'icon.searchIcons': 'Symbole suchen...',
  'icon.noIconsFound': 'Keine Symbole gefunden',
  'icon.typeToSearch': 'Tippen, um Iconify-Symbole zu suchen',
  'icon.iconsCount': '{{count}} Symbole',

  // ── Variables Panel ──
  'variables.addTheme': 'Theme hinzufügen',
  'variables.addVariant': 'Variante hinzufügen',
  'variables.addVariable': 'Variable hinzufügen',
  'variables.searchVariables': 'Variablen suchen...',
  'variables.noMatch': 'Keine Variablen entsprechen Ihrer Suche',
  'variables.noDefined': 'Keine Variablen definiert',
  'variables.closeShortcut': 'Schließen (\u2318\u21e7V)',
  'variables.presets': 'Vorlagen',
  'variables.savePreset': 'Aktuelles als Vorlage speichern…',
  'variables.loadPreset': 'Vorlage laden',
  'variables.importPreset': 'Aus Datei importieren…',
  'variables.exportPreset': 'In Datei exportieren…',
  'variables.presetName': 'Vorlagenname',
  'variables.noPresets': 'Keine gespeicherten Vorlagen',

  // ── Design System (design.md) ──
  'designMd.title': 'Designsystem',
  'designMd.import': 'design.md importieren',
  'designMd.export': 'design.md exportieren',
  'designMd.autoGenerate': 'Automatisch aus Design generieren',
  'designMd.empty': 'Kein Designsystem geladen',
  'designMd.importCta': 'design.md importieren',
  'designMd.autoGenerateCta': 'Automatisch generieren',
  'designMd.visualTheme': 'Visuelles Thema',
  'designMd.colors': 'Farben',
  'designMd.typography': 'Typografie',
  'designMd.font': 'Schriftart',
  'designMd.headings': 'Überschriften',
  'designMd.body': 'Fließtext',
  'designMd.componentStyles': 'Komponentenstile',
  'designMd.layoutPrinciples': 'Layout-Prinzipien',
  'designMd.generationNotes': 'Generierungshinweise',
  'designMd.syncAllToVariables': 'Alle mit Variablen synchronisieren',
  'designMd.addAsVariable': '+Var',
  'designMd.copyHex': 'Hex-Wert kopieren',
  'designMd.remove': 'Designsystem entfernen',
  'toolbar.designSystem': 'Designsystem',

  // ── AI Chat ──
  'ai.newChat': 'Neuer Chat',
  'ai.collapse': 'Einklappen',
  'ai.tryExample': 'Probiere ein Beispiel zum Gestalten...',
  'ai.tipSelectElements':
    'Tipp: Wähle Elemente auf der Arbeitsfläche aus, bevor du chattest, um Kontext zu geben.',
  'ai.generating': 'Generiere...',
  'ai.designWithAgent': 'Mit Agent gestalten...',
  'ai.attachImage': 'Bild anhängen',
  'ai.stopGenerating': 'Generierung stoppen',
  'ai.sendMessage': 'Nachricht senden',
  'ai.loadingModels': 'Modelle werden geladen...',
  'ai.noModelsConnected': 'Keine Modelle verbunden',
  'ai.searchModels': 'Modelle suchen...',
  'ai.noModelsFound': 'Keine Modelle gefunden',
  'ai.quickAction.loginScreen': 'Einen mobilen Anmeldebildschirm gestalten',
  'ai.quickAction.loginScreenPrompt':
    'Design a modern mobile login screen with email input, password input, login button, and social login options',
  'ai.quickAction.foodApp': 'Food-App Startseite',
  'ai.quickAction.foodAppPrompt':
    'Generate a well-designed food mobile app homepage',
  'ai.quickAction.bottomNav': 'Eine untere Navigationsleiste gestalten',
  'ai.quickAction.bottomNavPrompt':
    'Design a mobile app bottom navigation bar with 5 tabs: Home, Search, Add, Messages, Profile',
  'ai.quickAction.colorPalette': 'Eine Farbpalette für meine App vorschlagen',
  'ai.quickAction.colorPalettePrompt':
    'Suggest a modern color palette for a pet care app',

  // ── Code Panel ──
  'code.reactTailwind': 'React + Tailwind',
  'code.htmlCss': 'HTML + CSS',
  'code.cssVariables': 'CSS Variables',
  'code.copyClipboard': 'In Zwischenablage kopieren',
  'code.copied': 'Kopiert!',
  'code.download': 'Code-Datei herunterladen',
  'code.closeCodePanel': 'Code-Panel schließen',
  'code.genCssVars': 'CSS-Variablen für das gesamte Dokument generieren',
  'code.genSelected':
    'Code für {{count}} ausgewählte(s) Element(e) generieren',
  'code.genDocument': 'Code für das gesamte Dokument generieren',
  'code.aiEnhance': 'KI-Verbesserung',
  'code.cancelEnhance': 'Verbesserung abbrechen',
  'code.resetEnhance': 'Zurücksetzen',
  'code.enhancing': 'KI verbessert den Code...',
  'code.enhanced': 'Von KI verbessert',

  // ── Save Dialog ──
  'save.saveAs': 'Speichern unter',
  'save.fileName': 'Dateiname',

  // ── Agent Settings ──
  'agents.title': 'Agenten & MCP einrichten',
  'agents.agentsOnCanvas': 'Agenten auf der Arbeitsfläche',
  'agents.mcpIntegrations': 'MCP-Integrationen im Terminal',
  'agents.transport': 'Transport',
  'agents.port': 'Port',
  'agents.mcpRestart':
    'MCP-Integrationen werden nach einem Neustart des Terminals wirksam.',
  'agents.modelCount': '{{count}} Modell(e)',
  'agents.connectionFailed': 'Verbindung fehlgeschlagen',
  'agents.serverError': 'Serverfehler {{status}}',
  'agents.failedTo': '{{action}} fehlgeschlagen',
  'agents.failedToMcp': 'MCP-Server {{action}} fehlgeschlagen',
  'agents.failedTransport': 'Transport konnte nicht aktualisiert werden',
  'agents.failedMcpTransport': 'MCP-Transport konnte nicht aktualisiert werden',
  'agents.claudeCode': 'Claude Code',
  'agents.claudeModels': 'Claude-Modelle',
  'agents.codexCli': 'Codex CLI',
  'agents.openaiModels': 'OpenAI-Modelle',
  'agents.opencode': 'OpenCode',
  'agents.opencodeDesc': '75+ LLM-Anbieter',
  'agents.copilot': 'GitHub Copilot',
  'agents.copilotDesc': 'GitHub Copilot-Modelle',
  'agents.geminiCli': 'Gemini CLI',
  'agents.geminiDesc': 'Google Gemini-Modelle',
  'agents.mcpServer': 'MCP-Server',
  'agents.mcpServerStart': 'Starten',
  'agents.mcpServerStop': 'Stoppen',
  'agents.mcpServerRunning': 'Läuft',
  'agents.mcpServerStopped': 'Gestoppt',
  'agents.mcpLanAccess': 'LAN-Zugriff',
  'agents.mcpClientConfig': 'Client-Konfiguration',
  'agents.stdio': 'stdio',
  'agents.http': 'http',
  'agents.stdioHttp': 'stdio + http',
  'agents.autoUpdate': 'Automatisch nach Updates suchen',
  'agents.notInstalled': 'Nicht installiert',
  'agents.install': 'Installieren',
  'agents.installing': 'Installiere...',
  'agents.installFailed': 'Installation fehlgeschlagen',
  'agents.viewDocs': 'Doku',
  'settings.title': 'Einstellungen',
  'settings.agents': 'Agents',
  'settings.mcp': 'MCP',
  'settings.images': 'Images',
  'settings.system': 'System',
  'settings.autoUpdateDesc': 'Beim Start automatisch nach neuen Versionen suchen',
  'settings.systemDesktopOnly': 'Systemeinstellungen sind in der Desktop-App verfügbar.',
  'settings.envHint': 'Sie können zusätzliche Umgebungsvariablen in {{path}} festlegen.',

  // ── Figma Import ──
  'figma.title': 'Aus Figma importieren',
  'figma.dropFile': '.fig-Datei hier ablegen',
  'figma.orBrowse': 'oder zum Durchsuchen klicken',
  'figma.exportTip': 'Aus Figma exportieren: Datei \u2192 Lokale Kopie speichern (.fig)',
  'figma.selectFigFile': 'Bitte eine .fig-Datei auswählen',
  'figma.noPages': 'Keine Seiten in der .fig-Datei gefunden',
  'figma.parseFailed': '.fig-Datei konnte nicht gelesen werden',
  'figma.convertFailed': 'Figma-Datei konnte nicht konvertiert werden',
  'figma.parsing': '.fig-Datei wird gelesen...',
  'figma.converting': 'Knoten werden konvertiert...',
  'figma.selectPage':
    'Diese Datei hat {{count}} Seiten. Wähle, welche importiert werden sollen:',
  'figma.layers': '{{count}} Ebenen',
  'figma.importAll': 'Alle Seiten importieren',
  'figma.importComplete': 'Import abgeschlossen!',
  'figma.moreWarnings': '...und {{count}} weitere Warnungen',
  'figma.tryAgain': 'Erneut versuchen',
  'figma.layoutMode': 'Layout-Modus:',
  'figma.preserveLayout': 'Figma-Layout beibehalten',
  'figma.autoLayout': 'OpenPencil Auto-Layout',
  'figma.comingSoon': 'Demnächst',

  // ── Landing Page ──
  'landing.open': 'Open',
  'landing.pencil': 'Pencil',
  'landing.tagline':
    'Open-Source-Vektordesign-Werkzeug. Design as Code.',
  'landing.newDesign': 'Neues Design',
  'landing.shortcutHint': 'Drücke {{key1}} + {{key2}} für ein neues Design',

  // ── 404 ──
  'notFound.message': 'Seite nicht gefunden',

  // ── Component Browser ──
  'componentBrowser.title': 'UIKit-Browser',
  'componentBrowser.exportKit': 'Kit exportieren',
  'componentBrowser.importKit': 'Kit importieren',
  'componentBrowser.kit': 'Kit:',
  'componentBrowser.all': 'Alle',
  'componentBrowser.imported': '(importiert)',
  'componentBrowser.components': 'Komponenten',
  'componentBrowser.searchComponents': 'Komponenten suchen...',
  'componentBrowser.deleteKit': '{{name}} löschen',
  'componentBrowser.category.all': 'Alle',
  'componentBrowser.category.buttons': 'Schaltflächen',
  'componentBrowser.category.inputs': 'Eingabefelder',
  'componentBrowser.category.cards': 'Karten',
  'componentBrowser.category.nav': 'Navigation',
  'componentBrowser.category.layout': 'Layout',
  'componentBrowser.category.feedback': 'Feedback',
  'componentBrowser.category.data': 'Daten',
  'componentBrowser.category.other': 'Sonstige',

  // ── Variable Picker ──
  'variablePicker.boundTo': 'Gebunden an --{{name}}',
  'variablePicker.bindToVariable': 'An Variable binden',
  'variablePicker.unbind': 'Variable lösen',
  'variablePicker.noVariables': 'Keine {{type}}-Variablen definiert',
} as const

export default de
