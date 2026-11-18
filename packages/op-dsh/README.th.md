<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>ปลั๊กอิน DeepSeek Harness สำหรับ OpenPencil — ดูตัวอย่าง ตรวจสอบ และแก้ไขเอกสาร <code>.op</code> จริงภายในบทสนทนา</strong><br />
  <sub>พรีวิวหลายเฟรมที่แม่นยำ &bull; แคนวาสแบบอินเทอร์แอกทีฟ &bull; โปรแกรมแก้ไขแบบจัดการ &bull; เครื่องมือออกแบบสำหรับ Agent โดยเฉพาะ</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · เวอร์ชันปลั๊กอินปัจจุบัน: <code>0.1.0-rc.7</code> · ทดสอบถึง DSH <code>0.1.1-rc.2</code></sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md"><b>ไทย</b></a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><img src="https://img.shields.io/npm/v/%40zseven-w%2Fdsh-openpencil?style=flat&color=cfb537" alt="npm" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/dsh-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/dsh-check.yml?label=CI" alt="CI" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — พรีวิวหลายเฟรมและโปรแกรมแก้ไขแถบด้านข้าง" width="100%" />
</p>
<p align="center"><sub>พรีวิวหลายเฟรม <code>.op</code> ที่แม่นยำ พร้อมแคนวาสแบบอินเทอร์แอกทีฟและเวิร์กเบนช์โปรแกรมแก้ไขแบบจัดการ</sub></p>

## ทำไมต้อง DSH OpenPencil

