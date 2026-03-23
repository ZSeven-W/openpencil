# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [**Tiếng Việt**](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI cho [OpenPencil](https://github.com/ZSeven-W/openpencil) — điều khiển công cụ thiết kế từ terminal của bạn.

## Cài đặt

```bash
npm install -g @zseven-w/openpencil
```

## Hỗ trợ nền tảng

CLI tự động phát hiện và khởi chạy ứng dụng desktop OpenPencil trên tất cả các nền tảng:

| Nền tảng    | Đường dẫn cài đặt được phát hiện                                                                    |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS theo người dùng (`%LOCALAPPDATA%`), theo máy (`%PROGRAMFILES%`), di động                       |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Sử dụng

```bash
op <lệnh> [tùy-chọn]
```

### Phương thức nhập liệu

Các đối số chấp nhận JSON hoặc DSL có thể được truyền theo ba cách:

```bash
op design '...'              # Chuỗi nội tuyến (dữ liệu nhỏ)
op design @design.txt        # Đọc từ tệp (khuyến nghị cho thiết kế lớn)
cat design.txt | op design - # Đọc từ stdin (đường ống)
```

### Điều khiển ứng dụng

```bash
op start [--desktop|--web]   # Khởi chạy OpenPencil (mặc định: desktop)
op stop                      # Dừng phiên bản đang chạy
op status                    # Kiểm tra trạng thái hoạt động
```

### Thiết kế (Batch DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Thao tác tài liệu

```bash
op open [file.op]            # Mở tệp hoặc kết nối với canvas trực tiếp
op save <file.op>            # Lưu tài liệu hiện tại
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Lấy vùng chọn canvas hiện tại
```

### Thao tác nút

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Xuất mã nguồn

```bash
op export <format> [--out file]
# Định dạng: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Biến và giao diện

```bash
op vars                      # Lấy biến
op vars:set <json>           # Đặt biến
op themes                    # Lấy giao diện
op themes:set <json>         # Đặt giao diện
op theme:save <file.optheme> # Lưu bộ giao diện mẫu
op theme:load <file.optheme> # Tải bộ giao diện mẫu
op theme:list [dir]          # Liệt kê bộ giao diện mẫu
```

### Trang

```bash
op page list                 # Liệt kê trang
op page add [--name N]       # Thêm trang
op page remove <id>          # Xóa trang
op page rename <id> <name>   # Đổi tên trang
op page reorder <id> <index> # Sắp xếp lại trang
op page duplicate <id>       # Nhân bản trang
```

### Nhập

```bash
op import:svg <file.svg>     # Nhập tệp SVG
op import:figma <file.fig>   # Nhập tệp Figma .fig
```

### Bố cục

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Cờ toàn cục

```text
--file <path>     Tệp .op đích (mặc định: canvas trực tiếp)
--page <id>       ID trang đích
--pretty          Xuất JSON dễ đọc
--help            Hiển thị trợ giúp
--version         Hiển thị phiên bản
```

## Giấy phép

MIT
