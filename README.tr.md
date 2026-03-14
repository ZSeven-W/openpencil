<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Dunyanin ilk acik kaynakli AI-yerel vektor tasarim araci.</strong><br />
  <sub>Eszamanli Ajan Ekipleri &bull; Kod Olarak Tasarim &bull; Yerlesik MCP Sunucusu &bull; Coklu Model Zekasi</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <b>Türkçe</b> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/KwXp6BJD"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
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
| **Kanvas** | CanvasKit/Skia (WASM, GPU hizlandirmali) |
| **Durum Yönetimi** | Zustand v5 |
| **Sunucu** | Nitro |
| **Masaüstü** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Çalışma Ortamı** | Bun · Vite 7 |
| **Dosya Formatı** | `.op` — JSON tabanlı, insan tarafından okunabilir, Git dostu |

## Proje Yapısı

```text
src/
  canvas/          CanvasKit/Skia motoru — çizim, senkronizasyon, düzen, kılavuzlar, kalem aracı
  components/      React UI — editör, paneller, paylaşılan iletişim kutuları, simgeler
  services/ai/     AI sohbet, orkestratör, tasarım üretimi, akış
  services/figma/  Figma .fig ikili içe aktarma ardışık düzeni
  services/codegen React+Tailwind ve HTML+CSS kod üreticileri
  stores/          Zustand — kanvas, belge, sayfalar, geçmiş, AI, ayarlar
  variables/       Tasarım token çözümleme ve referans yönetimi
  mcp/             Harici CLI entegrasyonu için MCP sunucu araçları
  uikit/           Yeniden kullanılabilir bileşen kiti sistemi
server/
  api/ai/          Nitro API — akış sohbet, üretim, doğrulama
  utils/           Claude CLI, OpenCode, Codex, Copilot istemci sarmalayıcıları
electron/
  main.ts          Pencere, Nitro çatallanması, yerel menü, otomatik güncelleyici
  preload.ts       IPC köprüsü
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
bun run electron:dev       # Electron geliştirme modu
bun run electron:build     # Electron paketleme
```

## Katkıda Bulunma

Katkılarınızı bekliyoruz! Mimari ayrıntılar ve kod stili için [CLAUDE.md](./CLAUDE.md) dosyasına bakın.

1. Fork'layın ve klonlayın
2. Dal oluşturun: `git checkout -b feat/my-feature`
3. Kontrolleri çalıştırın: `npx tsc --noEmit && bun --bun run test`
4. [Conventional Commits](https://www.conventionalcommits.org/) formatıyla commit yapın: `feat(canvas): add rotation snapping`
5. `main` dalına PR açın

## Yol Haritası

- [x] CSS senkronizasyonlu tasarım değişkenleri ve tokenları
- [x] Bileşen sistemi (örnekler ve geçersiz kılmalar)
- [x] Orkestratörlü AI tasarım üretimi
- [x] Katmanlı tasarım iş akışı ile MCP sunucu entegrasyonu
- [x] Çok sayfa desteği
- [x] Figma `.fig` içe aktarma
- [x] Boolean işlemler (birleştirme, çıkarma, kesişim)
- [x] Çoklu model yetenek profilleri
- [ ] Ortak düzenleme
- [ ] Eklenti sistemi

## Katkıda Bulunanlar

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Topluluk

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
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
