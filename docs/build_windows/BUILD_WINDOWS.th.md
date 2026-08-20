
> [English](./BUILD_WINDOWS.md) · [简体中文](./BUILD_WINDOWS.zh.md) · [繁體中文](./BUILD_WINDOWS.zh-TW.md) · [日本語](./BUILD_WINDOWS.ja.md) · [한국어](./BUILD_WINDOWS.ko.md) · [Français](./BUILD_WINDOWS.fr.md) · [Español](./BUILD_WINDOWS.es.md) · [Deutsch](./BUILD_WINDOWS.de.md) · [Português](./BUILD_WINDOWS.pt.md) · [Русский](./BUILD_WINDOWS.ru.md) · [हिन्दी](./BUILD_WINDOWS.hi.md) · [Türkçe](./BUILD_WINDOWS.tr.md) · **ไทย** · [Tiếng Việt](./BUILD_WINDOWS.vi.md) · [Bahasa Indonesia](./BUILD_WINDOWS.id.md)

# คู่มือการคอมไพล์ OpenPencil สำหรับ Windows

## 1. ดาวน์โหลดซอร์สโค้ด

1. ดาวน์โหลดซอร์สโค้ดจากคลังเก็บ openpencil

2. openpencil มีโมดูลย่อย 3 ตัวในไดเรกทอรี `vendor` (jian, casement, agent) ที่ต้องดาวน์โหลดแยกต่างหาก

    - 2.1 jian (เฟรมเวิร์ก UI): [https://github.com/ZSeven-W/jian](https://github.com/ZSeven-W/jian) → แตกไฟล์ไปยัง `vendor\jian`

    - 2.2 casement (ตัวห่อหน้าต่าง): [https://github.com/ZSeven-W/casement](https://github.com/ZSeven-W/casement) → แตกไฟล์ไปยัง `vendor\casement`

    - 2.3 agent (รันไทม์ Agent): [https://github.com/ZSeven-W/agent-rs](https://github.com/ZSeven-W/agent-rs) → แตกไฟล์ไปยัง `vendor\agent`

3. หลังแตกไฟล์ ตรวจสอบว่าเส้นทางต่อไปนี้มีอยู่:

    ```plaintext
    vendor\jian\Cargo.toml
    vendor\casement\Cargo.toml
    vendor\agent\Cargo.toml
    ```

## 2. การเตรียมสภาพแวดล้อม — *Windows 10 x64 (MSVC toolchain)*

1. ติดตั้ง Visual Studio โดยเลือก **"การพัฒนาเดสก์ท็อปด้วย C++"**

2. ติดตั้ง toolchain Rust `stable-x86_64-pc-windows-msvc`

    - 2.1 กำหนดค่ามิเรอร์ rustup (เพื่อเร่งความเร็วในการดาวน์โหลด)

        ```powershell
        [Environment]::SetEnvironmentVariable("RUSTUP_DIST_SERVER", "https://rsproxy.cn", "User")
        [Environment]::SetEnvironmentVariable("RUSTUP_UPDATE_ROOT", "https://rsproxy.cn/rustup", "User")
        ```

    - 2.2 กำหนดค่ามิเรอร์依赖ของ Cargo เปิดไดเรกทอรีผู้ใช้ (`C:\Users\<username>\`) ค้นหาหรือสร้างโฟลเดอร์ `.cargo` แล้วสร้างไฟล์ `config.toml` ภายใน

        วางเนื้อหาต่อไปนี้ใน `C:\Users\<username>\.cargo\config.toml`:

        ```toml
        [source.crates-io]
        replace-with = 'rsproxy-sparse'

        [source.rsproxy]
        registry = "https://rsproxy.cn/crates.io-index"

        [source.rsproxy-sparse]
        registry = "sparse+https://rsproxy.cn/index/"

        [registries.rsproxy]
        index = "https://rsproxy.cn/crates.io-index"

        [net]
        git-fetch-with-cli = true
        ```

    - 2.3 ดาวน์โหลด `rustup-init.exe` จาก rustup.rs แล้วรัน เป็นคำสั่งใน PowerShell **ตรงกับ `openpencil 0.8.4`**:

        ```powershell
        rustup toolchain install 1.94
        rustup default 1.94-x86_64-pc-windows-msvc
        ```

    - 2.4 ติดตั้ง wasm-bindgen-cli สำหรับการสร้าง WASM-JS binding รันคำสั่งต่อไปนี้ใน PowerShell **ตรงกับ `openpencil 0.8.4`**:

        ```powershell
        cargo install -f wasm-bindgen-cli --version 0.2.117
        ```

## 3. การคอมไพล์

1. ในรูทของคลังเก็บ เปิด PowerShell แล้วรัน:

    ```powershell
    cargo build --workspace --release
    ```

    > คุณอาจพบข้อผิดพลาด `error: failed to run custom build command for skia-bindings v0.97.2`
    
    - 1.1 ดาวน์โหลดแพ็กเกจ GL backend ด้วยตนเอง: [skia-binaries-0.97.2 (GL)](https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz)
    
    - 1.2 ตั้งค่า environment variable (แทนที่ด้วยเส้นทางจริง):

         ```powershell
        $env:SKIA_BINARIES_URL="file:///path/to/skia-binaries-da8fc6731fc439bc3b6a-x86_64-pc-windows-msvc-gl-jpegd-jpege-pdf-textlayout.tar.gz"
        ```
    
    - 1.3 รันขั้นตอน 3.1 อีกครั้งเพื่อคอมไพล์ใหม่

2. การสร้าง Web Bundle

    - 2.1 คอมไพล์ไลบรารี WASM ในรูทของคลังเก็บ เปิด PowerShell แล้วรัน:

        ```powershell
        cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
        ```

    - 2.2 สร้างไฟล์ JS binding

        ```powershell
        wasm-bindgen target\wasm32-unknown-unknown\release\op_host_web.wasm --out-dir target\release\web-bundle --target web
        ```

    - 2.3 คัดลอกทรัพยากร CanvasKit rendering จาก `crates\op-host-web\assets\canvaskit` ไปยัง `web-bundle`:

        ```powershell
        Copy-Item -Recurse "crates\op-host-web\assets\canvaskit" -Destination "target\release\web-bundle\canvaskit" -Force
        ```

    - 2.4 โครงสร้างไดเรกทอรีสุดท้าย

        ```plaintext
        target\release\
        ├── openpencil-desktop.exe
        ├── op-host-web-server.exe
        ├── op.exe
        └── web-bundle\
            ├── canvaskit\
            │   ├── canvaskit.js
            │   └── canvaskit.wasm
            ├── snippets\
            ├── op_host-web.d.ts
            ├── op_host_web.js
            ├── op_host_web_bg.wasm
            └── op_host_web_bg.wasm.d.ts
        ```

## 4. กำหนดค่า environment variable สำหรับปลั๊กอิน dsh-openpencil

1. เปิด PowerShell ในฐานะผู้ดูแลระบบแล้วรันคำสั่งต่อไปนี้ (แทนที่เส้นทางด้วยรูทคลังเก็บจริง):

    ```powershell
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    [Environment]::SetEnvironmentVariable("DSH_OPENPENCIL_EDITOR_BINARY", "<openpencil-root>\target\release\openpencil-desktop.exe", "User")
    ```

## 5. การตรวจสอบ

1. openpencil-desktop.exe

    ```powershell
    cd target\release
    ./openpencil-desktop.exe --serve-web 3100
    # หน้าต่างจะกะพริบแล้วปิด — บริการทำงานในพื้นหลังและฟังพอร์ต 3100
    ```

2. op-host-web-server.exe  <i>` (ไม่จำเป็นสำหรับปลั๊กอิน dsh) `</i>

    ```powershell
    cd target\release
    ./op-host-web-server.exe --serve-web 3100
    # คุณจะเห็นสองบรรทัด "openpencil-desktop --serve-web:" และพอร์ต 3100 ใน PowerShell
    ```

---

# การแก้ไขปัญหา

1. ให้ความสำคัญกับพารามิเตอร์ `--features` เมื่อคอมไพล์ WASM

    ```powershell
    cargo build -p op-host-web --target wasm32-unknown-unknown --release --no-default-features --features "web canvaskit"
    ```

    > ใช้ `--features "web canvaskit"` แทน `--features "web"`
    > 
    > `--features "web"` คอมไพล์สำเร็จ แต่ไฟล์ JS ที่สร้างขึ้นจะไม่ส่งออกฟังก์ชัน `mount_ck`
    > 
    >

2. เซิร์ฟเวอร์จะแมปไดเรกทอรี `web-bundle` ทั้งหมดไปยังเส้นทาง HTTP `/pkg/`

    > เส้นทางดิ스크 `web-bundle/op_host_web.js` → URL ในเบราว์เซอร์ `/pkg/op_host_web.js`
    > 
    > ถ้าหน้าเว็บแจ้งว่าไม่พบ `/pkg/op_host_web.js` แสดงว่าไฟล์หายไปจาก `web-bundle/`

3. เอกสารนี้เขียนสำหรับ `openpencil 0.8.4` เวอร์ชัน rustup toolchain `1.94`、wasm-bindgen-cli `0.2.117` และ skia-bindings `0.97.2` สอดคล้องกับเวอร์ชันนี้ — เวอร์ชันอื่นอาจแตกต่างกัน
