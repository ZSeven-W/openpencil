import type { TranslationKeys } from './en'

const hi: TranslationKeys = {
  // ── Common ──
  'common.rename': 'नाम बदलें',
  'common.duplicate': 'डुप्लिकेट',
  'common.delete': 'हटाएँ',
  'common.cancel': 'रद्द करें',
  'common.save': 'सहेजें',
  'common.close': 'बंद करें',
  'common.connect': 'कनेक्ट करें',
  'common.disconnect': 'डिस्कनेक्ट करें',
  'common.import': 'आयात करें',
  'common.export': 'निर्यात करें',
  'common.name': 'नाम',
  'common.untitled': 'शीर्षकहीन',
  'common.best': 'सर्वश्रेष्ठ',
  'common.selected': '{{count}} चयनित',

  // ── Toolbar ──
  'toolbar.select': 'चयन करें',
  'toolbar.text': 'टेक्स्ट',
  'toolbar.frame': 'फ़्रेम',
  'toolbar.hand': 'हैंड',
  'toolbar.undo': 'पूर्ववत करें',
  'toolbar.redo': 'फिर से करें',
  'toolbar.variables': 'वेरिएबल',
  'toolbar.uikitBrowser': 'UIKit ब्राउज़र',

  // ── Shapes ──
  'shapes.rectangle': 'आयत',
  'shapes.ellipse': 'दीर्घवृत्त',
  'shapes.line': 'रेखा',
  'shapes.icon': 'आइकन',
  'shapes.importImageSvg': 'इमेज या SVG आयात करें\u2026',
  'shapes.pen': 'पेन',
  'shapes.shapeTools': 'आकार उपकरण',
  'shapes.moreShapeTools': 'और आकार उपकरण',

  // ── Top Bar ──
  'topbar.hideLayers': 'लेयर छिपाएँ',
  'topbar.showLayers': 'लेयर दिखाएँ',
  'topbar.new': 'नया',
  'topbar.open': 'खोलें',
  'topbar.save': 'सहेजें',
  'topbar.importFigma': 'Figma आयात करें',
  'topbar.codePanel': 'कोड',
  'topbar.lightMode': 'लाइट मोड',
  'topbar.darkMode': 'डार्क मोड',
  'topbar.fullscreen': 'पूर्ण स्क्रीन',
  'topbar.exitFullscreen': 'पूर्ण स्क्रीन से बाहर निकलें',
  'topbar.edited': '— संपादित',
  'topbar.agentsAndMcp': 'एजेंट और MCP',
  'topbar.setupAgentsMcp': 'एजेंट और MCP सेटअप करें',
  'topbar.connected': 'कनेक्टेड',
  'topbar.agentStatus': '{{agents}} एजेंट{{agentSuffix}} · {{mcp}} MCP',

  // ── Right Panel ──
  'rightPanel.design': 'डिज़ाइन',
  'rightPanel.code': 'कोड',
  'rightPanel.noSelection': 'एक तत्व चुनें',

  // ── Pages ──
  'pages.title': 'पेज',
  'pages.addPage': 'पेज जोड़ें',
  'pages.moveUp': 'ऊपर ले जाएँ',
  'pages.moveDown': 'नीचे ले जाएँ',

  // ── Status Bar ──
  'statusbar.zoomOut': 'ज़ूम आउट',
  'statusbar.zoomIn': 'ज़ूम इन',
  'statusbar.resetZoom': 'ज़ूम रीसेट करें',

  // ── Updater ──
  'updater.softwareUpdate': 'सॉफ़्टवेयर अपडेट',
  'updater.dismiss': 'खारिज करें',
  'updater.current': 'वर्तमान',
  'updater.latest': 'नवीनतम',
  'updater.unknown': 'अज्ञात',
  'updater.checking': 'जाँच हो रही है...',
  'updater.downloadProgress': 'डाउनलोड प्रगति',
  'updater.checkAgain': 'फिर से जाँचें',
  'updater.restartInstall': 'पुनः आरंभ करें और इंस्टॉल करें',
  'updater.installing': 'इंस्टॉल हो रहा है...',
  'updater.releaseDate': 'रिलीज़ तिथि: {{date}}',
  'updater.restartHint':
    'अपडेट लागू करने के लिए पुनः आरंभ करें। पुनः लॉन्च में आमतौर पर 10-15 सेकंड लगते हैं।',
  'updater.unknownError': 'अज्ञात अपडेटर त्रुटि।',
  'updater.title.checking': 'अपडेट की जाँच हो रही है',
  'updater.title.available': 'अपडेट मिला',
  'updater.title.downloading': 'अपडेट डाउनलोड हो रहा है',
  'updater.title.downloaded': 'इंस्टॉल के लिए तैयार',
  'updater.title.error': 'अपडेट विफल',
  'updater.subtitle.checking': 'नवीनतम रिलीज़ की खोज हो रही है...',
  'updater.subtitle.available': 'संस्करण {{version}} उपलब्ध है।',
  'updater.subtitle.availableGeneric': 'एक नया संस्करण उपलब्ध है।',
  'updater.subtitle.downloading':
    'संस्करण {{version}} बैकग्राउंड में डाउनलोड हो रहा है।',
  'updater.subtitle.downloadingGeneric':
    'अपडेट पैकेज बैकग्राउंड में डाउनलोड हो रहा है।',
  'updater.subtitle.downloaded': 'संस्करण {{version}} डाउनलोड हो गया है।',
  'updater.subtitle.downloadedGeneric': 'अपडेट डाउनलोड हो गया है।',
  'updater.subtitle.error': 'अपडेट की जाँच या डाउनलोड नहीं हो सका।',

  // ── Layers ──
  'layers.title': 'लेयर',
  'layers.empty': 'अभी कोई लेयर नहीं है। आकार बनाने के लिए टूलबार का उपयोग करें।',

  // ── Layer Context Menu ──
  'layerMenu.groupSelection': 'चयन को ग्रुप करें',
  'layerMenu.createComponent': 'कंपोनेंट बनाएँ',
  'layerMenu.detachComponent': 'कंपोनेंट अलग करें',
  'layerMenu.detachInstance': 'इंस्टेंस अलग करें',
  'layerMenu.booleanUnion': 'संयोजन',
  'layerMenu.booleanSubtract': 'घटाना',
  'layerMenu.booleanIntersect': 'प्रतिच्छेदन',
  'layerMenu.toggleLock': 'लॉक टॉगल करें',
  'layerMenu.toggleVisibility': 'दृश्यता टॉगल करें',

  // ── Property Panel ──
  'property.createComponent': 'कंपोनेंट बनाएँ',
  'property.detachComponent': 'कंपोनेंट अलग करें',
  'property.goToComponent': 'कंपोनेंट पर जाएँ',
  'property.detachInstance': 'इंस्टेंस अलग करें',

  // ── Fill ──
  'fill.title': 'भरण',
  'fill.solid': 'ठोस',
  'fill.linear': 'रैखिक',
  'fill.radial': 'वृत्तीय',
  'fill.image': 'चित्र',
  'fill.stops': 'स्टॉप',
  'fill.angle': 'कोण',

  // ── Image ──
  'image.title': 'चित्र',
  'image.fit': 'फिट मोड',
  'image.fill': 'भरें',
  'image.fitMode': 'फिट',
  'image.crop': 'काटें',
  'image.tile': 'टाइल',
  'image.clickToUpload': 'अपलोड करने के लिए क्लिक करें',
  'image.changeImage': 'चित्र बदलें',
  'image.adjustments': 'समायोजन',
  'image.exposure': 'एक्सपोज़र',
  'image.contrast': 'कंट्रास्ट',
  'image.saturation': 'संतृप्ति',
  'image.temperature': 'तापमान',
  'image.tint': 'टिंट',
  'image.highlights': 'हाइलाइट्स',
  'image.shadows': 'छायाएँ',
  'image.reset': 'रीसेट',

  // ── Stroke ──
  'stroke.title': 'स्ट्रोक',

  // ── Appearance ──
  'appearance.layer': 'लेयर',
  'appearance.opacity': 'अपारदर्शिता',

  // ── Layout ──
  'layout.flexLayout': 'फ़्लेक्स लेआउट',
  'layout.freedom': 'स्वतंत्र (कोई लेआउट नहीं)',
  'layout.vertical': 'लंबवत लेआउट',
  'layout.horizontal': 'क्षैतिज लेआउट',
  'layout.alignment': 'संरेखण',
  'layout.gap': 'अंतराल',
  'layout.spaceBetween': 'बीच में जगह',
  'layout.spaceAround': 'चारों ओर जगह',
  'layout.dimensions': 'आयाम',
  'layout.fillWidth': 'चौड़ाई भरें',
  'layout.fillHeight': 'ऊँचाई भरें',
  'layout.hugWidth': 'चौड़ाई समेटें',
  'layout.hugHeight': 'ऊँचाई समेटें',
  'layout.clipContent': 'कंटेंट क्लिप करें',

  // ── Padding ──
  'padding.title': 'पैडिंग',
  'padding.paddingMode': 'पैडिंग मोड',
  'padding.paddingValues': 'पैडिंग मान',
  'padding.oneValue': 'सभी भुजाओं के लिए एक मान',
  'padding.horizontalVertical': 'क्षैतिज/लंबवत',
  'padding.topRightBottomLeft': 'ऊपर/दाएँ/नीचे/बाएँ',

  // ── Typography ──
  'text.typography': 'टाइपोग्राफ़ी',
  'text.lineHeight': 'लाइन हाइट',
  'text.letterSpacing': 'अक्षर अंतराल',
  'text.horizontal': 'क्षैतिज',
  'text.vertical': 'लंबवत',
  'text.alignLeft': 'बाएँ संरेखित करें',
  'text.alignCenter': 'मध्य संरेखित करें',
  'text.alignRight': 'दाएँ संरेखित करें',
  'text.justify': 'जस्टिफ़ाइ',
  'text.top': 'ऊपर',
  'text.middle': 'मध्य',
  'text.bottom': 'नीचे',
  'text.weight.thin': 'पतला',
  'text.weight.light': 'हल्का',
  'text.weight.regular': 'सामान्य',
  'text.weight.medium': 'मध्यम',
  'text.weight.semibold': 'सेमीबोल्ड',
  'text.weight.bold': 'बोल्ड',
  'text.weight.black': 'ब्लैक',
  'text.font.search': 'फ़ॉन्ट खोजें\u2026',
  'text.font.bundled': 'बंडल',
  'text.font.system': 'सिस्टम',
  'text.font.loading': 'फ़ॉन्ट लोड हो रहे हैं\u2026',
  'text.font.noResults': 'कोई फ़ॉन्ट नहीं मिला',

  // ── Text Layout ──
  'textLayout.title': 'लेआउट',
  'textLayout.dimensions': 'आयाम',
  'textLayout.resizing': 'आकार बदलना',
  'textLayout.autoWidth': 'ऑटो W',
  'textLayout.autoWidthDesc': 'ऑटो चौड़ाई \u2014 टेक्स्ट क्षैतिज रूप से फैलता है',
  'textLayout.autoHeight': 'ऑटो H',
  'textLayout.autoHeightDesc':
    'ऑटो ऊँचाई \u2014 निश्चित चौड़ाई, ऊँचाई स्वतः समायोजित',
  'textLayout.fixed': 'निश्चित',
  'textLayout.fixedDesc':
    'निश्चित आकार \u2014 चौड़ाई और ऊँचाई दोनों निश्चित हैं',
  'textLayout.fillWidth': 'चौड़ाई भरें',
  'textLayout.fillHeight': 'ऊँचाई भरें',

  // ── Effects ──
  'effects.title': 'प्रभाव',
  'effects.dropShadow': 'ड्रॉप शैडो',
  'effects.blur': 'ब्लर',
  'effects.spread': 'स्प्रेड',
  'effects.color': 'रंग',

  // ── Export ──
  'export.title': 'निर्यात',
  'export.format': 'फ़ॉर्मेट',
  'export.scale': 'स्केल',
  'export.selectedOnly': 'केवल चयनित निर्यात करें',
  'export.exportFormat': '{{format}} निर्यात करें',
  'export.exportLayer': 'लेयर निर्यात करें',

  // ── Corner Radius ──
  'cornerRadius.title': 'कोने की गोलाई',

  // ── Size / Position ──
  'size.position': 'स्थिति',

  // ── Icon ──
  'icon.title': 'आइकन',
  'icon.searchIcons': 'आइकन खोजें...',
  'icon.noIconsFound': 'कोई आइकन नहीं मिला',
  'icon.typeToSearch': 'Iconify आइकन खोजने के लिए टाइप करें',
  'icon.iconsCount': '{{count}} आइकन',

  // ── Variables Panel ──
  'variables.addTheme': 'थीम जोड़ें',
  'variables.addVariant': 'वेरिएंट जोड़ें',
  'variables.addVariable': 'वेरिएबल जोड़ें',
  'variables.searchVariables': 'वेरिएबल खोजें...',
  'variables.noMatch': 'आपकी खोज से कोई वेरिएबल मेल नहीं खाता',
  'variables.noDefined': 'कोई वेरिएबल परिभाषित नहीं है',
  'variables.closeShortcut': 'बंद करें (\u2318\u21e7V)',
  'variables.presets': 'प्रीसेट',
  'variables.savePreset': 'वर्तमान को प्रीसेट के रूप में सहेजें…',
  'variables.loadPreset': 'प्रीसेट लोड करें',
  'variables.importPreset': 'फ़ाइल से आयात करें…',
  'variables.exportPreset': 'फ़ाइल में निर्यात करें…',
  'variables.presetName': 'प्रीसेट का नाम',
  'variables.noPresets': 'कोई सहेजा गया प्रीसेट नहीं',

  // ── AI Chat ──
  'ai.newChat': 'नई चैट',
  'ai.collapse': 'संक्षिप्त करें',
  'ai.tryExample': 'डिज़ाइन करने के लिए एक उदाहरण आज़माएँ...',
  'ai.tipSelectElements':
    'सुझाव: संदर्भ के लिए चैट से पहले कैनवस पर तत्वों का चयन करें।',
  'ai.generating': 'जनरेट हो रहा है...',
  'ai.designWithAgent': 'एजेंट के साथ डिज़ाइन करें...',
  'ai.attachImage': 'इमेज संलग्न करें',
  'ai.stopGenerating': 'जनरेशन रोकें',
  'ai.sendMessage': 'संदेश भेजें',
  'ai.loadingModels': 'मॉडल लोड हो रहे हैं...',
  'ai.noModelsConnected': 'कोई मॉडल कनेक्ट नहीं है',
  'ai.quickAction.loginScreen': 'मोबाइल लॉगिन स्क्रीन डिज़ाइन करें',
  'ai.quickAction.loginScreenPrompt':
    'ईमेल इनपुट, पासवर्ड इनपुट, लॉगिन बटन और सोशल लॉगिन विकल्पों के साथ एक आधुनिक मोबाइल लॉगिन स्क्रीन डिज़ाइन करें',
  'ai.quickAction.foodApp': 'फ़ूड ऐप होमपेज',
  'ai.quickAction.foodAppPrompt':
    'Generate a well-designed food mobile app homepage',
  'ai.quickAction.bottomNav': 'बॉटम नेविगेशन बार डिज़ाइन करें',
  'ai.quickAction.bottomNavPrompt':
    '5 टैब के साथ एक मोबाइल ऐप बॉटम नेविगेशन बार डिज़ाइन करें: होम, खोजें, जोड़ें, संदेश, प्रोफ़ाइल',
  'ai.quickAction.colorPalette': 'मेरे ऐप के लिए कलर पैलेट सुझाएँ',
  'ai.quickAction.colorPalettePrompt':
    'एक पेट केयर ऐप के लिए आधुनिक कलर पैलेट सुझाएँ',

  // ── Code Panel ──
  'code.reactTailwind': 'React + Tailwind',
  'code.htmlCss': 'HTML + CSS',
  'code.cssVariables': 'CSS Variables',
  'code.copyClipboard': 'क्लिपबोर्ड पर कॉपी करें',
  'code.copied': 'कॉपी हो गया!',
  'code.download': 'कोड फ़ाइल डाउनलोड करें',
  'code.closeCodePanel': 'कोड पैनल बंद करें',
  'code.genCssVars': 'संपूर्ण डॉक्यूमेंट के लिए CSS वेरिएबल जनरेट हो रहे हैं',
  'code.genSelected':
    '{{count}} चयनित तत्व(ओं) के लिए कोड जनरेट हो रहा है',
  'code.genDocument': 'संपूर्ण डॉक्यूमेंट के लिए कोड जनरेट हो रहा है',
  'code.aiEnhance': 'AI से सुधारें',
  'code.cancelEnhance': 'सुधार रद्द करें',
  'code.resetEnhance': 'मूल पर वापस जाएं',
  'code.enhancing': 'AI कोड सुधार रहा है...',
  'code.enhanced': 'AI द्वारा सुधारा गया',

  // ── Save Dialog ──
  'save.saveAs': 'इस रूप में सहेजें',
  'save.fileName': 'फ़ाइल का नाम',

  // ── Agent Settings ──
  'agents.title': 'एजेंट और MCP सेटअप करें',
  'agents.agentsOnCanvas': 'कैनवस पर एजेंट',
  'agents.mcpIntegrations': 'टर्मिनल में MCP इंटीग्रेशन',
  'agents.transport': 'ट्रांसपोर्ट',
  'agents.port': 'पोर्ट',
  'agents.mcpRestart':
    'MCP इंटीग्रेशन टर्मिनल पुनः आरंभ करने के बाद प्रभावी होंगे।',
  'agents.modelCount': '{{count}} मॉडल',
  'agents.connectionFailed': 'कनेक्शन विफल',
  'agents.serverError': 'सर्वर त्रुटि {{status}}',
  'agents.failedTo': '{{action}} विफल',
  'agents.failedToMcp': 'MCP सर्वर {{action}} विफल',
  'agents.failedTransport': 'ट्रांसपोर्ट अपडेट विफल',
  'agents.failedMcpTransport': 'MCP ट्रांसपोर्ट अपडेट विफल',
  'agents.claudeCode': 'Claude Code',
  'agents.claudeModels': 'Claude मॉडल',
  'agents.codexCli': 'Codex CLI',
  'agents.openaiModels': 'OpenAI मॉडल',
  'agents.opencode': 'OpenCode',
  'agents.opencodeDesc': '75+ LLM प्रदाता',
  'agents.copilot': 'GitHub Copilot',
  'agents.copilotDesc': 'GitHub Copilot मॉडल',
  'agents.mcpServer': 'MCP सर्वर',
  'agents.mcpServerStart': 'शुरू करें',
  'agents.mcpServerStop': 'रोकें',
  'agents.mcpServerRunning': 'चल रहा है',
  'agents.mcpServerStopped': 'रुका हुआ',
  'agents.mcpLanAccess': 'LAN एक्सेस',
  'agents.mcpClientConfig': 'क्लाइंट कॉन्फ़िग',
  'agents.stdio': 'stdio',
  'agents.http': 'http',
  'agents.stdioHttp': 'stdio + http',
  'agents.autoUpdate': 'स्वचालित अपडेट जाँच',
  'agents.notInstalled': 'स्थापित नहीं',
  'agents.install': 'स्थापित करें',
  'agents.installing': 'स्थापित हो रहा है...',
  'agents.installFailed': 'स्थापना विफल',
  'agents.viewDocs': 'दस्तावेज़',

  // ── Figma Import ──
  'figma.title': 'Figma से आयात करें',
  'figma.dropFile': 'यहाँ .fig फ़ाइल ड्रॉप करें',
  'figma.orBrowse': 'या ब्राउज़ करने के लिए क्लिक करें',
  'figma.exportTip': 'Figma से निर्यात: File \u2192 Save local copy (.fig)',
  'figma.selectFigFile': 'कृपया एक .fig फ़ाइल चुनें',
  'figma.noPages': '.fig फ़ाइल में कोई पेज नहीं मिला',
  'figma.parseFailed': '.fig फ़ाइल पार्स नहीं हो सकी',
  'figma.convertFailed': 'Figma फ़ाइल कन्वर्ट नहीं हो सकी',
  'figma.parsing': '.fig फ़ाइल पार्स हो रही है...',
  'figma.converting': 'नोड्स कन्वर्ट हो रहे हैं...',
  'figma.selectPage':
    'इस फ़ाइल में {{count}} पेज हैं। आयात के लिए चुनें:',
  'figma.layers': '{{count}} लेयर',
  'figma.importAll': 'सभी पेज आयात करें',
  'figma.importComplete': 'आयात पूरा हुआ!',
  'figma.moreWarnings': '...और {{count}} अन्य चेतावनियाँ',
  'figma.tryAgain': 'पुनः प्रयास करें',
  'figma.layoutMode': 'लेआउट मोड:',
  'figma.preserveLayout': 'Figma लेआउट संरक्षित करें',
  'figma.autoLayout': 'OpenPencil ऑटो लेआउट',
  'figma.comingSoon': 'जल्द आ रहा है',

  // ── Landing Page ──
  'landing.open': 'Open',
  'landing.pencil': 'Pencil',
  'landing.tagline': 'ओपन-सोर्स वेक्टर डिज़ाइन टूल। डिज़ाइन ऐज़ कोड।',
  'landing.newDesign': 'नया डिज़ाइन',
  'landing.shortcutHint': 'नया डिज़ाइन बनाने के लिए {{key1}} + {{key2}} दबाएँ',

  // ── 404 ──
  'notFound.message': 'पेज नहीं मिला',

  // ── Component Browser ──
  'componentBrowser.title': 'UIKit ब्राउज़र',
  'componentBrowser.exportKit': 'किट निर्यात करें',
  'componentBrowser.importKit': 'किट आयात करें',
  'componentBrowser.kit': 'किट:',
  'componentBrowser.all': 'सभी',
  'componentBrowser.imported': '(आयातित)',
  'componentBrowser.components': 'कंपोनेंट',
  'componentBrowser.searchComponents': 'कंपोनेंट खोजें...',
  'componentBrowser.deleteKit': '{{name}} हटाएँ',
  'componentBrowser.category.all': 'सभी',
  'componentBrowser.category.buttons': 'बटन',
  'componentBrowser.category.inputs': 'इनपुट',
  'componentBrowser.category.cards': 'कार्ड',
  'componentBrowser.category.nav': 'नेविगेशन',
  'componentBrowser.category.layout': 'लेआउट',
  'componentBrowser.category.feedback': 'फ़ीडबैक',
  'componentBrowser.category.data': 'डेटा',
  'componentBrowser.category.other': 'अन्य',

  // ── Variable Picker ──
  'variablePicker.boundTo': '--{{name}} से बाइंड',
  'variablePicker.bindToVariable': 'वेरिएबल से बाइंड करें',
  'variablePicker.unbind': 'वेरिएबल अनबाइंड करें',
  'variablePicker.noVariables': 'कोई {{type}} वेरिएबल परिभाषित नहीं है',
} as const

export default hi