DSH OpenPencil เชื่อมต่อ [DeepSeek Harness](https://github.com/deepseek-ai/DSH) เข้ากับ [OpenPencil](https://github.com/ZSeven-W/openpencil) เพื่อให้ Agent ขับเคลื่อนแคนวาสออกแบบจริงที่แก้ไขได้และโต้ตอบได้ แทนที่จะส่งกลับเป็นเพียงรูปภาพที่ถูกสร้างขึ้น

<table>
<tr>
<td width="50%">

### 🖼️ พรีวิวหลายเฟรมที่แม่นยำ

ตัวส่งออกแบบ headless ของ OpenPencil ที่ติดตั้งไว้จะเรนเดอร์พรีวิวที่ตรงตามงานออกแบบ: เฟรมระดับบนสุดเฟรมแรกเป็น PNG ขนาดใหญ่ที่เล่นซ้ำได้อย่างปลอดภัย พร้อมแถบภาพขนาดย่อที่เลื่อนแนวนอนได้ คลิกเพื่อเลือก และปุ่มนำทางก่อนหน้า/ถัดไปสำหรับเอกสารหลายเฟรม

</td>
<td width="50%">

### 🗺️ แคนวาสแบบอินเทอร์แอกทีฟ

คำสั่ง "Open interactive canvas" จะโหลด OpenPencil Web SDK แบบอ่านอย่างเดียวแบบ lazy พร้อมการแพน ซูม และจัดให้พอดี — ตรวจสอบหน้าใด ๆ โหนดที่ซ้อนกัน หรือหน้าที่ไม่ได้ใช้งานได้โดยไม่ต้องออกจากบทสนทนา

</td>
</tr>
<tr>
<td width="50%">

### ✏️ โปรแกรมแก้ไขแบบจัดการ

เมื่อตั้งค่า `editable: true` การแก้ไขจะเปิดโปรแกรมแก้ไข OpenPencil แบบจัดการ — การเลือก เลเยอร์ คุณสมบัติ เครื่องมือวาดภาพ เลิกทำ/ทำซ้ำ และความหมายของการบันทึกที่ชัดเจน — ในเวิร์กเบนช์ด้านขวาที่ปรับขนาดได้พร้อมตัวเลือกเต็มหน้าจอ

</td>
<td width="50%">

### 🤖 เครื่องมือออกแบบสำหรับ Agent โดยเฉพาะ

เครื่องมือจัดการแคนวาสโดยตรงห้าตัวและเครื่องมือ `openpencil_pipeline_*` อีกหกตัว ให้ Agent สร้าง ตรวจสอบ ปรับแต่ง เผยแพร่ แก้ไข และอ่านแคนวาสจริงผ่าน runtime ของ OpenPencil แบบจัดการ

</td>
</tr>
<tr>
<td width="50%">

### 🔐 การอนุญาตแบบกำหนดด้วยความสามารถ

สิทธิ์ภาพและเอกสารเป็นความสามารถที่ลงนามและผูกกับแฮช เมตาดาต้าในเบราว์เซอร์ไม่เปิดเผยพาธของโฮสต์ตามอำเภอใจ และความสามารถพรีวิว/แก้ไขที่ลงนามจะไม่เข้าไปในผลลัพธ์เครื่องมือหลักหรือบริบทของโมเดล

</td>
<td width="50%">

### ⚡ ความปลอดภัยแบบ Transactional

เอกสารใน pipeline เต็มรูปแบบจะคงเป็น draft ส่วนตัวที่ยังไม่เผยแพร่จนกว่าจะผ่าน quality gate ทั้งหมดของ native runtime และ DSH การเผยแพร่จะไม่เขียนทับพาธที่มีอยู่ และการยกเลิกหรือ batch ที่ล้มเหลวจะไม่ทิ้ง target ว่างไว้

</td>
</tr>
<tr>
<td width="50%">

### 🌍 ตามรูปลักษณ์และความรู้สึกของ DSH

การ์ดเครื่องมือและโปรแกรมแก้ไขแบบจัดการเป็นไปตามภาษา จีน/อังกฤษ และธีมสว่าง/มืดของ DSH โดยไม่ต้องโหลดเซสชันการแก้ไขใหม่

</td>
<td width="50%">

### 🎯 เวิร์กโฟลว์ครบวงจรเดียว

"ความต้องการ → private live canvas ใน sidebar → semantic batches สองชุดพร้อม exact PNG สำหรับผู้ใช้ → deterministic quality gates ของ native/DSH → เผยแพร่แบบ atomic" — วงจรที่ครบถ้วนภายใน DSH

</td>
</tr>
</table>

## ติดตั้งลงใน DSH

DSH เป็นแพ็กเกจแยกต่างหาก ถ้ายังไม่มีให้ติดตั้งหนึ่งครั้ง:

```sh
npm install -g @deepseek-ai/dsh@latest
```

จากนั้นเพิ่มปลั๊กอินเข้าโปรไฟล์แล้วเริ่มเว็บแอป:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

สำหรับการพัฒนาในเครื่อง ให้ build checkout นี้ เชื่อม path แบบ absolute เข้ากับโปรไฟล์ Web แล้วรีสตาร์ท DSH ทั้งหมด:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

dependency แบบ `link:` ทำให้ผลการ build ครั้งถัดไปมองเห็นได้โดยตรงจาก checkout นี้ แต่ต้องรีสตาร์ท DSH ทั้งหมดหลังเปลี่ยน dependency ของโปรไฟล์ เพราะโปรไฟล์ Web ที่ให้มาไม่ได้ hot-reload bundle ฝั่ง host โดยค่าเริ่มต้น

ไม่อยากติดตั้ง DSH แบบ global? รันสองขั้นตอนเดียวกันผ่าน `pnpm dlx`:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> ปลั๊กอิน OpenPencil เป็นแบบสาธารณะและไม่ต้องใช้ npm token หาก DSH prerelease เองต้องมีการยืนยันตัวตนของ registry ให้เก็บข้อมูลรับรองนั้นไว้ใน npm config ระดับผู้ใช้หรือชั่วคราวนอก checkout ที่เก็บนี้ตั้งใจไม่ให้มีข้อมูลรับรอง registry ใด ๆ

## เครื่องมือออกแบบ

การเลือกสไตล์เสริมใช้ `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin` โดยส่ง brief เดิมและ `style_selection` การดูแค็ตตาล็อกและเลือกไกด์ตรงไม่ต้องใช้คำแนะนำจากโมเดล โดยผู้ให้คำแนะนำปิดไว้ตามค่าเริ่มต้น การส่งต่อนี้รองรับ script ไม่รองรับ app-agent หรือ commerce แต่การสร้าง app-agent แบบปกติยังใช้ได้

| เครื่องมือ | หน้าที่ |
| --- | --- |
| `openpencil_new` | เส้นทางด่วนที่เข้ากันได้สำหรับงานง่าย: รันสคริปต์ QuickJS `batch_design` แบบ transactional หนึ่งชุด เผยแพร่เมื่อ target ยังไม่มีเท่านั้น และคืน presentation ที่แก้ไขได้ สำหรับงาน production ควรใช้ pipeline เต็มรูปแบบด้านล่าง |
| `openpencil_pipeline_begin` | เริ่ม draft ส่วนตัวของเซสชันพร้อม root เพียงตัวเดียว เปิด live canvas เดียวกันใน sidebar ทันที และคง target `.op` ไว้โดยยังไม่เผยแพร่ |
| `openpencil_pipeline_context` | โหลด guideline, style, theme หรือรายละเอียด UI kit แบบจำกัดเพียงหนึ่งรายการเมื่อ begin contract ขาดจริงเท่านั้น ไม่ใช่ startup refresh loop |
| `openpencil_pipeline_batch` | รันสคริปต์สร้างแบบ QuickJS โดยตรงได้สูงสุดสองชุด แต่ละ transaction ที่สำเร็จเพียงพยายามแสดง exact PNG ให้ผู้ใช้ เมื่อเครื่องมือคืนผลให้ทำตาม `next` ทันที |
| `openpencil_pipeline_inspect` | ให้ manual diagnostic เฉพาะเมื่อผู้ใช้ร้องขออย่างชัดเจน การสร้างตามปกติจะไม่ใช้เป็นขั้นตอน preview หรือให้โมเดลตรวจภาพ |
| `openpencil_pipeline_finish` | รัน deterministic gates ของ native และ DSH เรนเดอร์ exact PNG หลัง finalization และเผยแพร่แบบ atomic ด้วย `createIfAbsent` อนุญาต U-only repair หนึ่งครั้งและ finish ครั้งสุดท้ายเฉพาะเมื่อผลเป็น `needs_correction`, `canContinue: true`, มี `repairTargets` ครบและไม่ว่าง และ `omitted: 0`; ผลอื่นที่ยังไม่เผยแพร่ถือเป็น terminal |
| `openpencil_pipeline_abort` | ทิ้ง draft ที่ยังไม่เผยแพร่โดยไม่สร้างไฟล์ target |
| `openpencil_create` | ใช้โปรแกรม `batch_design` แบบ transactional เพื่อสร้างหรือจัดโครงสร้างโหนดใหม่บนแคนวาสสดที่มีอยู่ |
| `openpencil_edit` | แก้ไขโหนดที่ระบุชัดเจนหรือโหนดเดียวที่ผู้ใช้เลือก |
| `openpencil_render` | สร้างสแนปช็อต `.op` แบบ immutable และอ้างอิงตามเนื้อหา แล้วเรนเดอร์เฟรมระดับบนสุดทุกเฟรมบนหน้าที่ใช้งานอยู่ — รองรับ `scale` และ `editable` แบบไม่บังคับ |
| `openpencil_selection` | อ่านโหนดที่เลือกอยู่จริงในแคนวาสของโปรแกรมแก้ไขสด |

## เวิร์กโฟลว์การออกแบบของ Agent

สำหรับการสร้างตามปกติให้ใช้เส้นทางสั้นแบบตายตัว: `openpencil_pipeline_begin` → `openpencil_pipeline_batch` แบบ QuickJS โดยตรงสองครั้ง → `openpencil_pipeline_finish` หนึ่งครั้ง Begin สร้าง root เพียงตัวเดียวและเปิด private live canvas ใน sidebar ทันที โดยพาธ workspace ที่ขอจะยังไม่มีอยู่จนกว่าการเผยแพร่จะสำเร็จ หลัง begin หรือ batch สำเร็จแต่ละครั้ง ให้เรียกขั้นตอนถัดไปทันทีโดยไม่มีคำบรรยาย การวางแผน การเปรียบเทียบ การตรวจสอบ หรือเครื่องมือที่ไม่เกี่ยวข้อง ค่าเริ่มต้นคือใช้ image เพียงหนึ่งรายการทั้ง design และแต่ละ Hero/Product/Art/Media frame มี primary visual เพียงหนึ่งอย่าง ห้ามวาง image คู่กับ placeholder icon

batch ที่สำเร็จทั้งสองชุดจะพยายามแสดง exact PNG preview ให้ผู้ใช้ เมื่อเครื่องมือคืนผลให้ทำตาม `next` ทันที หาก `next` รายงาน `previewUnavailable` แสดงว่าสคริปต์ commit ลง live canvas แล้ว ห้ามรัน batch ซ้ำ และห้ามเรียก `openpencil_pipeline_inspect` หรือ `read_image` โดย `openpencil_pipeline_inspect` ใช้เฉพาะ diagnostic ที่ผู้ใช้ร้องขออย่างชัดเจน

Finish รัน native finalization, lint, contrast และ layout checks ของ OpenPencil พร้อม deterministic quality gates ของ DSH แล้วเรนเดอร์ exact final PNG และเผยแพร่ target แบบ atomic ใน healthy call เดียว อนุญาต repair เฉพาะเมื่อผลมี `stage: "needs_correction"`, `canContinue: true`, `repairTargets` ที่ครบและไม่ว่างพร้อม `omitted: 0` และทุก target มี `operation: "U"`, `nodeId` ที่ตรงและไม่ว่าง และ `patch` ที่ไม่ว่าง ให้นำ target ทั้งหมดไปใช้พร้อมกันใน U-only batch เพียงหนึ่งครั้ง แล้วเรียก finish ครั้งสุดท้ายอีกหนึ่งครั้งโดยไม่มีคำบรรยาย ผลอื่นที่ยังไม่เผยแพร่ error หรือ `canContinue: false` ถือเป็น terminal: รายงานครั้งเดียวและห้าม retry, inspect, อ่าน image/context, abort หรือสร้าง draft ใหม่ หาก gate ล้มเหลวหรือเรียก `openpencil_pipeline_abort` จะไม่สร้าง target exact final PNG และ live editor ใน published result เป็น authoritative อยู่แล้ว ให้คืนผลและจบทันที sidebar จะ auto-open เฉพาะเมื่อ idle และยังมี **แก้ไขแคนวาส** สำหรับสลับอย่างชัดเจนเสมอ

ภายใน DSH service เดิมที่ยังทำงานอยู่ เมื่อเปลี่ยน browser หรือ reload จะสามารถกู้ strictly parsed durable publication จาก `openpencil_new` หรือ `openpencil_pipeline_finish` กลับมาเป็น exact PNG พร้อม action **แก้ไขแคนวาส** ที่ชัดเจน การ์ดประวัติจะไม่ auto-open sidebar ผู้ใช้ต้องคลิก action นี้เอง `openpencil_render` แบบประวัติทั่วไปยังคง read-only และ connection ที่ไม่ใช่ loopback จะไม่ได้รับ editor grant

skill `openpencil-design` ที่บันเดิลมายังคงเป็นคู่มือด้าน scripting และคุณภาพ ส่วน runtime แบบจัดการไม่ขึ้นกับไบนารีเดสก์ท็อป `openpencil_new` ยังคงเป็นเส้นทางด่วนแบบ single-batch ที่เข้ากันได้ แต่งานสร้างระดับ production ควรเลือก pipeline เต็มรูปแบบ

ใช้ `openpencil_create` และ `openpencil_edit` เฉพาะกับแคนวาสสดที่มีอยู่เท่านั้น การแก้ไขของทั้งคู่จะยังไม่ถูกบันทึกจนกว่าผู้ใช้จะกดบันทึกในโปรแกรมแก้ไข

## สินทรัพย์ Web Viewer

DSH ให้บริการเฉพาะ `client.js` สำหรับปลั๊กอินฝั่งไคลเอนต์ ดังนั้น OpenPencil ESM SDK, WASM และ CanvasKit จึงถูกจัดเตรียมเป็นสินทรัพย์ same-origin อย่างชัดเจน:

```sh
pnpm run sync:viewer-assets
```

คำสั่ง sync จะใช้รากของ monorepo OpenPencil ที่ครอบอยู่ (`../..`) เป็นค่าเริ่มต้น โดยสำรองเป็น checkout `../openpencil` ที่อยู่ข้างเคียง ใช้ `OPENPENCIL_ROOT` หรือ `--openpencil-root` เพื่อแทนที่ ไลบรารีสินทรัพย์ที่ build ไว้ล่วงหน้าทั้งหมดสามารถเลือกได้ด้วย `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE` และการค้นหาตอนรันสามารถแทนที่ได้ด้วย `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR`

สินทรัพย์ viewer จะถูกโหลดแบบ lazy หลังจากผู้ใช้เปิดแคนวาสเท่านั้น หากสินทรัพย์ไม่มีหรือไม่ถูกต้อง พรีวิว PNG ยังคงใช้งานได้ และจะไม่มีการแสดงปุ่มแคนวาส

## โปรแกรมแก้ไขแบบจัดการ

เซสชันที่แก้ไขได้ใช้ managed web host ของ OpenPencil — สถาปัตยกรรมเดียวกับที่ `op-vscode` ใช้ ปลั๊กอินจะเริ่ม host ก็ต่อเมื่อมีการกระทำของผู้ใช้ที่ได้รับอนุญาต เก็บโทเค็น daemon ไว้ในหน่วยความจำ ตรวจสอบ source และ origin ของ iframe และปิดโพรเซสเมื่อเซสชันของโปรแกรมแก้ไขสิ้นสุด พื้นผิวตัวแก้ไขจะถูกเลือกแบบค่อยเป็นค่อยไป: รายละเอียดเครื่องมือแบบเนทีฟเมื่อ host ประกาศจุดเชื่อมต่อนั้น มิฉะนั้นจะใช้เวิร์กเบนช์ด้านขวาของปลั๊กอินพร้อมการปรับขนาดและปุ่มเต็มหน้าจอ

การเริ่มทำงานใช้ listening handshake ที่ปลอดภัยสำหรับ mount ที่ช้า โดยจะเริ่ม probe ความพร้อมหลังจาก host ที่มากับแพ็กเกจประกาศ address ที่ bind แล้วเท่านั้น ไม่ต้องติดตั้ง OpenPencil เดสก์ท็อป

การติดตั้งรุ่นเผยแพร่จะเลือกแพ็กเกจที่ตรงกับ OS/CPU ปัจจุบันจากแพ็กเกจแพลตฟอร์มเนทีฟหกแพ็กเกจ ได้แก่ `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64` และ `win32-x64` โดยแพ็กเกจ Linux ทั้งสองใช้ glibc แพ็กเกจรากประกาศแพ็กเกจเหล่านี้เป็น `optionalDependencies` แบบระบุเวอร์ชันตรงตัว เพื่อให้ตัวจัดการแพ็กเกจเลือกตัวแปรที่ถูกต้อง (เช่น `@zseven-w/dsh-openpencil-darwin-arm64`) แพ็กเกจนี้จัดส่ง `op-host-web-server`, เว็บบันเดิลของโปรแกรมแก้ไข และ CanvasKit ที่เข้าชุดกันเป็น runtime เดียว ดังนั้นโปรแกรมแก้ไขแบบจัดการจึงไม่ต้องพึ่ง `/Applications/OpenPencil.app`, `openpencil-desktop` ใน `PATH` หรือ checkout ซอร์สของ OpenPencil

หาก DSH โหลดซ้ำหรือยกเลิกการโหลดปลั๊กอินในขณะที่แคนวาสยังไม่บันทึก host จะเก็บร่างกู้คืนในเครื่องแบบ opaque ไว้นานสูงสุดเจ็ดวัน การเปิด source เดียวกันซ้ำจะถามก่อนกู้คืนลงในแคนวาสสด การกู้คืนจะไม่เขียนทับไฟล์ `.op` จนกว่าผู้ใช้จะบันทึกอย่างชัดเจน

แพ็กเกจอย่างเป็นทางการสำหรับทั้งหกแพลตฟอร์มจะได้รับการฉีด endpoint bootstrap สำหรับการทำงานร่วมกันในจีนและทั่วโลกระหว่าง build release ที่ได้รับการป้องกัน และค่าที่ฉีดจะถูกตรวจสอบก่อนเผยแพร่ สำหรับ build ที่สร้างเองในเครื่องโดยไม่มีการฉีดนี้ สามารถ override bootstrap ก่อนเริ่ม DSH ด้วย `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap` โดยค่าต้องใช้ `https` และ path ต้องเป็น `/api/v1/collaboration/bootstrap` อย่างเคร่งครัด

การซิงค์แคนวาสข้ามอุปกรณ์กำหนดให้ทั้ง runtime เนทีฟของ PC/DSH และแอปมือถืออัปเดตเป็นสาย release เดียวกันของ OpenPencil ที่มีการแก้ไขคิวการทำงานร่วมกันปัจจุบัน การใช้แอปมือถือรุ่นเก่าร่วมกับ runtime PC รุ่นใหม่กว่าอาจยังเห็นเคอร์เซอร์ระยะไกล แต่ไม่ได้รับ commit ของแคนวาส

เมื่อพัฒนาจาก repository นี้ ก่อนเริ่ม DSH ให้ build Web bundle ของโปรแกรมแก้ไขก่อน ตามด้วย native host แล้วจึง stage runtime ชุดที่ตรงกัน

`pnpm run build:editor-web` จะเรียก WASM bundle gate ที่ OpenPencil รองรับอย่างเป็นทางการ โดยต้องมี Bash, Cargo/Rust พร้อม target `wasm32-unknown-unknown`, CLI `wasm-bindgen`, `wasm-opt` จาก Binaryen, Node.js และ `gzip`; CanvasKit ไม่ต้องใช้ EMSDK Web build จะไม่ใช้ตัวแปร build ของ bootstrap สำหรับการทำงานร่วมกัน ก่อนเรียก `pnpm run build:editor-runtime` ต้องตั้งค่าทั้ง `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN` และ `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL` ตัวแปรเหล่านี้ใช้เฉพาะกับ native Cargo build ซึ่งจะ fail closed หากขาดตัวใดตัวหนึ่ง หลังจากทั้งสอง build สำเร็จแล้ว ให้ stage runtime ด้วยคำสั่งสุดท้าย

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

การ override runtime โดยระบุเองจะยอมรับเฉพาะเมื่อให้ครบทั้งชุดที่ตรงกัน:

- `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY` สำหรับ `op-host-web-server`;
- `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR` สำหรับเว็บบันเดิลของโปรแกรมแก้ไขที่ build แล้ว;
- `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR` สำหรับสินทรัพย์ CanvasKit

การให้เพียงบางรายการถือเป็นการกำหนดค่าที่ไม่ถูกต้อง ปลั๊กอินจะไม่ผสม path ที่กำหนดเองกับสินทรัพย์ runtime ที่มากับแพ็กเกจ

การบันทึกใช้แฮช source แบบ optimistic การแทนที่แบบ atomic และความสามารถของผู้สืบทอด หาก source เปลี่ยนแปลงนอกโปรแกรมแก้ไข ปลั๊กอินจะรายงานความขัดแย้งแทนที่จะเขียนทับ

## เมตาดาต้าของผลลัพธ์

ผลลัพธ์ที่โมเดลมองเห็นยังคงเป็น JSON ธรรมดา `presentationMeta.$dshOpenPencil` สำหรับเบราว์เซอร์เท่านั้นถือสิทธิ์แบบเพิ่มเติมสำหรับ:

- `image`: พาธ PNG, URL พรีวิว/ดาวน์โหลด และความกว้าง/ความสูงจริง;
- `frames`: เฟรมระดับบนสุดทุกเฟรมที่เรนเดอร์อย่างแม่นยำตามลำดับหน้าที่ใช้งานอยู่ รวมถึง node id/name/index และ URL PNG ที่ลงนาม;
- `document`: พาธการกระทำ source พร้อม URL สแนปช็อตแบบ immutable, ขนาดไบต์ และ SHA-256;
- `viewer`: URL SDK/WASM/CanvasKit แบบมีรุ่นเมื่อเชื่อมต่อเส้นทางสินทรัพย์;
- `editor`: ความสามารถเปิด/รีเฟรชแบบจำกัดขอบเขตเมื่อ `editable: true` ได้รับอนุญาต

ผลลัพธ์ยังบันทึก `renderer`, `rendererBinary`, `fidelity` และคำเตือนใด ๆ ข้อความ schema-v1 ที่เป็น PNG เท่านั้นยังคงเรนเดอร์ได้

DSH `0.1.1-rc.2` จะไม่คงเมตาดาต้าการนำเสนอในเบราว์เซอร์สำหรับเครื่องมือที่ซ้อนอยู่ใต้ PTC/Code Mode ปลั๊กอินจะกู้คืน projection แบบ UI-only นั้นผ่าน endpoint แบบ same-origin ที่ผูกกับเซสชัน: เบราว์เซอร์ส่งเฉพาะ session id, call id และ SHA-256 ของเอกสารแบบ immutable ในขณะที่ host แก้ไขผลลัพธ์ฉบับหลักจากบันทึกเซสชัน DSH ที่ทนทาน และใช้ marker ในโพรเซสอายุสั้นเพื่ออนุญาตการแก้ไขสดล่าสุดเท่านั้น ความสามารถพรีวิว/แก้ไขที่ลงนามจะไม่เข้าไปในผลลัพธ์เครื่องมือหลักหรือบริบทของโมเดล durable history ของ `openpencil_render` ทั่วไปยังคง read-only ส่วน strictly parsed durable publication จาก `openpencil_new` หรือ `openpencil_pipeline_finish` จะได้รับ editor grant ได้เฉพาะผ่าน loopback และหลังผู้ใช้คลิกอย่างชัดเจนเท่านั้น การ auto-open sidebar สงวนไว้สำหรับ recent trusted live results

สำหรับการเล่นซ้ำแบบจำกัดขอบเขต การกู้คืนเมตาดาต้าแบบซ้อนยอมรับเฟรมระดับบนสุดสูงสุด 128 เฟรม ผลลัพธ์ Code Mode ที่ใหญ่กว่ายังคงเข้าถึงได้ผ่าน canonical JSON fallback

## ข้อจำกัดปัจจุบัน

- การแก้ไขต่อเนื่องบนแคนวาสที่มีอยู่ต้องมีโปรแกรมแก้ไขแบบจัดการที่เปิดไว้แล้ว การเปลี่ยนแปลงจะยังไม่ถูกบันทึกจนกว่าผู้ใช้จะเรียกใช้การบันทึก
- แคนวาส Web SDK แบบน้ำหนักเบาเป็นแบบอ่านอย่างเดียว การแก้ไขเต็มรูปแบบใช้พื้นผิวโปรแกรมแก้ไขแบบจัดการแยกต่างหาก บน DSH `0.1.1-rc.2` ปลั๊กอินใช้เวิร์กเบนช์ด้านขวาที่ปรับขนาดได้พร้อมตัวเลือกเต็มหน้าจอ
- แกลเลอรีที่แม่นยำครอบคลุมเฟรมระดับบนสุดบนหน้าที่ใช้งานอยู่ แคนวาสแบบอินเทอร์แอกทีฟยังคงเป็นวิธีตรวจสอบหน้าที่ไม่ได้ใช้งานและโหนดที่ซ้อนกัน
- แคชการเรนเดอร์และสแนปช็อตยังต้องมีนโยบายการเก็บรักษาระดับผลิตภัณฑ์

## โครงสร้างโปรเจกต์

```text
packages/op-dsh/
├── src/                       Plugin sources (TypeScript)
│   ├── index.ts               Host plugin entry — Cordis service, tools, assets
│   ├── tool.ts / design-tools.ts / new-tool.ts   Host-side design tools
│   ├── renderer.ts            Exact OpenPencil renderer + Jian fallback
│   ├── editor-host.ts / editor-recovery.ts       Managed editor lifecycle + drafts
│   ├── viewer-assets.ts       Web SDK / WASM / CanvasKit asset staging
│   ├── mcp-client.ts          OpenPencil MCP connection
│   └── client/                Browser client — React workbench, gallery, selection dock
├── lib/                       Compiled output (published to npm)
├── scripts/                   Build helpers — viewer asset sync, client build, host tests
├── tests/                     Node test suites (client, host API, MCP, viewer assets)
├── docs/images/               Documentation screenshots
├── cordis.patch.yml           DSH bundle patch that mounts the plugin
├── tsconfig.json              Host / Node TypeScript config
└── tsconfig.client.json       Browser client TypeScript config
```

## Build และตรวจสอบ

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

การ build ต้องใช้ Node 24.11 ขึ้นไปและ pnpm แพ็กเกจ host/client ของ DSH เป็น peer dependencies ที่มาจากโปรไฟล์ DSH เป้าหมาย เครื่องมือ build ถูกแก้ไขจาก dev dependencies ในเครื่อง, checkout DSH ที่ลิงก์อยู่ หรือ DSH source bundle ที่ติดตั้งไว้; `DSH_SOURCE_ROOT` สามารถเลือก source checkout ได้อย่างชัดเจน ล็อกไฟล์จะตรึงเครื่องมือ build สาธารณะแบบ standalone เมื่อสภาพแวดล้อมนั้นถูกจัดเตรียมแยกต่างหาก

สำหรับ DSH prerelease ส่วนตัว ให้เก็บข้อมูลรับรอง npm ที่ออกให้ไว้นอกที่เก็บนี้ (เช่นใน `.npmrc` ระดับผู้ใช้หรือชั่วคราว) และรันเวอร์ชันที่ต้องการโดยตรง:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

ห้าม commit `.npmrc`, `NPM_TOKEN` หรือข้อมูลรับรอง registry ที่คัดลอกมา ที่เก็บนี้ละเว้นการกำหนดค่า npm ในเครื่องตามค่าเริ่มต้น

`test:host` ทำการเรนเดอร์ที่แม่นยำจริง ตรวจสอบเรขาคณิต PNG IHDR และ SHA-256 ทดสอบความสามารถภาพ/เอกสารแบบ immutable ผ่าน HTTP และตรวจสอบว่าสินทรัพย์ viewer อนุญาตได้ ขนาดที่คาดหวังขึ้นอยู่กับฟิกซ์เจอร์แต่ละชุด

## ระบบนิเวศ

DSH OpenPencil คือปลั๊กอิน DeepSeek Harness สำหรับ **[OpenPencil](https://github.com/ZSeven-W/openpencil)** — เครื่องมือออกแบบเวกเตอร์แบบ AI-native โอเพนซอร์สตัวแรกของโลก — และเป็นส่วนหนึ่งของตระกูล **[ZSeven-W](https://github.com/ZSeven-W)** เครื่องมือแบบ AI-native ที่เขียนด้วย Rust บริสุทธิ์

| โปรเจกต์ | คำอธิบาย |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | เครื่องมือออกแบบที่ปลั๊กอินนี้ขับเคลื่อน — การสร้าง prompt-to-canvas, ทีม agent แบบทำงานพร้อมกัน, ไฟล์ `.op` แบบ design-as-code และ MCP server ในตัว พรีวิวที่แม่นยำ แคนวาสแบบอินเทอร์แอกทีฟ และโปรแกรมแก้ไขแบบจัดการที่นี่ขับเคลื่อนโดย OpenPencil เอง |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | รันไทม์ async แบบ Rust บริสุทธิ์สำหรับส่งมอบ LLM agent — รองรับหลายผู้ให้บริการ, มีความสามารถใช้เครื่องมือแบบ end-to-end, การอนุญาตแบบมีโครงสร้าง, MCP จริง, `unsafe` เป็นศูนย์ ขับเคลื่อนรันไทม์ agent ในตัวของ OpenPencil |
| **[jian](https://github.com/ZSeven-W/jian)** | เฟรมเวิร์ก UI แบบ GPU-Skia และ Rust บริสุทธิ์ — วิดเจ็ต เลย์เอาต์ อีเวนต์ และ hot reload ในสแตกเดียว เป็นเฟรมเวิร์ก UI ของ OpenPencil และเป็นที่มาของตัวเรนเดอร์ fallback ของปลั๊กอินนี้ |
| **[Zode](https://github.com/ZSeven-W/zode)** | ผู้ช่วยเขียนโค้ดแบบ AI-native โอเพนซอร์สสำหรับเทอร์มินัลของคุณ — อ่านโค้ดของคุณ รันคำสั่ง และขับเคลื่อน OpenPencil ผ่าน MCP |
| **[noema](https://github.com/ZSeven-W/noema)** | ระบบหน่วยความจำแบบ local-first ที่ไม่ใช่เวกเตอร์สำหรับ coding agent — หน่วยความจำที่ทนทานในรูปไฟล์ที่ตรวจสอบได้ ทำงานได้ข้ามรันไทม์ |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | ปลั๊กอินทักษะ LLM ที่สอน AI agent วิธีออกแบบด้วย `op` — คู่หูของปลั๊กอิน DSH นี้ |

ปลั๊กอิน DSH อื่นในตระกูลเดียวกัน:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — อีมูเลเตอร์ Android หรืออุปกรณ์ผ่าน USB แบบสดภายในบทสนทนา ขับเคลื่อนทั้งหมดผ่าน adb
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — มอบหมายงานให้เอเจนต์ DSH จาก Claude Code / Codex
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — iOS Simulator ที่ทำงานจริง — และ iPhone ที่ต่อผ่าน USB — ภายในบทสนทนา
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — หน่วยความจำระยะยาวสำหรับ DSH

## การมีส่วนร่วม

ยินดีต้อนรับการมีส่วนร่วม! Fork และ clone สร้าง branch รัน `pnpm run build` และชุดทดสอบ commit ด้วย [Conventional Commits](https://www.conventionalcommits.org/) และเปิด PR ไปที่ `main`

## ชุมชน

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> เข้าร่วม Discord ของเรา</strong>
</a>
— ถามคำถาม แบ่งปันงานออกแบบ แนะนำฟีเจอร์

**ชุมชนที่ได้รับการยอมรับ: [LINUX DO](https://linux.do/)**

## สัญญาอนุญาต

[MIT](./LICENSE) — ลิขสิทธิ์ (c) 2026 ZSeven-W

คอมโพเนนต์ของบุคคลที่สามระบุไว้ใน [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)
