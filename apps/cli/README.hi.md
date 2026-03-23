# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [**हिन्दी**](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil) के लिए CLI — अपने टर्मिनल से डिज़ाइन टूल को नियंत्रित करें।

## इंस्टॉल करें

```bash
npm install -g @zseven-w/openpencil
```

## प्लेटफ़ॉर्म समर्थन

CLI सभी प्लेटफ़ॉर्म पर OpenPencil डेस्कटॉप ऐप को स्वचालित रूप से पहचानता और लॉन्च करता है:

| प्लेटफ़ॉर्म | पहचाने गए इंस्टॉलेशन पथ                                                                            |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS प्रति-उपयोगकर्ता (`%LOCALAPPDATA%`), प्रति-मशीन (`%PROGRAMFILES%`), पोर्टेबल                    |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## उपयोग

```bash
op <कमांड> [विकल्प]
```

### इनपुट विधियाँ

JSON या DSL स्वीकार करने वाले आर्गुमेंट तीन तरीकों से पास किए जा सकते हैं:

```bash
op design '...'              # इनलाइन स्ट्रिंग (छोटे पेलोड)
op design @design.txt        # फ़ाइल से पढ़ें (बड़े डिज़ाइन के लिए अनुशंसित)
cat design.txt | op design - # stdin से पढ़ें (पाइपिंग)
```

### ऐप नियंत्रण

```bash
op start [--desktop|--web]   # OpenPencil लॉन्च करें (डिफ़ॉल्ट रूप से डेस्कटॉप)
op stop                      # चल रहे इंस्टेंस को बंद करें
op status                    # जाँचें कि चल रहा है या नहीं
```

### डिज़ाइन (बैच DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### दस्तावेज़ संचालन

```bash
op open [file.op]            # फ़ाइल खोलें या लाइव कैनवास से कनेक्ट करें
op save <file.op>            # वर्तमान दस्तावेज़ सहेजें
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # वर्तमान कैनवास चयन प्राप्त करें
```

### नोड हेरफेर

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### कोड निर्यात

```bash
op export <format> [--out file]
# प्रारूप: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### वेरिएबल और थीम

```bash
op vars                      # वेरिएबल प्राप्त करें
op vars:set <json>           # वेरिएबल सेट करें
op themes                    # थीम प्राप्त करें
op themes:set <json>         # थीम सेट करें
op theme:save <file.optheme> # थीम प्रीसेट सहेजें
op theme:load <file.optheme> # थीम प्रीसेट लोड करें
op theme:list [dir]          # थीम प्रीसेट सूचीबद्ध करें
```

### पेज

```bash
op page list                 # पेज सूचीबद्ध करें
op page add [--name N]       # एक पेज जोड़ें
op page remove <id>          # एक पेज हटाएँ
op page rename <id> <name>   # एक पेज का नाम बदलें
op page reorder <id> <index> # एक पेज का क्रम बदलें
op page duplicate <id>       # एक पेज डुप्लिकेट करें
```

### आयात

```bash
op import:svg <file.svg>     # SVG फ़ाइल आयात करें
op import:figma <file.fig>   # Figma .fig फ़ाइल आयात करें
```

### लेआउट

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### वैश्विक फ़्लैग

```text
--file <path>     लक्ष्य .op फ़ाइल (डिफ़ॉल्ट: लाइव कैनवास)
--page <id>       लक्ष्य पेज ID
--pretty          मानव-पठनीय JSON आउटपुट
--help            सहायता दिखाएँ
--version         संस्करण दिखाएँ
```

## लाइसेंस

MIT
