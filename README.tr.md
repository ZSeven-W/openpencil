<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — demo videosunu izlemek için tıklayın" width="100%" />
  </a>
</p>
<p align="center"><sub>Demo videosunu izlemek için görsele tıklayın</sub></p>

<br />

> **Not:** Aynı ada sahip başka bir açık kaynak proje bulunmaktadır — [OpenPencil](https://github.com/open-pencil/open-pencil), Figma uyumlu görsel tasarım ve gerçek zamanlı iş birliğine odaklanmaktadır. Bu proje, AI-native tasarımdan koda iş akışlarına odaklanmaktadır.

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

Web uygulaması + macOS, Windows ve Linux'ta yerel masaüstü — tek bir Rust çekirdeği, tek bir bağımsız ikili, tarayıcı motoru yok. `.op` dosya ilişkilendirmesi — açmak için çift tıklayın.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Tasarım aracını terminalinizden kontrol edin. `op design`, `op insert` — toplu tasarım DSL, düğüm manipülasyonu. Dosyalardan veya stdin'den pipe ile besleyin. Masaüstü uygulama veya web sunucusuyla çalışır.

</td>
<td width="50%">

### 🎯 Çok Platformlu Kod Dışa Aktarımı

Tek bir `.op` dosyasından React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native'e dışa aktarın. Tasarım değişkenleri CSS özel özelliklerine dönüşür.

</td>
</tr>
</table>

## Hızlı Başlangıç

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Ya da masaüstü uygulaması olarak çalıştırın:

```bash
cargo run -p op-host-desktop
```

> **Ön koşullar:** Ürünü derlemek için [Rust](https://www.rust-lang.org/) (stable). [Bun](https://bun.sh/) >= 1.0 ve [Node.js](https://nodejs.org/) >= 18 yalnızca `packages/` altındaki web SDK için gereklidir.

### Docker

Etiketli Rust sürümleri tek bir web host görüntüsü yayımlar. AI CLI'ları içeren emekli TypeScript görüntüleri artık yayımlanmaz.

| Görüntü | İçerik |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:v0.8.1` | Rust web host, wasm bundle ve CanvasKit varlıkları |

Web UI yalnızca yerleşik agent profillerini gösterir; Claude/Codex/OpenCode/Copilot/Gemini CLI araçları Docker görüntülerine dahil değildir.

**Çalıştır:**

```bash
docker run -d -p 3100:3100 ghcr.io/zseven-w/openpencil-web:v0.8.1
```

Ardından `http://localhost:3100/` adresini açın.

**Yerel olarak derle:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI Destekli Tasarım

**Prompttan UI'ye**

- **Metinden tasarıma** — bir sayfayı tanımlayın, gerçek zamanlı akış animasyonuyla kanvasta oluşturulsun
- **Orkestratör** — karmaşık sayfaları paralel üretim için uzamsal alt görevlere ayırır
- **Tasarım değişikliği** — öğeleri seçin, ardından değişiklikleri doğal dille tanımlayın
- **Görsel girdi** — referans tabanlı tasarım için ekran görüntüleri veya maketler ekleyin

**Çok Ajanlı Destek**

| Ajan                        | Kurulum                                                                                                   |
| --------------------------- | --------------------------------------------------------------------------------------------------------- |
| **Yerleşik (9+ sağlayıcı)** | Sağlayıcı ön ayarlarından seçin ve bölge değiştirin — Anthropic, OpenAI, Google, DeepSeek ve daha fazlası |
| **Claude Code**             | Yapılandırma gerekmez — yerel OAuth ile Claude Agent SDK kullanır                                         |
| **Codex CLI**               | Ajan Ayarlarından bağlanın (`Cmd+,`)                                                                      |
| **OpenCode**                | Ajan Ayarlarından bağlanın (`Cmd+,`)                                                                      |
| **GitHub Copilot**          | `copilot login` ardından Ajan Ayarlarından bağlanın (`Cmd+,`)                                             |
| **Gemini CLI**              | Ajan Ayarlarından bağlanın (`Cmd+,`)                                                                      |

**Model Yetenek Profilleri** — promptları, düşünme modunu ve zaman aşımlarını model katmanına göre otomatik olarak uyarlar. Tam katman modeller (Claude) eksiksiz promptlar alır; standart katman (GPT-4o, Gemini, DeepSeek) düşünme modunu devre dışı bırakır; temel katman (MiniMax, Qwen, Llama, Mistral) maksimum güvenilirlik için basitleştirilmiş iç içe JSON promptları alır.

**i18n** — 15 dilde tam arayüz yerelleştirmesi: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**MCP Sunucusu**

- Yerleşik MCP sunucusu (`op-mcp` crate) — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI'larına tek tıkla kurulum
- Node.js gerekmez — masaüstü ikili dosyası aracılığıyla stdio aktarımı (`--mcp <path>`), ayrıca çalışan uygulamadan canlı bir HTTP uç noktası (`127.0.0.1:<port>/mcp`)
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # Masaüstü uygulamayı başlat
op start --headless --file design.op # Headless sunucuyu başlat
op design @landing.txt       # Dosyadan toplu tasarım
op design @ui.js             # Döngülü sandbox JavaScript
op insert '{"type":"rectangle"}' # Bir düğüm ekle
op import:figma design.fig   # Figma dosyasını içe aktar
cat design.dsl | op design - # stdin'den pipe ile besle
```

Satır içi metni, `@filepath` ve stdin'i (`-`) destekler. Masaüstü uygulama, web sunucusu veya dosya tabanlı headless sunucuyla çalışır. Tüm komutlar için [CLI komut referansına](./crates/op-cli/src/usage.txt) bakın.

**LLM Becerisi** — AI ajanlarına `op` ile tasarım yapmayı öğretmek için [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) eklentisini kurun. Algılanan ajanlar için `op install`, belirli bir hedef için `op install --target codex` çalıştırın.

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
- Yeniden kullanılabilir UIKit'ler — `.pen` dosyalarından bileşen kitlerini içe/dışa aktarın

**AI ve Ajanlar**

- Akışlı üretim ve orkestratör güdümlü uzamsal ayrıştırma ile prompt-to-canvas
- Eşzamanlı Ajan Ekipleri — birden çok tasarımcı farklı bölümler üzerinde paralel çalışır, üye başına kanvas göstergeleri ile
- Katmanlı iş akışı — `design_skeleton` → `design_content` → `design_refine`, her aşamada odaklı prompt'lar
- Stil Rehberleri — 50+ yerleşik stil (glassmorphism, brutalist, retro vb.), etiket tabanlı bulanık eşleştirme ile planlama ve üretime entegre
- Çoklu model yetenek profilleri — model katmanına göre düşünme modunu, çabayı ve prompt biçimini otomatik olarak uyarlar
- Yerleşik ajan çalışma ortamı (Rust) + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini sağlayıcıları
- Çinli LLM sağlayıcıları için Anthropic formatlı geçiş — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Git Entegrasyonu**

- SSH / HTTPS kimlik doğrulama ve SSH anahtarı yönetimi ile klonlama sihirbazı
- Dal seçici — oluştur, değiştir, sil, birleştir, hepsi Git panelinden
- Kimlik doğrulama yeniden denemeleri ve non-fast-forward yönetimi ile pull / push kademeleri
- Diskte `MERGE_HEAD` durum takibi ile klasör modu üç yönlü birleştirme
- Düğüm/alan başına üç yönlü kartlar, satır içi JSON editörü, toplu eylemler ve satır içi diff bloğu ile çakışma paneli
- Uzak ayarlar ve SSH anahtarları arayüzü; tüm Git yüzeyinde 15 dilde i18n

**Dışa Aktarma**

- Kanvas dışa aktarma — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Kod dışa aktarma — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Artımlı MCP kod üretimi hattı — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Figma İçe Aktarma**

- Düzen, dolgu, kontur, efektler, metin, görseller ve vektörler korunarak `.fig` dosyalarını içe aktarın

**Masaüstü Uygulaması**

- Yerel macOS, Windows ve Linux desteği — tek bir bağımsız ikili (winit + GPU Skia, Electron yok)
- `.op` dosya ilişkilendirmesi — açmak için çift tıklayın, tekli örnek kilidi
- GitHub Releases'e karşı arka planda güncelleme kontrolü
- Farklı Kaydet, Son Kullanılanları Aç ve kapatırken kaydedilmemiş değişiklikler iletişim kutusu içeren yerel uygulama menüsü
- Son kullanılan dosyaların kalıcılığı

## Teknoloji Yığını

|                         |                                                                                  |
| ----------------------- | -------------------------------------------------------------------------------- |
| **Çekirdek**            | Rust çalışma alanı (`crates/`) — editör durumu, widget'lar, host'lar, MCP, AI, codegen |
| **Görüntü İşleme**      | Her yerde GPU Skia — yerelde `skia-safe` (GL), tarayıcıda CanvasKit (WASM/WebGL2) |
| **UI çatısı**           | jian — vendored saf Rust GPU-Skia UI çatısı: widget'lar, düzen, olaylar, hot reload (`vendor/jian`) |
| **Pencere Yönetimi**    | winit (vendored `casement` çatalı)                                               |
| **Masaüstü**            | Yerel ikili `openpencil-desktop` — tarayıcı motoru yok                           |
| **Web SDK**             | `op-web-sdk` + React 19 / Vue 3 adaptörleri — salt okunur `.op` görüntüleyici (TypeScript) |
| **CLI**                 | `op` — terminal kontrolü, toplu tasarım DSL                                      |
| **AI**                  | Yerleşik Rust ajan çalışma ortamı · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**                | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                               |
| **Dosya Formatı**       | `.op` — JSON tabanlı, insan tarafından okunabilir, Git dostu                     |

## Ekosistem

OpenPencil, **[ZSeven-W](https://github.com/ZSeven-W)** tarafından geliştirilen saf Rust, AI-yerel araçlardan oluşan bir ailenin parçasıdır. Birbirini tamamlarlar: `jian` OpenPencil'ı işler, `agent-rs` ajanlarını çalıştırır, `noema` hatırlar ve `zode` terminalden tasarlar.

| Proje | Nedir |
| ----- | ----- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Terminaliniz için açık kaynaklı, AI-yerel kodlama asistanı — kodunuzu okuyan, komutları çalıştıran, dosyaları arayan ve git'i yöneten hızlı bir Rust TUI'si (`ratatui`). OpenPencil'ı MCP üzerinden sürer. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM ajanlarını sevk etmek için saf Rust asenkron çalışma ortamı — çoklu sağlayıcı, uçtan uca araç yetenekli, yapılandırılmış izinler, gerçek MCP ve sıfır `unsafe`. OpenPencil'ın yerleşik ajan çalışma ortamını (`vendor/agent`) ve Zode'u güçlendirir. |
| **[jian](https://github.com/ZSeven-W/jian)** | Saf Rust, GPU-Skia UI çatısı — tek bir yığında widget'lar, düzen, olaylar ve hot reload. Bildirimsel bir `.op` belgesini JS çalışma ortamı, DOM ve Electron olmadan yerel, AI ile kontrol edilebilir bir uygulamaya dönüştürür. OpenPencil'ın UI çatısı (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Kodlama ajanları için yerel öncelikli, vektörsüz bellek sistemi. İncelenebilir dosyalar olarak kalıcı bellek, yeni girdiler için bir inceleme kuyruğu ve sözcüksel (gömme içermeyen) geri çağırma — Zode, Codex, Claude Code ve MCP çalışma ortamlarında çalışır. |

## Neden Rust

OpenPencil, sıfırdan **Rust** ile yeniden yazıldı ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). Yeniden yazım tamamlandı — TypeScript + Electron editörü `v0.7.5` sürümünde kullanımdan kaldırıldı ve bu depodaki Rust çalışma alanı artık ürünün kendisidir: tek bir doğal çekirdek, çok daha küçük, çok daha hızlı ve tek bir kod tabanından daha fazla platformda çalışır.

|                          | TypeScript + Electron (kullanımdan kaldırıldı, `v0.7.5`) | Rust (bugün)                                                             |
| ------------------------ | ------------------------------------------------- | ------------------------------------------------------------------------ |
| **Masaüstü çalışma ortamı** | Electron — Chromium + Node.js içerir           | Yerel pencere (`winit` + GPU Skia), tarayıcı motoru yok                  |
| **Masaüstü boyutu**      | Kurulum başına tam Chromium çalışma ortamı        | Tek, bağımsız ikili — **55.5 MB**                                        |
| **Web yükü**             | JS + WASM paketi                                  | **8.2 MB** wasm / **2.18 MB** gzip ile transfer                          |
| **Görüntü işleme**       | Web'de CanvasKit/Skia                             | **Her** hedefte tek GPU hızlandırmalı Skia arka ucu                      |
| **Bellek**               | JavaScript GC duraklamaları                       | GC yok — Rust sahiplik modeli, öngörülebilir gecikme                     |
| **Kod tabanı**           | Web yığını + Electron             | Tek Rust çalışma alanı: editör · CLI · MCP · AI · codegen · Figma · Git  |
| **Hedefler**             | Web + masaüstü, iki ayrı yığın                   | Masaüstü (macOS/Win/Linux) · mobil (iOS/Android) · tarayıcı — tek çekirdek |

**Ölçülen iyileştirmeler**

- **Küçük boyut** — tüm masaüstü uygulaması, paketlenmiş bir tarayıcı motoru ve Node çalışma ortamı yerine tek bir **55.5 MB** yerel ikili dosyasıdır. Web derlemesi, simge kataloğu bölümlendirmesinin ardından ham **8.2 MB** / **2.18 MB** gzip boyutuna ulaşır (transfer üzerinde −48%).
- **Büyük belgelere ölçeklenir** — dört seviye derinliğinde iç içe otomatik düzen içeren **10,000 düğümlü** canlı bir kanvas; **panik olmadan ve ~0% boşta CPU** kullanımıyla yazar, okur ve düzeni anlık görüntüler; tüm 10k düğümün tam düzen anlık görüntüsü **~0.68 saniyede** döner.
- **Hızlı etkileşim** — kaydırma/yakınlaştırma artık her karede belgeyi yeniden serileştirmiyor (tek bir sıcak yol düzeltmesiyle tekerlek yakınlaştırma CPU kullanımı **~69%'dan ~0%'a** düştü); sürükleme işlemleri sahneyi artımlı olarak günceller, metin ölçümü önbelleğe alınır ve yeniden boyamalar kare başına birleştirilir.
- **Tek çekirdek, her ekran** — aynı editör durumu ve aynı görüntü işleme arka ucu; yerel masaüstüne, mobil cihazlara ve WASM aracılığıyla tarayıcıya derlenir — senkronda tutulacak paralel yeniden uygulama yoktur.
- **Her yerde GPU Skia** — yerel, bir GL bağlamında `skia-safe` üzerinden; tarayıcı ise WebGL2 üzerinde CanvasKit aracılığıyla görüntü işler — aynı çizim kodu, aynı çıktı.
- **Yerel erişilebilirlik** — tarayıcının a11y ağacına güvenmek yerine macOS, Windows ve Linux'ta AccessKit; web'de ise bir DOM aynası.
- **Tek tip denetimli çalışma alanı** — MCP sunucusu, CLI, AI sağlayıcıları, kod üretimi, Figma içe aktarma ve Git entegrasyonu; CI'da `cargo-deny` tedarik zinciri denetimi ile tek bir Rust çalışma alanında bulunur.

> **Durum:** TypeScript editörü `v0.7.5` sürümünde kullanımdan kaldırıldı ve yalnızca git geçmişinde bulunuyor; bu depo artık Rust çalışma alanıdır. `v0.8.1` Rust sürümü etkin geliştirme aşamasındadır (aşağıdaki Yol Haritası'na bakın).

## Proje Yapısı

```text
openpencil/
├── crates/                   Rust çalışma alanı — ürünün kendisi
│   ├── op-editor-core/       Standart `.op` (PenDocument) editör durumu + EditorCommand + tasarım değişkenleri
│   ├── op-editor-ui/         Platformdan bağımsız widget'lar + RenderBackend arayüzü (wasm32 uyumlu)
│   ├── op-editor-host-core/  Tüm host'lar tarafından paylaşılan, taşımadan bağımsız host durum makineleri
│   ├── op-host-native/       Yerel host kütüphanesi — winit + skia-safe GL (masaüstü + mobil)
│   ├── op-host-web/          Tarayıcı paketi — wasm32 cdylib, CanvasKit işleyici
│   ├── op-host-desktop/      Masaüstü ikili dosyası `openpencil-desktop`; ayrıca `--serve-web` daemon'ı
│   ├── op-host-services/     Headless serve-web / MCP daemon kütüphanesi
│   ├── op-host-web-server/   GL içermeyen ince web sunucusu ikili dosyası
│   ├── op-cli/               CLI aracı — `op` komutu
│   ├── op-mcp/               MCP sunucusu — araçlar, toplu tasarım, katmanlı iş akışı
│   ├── op-ai/                AI sağlayıcıları, sohbet çalışma ortamı, akış
│   ├── op-ai-skills/         AI prompt beceri motoru (aşamalı prompt yükleme)
│   ├── op-orchestrator/      Eşzamanlı ajan ekibi orkestrasyonu
│   ├── op-codegen/           Kod oluşturucular (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma .fig dosya ayrıştırıcı ve dönüştürücü
│   ├── op-git/               Git entegrasyonu — klonlama, dal, push/pull, birleştirme
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK çalışma alanı (Bun)
│   ├── op-web-sdk/           Salt okunur `.op` web görüntüleyici SDK'sı (wasm paketini sarmalar)
│   ├── op-web-sdk-react/     React 19 adaptörü
│   └── op-web-sdk-vue/       Vue 3 adaptörü
├── vendor/                   Vendored alt sistemler (git submodule'ları)
│   ├── jian/                 GPU-Skia UI çatısı — widget/render/event
│   ├── casement/             winit çatalı
│   └── agent/                Ürünler arası Rust ajan çalışma ortamı (agent-rs)
└── .githooks/                Dal adından ön-commit sürüm eşitleme
```

## Klavye Kısayolları

| Tuş         | İşlem             |     | Tuş           | İşlem                         |
| ----------- | ----------------- | --- | ------------- | ----------------------------- |
| `V`         | Seç               |     | `Cmd+S`       | Kaydet                        |
| `R`         | Dikdörtgen        |     | `Cmd+Z`       | Geri Al                       |
| `O`         | Elips             |     | `Cmd+Shift+Z` | Yeniden Yap                   |
| `L`         | Çizgi             |     | `Cmd+C/X/V/D` | Kopyala/Kes/Yapıştır/Çoğalt   |
| `T`         | Metin             |     | `Cmd+G`       | Grupla                        |
| `F`         | Frame             |     | `Cmd+Shift+G` | Grubu Çöz                     |
| `P`         | Kalem aracı       |     | `Cmd+Shift+P` | Dışa Aktar (PNG/JPG/WEBP/PDF) |
| `H`         | El (kaydır)       |     | `Cmd+Shift+C` | Kod paneli                    |
| `Del`       | Sil               |     | `Cmd+Shift+V` | Değişkenler paneli            |
| `[ / ]`     | Yeniden sırala    |     | `Cmd+J`       | AI sohbet                     |
| Oklar       | 1px kaydır        |     | `Cmd+,`       | Ajan ayarları                 |
| `Cmd+Alt+U` | Boolean birleştir |     | `Cmd+Alt+S`   | Boolean çıkar                 |
| `Cmd+Alt+I` | Boolean kesiştir  |     | `Cmd+Shift+S` | Farklı Kaydet                 |

## Betikler

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
cd packages && bun run bump <version>             # Sync SDK package.json versions
```

## Katkıda Bulunma

Katkılarınızı bekliyoruz! Mimari ayrıntılar ve kod stili için [CLAUDE.md](./CLAUDE.md) dosyasına bakın.

1. Fork'layın ve klonlayın
2. Sürüm eşitlemeyi ayarlayın: `git config core.hooksPath .githooks`
3. Dal oluşturun: `git checkout -b feat/my-feature`
4. Kontrolleri çalıştırın: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Yeniden kullanılabilir Rust crate'leri ve web SDK paketleri içeren Cargo workspace
- [x] Masaüstü ve web için Rust editörü
- [x] CLI aracı (`op`) terminal kontrolü
- [x] Çoklu sağlayıcı destekli yerleşik Rust Agent Runtime
- [x] i18n — 15 dil
- [x] JavaScript, React ve Vue için wasm tabanlı Viewer SDK'ları
- [x] Etiket tabanlı eşleştirme ve MCP araçlarıyla Style Guides
- [x] Delegasyon ve kanvas göstergeleriyle eşzamanlı Agent Teams
- [x] Git entegrasyonu (klonlama, dal, push/pull, klasör modu üç yönlü birleştirme)
- [x] Kanvas dışa aktarma (SVG / PNG / JPEG / WEBP / PDF)
- [ ] Ortak düzenleme
- [ ] Eklenti sistemi

## Katkıda Bulunanlar

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Sponsorlar

OpenPencil ücretsiz ve açık kaynaklıdır. Geliştirme, onu faydalı bulanlar tarafından finanse ediliyor — tuvali açık tuttuğunuz için teşekkürler.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

**[MrQyun](https://github.com/mrqyun)**'a teşekkürler — isminizi burada görmek ister misiniz? **[Sponsor ol →](https://github.com/sponsors/ZSeven-W)**

## Topluluk

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
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

## Değerlendirmeler

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Lisans

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
