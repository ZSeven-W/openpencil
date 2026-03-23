# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [**ไทย**](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI สำหรับ [OpenPencil](https://github.com/ZSeven-W/openpencil) — ควบคุมเครื่องมือออกแบบจากเทอร์มินัลของคุณ

## การติดตั้ง

```bash
npm install -g @zseven-w/openpencil
```

## การรองรับแพลตฟอร์ม

CLI จะตรวจจับและเปิดแอปเดสก์ท็อป OpenPencil โดยอัตโนมัติบนทุกแพลตฟอร์ม:

| แพลตฟอร์ม  | เส้นทางการติดตั้งที่ตรวจพบ                                                                          |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS ต่อผู้ใช้ (`%LOCALAPPDATA%`), ต่อเครื่อง (`%PROGRAMFILES%`), แบบพกพา                           |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## การใช้งาน

```bash
op <คำสั่ง> [ตัวเลือก]
```

### วิธีการป้อนข้อมูล

อาร์กิวเมนต์ที่รับ JSON หรือ DSL สามารถส่งได้สามวิธี:

```bash
op design '...'              # ข้อความแบบอินไลน์ (ข้อมูลขนาดเล็ก)
op design @design.txt        # อ่านจากไฟล์ (แนะนำสำหรับการออกแบบขนาดใหญ่)
cat design.txt | op design - # อ่านจาก stdin (การไพพ์)
```

### การควบคุมแอป

```bash
op start [--desktop|--web]   # เปิด OpenPencil (เดสก์ท็อปเป็นค่าเริ่มต้น)
op stop                      # หยุดอินสแตนซ์ที่กำลังทำงาน
op status                    # ตรวจสอบว่ากำลังทำงานอยู่หรือไม่
```

### การออกแบบ (Batch DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### การดำเนินการเอกสาร

```bash
op open [file.op]            # เปิดไฟล์หรือเชื่อมต่อกับแคนวาสสด
op save <file.op>            # บันทึกเอกสารปัจจุบัน
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # รับการเลือกแคนวาสปัจจุบัน
```

### การจัดการโหนด

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### การส่งออกโค้ด

```bash
op export <format> [--out file]
# รูปแบบ: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### ตัวแปรและธีม

```bash
op vars                      # รับตัวแปร
op vars:set <json>           # ตั้งค่าตัวแปร
op themes                    # รับธีม
op themes:set <json>         # ตั้งค่าธีม
op theme:save <file.optheme> # บันทึกพรีเซ็ตธีม
op theme:load <file.optheme> # โหลดพรีเซ็ตธีม
op theme:list [dir]          # แสดงรายการพรีเซ็ตธีม
```

### หน้า

```bash
op page list                 # แสดงรายการหน้า
op page add [--name N]       # เพิ่มหน้า
op page remove <id>          # ลบหน้า
op page rename <id> <name>   # เปลี่ยนชื่อหน้า
op page reorder <id> <index> # จัดลำดับหน้าใหม่
op page duplicate <id>       # ทำสำเนาหน้า
```

### การนำเข้า

```bash
op import:svg <file.svg>     # นำเข้าไฟล์ SVG
op import:figma <file.fig>   # นำเข้าไฟล์ Figma .fig
```

### เลย์เอาต์

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### แฟล็กทั่วไป

```text
--file <path>     ไฟล์ .op เป้าหมาย (ค่าเริ่มต้น: แคนวาสสด)
--page <id>       ID หน้าเป้าหมาย
--pretty          แสดงผล JSON แบบอ่านง่าย
--help            แสดงความช่วยเหลือ
--version         แสดงเวอร์ชัน
```

## สัญญาอนุญาต

MIT
