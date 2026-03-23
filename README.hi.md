<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>दुनिया का पहला ओपन-सोर्स AI-नेटिव वेक्टर डिज़ाइन टूल।</strong><br />
  <sub>समवर्ती एजेंट टीमें &bull; डिज़ाइन-एज़-कोड &bull; बिल्ट-इन MCP सर्वर &bull; मल्टी-मॉडल इंटेलिजेंस</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md"><b>हिन्दी</b></a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — डेमो देखने के लिए क्लिक करें" width="100%" />
  </a>
</p>
<p align="center"><sub>डेमो वीडियो देखने के लिए छवि पर क्लिक करें</sub></p>

<br />

## OpenPencil क्यों

<table>
<tr>
<td width="50%">

### 🎨 प्रॉम्प्ट → कैनवास

किसी भी UI का प्राकृतिक भाषा में वर्णन करें। स्ट्रीमिंग एनिमेशन के साथ रियल-टाइम में अनंत कैनवास पर प्रकट होते देखें। एलिमेंट चुनकर और चैट करके मौजूदा डिज़ाइन संशोधित करें।

</td>
<td width="50%">

### 🤖 समवर्ती एजेंट टीमें

ऑर्केस्ट्रेटर जटिल पेजों को स्थानिक सब-टास्क में विभाजित करता है। कई AI एजेंट एक साथ अलग-अलग सेक्शन पर काम करते हैं — हीरो, फ़ीचर, फ़ुटर — सभी समानांतर स्ट्रीमिंग करते हुए।

</td>
</tr>
<tr>
<td width="50%">

### 🧠 मल्टी-मॉडल इंटेलिजेंस

प्रत्येक मॉडल की क्षमताओं के अनुसार स्वचालित रूप से अनुकूलित होता है। Claude को थिंकिंग के साथ पूर्ण प्रॉम्प्ट मिलते हैं; GPT-4o/Gemini में थिंकिंग अक्षम होती है; छोटे मॉडल (MiniMax, Qwen, Llama) को विश्वसनीय आउटपुट के लिए सरलीकृत प्रॉम्प्ट मिलते हैं।

</td>
<td width="50%">

### 🔌 MCP सर्वर

Claude Code, Codex, Gemini, OpenCode, Kiro, या Copilot CLIs में वन-क्लिक इंस्टॉल। अपने टर्मिनल से डिज़ाइन करें — किसी भी MCP-संगत एजेंट के ज़रिए `.op` फ़ाइलें पढ़ें, बनाएँ और संशोधित करें।

</td>
</tr>
<tr>
<td width="50%">

### 📦 डिज़ाइन-एज़-कोड

`.op` फ़ाइलें JSON हैं — मानव-पठनीय, Git-फ्रेंडली, डिफ़ करने योग्य। डिज़ाइन वेरिएबल CSS कस्टम प्रॉपर्टीज़ जनरेट करते हैं। React + Tailwind या HTML + CSS में कोड एक्सपोर्ट।

</td>
<td width="50%">

### 🖥️ हर जगह चलता है

वेब ऐप + Electron के ज़रिए macOS, Windows और Linux पर नेटिव डेस्कटॉप। GitHub Releases से ऑटो-अपडेट। `.op` फ़ाइल एसोसिएशन — डबल-क्लिक से खोलें।

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

अपने टर्मिनल से डिज़ाइन टूल को नियंत्रित करें। `op design`, `op insert`, `op export` — बैच डिज़ाइन DSL, नोड मैनिपुलेशन, कोड एक्सपोर्ट। फ़ाइलों या stdin से पाइप करें। डेस्कटॉप ऐप या वेब सर्वर के साथ काम करता है।

</td>
<td width="50%">

### 🎯 मल्टी-प्लेटफ़ॉर्म कोड एक्सपोर्ट

एक `.op` फ़ाइल से React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native में एक्सपोर्ट करें। डिज़ाइन वेरिएबल CSS कस्टम प्रॉपर्टीज़ बन जाते हैं।

