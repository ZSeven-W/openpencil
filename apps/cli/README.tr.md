# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [**Türkçe**](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil) icin CLI — tasarim aracini terminalinizden kontrol edin.

## Kurulum

```bash
npm install -g @zseven-w/openpencil
```

## Platform Destegi

CLI, tum platformlarda OpenPencil masaustu uygulamasini otomatik olarak algilar ve baslatir:

| Platform    | Algilanan kurulum yollari                                                                           |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | Kullanici basina NSIS (`%LOCALAPPDATA%`), makine basina (`%PROGRAMFILES%`), tasinabilir              |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Kullanim

```bash
op <komut> [secenekler]
```

### Girdi Yontemleri

JSON veya DSL kabul eden argumanlar uc sekilde iletilebilir:

```bash
op design '...'              # Satir ici metin (kucuk veriler)
op design @design.txt        # Dosyadan oku (buyuk tasarimlar icin onerilir)
cat design.txt | op design - # Stdin'den oku (borulama)
```

### Uygulama Kontrolu

```bash
op start [--desktop|--web]   # OpenPencil'i baslat (varsayilan: masaustu)
op stop                      # Calisan ornegi durdur
op status                    # Calisip calismadigini kontrol et
```

### Tasarim (Toplu DSL)

```bash
op design <dsl|@dosya|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@dosya|->
op design:content <bolum-id> <json|@dosya|->
op design:refine --root-id <id>
```

### Belge Islemleri

```bash
op open [dosya.op]           # Dosya ac veya canli tuvale baglan
op save <dosya.op>           # Mevcut belgeyi kaydet
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Mevcut tuval secimini al
```

### Dugum Manipulasyonu

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Kod Disari Aktarimi

```bash
op export <format> [--out dosya]
# Formatlar: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Degiskenler ve Temalar

```bash
op vars                      # Degiskenleri al
op vars:set <json>           # Degiskenleri ayarla
op themes                    # Temalari al
op themes:set <json>         # Temalari ayarla
op theme:save <dosya.optheme> # Tema onayarini kaydet
op theme:load <dosya.optheme> # Tema onayarini yukle
op theme:list [dizin]        # Tema onayarlarini listele
```

### Sayfalar

```bash
op page list                 # Sayfalari listele
op page add [--name N]       # Sayfa ekle
op page remove <id>          # Sayfa kaldir
op page rename <id> <ad>     # Sayfayi yeniden adlandir
op page reorder <id> <indeks> # Sayfayi yeniden sirala
op page duplicate <id>       # Sayfayi cogalt
```

### Iceri Aktarma

```bash
op import:svg <dosya.svg>    # SVG dosyasi iceri aktar
op import:figma <dosya.fig>  # Figma .fig dosyasi iceri aktar
```

### Yerlesim

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Genel Bayraklar

```text
--file <yol>      Hedef .op dosyasi (varsayilan: canli tuval)
--page <id>       Hedef sayfa kimligi
--pretty          Okunabilir JSON ciktisi
--help            Yardimi goster
--version         Surumu goster
```

## Lisans

MIT
