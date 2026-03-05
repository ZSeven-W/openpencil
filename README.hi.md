<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI-नेटिव ओपन-सोर्स डिज़ाइन टूल। डिज़ाइन-एज़-कोड।</strong><br />
  प्रॉम्प्ट से कैनवास UI तक। मल्टी-एजेंट ऑर्केस्ट्रेशन। बिल्ट-इन MCP सर्वर। कोड जनरेशन।
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md"><b>हिन्दी</b></a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">त्वरित शुरुआत</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">विशेषताएँ</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">योगदान</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>डेमो वीडियो देखने के लिए छवि पर क्लिक करें</sub></p>

<br />

## त्वरित शुरुआत

```bash
# निर्भरताएँ इंस्टॉल करें
bun install

# http://localhost:3000 पर डेव सर्वर शुरू करें
bun --bun run dev
```

या डेस्कटॉप ऐप के रूप में चलाएँ:

```bash
bun run electron:dev
```

> **पूर्वापेक्षाएँ:** [Bun](https://bun.sh/) >= 1.0 और [Node.js](https://nodejs.org/) >= 18

## AI-नेटिव डिज़ाइन

OpenPencil को AI के इर्द-गिर्द शुरू से बनाया गया है — एक प्लगइन के रूप में नहीं, बल्कि एक मुख्य वर्कफ़्लो के रूप में।

**प्रॉम्प्ट से UI तक**
- **टेक्स्ट-टू-डिज़ाइन** — एक पेज का विवरण दें, और स्ट्रीमिंग एनिमेशन के साथ रियल-टाइम में कैनवास पर जनरेट करें
- **ऑर्केस्ट्रेटर** — जटिल पेजों को स्थानिक सब-टास्क में विभाजित करता है, समानांतर जनरेशन के लिए
- **डिज़ाइन संशोधन** — एलिमेंट चुनें, फिर प्राकृतिक भाषा में बदलाव का विवरण दें
- **विज़न इनपुट** — संदर्भ-आधारित डिज़ाइन के लिए स्क्रीनशॉट या मॉकअप संलग्न करें

**मल्टी-एजेंट सपोर्ट**

| एजेंट | सेटअप |
| --- | --- |
| **Claude Code** | कोई कॉन्फ़िग नहीं — लोकल OAuth के साथ Claude Agent SDK का उपयोग करता है |
| **Codex CLI** | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |
| **OpenCode** | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |

**MCP सर्वर**
- बिल्ट-इन MCP सर्वर — Claude Code / Codex / Gemini / OpenCode / Kiro CLIs में वन-क्लिक इंस्टॉल
- टर्मिनल से डिज़ाइन ऑटोमेशन: किसी भी MCP-संगत एजेंट के ज़रिए `.op` फ़ाइलें पढ़ें, बनाएँ और संपादित करें

**कोड जनरेशन**
- React + Tailwind CSS
- HTML + CSS
- डिज़ाइन टोकन से CSS Variables

## विशेषताएँ

**कैनवास और ड्रॉइंग**
- पैन, ज़ूम, स्मार्ट अलाइनमेंट गाइड और स्नैपिंग के साथ अनंत कैनवास
- Rectangle, Ellipse, Line, Polygon, Pen (Bezier), Frame, Text
- आइकन पिकर (Iconify) और इमेज इम्पोर्ट (PNG/JPEG/SVG/WebP/GIF)
- ऑटो-लेआउट — gap, padding, justify, align के साथ वर्टिकल/हॉरिज़ॉन्टल
- टैब नेवीगेशन के साथ मल्टी-पेज दस्तावेज़

**डिज़ाइन सिस्टम**
- डिज़ाइन वेरिएबल — `$variable` रेफ़रेंस के साथ कलर, नंबर, स्ट्रिंग टोकन
- मल्टी-थीम सपोर्ट — कई अक्ष, प्रत्येक में वेरिएंट (Light/Dark, Compact/Comfortable)
- कम्पोनेंट सिस्टम — इंस्टेंस और ओवरराइड के साथ पुन: उपयोगी कम्पोनेंट
- CSS सिंक — स्वतः-जनरेटेड कस्टम प्रॉपर्टीज़, कोड आउटपुट में `var(--name)`

**Figma इम्पोर्ट**
- लेआउट, फ़िल, स्ट्रोक, इफ़ेक्ट, टेक्स्ट, इमेज और वेक्टर को सुरक्षित रखते हुए `.fig` फ़ाइलें इम्पोर्ट करें

**डेस्कटॉप ऐप**
- Electron के ज़रिए नेटिव macOS, Windows और Linux सपोर्ट
- GitHub Releases से ऑटो-अपडेट
- नेटिव एप्लिकेशन मेनू और फ़ाइल डायलॉग

## टेक स्टैक

| | |
| --- | --- |
| **फ्रंटएंड** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **कैनवास** | Fabric.js v7 |
| **स्टेट** | Zustand v5 |
| **सर्वर** | Nitro |
| **डेस्कटॉप** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **रनटाइम** | Bun · Vite 7 |
| **फ़ाइल फ़ॉर्मेट** | `.op` — JSON-आधारित, मानव-पठनीय, Git-फ्रेंडली |

## प्रोजेक्ट संरचना

```text
src/
  canvas/          Fabric.js इंजन — ड्रॉइंग, सिंक, लेआउट, गाइड, पेन टूल
  components/      React UI — एडिटर, पैनल, शेयर्ड डायलॉग, आइकन
  services/ai/     AI चैट, ऑर्केस्ट्रेटर, डिज़ाइन जनरेशन, स्ट्रीमिंग
  services/figma/  Figma .fig बाइनरी इम्पोर्ट पाइपलाइन
  services/codegen React+Tailwind और HTML+CSS कोड जनरेटर
  stores/          Zustand — कैनवास, दस्तावेज़, पेज, हिस्ट्री, AI, सेटिंग्स
  variables/       डिज़ाइन टोकन रिज़ॉल्यूशन और रेफ़रेंस मैनेजमेंट
  mcp/             बाहरी CLI इंटीग्रेशन के लिए MCP सर्वर टूल
  uikit/           पुन: उपयोगी कम्पोनेंट किट सिस्टम
server/
  api/ai/          Nitro API — स्ट्रीमिंग चैट, जनरेशन, वैलिडेशन
  utils/           Claude CLI, OpenCode, Codex क्लाइंट रैपर
electron/
  main.ts          विंडो, Nitro फ़ोर्क, नेटिव मेनू, ऑटो-अपडेटर
  preload.ts       IPC ब्रिज
```

## कीबोर्ड शॉर्टकट

| कुंजी | क्रिया | | कुंजी | क्रिया |
| --- | --- | --- | --- | --- |
| `V` | चुनें | | `Cmd+S` | सहेजें |
| `R` | Rectangle | | `Cmd+Z` | पूर्ववत करें |
| `O` | Ellipse | | `Cmd+Shift+Z` | फिर से करें |
| `L` | Line | | `Cmd+C/X/V/D` | कॉपी/कट/पेस्ट/डुप्लिकेट |
| `T` | Text | | `Cmd+G` | ग्रुप करें |
| `F` | Frame | | `Cmd+Shift+G` | अनग्रुप करें |
| `P` | Pen tool | | `Cmd+Shift+E` | एक्सपोर्ट |
| `H` | Hand (pan) | | `Cmd+Shift+C` | कोड पैनल |
| `Del` | हटाएँ | | `Cmd+Shift+V` | वेरिएबल पैनल |
| `[ / ]` | क्रम बदलें | | `Cmd+J` | AI चैट |
| Arrows | 1px नज | | `Cmd+,` | एजेंट सेटिंग्स |

## स्क्रिप्ट

```bash
bun --bun run dev          # डेव सर्वर (पोर्ट 3000)
bun --bun run build        # प्रोडक्शन बिल्ड
bun --bun run test         # टेस्ट चलाएँ (Vitest)
npx tsc --noEmit           # टाइप चेक
bun run electron:dev       # Electron डेव
bun run electron:build     # Electron पैकेज
```

## योगदान

योगदान का स्वागत है! आर्किटेक्चर विवरण और कोड स्टाइल के लिए [CLAUDE.md](./CLAUDE.md) देखें।

1. फ़ोर्क और क्लोन करें
2. ब्रांच बनाएँ: `git checkout -b feat/my-feature`
3. चेक चलाएँ: `npx tsc --noEmit && bun --bun run test`
4. [Conventional Commits](https://www.conventionalcommits.org/) के साथ कमिट करें: `feat(canvas): add rotation snapping`
5. `main` के विरुद्ध PR खोलें

## रोडमैप

- [x] CSS सिंक के साथ डिज़ाइन वेरिएबल और टोकन
- [x] कम्पोनेंट सिस्टम (इंस्टेंस और ओवरराइड)
- [x] ऑर्केस्ट्रेटर के साथ AI डिज़ाइन जनरेशन
- [x] MCP सर्वर इंटीग्रेशन
- [x] मल्टी-पेज सपोर्ट
- [x] Figma `.fig` इम्पोर्ट
- [ ] बूलियन ऑपरेशन (यूनियन, सबट्रैक्ट, इंटरसेक्ट)
- [ ] सहयोगी संपादन
- [ ] प्लगइन सिस्टम

## योगदानकर्ता

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## समुदाय

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> हमारे Discord में शामिल हों</strong>
</a>
— प्रश्न पूछें, डिज़ाइन साझा करें, सुविधाएँ सुझाएँ।

## लाइसेंस

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