</td>
</tr>
</table>

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

### Docker

कई इमेज वेरिएंट उपलब्ध हैं — अपनी ज़रूरत के अनुसार चुनें:

| इमेज | आकार | शामिल |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | केवल वेब ऐप |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | सभी CLI टूल |

**चलाएँ (केवल वेब):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**AI CLI के साथ चलाएँ (उदा. Claude Code):**

AI चैट Claude CLI OAuth लॉगिन पर निर्भर करता है। लॉगिन सत्र को बनाए रखने के लिए Docker वॉल्यूम का उपयोग करें:

```bash
# चरण 1 — लॉगिन (एक बार)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# चरण 2 — शुरू करें
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**स्थानीय रूप से बिल्ड करें:**

```bash
# बेस (केवल वेब)
docker build --target base -t openpencil .

# किसी विशिष्ट CLI के साथ
docker build --target with-claude -t openpencil-claude .

# पूर्ण (सभी CLI)
docker build --target full -t openpencil-full .
```

## AI-नेटिव डिज़ाइन

**प्रॉम्प्ट से UI तक**
- **टेक्स्ट-टू-डिज़ाइन** — एक पेज का विवरण दें, और स्ट्रीमिंग एनिमेशन के साथ रियल-टाइम में कैनवास पर जनरेट करें
- **ऑर्केस्ट्रेटर** — जटिल पेजों को समानांतर जनरेशन के लिए स्थानिक सब-टास्क में विभाजित करता है
- **डिज़ाइन संशोधन** — एलिमेंट चुनें, फिर प्राकृतिक भाषा में बदलाव का विवरण दें
- **विज़न इनपुट** — संदर्भ-आधारित डिज़ाइन के लिए स्क्रीनशॉट या मॉकअप संलग्न करें

**मल्टी-एजेंट सपोर्ट**

| एजेंट | सेटअप |
| --- | --- |
| **Claude Code** | कोई कॉन्फ़िग नहीं — लोकल OAuth के साथ Claude Agent SDK का उपयोग करता है |
| **Codex CLI** | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |
| **OpenCode** | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` फिर एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |
| **Gemini CLI** | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`) |

**मॉडल क्षमता प्रोफ़ाइल** — प्रत्येक मॉडल टियर के अनुसार प्रॉम्प्ट, थिंकिंग मोड और टाइमआउट को स्वचालित रूप से अनुकूलित करता है। फुल-टियर मॉडल (Claude) को पूर्ण प्रॉम्प्ट मिलते हैं; स्टैंडर्ड-टियर (GPT-4o, Gemini, DeepSeek) में थिंकिंग अक्षम होती है; बेसिक-टियर (MiniMax, Qwen, Llama, Mistral) को अधिकतम विश्वसनीयता के लिए सरलीकृत नेस्टेड-JSON प्रॉम्प्ट मिलते हैं।

**MCP सर्वर**
- बिल्ट-इन MCP सर्वर — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs में वन-क्लिक इंस्टॉल
- Node.js स्वचालित पहचान — यदि इंस्टॉल नहीं है तो HTTP ट्रांसपोर्ट पर स्वचालित फ़ॉलबैक और MCP HTTP सर्वर ऑटो-स्टार्ट
- टर्मिनल से डिज़ाइन ऑटोमेशन: किसी भी MCP-संगत एजेंट के ज़रिए `.op` फ़ाइलें पढ़ें, बनाएँ और संपादित करें
- **लेयर्ड डिज़ाइन वर्कफ़्लो** — उच्च-फ़िडेलिटी मल्टी-सेक्शन डिज़ाइन के लिए `design_skeleton` → `design_content` → `design_refine`
- **सेगमेंटेड प्रॉम्प्ट रिट्रीवल** — केवल आवश्यक डिज़ाइन ज्ञान लोड करें (schema, layout, roles, icons, planning, आदि)
- मल्टी-पेज सपोर्ट — MCP टूल के ज़रिए पेज बनाएँ, नाम बदलें, क्रम बदलें और डुप्लिकेट करें

**कोड जनरेशन**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

वैश्विक रूप से इंस्टॉल करें और अपने टर्मिनल से डिज़ाइन टूल को नियंत्रित करें:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # डेस्कटॉप ऐप लॉन्च करें
op design @landing.txt       # फ़ाइल से बैच डिज़ाइन
op insert '{"type":"RECT"}'  # एक नोड डालें
op export react --out .      # React + Tailwind में एक्सपोर्ट
op import:figma design.fig   # Figma फ़ाइल इम्पोर्ट करें
cat design.dsl | op design - # stdin से पाइप करें
```

