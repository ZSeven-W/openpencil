<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Dünyanın ilk açık kaynaklı AI-yerel vektör tasarım aracı.</strong><br />
  <sub>Eşzamanlı Ajan Ekipleri &bull; Kod Olarak Tasarım &bull; Yerleşik MCP Sunucusu &bull; Çoklu Model Zekası</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — demo videosunu izlemek için tıklayın" width="100%" />
  </a>
</p>
<p align="center"><sub>Demo videosunu izlemek için görsele tıklayın</sub></p>

<br />

## Neden OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Kanvas

Herhangi bir arayüzü doğal dilde tanımlayın. Gerçek zamanlı akış animasyonuyla sonsuz kanvasta belirmesini izleyin. Öğeleri seçip sohbet ederek mevcut tasarımları düzenleyin.

</td>
<td width="50%">

### 🤖 Eşzamanlı Ajan Ekipleri

Orkestratör, karmaşık sayfaları uzamsal alt görevlere ayırır. Birden fazla AI ajanı farklı bölümlerde eşzamanlı çalışır — hero, özellikler, footer — hepsi paralel olarak akış halinde.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Çoklu Model Zekası

Her modelin yeteneklerine otomatik olarak uyum sağlar. Claude tam promptlar ve düşünme modu alır; GPT-4o/Gemini'de düşünme modu devre dışı bırakılır; küçük modeller (MiniMax, Qwen, Llama) güvenilir çıktı için basitleştirilmiş promptlar alır.

</td>
<td width="50%">

### 🔌 MCP Sunucusu

Claude Code, Codex, Gemini, OpenCode, Kiro veya Copilot CLI'larına tek tıkla kurulum. Terminalinizden tasarım yapın — herhangi bir MCP uyumlu ajan aracılığıyla `.op` dosyalarını okuyun, oluşturun ve düzenleyin.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Kod Olarak Tasarım

`.op` dosyaları JSON formatındadır — insan tarafından okunabilir, Git dostu, diff edilebilir. Tasarım değişkenleri CSS özel özellikleri üretir. React + Tailwind veya HTML + CSS olarak kod dışa aktarımı.

</td>
<td width="50%">

### 🖥️ Her Yerde Çalışır

Web uygulaması + Electron ile macOS, Windows ve Linux'ta yerel masaüstü. GitHub Releases'ten otomatik güncelleme. `.op` dosya ilişkilendirmesi — açmak için çift tıklayın.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Tasarım aracını terminalinizden kontrol edin. `op design`, `op insert`, `op export` — toplu tasarım DSL, düğüm manipülasyonu, kod dışa aktarımı. Dosyalardan veya stdin'den pipe ile besleyin. Masaüstü uygulama veya web sunucusuyla çalışır.

</td>
<td width="50%">

### 🎯 Çok Platformlu Kod Dışa Aktarımı

Tek bir `.op` dosyasından React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native'e dışa aktarın. Tasarım değişkenleri CSS özel özelliklerine dönüşür.

</td>
</tr>
</table>

## Hızlı Başlangıç

```bash
# Bağımlılıkları yükle
bun install

# http://localhost:3000 adresinde geliştirme sunucusunu başlat
bun --bun run dev
```

Ya da masaüstü uygulaması olarak çalıştırın:

```bash
bun run electron:dev
```

