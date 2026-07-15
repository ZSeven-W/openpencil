<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>เครื่องมือออกแบบเวกเตอร์โอเพนซอร์สที่ขับเคลื่อนด้วย AI ตัวแรกของโลก</strong><br />
  <sub>ทีม Agent ทำงานพร้อมกัน &bull; Design-as-Code &bull; MCP Server ในตัว &bull; ปัญญาหลายโมเดล</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — คลิกเพื่อดูวิดีโอสาธิต" width="100%" />
  </a>
</p>
<p align="center"><sub>คลิกที่รูปภาพเพื่อดูวิดีโอสาธิต</sub></p>

<br />

> **หมายเหตุ:** มีโปรเจกต์โอเพนซอร์สอีกโปรเจกต์หนึ่งที่ใช้ชื่อเดียวกัน — [OpenPencil](https://github.com/open-pencil/open-pencil) ซึ่งเน้นการออกแบบภาพที่เข้ากันได้กับ Figma พร้อมการทำงานร่วมกันแบบเรียลไทม์ โปรเจกต์นี้เน้นเวิร์กโฟลว์ AI-native สำหรับการแปลงดีไซน์เป็นโค้ด

## ทำไมต้อง OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

อธิบาย UI ใดก็ได้ด้วยภาษาธรรมชาติ ดูมันปรากฏบน Canvas ไม่จำกัดขนาดแบบเรียลไทม์พร้อม animation แบบ streaming แก้ไขดีไซน์ที่มีอยู่โดยเลือกองค์ประกอบแล้วพิมพ์สนทนา

</td>
<td width="50%">

### 🤖 ทีม Agent ทำงานพร้อมกัน

Orchestrator แบ่งหน้าที่ซับซ้อนออกเป็น sub-task เชิงพื้นที่ AI agent หลายตัวทำงานในส่วนต่าง ๆ พร้อมกัน — hero, features, footer — ทั้งหมด streaming แบบขนาน

</td>
</tr>
<tr>
<td width="50%">

### 🧠 ปัญญาหลายโมเดล

ปรับตัวตามความสามารถของแต่ละโมเดลโดยอัตโนมัติ Claude ได้ prompt เต็มรูปแบบพร้อม thinking; GPT-4o/Gemini ปิด thinking; โมเดลขนาดเล็ก (MiniMax, Qwen, Llama) ได้ prompt แบบย่อเพื่อผลลัพธ์ที่เสถียร

</td>
<td width="50%">

### 🔌 MCP Server

ติดตั้งได้ด้วยคลิกเดียวใน Claude Code, Codex, Gemini, OpenCode, Kiro หรือ Copilot CLIs ออกแบบจาก terminal ของคุณ — อ่าน สร้าง และแก้ไขไฟล์ `.op` ผ่าน agent ที่รองรับ MCP

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

ไฟล์ `.op` เป็น JSON — อ่านได้โดยมนุษย์, Git-friendly, เปรียบเทียบความแตกต่างได้ Design variables สร้าง CSS custom properties ส่งออกโค้ดเป็น React + Tailwind หรือ HTML + CSS

</td>
<td width="50%">

### 🖥️ ใช้งานได้ทุกที่

เว็บแอป + เดสก์ท็อปแบบ native บน macOS, Windows และ Linux — Rust core เดียว, binary แบบ self-contained เดียว ไม่มี browser engine เชื่อมโยงไฟล์ `.op` — ดับเบิลคลิกเพื่อเปิด

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

ควบคุมเครื่องมือออกแบบจาก terminal ของคุณ `op design`, `op insert` — batch design DSL, จัดการ node Pipe จากไฟล์หรือ stdin ทำงานร่วมกับแอปเดสก์ท็อปหรือ web server

</td>
<td width="50%">

### 🎯 ส่งออกโค้ดหลายแพลตฟอร์ม

ส่งออกจากไฟล์ `.op` ไฟล์เดียวไปยัง React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native Design variables กลายเป็น CSS custom properties

</td>
</tr>
</table>

## เริ่มต้นอย่างรวดเร็ว

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

หรือรันเป็นแอปพลิเคชัน Desktop:

```bash
cargo run -p op-host-desktop
```

> **ข้อกำหนดเบื้องต้น:** ต้องใช้ [Rust](https://www.rust-lang.org/) (stable) เพื่อ build ผลิตภัณฑ์ [Bun](https://bun.sh/) >= 1.0 และ [Node.js](https://nodejs.org/) >= 18 จำเป็นเฉพาะสำหรับ web SDK ใน `packages/` เท่านั้น

### Docker

Rust release ที่มี tag จะเผยแพร่ image แบบ web host เพียงตัวเดียว ส่วน image TypeScript เดิมที่รวม AI CLI จะไม่ถูกเผยแพร่อีกต่อไป

| Image | รวม |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host, wasm bundle และ CanvasKit assets |

Web UI แสดงเฉพาะ built-in agent profiles เท่านั้น เครื่องมือ Claude/Codex/OpenCode/Copilot/Gemini CLI จะไม่ถูกบันเดิลใน Docker images

**รัน:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

จากนั้นเปิด `http://localhost:3100/`

**Build ในเครื่อง:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## การออกแบบที่ขับเคลื่อนด้วย AI

**จาก Prompt สู่ UI**

- **ข้อความเป็นดีไซน์** — อธิบายหน้า แล้วสร้างขึ้นบน Canvas แบบเรียลไทม์พร้อม animation แบบ streaming
- **Orchestrator** — แบ่งหน้าที่ซับซ้อนออกเป็น sub-task เชิงพื้นที่เพื่อการสร้างแบบขนาน
- **การแก้ไขดีไซน์** — เลือกองค์ประกอบ แล้วอธิบายการเปลี่ยนแปลงด้วยภาษาธรรมชาติ
- **Vision input** — แนบ screenshot หรือ mockup เพื่อใช้เป็นข้อมูลอ้างอิงในการออกแบบ

**รองรับหลาย Agent**

| Agent                       | วิธีตั้งค่า                                                                                    |
| --------------------------- | ---------------------------------------------------------------------------------------------- |
| **ในตัว (9+ ผู้ให้บริการ)** | เลือกจากพรีเซ็ตผู้ให้บริการพร้อมตัวสลับภูมิภาค — Anthropic, OpenAI, Google, DeepSeek และอื่น ๆ |
| **Claude Code**             | ไม่ต้องตั้งค่า — ใช้ Claude Agent SDK พร้อม local OAuth                                        |
| **Codex CLI**               | เชื่อมต่อใน Agent Settings (`Cmd+,`)                                                           |
| **OpenCode**                | เชื่อมต่อใน Agent Settings (`Cmd+,`)                                                           |
| **GitHub Copilot**          | `copilot login` จากนั้นเชื่อมต่อใน Agent Settings (`Cmd+,`)                                    |
| **Gemini CLI**              | เชื่อมต่อใน Agent Settings (`Cmd+,`)                                                           |

**โปรไฟล์ความสามารถของโมเดล** — ปรับ prompt, โหมด thinking และ timeout ตามระดับโมเดลโดยอัตโนมัติ โมเดลระดับเต็ม (Claude) ได้ prompt ครบถ้วน; โมเดลระดับมาตรฐาน (GPT-4o, Gemini, DeepSeek) ปิด thinking; โมเดลระดับพื้นฐาน (MiniMax, Qwen, Llama, Mistral) ได้ prompt แบบ nested-JSON ที่ย่อลงเพื่อความเสถียรสูงสุด

**i18n** — การแปลภาษาเต็มรูปแบบใน 15 ภาษา: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia

**MCP Server**

- MCP Server ในตัว (`op-mcp` crate) — ติดตั้งได้ด้วยคลิกเดียวใน Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- ไม่ต้องใช้ Node.js — stdio transport ผ่าน binary เดสก์ท็อป (`--mcp <path>`) พร้อม live HTTP endpoint (`127.0.0.1:<port>/mcp`) จากแอปที่กำลังทำงาน
- การทำ Design automation จาก terminal: อ่าน สร้าง และแก้ไขไฟล์ `.op` ผ่าน agent ที่รองรับ MCP
- **Layered design workflow** — `design_skeleton` → `design_content` → `design_refine` สำหรับดีไซน์หลายส่วนที่มีความละเอียดสูงขึ้น
- **Segmented prompt retrieval** — โหลดเฉพาะความรู้ด้านดีไซน์ที่ต้องการ (schema, layout, roles, icons, planning ฯลฯ)
- รองรับหลายหน้า — สร้าง เปลี่ยนชื่อ เรียงลำดับ และทำซ้ำหน้าผ่าน MCP tools

**การสร้างโค้ด**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

ติดตั้งแบบ global และควบคุมเครื่องมือออกแบบจาก terminal ของคุณ:

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # เปิดแอปเดสก์ท็อป
op start --headless --file design.op # เริ่มเซิร์ฟเวอร์แบบ headless
op design @landing.txt       # ออกแบบแบบ batch จากไฟล์
op design @ui.js             # JavaScript แบบ sandbox พร้อม loop
op insert '{"type":"rectangle"}' # แทรก node
op import:figma design.fig   # นำเข้าไฟล์ Figma
cat design.dsl | op design - # Pipe จาก stdin
```

รองรับสตริง inline, `@filepath` และ stdin (`-`) ทำงานร่วมกับแอปเดสก์ท็อป เว็บเซิร์ฟเวอร์ หรือเซิร์ฟเวอร์ headless ที่ใช้ไฟล์ ดูคำสั่งทั้งหมดใน [คู่มือคำสั่ง CLI](./crates/op-cli/src/usage.txt)

**LLM Skill** — ติดตั้งปลั๊กอิน [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) เพื่อสอน AI agent ออกแบบด้วย `op` ใช้ `op install` สำหรับ agent ที่ตรวจพบ หรือ `op install --target codex` เพื่อระบุเป้าหมาย

## ฟีเจอร์

**Canvas และการวาด**

- Canvas ไม่จำกัดขนาดพร้อม pan, zoom, smart alignment guides และ snapping
- Rectangle, Ellipse, Line, Polygon, Pen (Bezier), Frame, Text
- การดำเนินการบูลีน — รวม ลบ ตัดกัน พร้อมแถบเครื่องมือตามบริบท
- ตัวเลือก Icon (Iconify) และนำเข้ารูปภาพ (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — แนวตั้ง/แนวนอนพร้อม gap, padding, justify, align
- เอกสารหลายหน้าพร้อมการนำทางด้วย tab

**Design System**

- Design variables — color, number, string tokens พร้อมการอ้างอิง `$variable`
- รองรับหลาย theme — หลาย axis แต่ละ axis มี variants (Light/Dark, Compact/Comfortable)
- ระบบ Component — component ที่นำกลับมาใช้ใหม่ได้พร้อม instance และ override
- CSS sync — สร้าง custom properties อัตโนมัติ, `var(--name)` ในผลลัพธ์โค้ด
- UIKits ที่นำกลับมาใช้ใหม่ได้ — นำเข้า/ส่งออกชุด component จากไฟล์ `.pen`

**AI และ Agents**

- Prompt-to-canvas พร้อมการสร้างแบบ streaming และการแตกย่อยเชิงพื้นที่ที่ขับเคลื่อนโดย orchestrator
- Concurrent Agent Teams — นักออกแบบหลายคนทำงานกับส่วนต่าง ๆ พร้อมกัน พร้อมตัวบ่งชี้บน canvas สำหรับแต่ละสมาชิก
- Layered workflow — `design_skeleton` → `design_content` → `design_refine` พร้อม prompt ที่เน้นเฉพาะในแต่ละเฟส
- Style Guides — สไตล์ในตัวกว่า 50 แบบ (glassmorphism, brutalist, retro ฯลฯ) พร้อม fuzzy matching ตาม tag ใช้ในการวางแผนและการสร้าง
- โปรไฟล์ความสามารถหลายโมเดล — ปรับโหมดการคิด ความพยายาม และรูปแบบ prompt อัตโนมัติตามระดับโมเดล
- Agent runtime ในตัว (Rust) + ผู้ให้บริการ Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Passthrough รูปแบบ Anthropic สำหรับผู้ให้บริการ LLM จีน — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**การเชื่อมต่อ Git**

- ตัวช่วย Clone พร้อมการยืนยันตัวตน SSH / HTTPS และการจัดการ SSH key
- ตัวเลือก branch — สร้าง สลับ ลบ merge ทั้งหมดจาก Git panel
- Cascade สำหรับ pull / push พร้อม auth retry และการจัดการ non-fast-forward
- Three-way merge โหมดโฟลเดอร์พร้อมการติดตามสถานะ `MERGE_HEAD` บนดิสก์
- Conflict panel พร้อมการ์ด three-way ต่อ node / field, ตัวแก้ไข JSON inline, bulk actions และบล็อก diff inline
- UI สำหรับตั้งค่า remote และ SSH keys; i18n 15 ภาษาครอบคลุมทั้งส่วนของ Git

**การส่งออก**

- ส่งออก Canvas — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- ส่งออกโค้ด — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Pipeline การสร้างโค้ด MCP แบบเพิ่มทีละส่วน — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**นำเข้าจาก Figma**

- นำเข้าไฟล์ `.fig` โดยคงไว้ซึ่ง layout, fills, strokes, effects, text, images และ vectors

**Desktop App**

- รองรับ macOS, Windows และ Linux แบบ native — binary แบบ self-contained เดียว (winit + GPU Skia, ไม่มี Electron)
- เชื่อมโยงไฟล์ `.op` — ดับเบิลคลิกเพื่อเปิด, single-instance lock
- ตรวจสอบอัปเดตแบบเบื้องหลังจาก GitHub Releases
- เมนูแอปพลิเคชันแบบ native พร้อม Save As, Open Recent และกล่องโต้ตอบการเปลี่ยนแปลงที่ไม่ได้บันทึกเมื่อปิด
- การเก็บรักษาไฟล์ล่าสุด

## Tech Stack

|                |                                                                                          |
| -------------- | ---------------------------------------------------------------------------------------- |
| **Core**       | Rust workspace (`crates/`) — editor state, widgets, hosts, MCP, AI, codegen              |
| **Rendering**  | GPU Skia ทุกที่ — `skia-safe` (GL) บน native, CanvasKit (WASM/WebGL2) บนเบราว์เซอร์      |
| **เฟรมเวิร์ก UI** | jian — เฟรมเวิร์ก UI แบบ GPU-Skia ที่เป็น pure-Rust และ vendored: widgets, layout, events, hot reload (`vendor/jian`) |
| **Windowing**  | winit (vendored `casement` fork)                                                         |
| **Desktop**    | Native binary `openpencil-desktop` — ไม่มี browser engine                                |
| **Web SDK**    | `op-web-sdk` + React 19 / Vue 3 adapters — `.op` viewer แบบอ่านอย่างเดียว (TypeScript)   |
| **CLI**        | `op` — ควบคุมจาก terminal, batch design DSL                                              |
| **AI**         | Agent runtime ในตัว (Rust) · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**       | clippy · rustfmt (Rust) · oxlint · oxfmt (web SDK)                                       |
| **รูปแบบไฟล์** | `.op` — ใช้ JSON, อ่านได้โดยมนุษย์, Git-friendly                                         |

## ระบบนิเวศ

OpenPencil เป็นส่วนหนึ่งของตระกูลเครื่องมือแบบ pure-Rust ที่ขับเคลื่อนด้วย AI จาก **[ZSeven-W](https://github.com/ZSeven-W)** ซึ่งประกอบเข้าด้วยกัน: `jian` เรนเดอร์ OpenPencil, `agent-rs` รันเอเจนต์ของมัน, `noema` จดจำ, และ `zode` ออกแบบจากเทอร์มินัล

| โปรเจกต์ | คืออะไร |
| -------- | ------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | ผู้ช่วยเขียนโค้ดแบบโอเพนซอร์สที่ขับเคลื่อนด้วย AI สำหรับเทอร์มินัลของคุณ — Rust TUI (`ratatui`) ที่รวดเร็วซึ่งอ่านโค้ดของคุณ รันคำสั่ง ค้นหาไฟล์ และจัดการ git ขับเคลื่อน OpenPencil ผ่าน MCP |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | async runtime แบบ pure-Rust สำหรับส่งมอบ LLM agent — รองรับหลายผู้ให้บริการ ใช้เครื่องมือได้แบบ end-to-end สิทธิ์แบบมีโครงสร้าง MCP จริง และ `unsafe` เป็นศูนย์ ขับเคลื่อน agent runtime ในตัวของ OpenPencil (`vendor/agent`) และ Zode |
| **[jian](https://github.com/ZSeven-W/jian)** | เฟรมเวิร์ก UI แบบ GPU-Skia ที่เป็น pure-Rust — widgets, layout, events และ hot reload ในสแตกเดียว เปลี่ยนเอกสาร `.op` แบบ declarative ให้เป็นแอปแบบ native ที่ควบคุมด้วย AI ได้ โดยไม่มี JS runtime, ไม่มี DOM, ไม่มี Electron เฟรมเวิร์ก UI ของ OpenPencil (`vendor/jian`) |
| **[noema](https://github.com/ZSeven-W/noema)** | ระบบหน่วยความจำแบบ local-first ที่ไม่ใช่เวกเตอร์สำหรับ coding agent หน่วยความจำที่คงทนในรูปแบบไฟล์ที่ตรวจสอบได้ คิวสำหรับตรวจทานรายการใหม่ และการเรียกคืนแบบ lexical (ไม่ใช้ embedding) — ทำงานได้ทั้งบน Zode, Codex, Claude Code และ MCP runtime |

## ทำไมต้อง Rust

OpenPencil ถูกเขียนใหม่ตั้งแต่ต้นด้วย **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)) การเขียนใหม่เสร็จสมบูรณ์แล้ว — editor แบบ TypeScript + Electron ถูกเลิกใช้ที่ `v0.7.5` และ Rust workspace ในรีโปนี้คือตัวผลิตภัณฑ์: core แบบ native เดียวที่เล็กและเร็วกว่าอย่างเห็นได้ชัด และรองรับได้บนหลายแพลตฟอร์มจาก codebase เดียว

|                        | TypeScript + Electron (เลิกใช้แล้ว, `v0.7.5`) | Rust (ปัจจุบัน)                                                       |
| ---------------------- | --------------------------------------------- | -------------------------------------------------------------------- |
| **Desktop runtime**    | Electron — รวม Chromium + Node.js             | หน้าต่างแบบ native (`winit` + GPU Skia) ไม่มี browser engine         |
| **ขนาดบน Desktop**     | Chromium runtime เต็มรูปแบบต่อการติดตั้ง     | ไฟล์ binary เดียวที่พร้อมใช้งาน — **55.5 MB**                        |
| **ขนาด payload บนเว็บ** | JS + WASM bundle                              | **8.2 MB** wasm / **2.18 MB** gzip บนเครือข่าย                       |
| **การ Render**          | CanvasKit/Skia บนเว็บ                         | Skia backend เดียวที่เร่งด้วย GPU บน **ทุก** เป้าหมาย               |
| **หน่วยความจำ**         | JavaScript GC หยุดชั่วคราว                   | ไม่มี GC — Rust ownership latency คาดเดาได้                          |
| **Codebase**            | Web stack + Electron         | Rust workspace เดียว: editor · CLI · MCP · AI · codegen · Figma · Git |
| **เป้าหมาย**           | Web + desktop สอง stack แยกกัน               | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · browser — one core |

**ผลการวัดที่ได้จริง**

- **ขนาดเล็กมาก** — แอปเดสก์ท็อปทั้งหมดเป็น binary แบบ native เดียวขนาด **55.5 MB** แทนที่จะเป็น browser engine รวมกับ Node runtime เว็บบิลด์มีขนาด **8.2 MB** raw / **2.18 MB** gzip หลังจากแยก icon catalog (−48% บนเครือข่าย)
- **รองรับเอกสารขนาดใหญ่** — canvas ที่มี **10,000-node** (nested auto-layout สี่ระดับ) เขียน อ่าน และ snapshot layout **โดยไม่มี panic และ ~0% idle CPU**; การ snapshot layout ทั้งหมด 10k node คืนค่าใน **~0.68 s**
- **การโต้ตอบที่รวดเร็ว** — pan/zoom ไม่ต้อง re-serialize เอกสารทุก frame อีกต่อไป (การแก้ไข hot-path เพียงครั้งเดียวลด CPU ของ wheel-zoom จาก **~69% เหลือ ~0%**); การลากแก้ไข scene แบบ incremental การวัดตัวอักษรถูก cache และการวาดซ้ำรวมเป็นหนึ่งต่อ frame
- **Core เดียว ทุกหน้าจอ** — editor state และ render backend เดียวกันคอมไพล์ไปยัง native desktop, mobile และ browser ผ่าน WASM — ไม่ต้องคงการ implement แบบคู่ขนานให้ตรงกัน
- **GPU Skia ทุกที่** — native render ผ่าน `skia-safe` บน GL context; browser render ผ่าน CanvasKit บน WebGL2 — โค้ดการวาดเดียวกัน ผลลัพธ์เดียวกัน
- **Accessibility แบบ native** — AccessKit บน macOS, Windows และ Linux พร้อม DOM mirror บนเว็บ แทนที่จะพึ่งพา a11y tree ของ browser
- **Workspace ที่ตรวจสอบ type เดียว** — MCP host, CLI, AI providers, code generation, Figma import และ Git integration ทั้งหมดอยู่ใน Rust workspace เดียว พร้อม `cargo-deny` ตรวจสอบ supply-chain ใน CI

> **สถานะ:** editor แบบ TypeScript ถูกเลิกใช้ที่ `v0.7.5` และเหลืออยู่เพียงใน git history เท่านั้น รีโปนี้คือ Rust workspace ผลิตภัณฑ์ Rust อยู่ระหว่างการพัฒนาอย่างต่อเนื่อง (ดู Roadmap ด้านล่าง)

## โครงสร้างโปรเจกต์

```text
openpencil/
├── crates/                   Rust workspace — ตัวผลิตภัณฑ์
│   ├── op-editor-core/       Editor state ของ `.op` (PenDocument) ที่เป็นแหล่งจริง + EditorCommand + design variables
│   ├── op-editor-ui/         Widget ที่ไม่ผูกกับแพลตฟอร์ม + RenderBackend facade (wasm32-clean)
│   ├── op-editor-host-core/  Host state machine ที่ไม่ผูกกับ transport ใช้ร่วมกันทุก host
│   ├── op-host-native/       Native host lib — winit + skia-safe GL (เดสก์ท็อป + มือถือ)
│   ├── op-host-web/          Browser bundle — wasm32 cdylib, CanvasKit renderer
│   ├── op-host-desktop/      Desktop binary `openpencil-desktop`; เป็น daemon `--serve-web` ด้วย
│   ├── op-host-services/     Headless serve-web / MCP daemon lib
│   ├── op-host-web-server/   Web-server binary แบบไม่มี GL
│   ├── op-cli/               เครื่องมือ CLI — คำสั่ง `op`
│   ├── op-mcp/               MCP server — tools, batch design, layered workflow
│   ├── op-ai/                AI providers, chat runtime, streaming
│   ├── op-ai-skills/         AI prompt skill engine (โหลด prompt ตามเฟส)
│   ├── op-orchestrator/      การจัดการทีม agent ที่ทำงานพร้อมกัน
│   ├── op-codegen/           Code generators (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma .fig file parser และ converter
│   ├── op-git/               การเชื่อมต่อ Git — clone, branch, push/pull, merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Web SDK workspace (Bun)
│   ├── op-web-sdk/           `.op` web viewer SDK แบบอ่านอย่างเดียว (wrap wasm bundle)
│   ├── op-web-sdk-react/     React 19 adapter
│   └── op-web-sdk-vue/       Vue 3 adapter
├── vendor/                   Subsystem แบบ vendored (git submodules)
│   ├── jian/                 เฟรมเวิร์ก UI แบบ GPU-Skia — widget/render/event
│   ├── casement/             winit fork
│   └── agent/                Rust agent runtime ข้ามผลิตภัณฑ์ (agent-rs)
└── .githooks/                การตรวจสอบความคลาดเคลื่อนของเวอร์ชันก่อน commit
```

## คีย์ลัด

| คีย์        | การทำงาน    |     | คีย์          | การทำงาน                  |
| ----------- | ----------- | --- | ------------- | ------------------------- |
| `V`         | เลือก       |     | `Cmd+S`       | บันทึก                    |
| `R`         | Rectangle   |     | `Cmd+Z`       | เลิกทำ                    |
| `O`         | Ellipse     |     | `Cmd+Shift+Z` | ทำซ้ำ                     |
| `L`         | Line        |     | `Cmd+C/X/V/D` | คัดลอก/ตัด/วาง/ทำซ้ำ      |
| `T`         | Text        |     | `Cmd+G`       | จัดกลุ่ม                  |
| `F`         | Frame       |     | `Cmd+Shift+G` | ยกเลิกการจัดกลุ่ม         |
| `P`         | Pen tool    |     | `Cmd+Shift+P` | ส่งออก (PNG/JPG/WEBP/PDF) |
| `H`         | Hand (pan)  |     | `Cmd+Shift+C` | Code panel                |
| `Del`       | ลบ          |     | `Cmd+Shift+V` | Variables panel           |
| `[ / ]`     | เรียงลำดับ  |     | `Cmd+J`       | AI chat                   |
| ลูกศร       | เลื่อน 1px  |     | `Cmd+,`       | Agent settings            |
| `Cmd+Alt+U` | รวมบูลีน    |     | `Cmd+Alt+S`   | ลบบูลีน                   |
| `Cmd+Alt+I` | ตัดกันบูลีน |     | `Cmd+Shift+S` | บันทึกเป็น                |

## Scripts

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

# การซิงค์เวอร์ชัน (เรียกใช้จากรากของ repository)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## การมีส่วนร่วม

ยินดีต้อนรับการมีส่วนร่วมทุกรูปแบบ! ดู [CLAUDE.md](./CLAUDE.md) สำหรับรายละเอียดสถาปัตยกรรมและรูปแบบโค้ด

1. Fork และ clone
2. เปิดใช้การตรวจสอบความคลาดเคลื่อนของเวอร์ชัน: `git config core.hooksPath .githooks`
3. สร้าง branch: `git checkout -b feat/my-feature`
4. รันการตรวจสอบ: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. Commit ด้วย [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. เปิด PR เข้า `main`

## Roadmap

- [x] Design variables และ tokens พร้อม CSS sync
- [x] ระบบ Component (instances และ overrides)
- [x] การสร้างดีไซน์ด้วย AI พร้อม orchestrator
- [x] การเชื่อมต่อ MCP server พร้อม layered design workflow
- [x] รองรับหลายหน้า
- [x] นำเข้า Figma `.fig`
- [x] Boolean operations (union, subtract, intersect)
- [x] โปรไฟล์ความสามารถหลายโมเดล
- [x] Cargo workspace พร้อม Rust crate และแพ็กเกจ Web SDK ที่นำกลับมาใช้ใหม่ได้
- [x] Rust editor สำหรับเดสก์ท็อปและ Web
- [x] เครื่องมือ CLI (`op`) ควบคุมจาก terminal
- [x] Rust Agent Runtime ในตัว รองรับหลายผู้ให้บริการ
- [x] i18n — 15 ภาษา
- [x] Viewer SDK ที่ใช้ wasm สำหรับ JavaScript, React และ Vue
- [x] Style Guides พร้อมการจับคู่ตามแท็กและเครื่องมือ MCP
- [x] Concurrent Agent Teams พร้อมการมอบหมายงานและตัวบ่งชี้บน Canvas
- [x] การเชื่อมต่อ Git (clone, branch, push/pull, three-way merge โหมดโฟลเดอร์)
- [x] การส่งออก Canvas (SVG / PNG / JPEG / WEBP / PDF)
- [ ] การแก้ไขร่วมกัน
- [ ] ระบบปลั๊กอิน

## ผู้มีส่วนร่วม

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## ผู้สนับสนุน

OpenPencil เป็นซอฟต์แวร์ฟรีและโอเพนซอร์ส การพัฒนาได้รับการสนับสนุนจากผู้ที่เห็นว่ามันมีประโยชน์ — ขอบคุณที่ช่วยให้ผืนผ้าใบยังคงเปิดอยู่

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

ขอบคุณ **[MrQyun](https://github.com/mrqyun)** — อยากเห็นชื่อของคุณตรงนี้ใช่ไหม? **[เป็นผู้สนับสนุน →](https://github.com/sponsors/ZSeven-W)**

## ชุมชน

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> เข้าร่วม Discord ของเรา</strong>
</a>
— ถามคำถาม แชร์ดีไซน์ เสนอฟีเจอร์

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## การประเมิน

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## สัญญาอนุญาต

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