तीन इनपुट विधियाँ समर्थित हैं: इनलाइन स्ट्रिंग, `@filepath` (फ़ाइल से पढ़ें), या `-` (stdin से पढ़ें)। डेस्कटॉप ऐप या वेब डेव सर्वर के साथ काम करता है। पूर्ण कमांड संदर्भ के लिए [CLI README](./apps/cli/README.md) देखें।

## विशेषताएँ

**कैनवास और ड्रॉइंग**
- पैन, ज़ूम, स्मार्ट अलाइनमेंट गाइड और स्नैपिंग के साथ अनंत कैनवास
- Rectangle, Ellipse, Line, Polygon, Pen (Bezier), Frame, Text
- बूलियन ऑपरेशन — संयोजन, घटाना, प्रतिच्छेदन संदर्भ टूलबार के साथ
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
- `.op` फ़ाइल एसोसिएशन — डबल-क्लिक से खोलें, सिंगल-इंस्टेंस लॉक
- GitHub Releases से ऑटो-अपडेट
- नेटिव एप्लिकेशन मेनू और फ़ाइल डायलॉग

## टेक स्टैक

| | |
| --- | --- |
| **फ्रंटएंड** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **कैनवास** | CanvasKit/Skia (WASM, GPU-एक्सेलेरेटेड) |
| **स्टेट** | Zustand v5 |
| **सर्वर** | Nitro |
| **डेस्कटॉप** | Electron 35 |
| **CLI** | `op` — टर्मिनल नियंत्रण, बैच डिज़ाइन DSL, कोड एक्सपोर्ट |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **रनटाइम** | Bun · Vite 7 |
| **फ़ाइल फ़ॉर्मेट** | `.op` — JSON-आधारित, मानव-पठनीय, Git-फ्रेंडली |

