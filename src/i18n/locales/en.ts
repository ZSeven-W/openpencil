const en = {
  // ── Common ──
  'common.rename': 'Rename',
  'common.duplicate': 'Duplicate',
  'common.delete': 'Delete',
  'common.cancel': 'Cancel',
  'common.save': 'Save',
  'common.close': 'Close',
  'common.connect': 'Connect',
  'common.disconnect': 'Disconnect',
  'common.import': 'Import',
  'common.export': 'Export',
  'common.name': 'Name',
  'common.untitled': 'Untitled',
  'common.best': 'Best',
  'common.selected': '{{count}} selected',

  // ── Toolbar ──
  'toolbar.select': 'Select',
  'toolbar.text': 'Text',
  'toolbar.frame': 'Frame',
  'toolbar.hand': 'Hand',
  'toolbar.undo': 'Undo',
  'toolbar.redo': 'Redo',
  'toolbar.variables': 'Variables',
  'toolbar.uikitBrowser': 'UIKit Browser',

  // ── Shapes ──
  'shapes.rectangle': 'Rectangle',
  'shapes.ellipse': 'Ellipse',
  'shapes.line': 'Line',
  'shapes.icon': 'Icon',
  'shapes.importImageSvg': 'Import Image or SVG\u2026',
  'shapes.pen': 'Pen',
  'shapes.shapeTools': 'Shape tools',
  'shapes.moreShapeTools': 'More shape tools',

  // ── Top Bar ──
  'topbar.hideLayers': 'Hide layers',
  'topbar.showLayers': 'Show layers',
  'topbar.new': 'New',
  'topbar.open': 'Open',
  'topbar.save': 'Save',
  'topbar.importFigma': 'Import Figma',
  'topbar.codePanel': 'Code',
  'topbar.lightMode': 'Light mode',
  'topbar.darkMode': 'Dark mode',
  'topbar.fullscreen': 'Fullscreen',
  'topbar.exitFullscreen': 'Exit fullscreen',
  'topbar.edited': '— Edited',
  'topbar.agentsAndMcp': 'Agents & MCP',
  'topbar.setupAgentsMcp': 'Setup Agents & MCP',
  'topbar.connected': 'connected',
  'topbar.agentStatus': '{{agents}} agent{{agentSuffix}} · {{mcp}} MCP',

  // ── Right Panel ──
  'rightPanel.design': 'Design',
  'rightPanel.code': 'Code',
  'rightPanel.noSelection': 'Select an element',

  // ── Pages ──
  'pages.title': 'Pages',
  'pages.addPage': 'Add page',
  'pages.moveUp': 'Move Up',
  'pages.moveDown': 'Move Down',

  // ── Status Bar ──
  'statusbar.zoomOut': 'Zoom out',
  'statusbar.zoomIn': 'Zoom in',
  'statusbar.resetZoom': 'Reset zoom',

  // ── Updater ──
  'updater.softwareUpdate': 'Software Update',
  'updater.dismiss': 'Dismiss',
  'updater.current': 'Current',
  'updater.latest': 'Latest',
  'updater.unknown': 'Unknown',
  'updater.checking': 'Checking...',
  'updater.downloadProgress': 'Download Progress',
  'updater.checkAgain': 'Check Again',
  'updater.restartInstall': 'Restart & Install',
  'updater.installing': 'Installing...',
  'updater.releaseDate': 'Release date: {{date}}',
  'updater.restartHint':
    'Restart to apply update. Relaunch usually takes 10-15 seconds.',
  'updater.unknownError': 'Unknown updater error.',
  'updater.title.checking': 'Checking for updates',
  'updater.title.available': 'Update found',
  'updater.title.downloading': 'Downloading update',
  'updater.title.downloaded': 'Ready to install',
  'updater.title.error': 'Update failed',
  'updater.subtitle.checking': 'Looking for the latest release...',
  'updater.subtitle.available': 'Version {{version}} is available.',
  'updater.subtitle.availableGeneric': 'A new version is available.',
  'updater.subtitle.downloading':
    'Version {{version}} is downloading in the background.',
  'updater.subtitle.downloadingGeneric':
    'Downloading update package in the background.',
  'updater.subtitle.downloaded': 'Version {{version}} has been downloaded.',
  'updater.subtitle.downloadedGeneric': 'The update has been downloaded.',
  'updater.subtitle.error': 'Unable to check or download the update.',

  // ── Layers ──
  'layers.title': 'Layers',
  'layers.empty': 'No layers yet. Use the toolbar to draw shapes.',

  // ── Layer Context Menu ──
  'layerMenu.groupSelection': 'Group Selection',
  'layerMenu.createComponent': 'Create Component',
  'layerMenu.detachComponent': 'Detach Component',
  'layerMenu.detachInstance': 'Detach Instance',
  'layerMenu.booleanUnion': 'Union',
  'layerMenu.booleanSubtract': 'Subtract',
  'layerMenu.booleanIntersect': 'Intersect',
  'layerMenu.toggleLock': 'Toggle Lock',
  'layerMenu.toggleVisibility': 'Toggle Visibility',

  // ── Property Panel ──
  'property.createComponent': 'Create Component',
  'property.detachComponent': 'Detach Component',
  'property.goToComponent': 'Go to component',
  'property.detachInstance': 'Detach instance',

  // ── Fill ──
  'fill.title': 'Fill',
  'fill.solid': 'Solid',
  'fill.linear': 'Linear',
  'fill.radial': 'Radial',
  'fill.image': 'Image',
  'fill.stops': 'Stops',
  'fill.angle': 'Angle',

  // ── Image ──
  'image.title': 'Image',
  'image.fit': 'Fit Mode',
  'image.fill': 'Fill',
  'image.fitMode': 'Fit',
  'image.crop': 'Crop',
  'image.tile': 'Tile',
  'image.clickToUpload': 'Click to upload',
  'image.changeImage': 'Change image',
  'image.adjustments': 'Adjustments',
  'image.exposure': 'Exposure',
  'image.contrast': 'Contrast',
  'image.saturation': 'Saturation',
  'image.temperature': 'Temperature',
  'image.tint': 'Tint',
  'image.highlights': 'Highlights',
  'image.shadows': 'Shadows',
  'image.reset': 'Reset',

  // ── Stroke ──
  'stroke.title': 'Stroke',

  // ── Appearance ──
  'appearance.layer': 'Layer',
  'appearance.opacity': 'Opacity',

  // ── Layout ──
  'layout.flexLayout': 'Flex Layout',
  'layout.freedom': 'Freedom (no layout)',
  'layout.vertical': 'Vertical layout',
  'layout.horizontal': 'Horizontal layout',
  'layout.alignment': 'Alignment',
  'layout.gap': 'Gap',
  'layout.spaceBetween': 'Space Between',
  'layout.spaceAround': 'Space Around',
  'layout.dimensions': 'Dimensions',
  'layout.fillWidth': 'Fill Width',
  'layout.fillHeight': 'Fill Height',
  'layout.hugWidth': 'Hug Width',
  'layout.hugHeight': 'Hug Height',
  'layout.clipContent': 'Clip Content',

  // ── Padding ──
  'padding.title': 'Padding',
  'padding.paddingMode': 'Padding mode',
  'padding.paddingValues': 'Padding Values',
  'padding.oneValue': 'One value for all sides',
  'padding.horizontalVertical': 'Horizontal/Vertical',
  'padding.topRightBottomLeft': 'Top/Right/Bottom/Left',

  // ── Typography ──
  'text.typography': 'Typography',
  'text.lineHeight': 'Line height',
  'text.letterSpacing': 'Letter spacing',
  'text.horizontal': 'Horizontal',
  'text.vertical': 'Vertical',
  'text.alignLeft': 'Align left',
  'text.alignCenter': 'Align center',
  'text.alignRight': 'Align right',
  'text.justify': 'Justify',
  'text.top': 'Top',
  'text.middle': 'Middle',
  'text.bottom': 'Bottom',
  'text.weight.thin': 'Thin',
  'text.weight.light': 'Light',
  'text.weight.regular': 'Regular',
  'text.weight.medium': 'Medium',
  'text.weight.semibold': 'Semibold',
  'text.weight.bold': 'Bold',
  'text.weight.black': 'Black',
  'text.font.search': 'Search fonts…',
  'text.font.bundled': 'Bundled',
  'text.font.system': 'System',
  'text.font.loading': 'Loading fonts…',
  'text.font.noResults': 'No fonts found',

  // ── Text Layout ──
  'textLayout.title': 'Layout',
  'textLayout.dimensions': 'Dimensions',
  'textLayout.resizing': 'Resizing',
  'textLayout.autoWidth': 'Auto W',
  'textLayout.autoWidthDesc': 'Auto Width \u2014 text expands horizontally',
  'textLayout.autoHeight': 'Auto H',
  'textLayout.autoHeightDesc':
    'Auto Height \u2014 fixed width, height auto-sizes',
  'textLayout.fixed': 'Fixed',
  'textLayout.fixedDesc':
    'Fixed Size \u2014 both width and height are fixed',
  'textLayout.fillWidth': 'Fill Width',
  'textLayout.fillHeight': 'Fill Height',

  // ── Effects ──
  'effects.title': 'Effects',
  'effects.dropShadow': 'Drop shadow',
  'effects.blur': 'Blur',
  'effects.spread': 'Spread',
  'effects.color': 'Color',

  // ── Export ──
  'export.title': 'Export',
  'export.format': 'Format',
  'export.scale': 'Scale',
  'export.selectedOnly': 'Export selected only',
  'export.exportFormat': 'Export {{format}}',
  'export.exportLayer': 'Export layer',

  // ── Corner Radius ──
  'cornerRadius.title': 'Corner Radius',

  // ── Size / Position ──
  'size.position': 'Position',

  // ── Icon ──
  'icon.title': 'Icon',
  'icon.searchIcons': 'Search icons...',
  'icon.noIconsFound': 'No icons found',
  'icon.typeToSearch': 'Type to search Iconify icons',
  'icon.iconsCount': '{{count}} icons',

  // ── Variables Panel ──
  'variables.addTheme': 'Add theme',
  'variables.addVariant': 'Add variant',
  'variables.addVariable': 'Add variable',
  'variables.searchVariables': 'Search variables...',
  'variables.noMatch': 'No variables match your search',
  'variables.noDefined': 'No variables defined',
  'variables.closeShortcut': 'Close (\u2318\u21e7V)',
  'variables.presets': 'Presets',
  'variables.savePreset': 'Save Current as Preset\u2026',
  'variables.loadPreset': 'Load Preset',
  'variables.importPreset': 'Import from File\u2026',
  'variables.exportPreset': 'Export to File\u2026',
  'variables.presetName': 'Preset name',
  'variables.noPresets': 'No saved presets',

  // ── AI Chat ──
  'ai.newChat': 'New chat',
  'ai.collapse': 'Collapse',
  'ai.tryExample': 'Try an example to design...',
  'ai.tipSelectElements':
    'Tip: Select elements on canvas before chatting for context.',
  'ai.generating': 'Generating...',
  'ai.designWithAgent': 'Design with Agent...',
  'ai.attachImage': 'Attach image',
  'ai.stopGenerating': 'Stop generating',
  'ai.sendMessage': 'Send message',
  'ai.loadingModels': 'Loading models...',
  'ai.noModelsConnected': 'No models connected',
  'ai.quickAction.loginScreen': 'Design a mobile login screen',
  'ai.quickAction.loginScreenPrompt':
    'Design a modern mobile login screen with email input, password input, login button, and social login options',
  'ai.quickAction.foodApp': 'Food app homepage',
  'ai.quickAction.foodAppPrompt':
    'Generate a well-designed food mobile app homepage',
  'ai.quickAction.bottomNav': 'Design a bottom navigation bar',
  'ai.quickAction.bottomNavPrompt':
    'Design a mobile app bottom navigation bar with 5 tabs: Home, Search, Add, Messages, Profile',
  'ai.quickAction.colorPalette': 'Suggest a color palette for my app',
  'ai.quickAction.colorPalettePrompt':
    'Suggest a modern color palette for a pet care app',

  // ── Code Panel ──
  'code.reactTailwind': 'React + Tailwind',
  'code.htmlCss': 'HTML + CSS',
  'code.cssVariables': 'CSS Variables',
  'code.copyClipboard': 'Copy to clipboard',
  'code.copied': 'Copied!',
  'code.download': 'Download code file',
  'code.closeCodePanel': 'Close code panel',
  'code.genCssVars': 'Generating CSS variables for entire document',
  'code.genSelected':
    'Generating code for {{count}} selected element(s)',
  'code.genDocument': 'Generating code for entire document',
  'code.aiEnhance': 'AI Enhance',
  'code.cancelEnhance': 'Cancel enhancement',
  'code.resetEnhance': 'Reset to original',
  'code.enhancing': 'AI is enhancing code...',
  'code.enhanced': 'Enhanced by AI',

  // ── Save Dialog ──
  'save.saveAs': 'Save As',
  'save.fileName': 'File name',

  // ── Agent Settings ──
  'agents.title': 'Setup Agents & MCP',
  'agents.agentsOnCanvas': 'Agents on Canvas',
  'agents.mcpIntegrations': 'MCP Integrations in Terminal',
  'agents.transport': 'Transport',
  'agents.port': 'Port',
  'agents.mcpRestart':
    'MCP integrations will take effect after restarting the terminal.',
  'agents.modelCount': '{{count}} model(s)',
  'agents.connectionFailed': 'Connection failed',
  'agents.serverError': 'Server error {{status}}',
  'agents.failedTo': 'Failed to {{action}}',
  'agents.failedToMcp': 'Failed to {{action}} MCP server',
  'agents.failedTransport': 'Failed to update transport',
  'agents.failedMcpTransport': 'Failed to update MCP transport',
  'agents.claudeCode': 'Claude Code',
  'agents.claudeModels': 'Claude models',
  'agents.codexCli': 'Codex CLI',
  'agents.openaiModels': 'OpenAI models',
  'agents.opencode': 'OpenCode',
  'agents.opencodeDesc': '75+ LLM providers',
  'agents.copilot': 'GitHub Copilot',
  'agents.copilotDesc': 'GitHub Copilot models',
  'agents.mcpServer': 'MCP Server',
  'agents.mcpServerStart': 'Start',
  'agents.mcpServerStop': 'Stop',
  'agents.mcpServerRunning': 'Running',
  'agents.mcpServerStopped': 'Stopped',
  'agents.mcpLanAccess': 'LAN Access',
  'agents.mcpClientConfig': 'Client Config',
  'agents.stdio': 'stdio',
  'agents.http': 'http',
  'agents.stdioHttp': 'stdio + http',
  'agents.autoUpdate': 'Auto-check for updates',
  'agents.notInstalled': 'Not installed',
  'agents.install': 'Install',
  'agents.installing': 'Installing...',
  'agents.installFailed': 'Installation failed',
  'agents.viewDocs': 'Docs',

  // ── Figma Import ──
  'figma.title': 'Import from Figma',
  'figma.dropFile': 'Drop a .fig file here',
  'figma.orBrowse': 'or click to browse',
  'figma.exportTip': 'Export from Figma: File \u2192 Save local copy (.fig)',
  'figma.selectFigFile': 'Please select a .fig file',
  'figma.noPages': 'No pages found in the .fig file',
  'figma.parseFailed': 'Failed to parse .fig file',
  'figma.convertFailed': 'Failed to convert Figma file',
  'figma.parsing': 'Parsing .fig file...',
  'figma.converting': 'Converting nodes...',
  'figma.selectPage':
    'This file has {{count}} pages. Select which to import:',
  'figma.layers': '{{count}} layers',
  'figma.importAll': 'Import All Pages',
  'figma.importComplete': 'Import complete!',
  'figma.moreWarnings': '...and {{count}} more warnings',
  'figma.tryAgain': 'Try Again',
  'figma.layoutMode': 'Layout mode:',
  'figma.preserveLayout': 'Preserve Figma layout',
  'figma.autoLayout': 'OpenPencil auto layout',
  'figma.comingSoon': 'Coming soon',

  // ── Landing Page ──
  'landing.open': 'Open',
  'landing.pencil': 'Pencil',
  'landing.tagline': 'Open-source vector design tool. Design as Code.',
  'landing.newDesign': 'New Design',
  'landing.shortcutHint': 'Press {{key1}} + {{key2}} to create a new design',

  // ── 404 ──
  'notFound.message': 'Page not found',

  // ── Component Browser ──
  'componentBrowser.title': 'UIKit Browser',
  'componentBrowser.exportKit': 'Export kit',
  'componentBrowser.importKit': 'Import kit',
  'componentBrowser.kit': 'Kit:',
  'componentBrowser.all': 'All',
  'componentBrowser.imported': '(imported)',
  'componentBrowser.components': 'components',
  'componentBrowser.searchComponents': 'Search components...',
  'componentBrowser.deleteKit': 'Delete {{name}}',
  'componentBrowser.category.all': 'All',
  'componentBrowser.category.buttons': 'Buttons',
  'componentBrowser.category.inputs': 'Inputs',
  'componentBrowser.category.cards': 'Cards',
  'componentBrowser.category.nav': 'Nav',
  'componentBrowser.category.layout': 'Layout',
  'componentBrowser.category.feedback': 'Feedback',
  'componentBrowser.category.data': 'Data',
  'componentBrowser.category.other': 'Other',

  // ── Variable Picker ──
  'variablePicker.boundTo': 'Bound to --{{name}}',
  'variablePicker.bindToVariable': 'Bind to variable',
  'variablePicker.unbind': 'Unbind variable',
  'variablePicker.noVariables': 'No {{type}} variables defined',
} as const

export default en
export type TranslationKeys = { [K in keyof typeof en]: string }
