<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — डेमो देखने के लिए क्लिक करें" width="100%" />
  </a>
</p>
<p align="center"><sub>डेमो वीडियो देखने के लिए छवि पर क्लिक करें</sub></p>

<br />

> **नोट:** इसी नाम का एक और ओपन-सोर्स प्रोजेक्ट है — [OpenPencil](https://github.com/open-pencil/open-pencil), जो रियल-टाइम सहयोग के साथ Figma-संगत विज़ुअल डिज़ाइन पर केंद्रित है। यह प्रोजेक्ट AI-नेटिव डिज़ाइन-टू-कोड वर्कफ़्लो पर केंद्रित है।

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

वेब ऐप + macOS, Windows और Linux पर नेटिव डेस्कटॉप — एक Rust कोर, एक एकल स्व-निहित बाइनरी, कोई ब्राउज़र इंजन नहीं। `.op` फ़ाइल एसोसिएशन — डबल-क्लिक से खोलें।

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

अपने टर्मिनल से डिज़ाइन टूल को नियंत्रित करें। `op design`, `op insert` — बैच डिज़ाइन DSL, नोड मैनिपुलेशन। फ़ाइलों या stdin से पाइप करें। डेस्कटॉप ऐप या वेब सर्वर के साथ काम करता है।

</td>
<td width="50%">

### 🎯 मल्टी-प्लेटफ़ॉर्म कोड एक्सपोर्ट

एक `.op` फ़ाइल से React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native में एक्सपोर्ट करें। डिज़ाइन वेरिएबल CSS कस्टम प्रॉपर्टीज़ बन जाते हैं।

</td>
</tr>
</table>

## त्वरित शुरुआत

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

या डेस्कटॉप ऐप के रूप में चलाएँ:

```bash
cargo run -p op-host-desktop
```

> **पूर्वापेक्षाएँ:** उत्पाद बनाने के लिए [Rust](https://www.rust-lang.org/) (stable)। [Bun](https://bun.sh/) >= 1.0 और [Node.js](https://nodejs.org/) >= 18 केवल `packages/` के अंतर्गत web SDK के लिए आवश्यक हैं।

### Docker

टैग किए गए Rust releases एक ही web host image प्रकाशित करते हैं। AI CLI के साथ पुराने TypeScript images अब प्रकाशित नहीं किए जाते।

| इमेज | शामिल |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host, wasm bundle और CanvasKit assets |

Web UI केवल built-in agent profiles दिखाता है; Claude/Codex/OpenCode/Copilot/Gemini CLI tools Docker images में शामिल नहीं हैं।

**चलाएँ:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

फिर `http://localhost:3100/` खोलें।

**स्थानीय रूप से बिल्ड करें:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI-नेटिव डिज़ाइन

**प्रॉम्प्ट से UI तक**

- **टेक्स्ट-टू-डिज़ाइन** — एक पेज का विवरण दें, और स्ट्रीमिंग एनिमेशन के साथ रियल-टाइम में कैनवास पर जनरेट करें
- **ऑर्केस्ट्रेटर** — जटिल पेजों को समानांतर जनरेशन के लिए स्थानिक सब-टास्क में विभाजित करता है
- **डिज़ाइन संशोधन** — एलिमेंट चुनें, फिर प्राकृतिक भाषा में बदलाव का विवरण दें
- **विज़न इनपुट** — संदर्भ-आधारित डिज़ाइन के लिए स्क्रीनशॉट या मॉकअप संलग्न करें

**मल्टी-एजेंट सपोर्ट**

| एजेंट                     | सेटअप                                                                                        |
| ------------------------- | -------------------------------------------------------------------------------------------- |
| **बिल्ट-इन (9+ प्रदाता)** | प्रदाता प्रीसेट से चुनें और क्षेत्र स्विच करें — Anthropic, OpenAI, Google, DeepSeek और अन्य |
| **Claude Code**           | कोई कॉन्फ़िग नहीं — लोकल OAuth के साथ Claude Agent SDK का उपयोग करता है                      |
| **Codex CLI**             | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`)                                                     |
| **OpenCode**              | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`)                                                     |
| **GitHub Copilot**        | `copilot login` फिर एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`)                                 |
| **Gemini CLI**            | एजेंट सेटिंग्स में कनेक्ट करें (`Cmd+,`)                                                     |

**मॉडल क्षमता प्रोफ़ाइल** — प्रत्येक मॉडल टियर के अनुसार प्रॉम्प्ट, थिंकिंग मोड और टाइमआउट को स्वचालित रूप से अनुकूलित करता है। फुल-टियर मॉडल (Claude) को पूर्ण प्रॉम्प्ट मिलते हैं; स्टैंडर्ड-टियर (GPT-4o, Gemini, DeepSeek) में थिंकिंग अक्षम होती है; बेसिक-टियर (MiniMax, Qwen, Llama, Mistral) को अधिकतम विश्वसनीयता के लिए सरलीकृत नेस्टेड-JSON प्रॉम्प्ट मिलते हैं।

**i18n** — 15 भाषाओं में पूर्ण इंटरफ़ेस स्थानीयकरण: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia।

**MCP सर्वर**

- बिल्ट-इन MCP सर्वर (`op-mcp` crate) — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs में वन-क्लिक इंस्टॉल
- Node.js की कोई आवश्यकता नहीं — डेस्कटॉप बाइनरी (`--mcp <path>`) के ज़रिए stdio ट्रांसपोर्ट, साथ ही चल रहे ऐप से एक लाइव HTTP एंडपॉइंट (`127.0.0.1:<port>/mcp`)
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # डेस्कटॉप ऐप लॉन्च करें
op start --headless --file design.op # हेडलेस सर्वर चलाएँ
op design @landing.txt       # फ़ाइल से बैच डिज़ाइन
op design @ui.js             # लूप के साथ सैंडबॉक्स JavaScript
op insert '{"type":"rectangle"}' # एक नोड डालें
op import:figma design.fig   # Figma फ़ाइल इम्पोर्ट करें
cat design.dsl | op design - # stdin से पाइप करें
```

इनलाइन स्ट्रिंग, `@filepath` और stdin (`-`) समर्थित हैं। यह डेस्कटॉप ऐप, वेब सर्वर या फ़ाइल-आधारित हेडलेस सर्वर के साथ काम करता है। सभी कमांड के लिए [CLI कमांड संदर्भ](./crates/op-cli/src/usage.txt) देखें।

**LLM स्किल** — AI एजेंट को `op` से डिज़ाइन सिखाने के लिए [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) प्लगइन इंस्टॉल करें। पहचाने गए एजेंट के लिए `op install` या किसी खास लक्ष्य के लिए `op install --target codex` चलाएँ।

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
- पुन: उपयोगी UIKits — `.pen` फ़ाइलों से कम्पोनेंट किट इम्पोर्ट/एक्सपोर्ट करें

**AI और एजेंट**

- स्ट्रीमिंग जनरेशन और ऑर्केस्ट्रेटर-संचालित स्थानिक विभाजन के साथ प्रॉम्प्ट-टू-कैनवास
- समवर्ती एजेंट टीमें — कई डिज़ाइनर अलग-अलग सेक्शनों पर समानांतर में काम करते हैं, प्रति-सदस्य कैनवास संकेतकों के साथ
- लेयर्ड वर्कफ़्लो — `design_skeleton` → `design_content` → `design_refine`, प्रत्येक चरण के लिए केंद्रित प्रॉम्प्ट
- स्टाइल गाइड — 50+ इन-बिल्ट स्टाइल (glassmorphism, brutalist, retro आदि), टैग-आधारित फ़ज़ी मैचिंग, प्लानिंग और जनरेशन में एकीकृत
- मल्टी-मॉडल क्षमता प्रोफ़ाइल — मॉडल स्तर के अनुसार थिंकिंग मोड, प्रयास और प्रॉम्प्ट रूप को स्वचालित रूप से अनुकूलित करता है
- बिल्ट-इन एजेंट रनटाइम (Rust) + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini प्रदाता
- चीनी LLM प्रदाताओं के लिए Anthropic फ़ॉर्मेट पासथ्रू — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Git इंटीग्रेशन**

- SSH / HTTPS प्रमाणीकरण और SSH कुंजी प्रबंधन के साथ क्लोन विज़ार्ड
- ब्रांच पिकर — बनाना, स्विच करना, हटाना, मर्ज — सब कुछ Git पैनल से
- प्रमाणीकरण रीट्राई और non-fast-forward हैंडलिंग के साथ पुल / पुश कैस्केड
- डिस्क पर `MERGE_HEAD` स्टेट ट्रैकिंग के साथ फ़ोल्डर-मोड थ्री-वे मर्ज
- प्रति-नोड / प्रति-फ़ील्ड थ्री-वे कार्ड, इनलाइन JSON एडिटर, बल्क एक्शन और इनलाइन diff ब्लॉक के साथ कॉन्फ्लिक्ट पैनल
- रिमोट सेटिंग्स और SSH कीज़ UI; संपूर्ण Git सतह पर 15 भाषाओं में i18n

**एक्सपोर्ट**

- कैनवास एक्सपोर्ट — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- कोड एक्सपोर्ट — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- इन्क्रीमेंटल MCP कोडजेन पाइपलाइन — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Figma इम्पोर्ट**

- लेआउट, फ़िल, स्ट्रोक, इफ़ेक्ट, टेक्स्ट, इमेज और वेक्टर को सुरक्षित रखते हुए `.fig` फ़ाइलें इम्पोर्ट करें

**डेस्कटॉप ऐप**

- नेटिव macOS, Windows और Linux सपोर्ट — एक एकल स्व-निहित बाइनरी (winit + GPU Skia, कोई Electron नहीं)
- `.op` फ़ाइल एसोसिएशन — डबल-क्लिक से खोलें, सिंगल-इंस्टेंस लॉक
- GitHub Releases के विरुद्ध बैकग्राउंड में अपडेट जांच
- इस रूप में सहेजें, हाल के खोलें और बंद करते समय असहेजे परिवर्तनों के डायलॉग वाला नेटिव एप्लिकेशन मेनू
- हाल की फ़ाइलों का पर्सिस्टेंस

## टेक स्टैक

|                    |                                                                                  |
| ------------------ | -------------------------------------------------------------------------------- |
| **कोर**            | Rust वर्कस्पेस (`crates/`) — एडिटर स्टेट, विजेट्स, होस्ट्स, MCP, AI, codegen     |
| **रेंडरिंग**       | हर जगह GPU Skia — नेटिव पर `skia-safe` (GL), ब्राउज़र में CanvasKit (WASM/WebGL2) |
| **UI फ़्रेमवर्क**  | jian — वेंडर्ड शुद्ध-Rust GPU-Skia UI फ़्रेमवर्क: विजेट्स, लेआउट, इवेंट, हॉट रीलोड (`vendor/jian`) |
| **विंडोइंग**       | winit (वेंडर्ड `casement` फ़ोर्क)                                                |
| **डेस्कटॉप**       | नेटिव बाइनरी `openpencil-desktop` — कोई ब्राउज़र इंजन नहीं                       |
| **वेब SDK**        | `op-web-sdk` + React 19 / Vue 3 एडाप्टर — रीड-ओनली `.op` व्यूअर (TypeScript)     |
| **CLI**            | `op` — टर्मिनल नियंत्रण, बैच डिज़ाइन DSL                                         |
| **AI**             | बिल्ट-इन Rust एजेंट रनटाइम · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**           | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                               |
| **फ़ाइल फ़ॉर्मेट** | `.op` — JSON-आधारित, मानव-पठनीय, Git-फ्रेंडली                                    |

## पारिस्थितिकी तंत्र

OpenPencil, **[ZSeven-W](https://github.com/ZSeven-W)** के शुद्ध-Rust, AI-नेटिव टूल्स के परिवार का हिस्सा है। ये आपस में जुड़ते हैं: `jian` OpenPencil को रेंडर करता है, `agent-rs` इसके एजेंट चलाता है, `noema` याद रखता है, और `zode` टर्मिनल से डिज़ाइन करता है।

| प्रोजेक्ट | यह क्या है |
| --------- | ---------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | आपके टर्मिनल के लिए ओपन-सोर्स, AI-नेटिव कोडिंग असिस्टेंट — एक तेज़ Rust TUI (`ratatui`) जो आपका कोड पढ़ता है, कमांड चलाता है, फ़ाइलें खोजता है और git प्रबंधित करता है। MCP के ज़रिए OpenPencil को चलाता है। |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM एजेंट शिप करने के लिए एक शुद्ध-Rust async रनटाइम — मल्टी-प्रोवाइडर, एंड-टू-एंड टूल-सक्षम, संरचित अनुमतियाँ, वास्तविक MCP, शून्य `unsafe`। OpenPencil के बिल्ट-इन एजेंट रनटाइम (`vendor/agent`) और Zode को शक्ति देता है। |
| **[jian](https://github.com/ZSeven-W/jian)** | शुद्ध-Rust, GPU-Skia UI फ़्रेमवर्क — एक ही स्टैक में विजेट्स, लेआउट, इवेंट और हॉट रीलोड। एक घोषणात्मक `.op` दस्तावेज़ को बिना JS रनटाइम, बिना DOM, बिना Electron के एक नेटिव, AI-नियंत्रणीय ऐप में बदल देता है। OpenPencil का UI फ़्रेमवर्क (`vendor/jian`)। |
| **[noema](https://github.com/ZSeven-W/noema)** | कोडिंग एजेंट्स के लिए लोकल-फ़र्स्ट, नॉन-वेक्टर मेमोरी सिस्टम। निरीक्षण-योग्य फ़ाइलों के रूप में टिकाऊ मेमोरी, नई प्रविष्टियों के लिए एक समीक्षा कतार, और लेक्सिकल (एम्बेडिंग-मुक्त) रिकॉल — Zode, Codex, Claude Code और MCP रनटाइम में काम करता है। |

## Rust क्यों

OpenPencil को पूरी तरह **Rust** में नए सिरे से लिखा गया है ([#129](https://github.com/ZSeven-W/openpencil/issues/129))। रीराइट पूरा हो चुका है — TypeScript + Electron एडिटर को `v0.7.5` पर रिटायर कर दिया गया, और इस रिपॉज़िटरी का Rust वर्कस्पेस अब स्वयं प्रोडक्ट है: एक नेटिव कोर जो काफ़ी छोटा और तेज़ है, और एक ही कोडबेस से अधिक प्लेटफ़ॉर्म पर चलता है।

|                         | TypeScript + Electron (रिटायर, `v0.7.5`)         | Rust (आज)                                                             |
| ----------------------- | ----------------------------------------------- | -------------------------------------------------------------------- |
| **डेस्कटॉप रनटाइम**    | Electron — Chromium + Node.js बंडल करता है      | नेटिव विंडो (`winit` + GPU Skia), कोई ब्राउज़र इंजन नहीं            |
| **डेस्कटॉप फ़ुटप्रिंट** | प्रति इंस्टॉल पूरा Chromium रनटाइम              | एकल स्व-निहित बाइनरी — **55.5 MB**                                  |
| **वेब पेलोड**           | JS + WASM बंडल                                  | **8.2 MB** wasm / **2.18 MB** gzip वायर पर                          |
| **रेंडरिंग**            | CanvasKit/Skia वेब पर                           | **हर** टार्गेट पर एक GPU-एक्सेलेरेटेड Skia बैकएंड                   |
| **मेमोरी**              | JavaScript GC पॉज़                              | कोई GC नहीं — Rust ओनरशिप, अनुमानित लेटेंसी                         |
| **कोडबेस**              | वेब स्टैक + Electron           | एक Rust वर्कस्पेस: editor · CLI · MCP · AI · codegen · Figma · Git   |
| **टार्गेट**             | Web + desktop, दो अलग स्टैक                     | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · browser — एक कोर |

**मापे गए सुधार**

- **छोटा फ़ुटप्रिंट** — पूरा डेस्कटॉप ऐप एक **55.5 MB** नेटिव बाइनरी है, न कि बंडल किया हुआ ब्राउज़र इंजन और Node रनटाइम। आइकन कैटलॉग विभाजन के बाद वेब बिल्ड **8.2 MB** रॉ / **2.18 MB** gzip है (वायर पर −48%)।
- **बड़े दस्तावेज़ों में स्केल** — **10,000-node** लाइव कैनवास (नेस्टेड ऑटो-लेआउट, चार स्तर गहरा) बिना किसी पैनिक और **~0% आइडल CPU** के लिखता, पढ़ता और लेआउट स्नैपशॉट करता है; सभी 10k नोड्स का पूर्ण लेआउट स्नैपशॉट **~0.68 s** में वापस आता है।
- **तेज़ इंटरैक्शन** — pan/zoom अब हर फ़्रेम में दस्तावेज़ को री-सीरियलाइज़ नहीं करता (एकल हॉट-पाथ फ़िक्स ने व्हील-ज़ूम CPU को **~69% से ~0%** तक घटा दिया); ड्रैग दृश्य को इन्क्रीमेंटली पैच करते हैं, टेक्स्ट मेज़रमेंट कैश्ड है, और रिपेंट प्रति फ़्रेम एक में संयोजित होते हैं।
- **एक कोर, हर स्क्रीन** — वही एडिटर स्टेट और वही रेंडर बैकएंड नेटिव डेस्कटॉप, मोबाइल और WASM के ज़रिए ब्राउज़र में कंपाइल होते हैं — सिंक में रखने के लिए कोई समानांतर पुनर्कार्यान्वयन नहीं।
- **हर जगह GPU Skia** — नेटिव GL कॉन्टेक्स्ट पर `skia-safe` के ज़रिए रेंडर करता है; ब्राउज़र WebGL2 पर CanvasKit के ज़रिए रेंडर करता है — वही ड्रॉइंग कोड, वही आउटपुट।
- **नेटिव एक्सेसिबिलिटी** — macOS, Windows और Linux पर AccessKit, साथ ही वेब पर DOM मिरर — ब्राउज़र के a11y ट्री पर निर्भर रहने के बजाय।
- **एक टाइप-चेक्ड वर्कस्पेस** — MCP होस्ट, CLI, AI प्रदाता, कोड जनरेशन, Figma इम्पोर्ट और Git इंटीग्रेशन सभी एक ही Rust वर्कस्पेस में रहते हैं, CI में `cargo-deny` सप्लाई-चेन गेटिंग के साथ।

> **स्थिति:** TypeScript एडिटर को `v0.7.5` पर रिटायर कर दिया गया था और अब यह केवल git इतिहास में मौजूद है; यह रिपॉज़िटरी अब Rust वर्कस्पेस है। Rust प्रोडक्ट सक्रिय विकास में है (नीचे रोडमैप देखें)।

## प्रोजेक्ट संरचना

```text
openpencil/
├── crates/                   Rust वर्कस्पेस — प्रोडक्ट
│   ├── op-editor-core/       कैननिकल `.op` (PenDocument) एडिटर स्टेट + EditorCommand + डिज़ाइन वेरिएबल
│   ├── op-editor-ui/         प्लेटफ़ॉर्म-मुक्त विजेट्स + RenderBackend फ़साड (wasm32-clean)
│   ├── op-editor-host-core/  सभी होस्ट्स द्वारा साझा ट्रांसपोर्ट-मुक्त होस्ट स्टेट मशीनें
│   ├── op-host-native/       नेटिव होस्ट लाइब्रेरी — winit + skia-safe GL (डेस्कटॉप + मोबाइल)
│   ├── op-host-web/          ब्राउज़र बंडल — wasm32 cdylib, CanvasKit रेंडरर
│   ├── op-host-desktop/      डेस्कटॉप बाइनरी `openpencil-desktop`; `--serve-web` डेमॉन भी
│   ├── op-host-services/     हेडलेस serve-web / MCP डेमॉन लाइब्रेरी
│   ├── op-host-web-server/   पतली GL-मुक्त वेब-सर्वर बाइनरी
│   ├── op-cli/               CLI टूल — `op` कमांड
│   ├── op-mcp/               MCP सर्वर — टूल्स, बैच डिज़ाइन, लेयर्ड वर्कफ़्लो
│   ├── op-ai/                AI प्रदाता, चैट रनटाइम, स्ट्रीमिंग
│   ├── op-ai-skills/         AI प्रॉम्प्ट स्किल इंजन (चरणबद्ध प्रॉम्प्ट लोडिंग)
│   ├── op-orchestrator/      समवर्ती एजेंट-टीम ऑर्केस्ट्रेशन
│   ├── op-codegen/           कोड जनरेटर (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma .fig फ़ाइल पार्सर और कनवर्टर
│   ├── op-git/               Git इंटीग्रेशन — क्लोन, ब्रांच, पुश/पुल, मर्ज
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 वेब SDK वर्कस्पेस (Bun)
│   ├── op-web-sdk/           रीड-ओनली `.op` वेब व्यूअर SDK (wasm बंडल को रैप करता है)
│   ├── op-web-sdk-react/     React 19 एडाप्टर
│   └── op-web-sdk-vue/       Vue 3 एडाप्टर
├── vendor/                   वेंडर्ड सबसिस्टम (git सबमॉड्यूल)
│   ├── jian/                 GPU-Skia UI फ़्रेमवर्क — विजेट/रेंडर/इवेंट
│   ├── casement/             winit फ़ोर्क
│   └── agent/                क्रॉस-प्रोडक्ट Rust एजेंट रनटाइम (agent-rs)
└── .githooks/                प्री-कमिट वर्शन ड्रिफ्ट जाँच
```

## कीबोर्ड शॉर्टकट

| कुंजी       | क्रिया             |     | कुंजी         | क्रिया                       |
| ----------- | ------------------ | --- | ------------- | ---------------------------- |
| `V`         | चुनें              |     | `Cmd+S`       | सहेजें                       |
| `R`         | Rectangle          |     | `Cmd+Z`       | पूर्ववत करें                 |
| `O`         | Ellipse            |     | `Cmd+Shift+Z` | फिर से करें                  |
| `L`         | Line               |     | `Cmd+C/X/V/D` | कॉपी/कट/पेस्ट/डुप्लिकेट      |
| `T`         | Text               |     | `Cmd+G`       | ग्रुप करें                   |
| `F`         | Frame              |     | `Cmd+Shift+G` | अनग्रुप करें                 |
| `P`         | Pen tool           |     | `Cmd+Shift+P` | एक्सपोर्ट (PNG/JPG/WEBP/PDF) |
| `H`         | Hand (pan)         |     | `Cmd+Shift+C` | कोड पैनल                     |
| `Del`       | हटाएँ              |     | `Cmd+Shift+V` | वेरिएबल पैनल                 |
| `[ / ]`     | क्रम बदलें         |     | `Cmd+J`       | AI चैट                       |
| Arrows      | 1px नज             |     | `Cmd+,`       | एजेंट सेटिंग्स               |
| `Cmd+Alt+U` | बूलियन संयोजन      |     | `Cmd+Alt+S`   | बूलियन घटाना                 |
| `Cmd+Alt+I` | बूलियन प्रतिच्छेदन |     | `Cmd+Shift+S` | इस रूप में सहेजें            |

## स्क्रिप्ट

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

# वर्शन सिंक्रोनाइज़ेशन (रिपॉज़िटरी रूट से चलाएँ)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## योगदान

योगदान का स्वागत है! आर्किटेक्चर विवरण और कोड स्टाइल के लिए [CLAUDE.md](./CLAUDE.md) देखें।

1. फ़ोर्क और क्लोन करें
2. वर्शन ड्रिफ्ट जाँच सक्षम करें: `git config core.hooksPath .githooks`
3. ब्रांच बनाएँ: `git checkout -b feat/my-feature`
4. चेक चलाएँ: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] पुन: उपयोगी Rust crates और Web SDK पैकेज के साथ Cargo workspace
- [x] डेस्कटॉप और Web के लिए Rust एडिटर
- [x] CLI टूल (`op`) टर्मिनल नियंत्रण
- [x] बिल्ट-इन Rust Agent Runtime, मल्टी-प्रदाता समर्थन
- [x] i18n — 15 भाषाएँ
- [x] JavaScript, React और Vue के लिए wasm-आधारित Viewer SDKs
- [x] टैग-आधारित मिलान और MCP टूल के साथ Style Guides
- [x] डेलिगेशन और कैनवास इंडिकेटर के साथ समवर्ती Agent Teams
- [x] Git इंटीग्रेशन (क्लोन, ब्रांच, पुश/पुल, फ़ोल्डर-मोड थ्री-वे मर्ज)
- [x] कैनवास एक्सपोर्ट (SVG / PNG / JPEG / WEBP / PDF)
- [ ] सहयोगी संपादन
- [ ] प्लगइन सिस्टम

## योगदानकर्ता

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## प्रायोजक

OpenPencil मुफ़्त और ओपन-सोर्स है। इसका विकास उन लोगों के सहयोग से चलता है जिन्हें यह उपयोगी लगता है — कैनवस को खुला रखने के लिए धन्यवाद।

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

**[MrQyun](https://github.com/mrqyun)** को धन्यवाद — अपना नाम यहाँ देखना चाहते हैं? **[प्रायोजक बनें →](https://github.com/sponsors/ZSeven-W)**

## समुदाय

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
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

## मूल्यांकन

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## लाइसेंस

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