## प्रोजेक्ट संरचना

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start वेब ऐप
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia इंजन — ड्रॉइंग, सिंक, लेआउट
│   │   │   ├── components/  React UI — एडिटर, पैनल, शेयर्ड डायलॉग, आइकन
│   │   │   ├── services/ai/ AI चैट, ऑर्केस्ट्रेटर, डिज़ाइन जनरेशन, स्ट्रीमिंग
│   │   │   ├── stores/      Zustand — कैनवास, दस्तावेज़, पेज, हिस्ट्री, AI
│   │   │   ├── mcp/         बाहरी CLI इंटीग्रेशन के लिए MCP सर्वर टूल
│   │   │   ├── hooks/       कीबोर्ड शॉर्टकट, फ़ाइल ड्रॉप, Figma पेस्ट
│   │   │   └── uikit/       पुन: उपयोगी कम्पोनेंट किट सिस्टम
│   │   └── server/
│   │       ├── api/ai/      Nitro API — स्ट्रीमिंग चैट, जनरेशन, वैलिडेशन
│   │       └── utils/       Claude CLI, OpenCode, Codex, Copilot रैपर
│   ├── desktop/             Electron डेस्कटॉप ऐप
│   │   ├── main.ts          विंडो, Nitro फ़ोर्क, नेटिव मेनू, ऑटो-अपडेटर
│   │   ├── ipc-handlers.ts  नेटिव फ़ाइल डायलॉग, थीम सिंक, प्राथमिकताएँ IPC
│   │   └── preload.ts       IPC ब्रिज
│   └── cli/                 CLI टूल — `op` कमांड
│       ├── src/commands/    डिज़ाइन, दस्तावेज़, एक्सपोर्ट, इम्पोर्ट, नोड, पेज, वेरिएबल कमांड
│       ├── connection.ts    चालू ऐप से WebSocket कनेक्शन
│       └── launcher.ts      डेस्कटॉप ऐप या वेब सर्वर का स्वचालित पता लगाना और लॉन्च
├── packages/
│   ├── pen-types/           PenDocument मॉडल के लिए टाइप परिभाषाएँ
│   ├── pen-core/            दस्तावेज़ ट्री ऑपरेशन, लेआउट इंजन, वेरिएबल
│   ├── pen-codegen/         कोड जनरेटर (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Figma .fig फ़ाइल पार्सर और कनवर्टर
│   ├── pen-renderer/        स्टैंडअलोन CanvasKit/Skia रेंडरर
│   └── pen-sdk/             अम्ब्रेला SDK (सभी पैकेज री-एक्सपोर्ट)
└── .githooks/               ब्रांच नाम से प्री-कमिट वर्शन सिंक
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
| `Cmd+Alt+U` | बूलियन संयोजन | | `Cmd+Alt+S` | बूलियन घटाना |
| `Cmd+Alt+I` | बूलियन प्रतिच्छेदन | | | |

## स्क्रिप्ट

```bash
bun --bun run dev          # डेव सर्वर (पोर्ट 3000)
bun --bun run build        # प्रोडक्शन बिल्ड
bun --bun run test         # टेस्ट चलाएँ (Vitest)
npx tsc --noEmit           # टाइप चेक
bun run bump <version>     # सभी package.json में वर्शन सिंक करें
bun run electron:dev       # Electron डेव
bun run electron:build     # Electron पैकेज
bun run cli:dev            # सोर्स से CLI चलाएँ
bun run cli:compile        # CLI को dist में कंपाइल करें
```

## योगदान

योगदान का स्वागत है! आर्किटेक्चर विवरण और कोड स्टाइल के लिए [CLAUDE.md](./CLAUDE.md) देखें।

1. फ़ोर्क और क्लोन करें
2. वर्शन सिंक सेटअप करें: `git config core.hooksPath .githooks`
3. ब्रांच बनाएँ: `git checkout -b feat/my-feature`
4. चेक चलाएँ: `npx tsc --noEmit && bun --bun run test`
5. [Conventional Commits](https://www.conventionalcommits.org/) के साथ कमिट करें: `feat(canvas): add rotation snapping`
6. `main` के विरुद्ध PR खोलें

## रोडमैप

- [x] CSS सिंक के साथ डिज़ाइन वेरिएबल और टोकन
- [x] कम्पोनेंट सिस्टम (इंस्टेंस और ओवरराइड)
- [x] ऑर्केस्ट्रेटर के साथ AI डिज़ाइन जनरेशन
- [x] लेयर्ड डिज़ाइन वर्कफ़्लो के साथ MCP सर्वर इंटीग्रेशन
- [x] मल्टी-पेज सपोर्ट
- [x] Figma `.fig` इम्पोर्ट
- [x] बूलियन ऑपरेशन (यूनियन, सबट्रैक्ट, इंटरसेक्ट)
- [x] मल्टी-मॉडल क्षमता प्रोफ़ाइल
- [x] पुन: उपयोगी पैकेज के साथ मोनोरेपो पुनर्गठन
- [x] CLI टूल (`op`) टर्मिनल नियंत्रण
- [ ] सहयोगी संपादन
- [ ] प्लगइन सिस्टम

## योगदानकर्ता

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## समुदाय

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> हमारे Discord में शामिल हों</strong>
</a>
— प्रश्न पूछें, डिज़ाइन साझा करें, सुविधाएँ सुझाएँ।

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## लाइसेंस

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
