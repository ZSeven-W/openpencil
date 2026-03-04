<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI destekli açık kaynak tasarım aracı. Kod Olarak Tasarım.</strong><br />
  Prompttan kanvas UI'ye. Çok ajanlı orkestrasyon. Yerleşik MCP sunucusu. Kod üretimi.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <b>Türkçe</b> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#hızlı-başlangıç">Hızlı Başlangıç</a> ·
  <a href="#ai-destekli-tasarım">AI</a> ·
  <a href="#özellikler">Özellikler</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#katkıda-bulunma">Katkıda Bulunma</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Demo videosunu izlemek için görsele tıklayın</sub></p>

<br />

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

OpenPencil, AI'yi bir eklenti olarak değil, temel iş akışı olarak sıfırdan inşa edilmiştir.

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

**MCP Sunucusu**
- Yerleşik MCP sunucusu — Claude Code / Codex / Gemini / OpenCode / Kiro CLI'larına tek tıkla kurulum
- Terminalden tasarım otomasyonu: herhangi bir MCP uyumlu ajan aracılığıyla `.op` dosyalarını okuyun, oluşturun ve düzenleyin

**Kod Üretimi**
- React + Tailwind CSS
- HTML + CSS
- Tasarım tokenlarından CSS Variables

## Özellikler

**Kanvas ve Çizim**
- Kaydırma, yakınlaştırma, akıllı hizalama kılavuzları ve yakalamayı destekleyen sonsuz kanvas
- Dikdörtgen, Elips, Çizgi, Çokgen, Kalem (Bezier), Frame, Metin
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
- GitHub Releases'ten otomatik güncelleme
- Yerel uygulama menüsü ve dosya iletişim kutuları

## Teknoloji Yığını

| | |
| --- | --- |
| **Ön Uç** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Kanvas** | Fabric.js v7 |
| **Durum Yönetimi** | Zustand v5 |
| **Sunucu** | Nitro |
| **Masaüstü** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Çalışma Ortamı** | Bun · Vite 7 |
| **Dosya Formatı** | `.op` — JSON tabanlı, insan tarafından okunabilir, Git dostu |

## Proje Yapısı

```text
src/
  canvas/          Fabric.js motoru — çizim, senkronizasyon, düzen, kılavuzlar, kalem aracı
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
  utils/           Claude CLI, OpenCode, Codex istemci sarmalayıcıları
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
- [x] MCP sunucu entegrasyonu
- [x] Çok sayfa desteği
- [x] Figma `.fig` içe aktarma
- [ ] Boolean işlemler (birleştirme, çıkarma, kesişim)
- [ ] Ortak düzenleme
- [ ] Eklenti sistemi

## Katkıda Bulunanlar

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Topluluk

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord'umuza katılın</strong>
</a>
— Soru sorun, tasarımlarınızı paylaşın, özellik önerin.

## Lisans

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