> **Ön koşullar:** [Bun](https://bun.sh/) >= 1.0 ve [Node.js](https://nodejs.org/) >= 18

### Docker

Birden fazla görüntü varyantı mevcuttur — ihtiyaçlarınıza uygun olanı seçin:

| Görüntü | Boyut | İçerik |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | Yalnızca web uygulaması |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | Tüm CLI araçları |

**Çalıştır (yalnızca web):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**AI CLI ile çalıştır (ör. Claude Code):**

AI sohbeti Claude CLI OAuth girişine bağlıdır. Giriş oturumunu kalıcı hale getirmek için bir Docker hacmi kullanın:

```bash
# Adım 1 — Giriş (tek seferlik)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Adım 2 — Başlat
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Yerel olarak derle:**

```bash
# Temel (yalnızca web)
docker build --target base -t openpencil .

# Belirli bir CLI ile
docker build --target with-claude -t openpencil-claude .

# Tam (tüm CLI'lar)
docker build --target full -t openpencil-full .
```

## AI Destekli Tasarım

**Prompttan UI'ye**
- **Metinden tasarıma** — bir sayfayı tanımlayın, gerçek zamanlı akış animasyonuyla kanvasta oluşturulsun
- **Orkestratör** — karmaşık sayfaları paralel üretim için uzamsal alt görevlere ayırır
- **Tasarım değişikliği** — öğeleri seçin, ardından değişiklikleri doğal dille tanımlayın
- **Görsel girdi** — referans tabanlı tasarım için ekran görüntüleri veya maketler ekleyin

**Çok Ajanlı Destek**

| Ajan | Kurulum |
| --- | --- |
| **Claude Code** | Yapılandırma gerekmez — yerel OAuth ile Claude Agent SDK kullanır |
| **Codex CLI** | Ajan Ayarlarından bağlanın (`Cmd+,`) |
| **OpenCode** | Ajan Ayarlarından bağlanın (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` ardından Ajan Ayarlarından bağlanın (`Cmd+,`) |
| **Gemini CLI** | Ajan Ayarlarından bağlanın (`Cmd+,`) |

**Model Yetenek Profilleri** — promptları, düşünme modunu ve zaman aşımlarını model katmanına göre otomatik olarak uyarlar. Tam katman modeller (Claude) eksiksiz promptlar alır; standart katman (GPT-4o, Gemini, DeepSeek) düşünme modunu devre dışı bırakır; temel katman (MiniMax, Qwen, Llama, Mistral) maksimum güvenilirlik için basitleştirilmiş iç içe JSON promptları alır.

**MCP Sunucusu**
- Yerleşik MCP sunucusu — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI'larına tek tıkla kurulum
- Otomatik Node.js algılama — kurulu değilse otomatik olarak HTTP aktarımına geçer ve MCP HTTP sunucusunu otomatik başlatır
- Terminalden tasarım otomasyonu: herhangi bir MCP uyumlu ajan aracılığıyla `.op` dosyalarını okuyun, oluşturun ve düzenleyin
- **Katmanlı tasarım iş akışı** — daha yüksek kaliteli çok bölümlü tasarımlar için `design_skeleton` → `design_content` → `design_refine`
- **Bölümlenmiş prompt alımı** — yalnızca ihtiyacınız olan tasarım bilgisini yükleyin (şema, düzen, roller, simgeler, planlama vb.)
- Çok sayfa desteği — MCP araçları ile sayfaları oluşturun, yeniden adlandırın, sıralayın ve çoğaltın

**Kod Üretimi**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Global olarak yükleyin ve tasarım aracını terminalinizden kontrol edin:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Masaüstü uygulamayı başlat
op design @landing.txt       # Dosyadan toplu tasarım
op insert '{"type":"RECT"}'  # Bir düğüm ekle
op export react --out .      # React + Tailwind'e dışa aktar
op import:figma design.fig   # Figma dosyasını içe aktar
cat design.dsl | op design - # stdin'den pipe ile besle
```

Üç giriş yöntemini destekler: satır içi metin, `@filepath` (dosyadan oku) veya `-` (stdin'den oku). Masaüstü uygulama veya web geliştirme sunucusuyla çalışır. Tam komut referansı için [CLI README](./apps/cli/README.md) dosyasına bakın.

## Özellikler

**Kanvas ve Çizim**
- Kaydırma, yakınlaştırma, akıllı hizalama kılavuzları ve yakalamayı destekleyen sonsuz kanvas
- Dikdörtgen, Elips, Çizgi, Çokgen, Kalem (Bezier), Frame, Metin
- Boolean işlemler — bağlamsal araç çubuğuyla birleştir, çıkar, kesiştir
- Simge seçici (Iconify) ve görsel içe aktarma (PNG/JPEG/SVG/WebP/GIF)
- Otomatik düzen — boşluk, dolgu, justify, align ile dikey/yatay
- Sekme navigasyonlu çok sayfalı belgeler

**Tasarım Sistemi**
- Tasarım değişkenleri — `$variable` referanslı renk, sayı, metin tokenları
- Çok tema desteği — birden fazla tema ekseni, her biri varyantlarıyla (Açık/Koyu, Kompakt/Rahat)
- Bileşen sistemi — örnekler ve geçersiz kılmalarla yeniden kullanılabilir bileşenler
- CSS senkronizasyonu — otomatik oluşturulan özel özellikler, kod çıktısında `var(--name)`

**Figma İçe Aktarma**
- Düzen, dolgu, kontur, efektler, metin, görseller ve vektörler korunarak `.fig` dosyalarını içe aktarın

**Masaüstü Uygulaması**
- Electron aracılığıyla yerel macOS, Windows ve Linux desteği
- `.op` dosya ilişkilendirmesi — açmak için çift tıklayın, tekli örnek kilidi
- GitHub Releases'ten otomatik güncelleme
- Yerel uygulama menüsü ve dosya iletişim kutuları

## Teknoloji Yığını

| | |
| --- | --- |
| **Ön Uç** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Kanvas** | CanvasKit/Skia (WASM, GPU hızlandırmalı) |
| **Durum Yönetimi** | Zustand v5 |
| **Sunucu** | Nitro |
| **Masaüstü** | Electron 35 |
| **CLI** | `op` — terminal kontrolü, toplu tasarım DSL, kod dışa aktarımı |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Çalışma Ortamı** | Bun · Vite 7 |
| **Dosya Formatı** | `.op` — JSON tabanlı, insan tarafından okunabilir, Git dostu |

## Proje Yapısı

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start web uygulaması
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia motoru — çizim, senkronizasyon, düzen
│   │   │   ├── components/  React UI — editör, paneller, paylaşılan iletişim kutuları, simgeler
│   │   │   ├── services/ai/ AI sohbet, orkestratör, tasarım üretimi, akış
│   │   │   ├── stores/      Zustand — kanvas, belge, sayfalar, geçmiş, AI
│   │   │   ├── mcp/         Harici CLI entegrasyonu için MCP sunucu araçları
│   │   │   ├── hooks/       Klavye kısayolları, dosya bırakma, Figma yapıştırma
│   │   │   └── uikit/       Yeniden kullanılabilir bileşen kiti sistemi
│   │   └── server/
│   │       ├── api/ai/      Nitro API — akış sohbet, üretim, doğrulama
│   │       └── utils/       Claude CLI, OpenCode, Codex, Copilot sarmalayıcıları
│   ├── desktop/             Electron masaüstü uygulaması
│   │   ├── main.ts          Pencere, Nitro çatallanması, yerel menü, otomatik güncelleyici
│   │   ├── ipc-handlers.ts  Yerel dosya diyalogları, tema senkronizasyonu, tercihler IPC
│   │   └── preload.ts       IPC köprüsü
│   └── cli/                 CLI aracı — `op` komutu
│       ├── src/commands/    Tasarım, belge, dışa aktarma, içe aktarma, düğüm, sayfa, değişken komutları
│       ├── connection.ts    Çalışan uygulamaya WebSocket bağlantısı
│       └── launcher.ts      Masaüstü uygulamayı veya web sunucusunu otomatik algıla ve başlat
├── packages/
│   ├── pen-types/           PenDocument modeli için tür tanımları
│   ├── pen-core/            Belge ağacı işlemleri, düzen motoru, değişkenler
│   ├── pen-codegen/         Kod oluşturucular (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Figma .fig dosya ayrıştırıcı ve dönüştürücü
│   ├── pen-renderer/        Bağımsız CanvasKit/Skia işleyici
│   └── pen-sdk/             Şemsiye SDK (tüm paketleri yeniden dışa aktarır)
└── .githooks/               Dal adından ön-commit sürüm eşitleme
```

## Klavye Kısayolları

| Tuş | İşlem | | Tuş | İşlem |
| --- | --- | --- | --- | --- |
| `V` | Seç | | `Cmd+S` | Kaydet |
| `R` | Dikdörtgen | | `Cmd+Z` | Geri Al |
| `O` | Elips | | `Cmd+Shift+Z` | Yeniden Yap |
| `L` | Çizgi | | `Cmd+C/X/V/D` | Kopyala/Kes/Yapıştır/Çoğalt |
| `T` | Metin | | `Cmd+G` | Grupla |
| `F` | Frame | | `Cmd+Shift+G` | Grubu Çöz |
| `P` | Kalem aracı | | `Cmd+Shift+E` | Dışa Aktar |
| `H` | El (kaydır) | | `Cmd+Shift+C` | Kod paneli |
| `Del` | Sil | | `Cmd+Shift+V` | Değişkenler paneli |
| `[ / ]` | Yeniden sırala | | `Cmd+J` | AI sohbet |
| Oklar | 1px kaydır | | `Cmd+,` | Ajan ayarları |
| `Cmd+Alt+U` | Boolean birleştir | | `Cmd+Alt+S` | Boolean çıkar |
| `Cmd+Alt+I` | Boolean kesiştir | | | |

## Betikler

```bash
bun --bun run dev          # Geliştirme sunucusu (port 3000)
bun --bun run build        # Üretim derlemesi
bun --bun run test         # Testleri çalıştır (Vitest)
npx tsc --noEmit           # Tür denetimi
bun run bump <version>     # Tüm package.json dosyalarında sürümü eşitle
bun run electron:dev       # Electron geliştirme modu
bun run electron:build     # Electron paketleme
bun run cli:dev            # CLI'yi kaynaktan çalıştır
bun run cli:compile        # CLI'yi dist'e derle
```

## Katkıda Bulunma

Katkılarınızı bekliyoruz! Mimari ayrıntılar ve kod stili için [CLAUDE.md](./CLAUDE.md) dosyasına bakın.

1. Fork'layın ve klonlayın
2. Sürüm eşitlemeyi ayarlayın: `git config core.hooksPath .githooks`
3. Dal oluşturun: `git checkout -b feat/my-feature`
4. Kontrolleri çalıştırın: `npx tsc --noEmit && bun --bun run test`
5. [Conventional Commits](https://www.conventionalcommits.org/) formatıyla commit yapın: `feat(canvas): add rotation snapping`
6. `main` dalına PR açın

## Yol Haritası

- [x] CSS senkronizasyonlu tasarım değişkenleri ve tokenları
- [x] Bileşen sistemi (örnekler ve geçersiz kılmalar)
- [x] Orkestratörlü AI tasarım üretimi
- [x] Katmanlı tasarım iş akışı ile MCP sunucu entegrasyonu
- [x] Çok sayfa desteği
- [x] Figma `.fig` içe aktarma
- [x] Boolean işlemler (birleştirme, çıkarma, kesişim)
- [x] Çoklu model yetenek profilleri
- [x] Yeniden kullanılabilir paketlerle monorepo yapılandırması
- [x] CLI aracı (`op`) terminal kontrolü
- [ ] Ortak düzenleme
- [ ] Eklenti sistemi

## Katkıda Bulunanlar

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Topluluk

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord'umuza katılın</strong>
</a>
— Soru sorun, tasarımlarınızı paylaşın, özellik önerin.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Lisans

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
