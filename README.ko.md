<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>세계 최초의 오픈소스 AI 네이티브 벡터 디자인 툴.</strong><br />
  <sub>동시 에이전트 팀 &bull; 디자인-애즈-코드 &bull; 내장 MCP 서버 &bull; 멀티 모델 인텔리전스</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md"><b>한국어</b></a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>이미지를 클릭하여 데모 영상 보기</sub></p>

<br />

> **참고:** 같은 이름의 다른 오픈소스 프로젝트가 있습니다 — [OpenPencil](https://github.com/open-pencil/open-pencil). 해당 프로젝트는 Figma 호환 비주얼 디자인과 실시간 협업에 중점을 둡니다. 본 프로젝트는 AI 네이티브 디자인-투-코드 워크플로에 중점을 둡니다.

## OpenPencil을 선택하는 이유

<table>
<tr>
<td width="50%">

### 🎨 프롬프트 → 캔버스

자연어로 어떤 UI든 설명하세요. 무한 캔버스 위에 스트리밍 애니메이션과 함께 실시간으로 나타납니다. 요소를 선택하고 대화하여 기존 디자인을 수정할 수 있습니다.

</td>
<td width="50%">

### 🤖 동시 에이전트 팀

오케스트레이터가 복잡한 페이지를 공간적 서브태스크로 분해합니다. 여러 AI 에이전트가 히어로, 기능 소개, 푸터 등 각기 다른 섹션을 동시에 작업하며 모두 병렬로 스트리밍됩니다.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 멀티 모델 인텔리전스

각 모델의 역량에 자동 적응합니다. Claude는 사고 모드가 포함된 전체 프롬프트를 받고, GPT-4o/Gemini는 사고 모드를 비활성화하며, 소형 모델(MiniMax, Qwen, Llama)은 안정적인 출력을 위해 단순화된 프롬프트를 받습니다.

</td>
<td width="50%">

### 🔌 MCP 서버

Claude Code, Codex, Gemini, OpenCode, Kiro 또는 Copilot CLI에 원클릭 설치. 터미널에서 디자인하세요 — MCP 호환 에이전트를 통해 `.op` 파일을 읽고, 생성하고, 편집할 수 있습니다.

</td>
</tr>
<tr>
<td width="50%">

### 📦 디자인-애즈-코드

`.op` 파일은 JSON입니다 — 사람이 읽을 수 있고, Git 친화적이며, diff가 가능합니다. 디자인 변수는 CSS 커스텀 프로퍼티를 생성합니다. React + Tailwind 또는 HTML + CSS로 코드 내보내기가 가능합니다.

</td>
<td width="50%">

### 🖥️ 어디서든 실행

웹 앱 + macOS, Windows, Linux 네이티브 데스크톱 — 하나의 Rust 코어, 단일 자급자족 바이너리, 브라우저 엔진 없음. `.op` 파일 연결 — 더블 클릭으로 열기.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

터미널에서 디자인 도구 제어. `op design`, `op insert` — 배치 디자인 DSL, 노드 조작. 파일이나 stdin에서 파이프 입력 지원. 데스크톱 앱 또는 웹 서버와 연동.

</td>
<td width="50%">

### 🎯 멀티 플랫폼 코드 내보내기

하나의 `.op` 파일에서 React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native로 내보내기. 디자인 변수는 CSS 커스텀 프로퍼티로 변환.

</td>
</tr>
</table>

## 빠른 시작

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

또는 데스크톱 앱으로 실행:

```bash
cargo run -p op-host-desktop
```

> **필수 조건:** 제품을 빌드하려면 [Rust](https://www.rust-lang.org/)(stable)가 필요합니다. [Bun](https://bun.sh/) >= 1.0 및 [Node.js](https://nodejs.org/) >= 18은 `packages/` 아래의 web SDK에만 필요합니다.

### Docker

태그된 Rust 릴리스는 단일 web-host 이미지를 게시합니다. AI CLI가 포함되던 은퇴한 TypeScript 이미지는 더 이상 게시되지 않습니다.

| 이미지 | 포함 내용 |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:vX.Y.Z` | Rust web host, wasm bundle, CanvasKit assets |

웹 UI는 내장 agent profiles만 노출합니다. Claude/Codex/OpenCode/Copilot/Gemini CLI 도구는 Docker 이미지에 포함되지 않습니다.

**실행:**

```bash
VERSION="$(scripts/workspace-version.sh)"
docker run -d -p 3100:3100 "ghcr.io/zseven-w/openpencil-web:v${VERSION}"
```

그런 다음 `http://localhost:3100/`을 여세요.

**로컬 빌드:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI 네이티브 디자인

**프롬프트에서 UI로**

- **텍스트-투-디자인** — 페이지를 설명하면 스트리밍 애니메이션으로 실시간으로 캔버스에 생성
- **오케스트레이터** — 복잡한 페이지를 공간적 서브태스크로 분해하여 병렬 생성
- **디자인 수정** — 요소를 선택하고 자연어로 변경 사항을 설명
- **비전 입력** — 스크린샷이나 목업을 참조로 첨부하여 디자인

**멀티 에이전트 지원**

| 에이전트             | 설정 방법                                                                       |
| -------------------- | ------------------------------------------------------------------------------- |
| **내장 (9+ 제공자)** | 제공자 프리셋에서 선택하고 지역을 전환 — Anthropic, OpenAI, Google, DeepSeek 등 |
| **Claude Code**      | 설정 불필요 — 로컬 OAuth로 Claude Agent SDK 사용                                |
| **Codex CLI**        | 에이전트 설정에서 연결 (`Cmd+,`)                                                |
| **OpenCode**         | 에이전트 설정에서 연결 (`Cmd+,`)                                                |
| **GitHub Copilot**   | `copilot login` 후 에이전트 설정에서 연결 (`Cmd+,`)                             |
| **Gemini CLI**       | 에이전트 설정에서 연결 (`Cmd+,`)                                                |

**모델 역량 프로파일** — 모델 티어에 따라 프롬프트, 사고 모드, 타임아웃을 자동 조정합니다. 풀 티어 모델(Claude)은 완전한 프롬프트를 받고, 스탠다드 티어(GPT-4o, Gemini, DeepSeek)는 사고 모드를 비활성화하며, 베이직 티어(MiniMax, Qwen, Llama, Mistral)는 최대 안정성을 위해 단순화된 중첩 JSON 프롬프트를 받습니다.

**i18n** — 15개 언어로 완전한 인터페이스 지역화: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**MCP 서버**

- 내장 MCP 서버 (`op-mcp` 크레이트) — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI에 원클릭 설치
- Node.js 불필요 — 데스크톱 바이너리(`--mcp <path>`)를 통한 stdio 전송, 그리고 실행 중인 앱이 제공하는 실시간 HTTP 엔드포인트(`127.0.0.1:<port>/mcp`)
- 터미널에서 디자인 자동화: MCP 호환 에이전트를 통해 `.op` 파일 읽기, 생성, 편집
- **계층적 디자인 워크플로** — `design_skeleton` → `design_content` → `design_refine`으로 더 높은 충실도의 멀티 섹션 디자인
- **세그먼트 프롬프트 검색** — 필요한 디자인 지식만 로드 (schema, layout, roles, icons, planning 등)
- 멀티 페이지 지원 — MCP 도구를 통해 페이지 생성, 이름 변경, 순서 변경, 복제

**코드 생성**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

전역 설치 후 터미널에서 디자인 도구를 제어하세요:

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # 데스크톱 앱 실행
op start --headless --file design.op # 헤드리스 서버 실행
op design @landing.txt       # 파일에서 배치 디자인
op design @ui.js             # 반복문을 지원하는 샌드박스 JavaScript
op insert '{"type":"rectangle"}' # 노드 삽입
op import:figma design.fig   # Figma 파일 가져오기
cat design.dsl | op design - # stdin에서 파이프 입력
```

인라인 문자열, `@filepath`, stdin(`-`)을 지원하며 데스크톱 앱, 웹 서버 또는 파일 기반 헤드리스 서버와 연동됩니다. 전체 명령은 [CLI 명령 레퍼런스](./crates/op-cli/src/usage.txt)를 참고하세요.

**LLM 스킬** — [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill)을 설치해 AI 에이전트가 `op`로 디자인하도록 안내하세요. 감지된 에이전트에는 `op install`, 특정 대상에는 `op install --target codex`를 실행합니다.

## 기능

**캔버스 & 드로잉**

- 팬, 줌, 스마트 정렬 가이드, 스냅 지원의 무한 캔버스
- Rectangle, Ellipse, Line, Polygon, Pen(Bezier), Frame, Text
- 불리언 연산 — 합치기, 빼기, 교차 (컨텍스트 툴바)
- 아이콘 피커(Iconify)와 이미지 가져오기(PNG/JPEG/SVG/WebP/GIF)
- 오토 레이아웃 — 수직/수평 방향, gap, padding, justify, align 지원
- 탭 내비게이션이 있는 멀티 페이지 문서

**디자인 시스템**

- 디자인 변수 — 컬러, 숫자, 문자열 토큰, `$variable` 참조 지원
- 멀티 테마 지원 — 여러 테마 축, 각 축에 변형(Light/Dark, Compact/Comfortable)
- 컴포넌트 시스템 — 인스턴스와 오버라이드를 가진 재사용 가능한 컴포넌트
- CSS 동기화 — 커스텀 프로퍼티 자동 생성, 코드 출력에 `var(--name)` 사용
- 재사용 가능한 UIKit — `.pen` 파일에서 컴포넌트 킷을 가져오기/내보내기

**AI & 에이전트**

- 스트리밍 생성과 오케스트레이터 기반 공간 분해를 통한 프롬프트-투-캔버스
- 동시 에이전트 팀 — 여러 디자이너가 다양한 섹션에서 병렬로 작업하며 멤버별 캔버스 인디케이터 제공
- 계층적 워크플로 — `design_skeleton` → `design_content` → `design_refine`, 각 단계에 집중된 프롬프트 사용
- 스타일 가이드 — 50개 이상의 내장 스타일(glassmorphism, brutalist, retro 등), 태그 기반 퍼지 매칭 지원, 플래닝과 생성에 통합
- 멀티 모델 역량 프로파일 — 모델 등급별로 싱킹 모드, 노력도, 프롬프트 형태를 자동 적응
- 내장 에이전트 런타임(Rust) + Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini 제공자
- 중국 LLM 제공자를 위한 Anthropic 형식 패스스루 — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Git 통합**

- SSH / HTTPS 인증과 SSH 키 관리를 지원하는 클론 마법사
- 브랜치 피커 — 생성, 전환, 삭제, 병합을 모두 Git 패널에서
- 인증 재시도와 non-fast-forward 처리가 포함된 Pull / Push 캐스케이드
- 디스크 상의 `MERGE_HEAD` 상태 추적을 포함한 폴더 모드 3방향 병합
- 노드/필드 단위 3방향 카드, 인라인 JSON 편집기, 일괄 작업, 인라인 diff 블록을 갖춘 충돌 패널
- 원격 설정 및 SSH 키 UI; Git 전반에 걸친 15개 언어 i18n

**내보내기**

- 캔버스 내보내기 — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- 코드 내보내기 — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- 증분 MCP 코드 생성 파이프라인 — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Figma 가져오기**

- 레이아웃, 채우기, 선, 효과, 텍스트, 이미지, 벡터를 유지하며 `.fig` 파일 가져오기

**데스크톱 앱**

- 네이티브 macOS, Windows, Linux — 단일 자급자족 바이너리 (winit + GPU Skia, Electron 없음)
- `.op` 파일 연결 — 더블 클릭으로 열기, 단일 인스턴스 잠금
- GitHub Releases 대비 백그라운드 업데이트 확인
- 다른 이름으로 저장, 최근 항목 열기, 닫을 때 저장되지 않은 변경 사항 대화상자를 지원하는 네이티브 애플리케이션 메뉴
- 최근 파일 영속성

## 기술 스택

|               |                                                                                |
| ------------- | ------------------------------------------------------------------------------ |
| **코어**      | Rust 워크스페이스 (`crates/`) — 에디터 상태, 위젯, 호스트, MCP, AI, 코드젠     |
| **렌더링**    | 모든 곳에서 GPU Skia — 네이티브는 `skia-safe` (GL), 브라우저는 CanvasKit (WASM/WebGL2) |
| **UI 프레임워크** | jian — 벤더링된 순수 Rust GPU-Skia UI 프레임워크: 위젯, 레이아웃, 이벤트, 핫 리로드 (`vendor/jian`) |
| **윈도잉**    | winit (벤더링된 `casement` 포크)                                               |
| **데스크톱**  | 네이티브 바이너리 `openpencil-desktop` — 브라우저 엔진 없음                    |
| **웹 SDK**    | `op-web-sdk` + React 19 / Vue 3 어댑터 — 읽기 전용 `.op` 뷰어 (TypeScript)      |
| **CLI**       | `op` — 터미널 제어, 배치 디자인 DSL                                            |
| **AI**        | 내장 Rust 에이전트 런타임 · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **린트**      | clippy · rustfmt (Rust) · oxlint · oxfmt (웹 SDK)                              |
| **파일 형식** | `.op` — JSON 기반, 사람이 읽을 수 있는, Git 친화적                             |

## 에코시스템

OpenPencil은 **[ZSeven-W](https://github.com/ZSeven-W)**의 순수 Rust, AI 네이티브 도구 제품군의 일부입니다. 이들은 서로 맞물려 작동합니다: `jian`이 OpenPencil을 렌더링하고, `agent-rs`가 그 에이전트를 실행하며, `noema`가 기억하고, `zode`가 터미널에서 디자인합니다.

| 프로젝트 | 내용 |
| ------- | ---------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | 터미널을 위한 오픈소스 AI 네이티브 코딩 어시스턴트 — 코드를 읽고, 명령을 실행하고, 파일을 검색하고, git을 관리하는 빠른 Rust TUI(`ratatui`). MCP를 통해 OpenPencil을 구동합니다. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM 에이전트를 배포하기 위한 순수 Rust 비동기 런타임 — 멀티 프로바이더, 엔드투엔드 도구 지원, 구조화된 권한, 진짜 MCP, `unsafe` 제로. OpenPencil의 내장 에이전트 런타임(`vendor/agent`)과 Zode를 구동합니다. |
| **[jian](https://github.com/ZSeven-W/jian)** | 순수 Rust GPU-Skia UI 프레임워크 — 위젯, 레이아웃, 이벤트, 핫 리로드를 하나의 스택에. 선언적 `.op` 문서를 JS 런타임도, DOM도, Electron도 없는 네이티브하고 AI로 제어 가능한 앱으로 바꿉니다. OpenPencil의 UI 프레임워크(`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | 코딩 에이전트를 위한 로컬 우선, 비벡터 메모리 시스템. 검사 가능한 파일로서의 지속적 메모리, 새 항목을 위한 리뷰 큐, 그리고 어휘 기반(임베딩 없는) 회상 — Zode, Codex, Claude Code, MCP 런타임 전반에서 작동합니다. |

## Rust를 선택한 이유

OpenPencil은 처음부터 **Rust**로 다시 작성되었습니다 ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). 재작성이 완료되었습니다 — TypeScript + Electron 에디터는 `v0.7.5`에서 은퇴했고, 이 저장소의 Rust 워크스페이스가 바로 제품입니다: 단일 네이티브 코어로 크기는 획기적으로 줄고 속도는 빨라지며, 하나의 코드베이스에서 더 많은 플랫폼을 지원합니다.

|                         | TypeScript + Electron (은퇴, `v0.7.5`)                    | Rust (현재)                                                              |
| ----------------------- | --------------------------------------------------------- | ------------------------------------------------------------------------ |
| **데스크톱 런타임**     | Electron — Chromium + Node.js 번들                        | 네이티브 윈도우 (`winit` + GPU Skia), 브라우저 엔진 없음                 |
| **데스크톱 설치 크기**  | 설치마다 전체 Chromium 런타임 포함                        | 단일 자급자족 바이너리 — **55.5 MB**                                     |
| **웹 페이로드**         | JS + WASM 번들                                            | **8.2 MB** wasm / 전송 시 **2.18 MB** gzip                              |
| **렌더링**              | 웹에서 CanvasKit/Skia                                     | **모든** 타깃에서 GPU 가속 Skia 백엔드 하나                              |
| **메모리**              | JavaScript GC 일시 중단                                   | GC 없음 — Rust 소유권, 예측 가능한 레이턴시                              |
| **코드베이스**          | 웹 스택 + Electron                    | 단일 Rust 워크스페이스: 에디터 · CLI · MCP · AI · 코드젠 · Figma · Git  |
| **지원 플랫폼**         | 웹 + 데스크톱, 두 개의 별도 스택                         | 데스크톱 (macOS/Win/Linux) · 모바일 (iOS/Android) · 브라우저 — 코어 하나 |

**측정된 개선 사항**

- **작은 설치 크기** — 전체 데스크톱 앱이 번들된 브라우저 엔진과 Node 런타임 대신 단 하나의 **55.5 MB** 네이티브 바이너리입니다. 아이콘 카탈로그 분할 후 웹 빌드는 원본 **8.2 MB** / gzip **2.18 MB**입니다 (전송량 −48%).
- **대용량 문서 확장성** — **10,000-node** 라이브 캔버스(중첩 오토 레이아웃, 4단계 깊이)에서 쓰기, 읽기, 레이아웃 스냅샷을 **패닉 없이 유휴 CPU ~0%**로 처리하며, 10k 노드 전체 레이아웃 스냅샷이 **~0.68 s** 안에 반환됩니다.
- **빠른 인터랙션** — 팬/줌 시 매 프레임마다 문서를 재직렬화하지 않습니다 (핫 경로 수정 하나로 휠 줌 CPU 사용률이 **~69%에서 ~0%로** 감소). 드래그는 씬을 증분 방식으로 패치하고, 텍스트 측정은 캐시되며, 리페인트는 프레임당 하나로 합산됩니다.
- **코어 하나로 모든 화면** — 동일한 에디터 상태와 동일한 렌더 백엔드가 WASM을 통해 네이티브 데스크톱, 모바일, 브라우저로 컴파일됩니다 — 동기화가 필요한 별도 구현체가 없습니다.
- **어디서나 GPU Skia** — 네이티브는 GL 컨텍스트 위의 `skia-safe`로 렌더링하고, 브라우저는 WebGL2 위의 CanvasKit으로 렌더링합니다 — 동일한 드로잉 코드, 동일한 출력.
- **네이티브 접근성** — 브라우저의 접근성 트리에 의존하는 대신 macOS, Windows, Linux에서는 AccessKit을, 웹에서는 DOM 미러를 사용합니다.
- **타입 검사된 단일 워크스페이스** — MCP 호스트, CLI, AI 제공자, 코드 생성, Figma 가져오기, Git 통합이 모두 단일 Rust 워크스페이스 안에 있으며, CI에서 `cargo-deny`로 공급망을 검증합니다.

> **상태:** TypeScript 에디터는 `v0.7.5`에서 은퇴하여 이제 git 히스토리에만 남아 있으며, 이 저장소는 Rust 워크스페이스입니다. Rust 제품은 활발히 개발 중입니다 (아래 로드맵 참고).

## 프로젝트 구조

```text
openpencil/
├── crates/                   Rust 워크스페이스 — 제품 본체
│   ├── op-editor-core/       정규 `.op` (PenDocument) 에디터 상태 + EditorCommand + 디자인 변수
│   ├── op-editor-ui/         플랫폼 독립적 위젯 + RenderBackend 파사드 (wasm32-clean)
│   ├── op-editor-host-core/  모든 호스트가 공유하는 전송 계층 독립적 호스트 상태 머신
│   ├── op-host-native/       네이티브 호스트 라이브러리 — winit + skia-safe GL (데스크톱 + 모바일)
│   ├── op-host-web/          브라우저 번들 — wasm32 cdylib, CanvasKit 렌더러
│   ├── op-host-desktop/      데스크톱 바이너리 `openpencil-desktop`; `--serve-web` 데몬 겸용
│   ├── op-host-services/     헤드리스 serve-web / MCP 데몬 라이브러리
│   ├── op-host-web-server/   GL 없는 경량 web-server 바이너리
│   ├── op-cli/               CLI 도구 — `op` 명령어
│   ├── op-mcp/               MCP 서버 — 도구, 배치 디자인, 계층적 워크플로
│   ├── op-ai/                AI 제공자, 채팅 런타임, 스트리밍
│   ├── op-ai-skills/         AI 프롬프트 스킬 엔진 (단계별 프롬프트 로딩)
│   ├── op-orchestrator/      동시 에이전트 팀 오케스트레이션
│   ├── op-codegen/           코드 생성기 (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Figma .fig 파일 파서 및 변환기
│   ├── op-git/               Git 통합 — 클론, 브랜치, 푸시/풀, 병합
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 웹 SDK 워크스페이스 (Bun)
│   ├── op-web-sdk/           읽기 전용 `.op` 웹 뷰어 SDK (wasm 번들을 래핑)
│   ├── op-web-sdk-react/     React 19 어댑터
│   └── op-web-sdk-vue/       Vue 3 어댑터
├── vendor/                   벤더링된 서브시스템 (git submodules)
│   ├── jian/                 GPU-Skia UI 프레임워크 — 위젯/렌더/이벤트
│   ├── casement/             winit 포크
│   └── agent/                제품 공용 Rust 에이전트 런타임 (agent-rs)
└── .githooks/                pre-commit 버전 드리프트 검사
```

## 키보드 단축키

| 키          | 동작          |     | 키            | 동작                        |
| ----------- | ------------- | --- | ------------- | --------------------------- |
| `V`         | 선택          |     | `Cmd+S`       | 저장                        |
| `R`         | 사각형        |     | `Cmd+Z`       | 실행 취소                   |
| `O`         | 타원          |     | `Cmd+Shift+Z` | 다시 실행                   |
| `L`         | 직선          |     | `Cmd+C/X/V/D` | 복사/잘라내기/붙여넣기/복제 |
| `T`         | 텍스트        |     | `Cmd+G`       | 그룹화                      |
| `F`         | Frame         |     | `Cmd+Shift+G` | 그룹 해제                   |
| `P`         | 펜 툴         |     | `Cmd+Shift+P` | 내보내기 (PNG/JPG/WEBP/PDF) |
| `H`         | 핸드(팬)      |     | `Cmd+Shift+C` | 코드 패널                   |
| `Del`       | 삭제          |     | `Cmd+Shift+V` | 변수 패널                   |
| `[ / ]`     | 순서 변경     |     | `Cmd+J`       | AI 채팅                     |
| 화살표 키   | 1px 이동      |     | `Cmd+,`       | 에이전트 설정               |
| `Cmd+Alt+U` | 불리언 합치기 |     | `Cmd+Alt+S`   | 불리언 빼기                 |
| `Cmd+Alt+I` | 불리언 교차   |     | `Cmd+Shift+S` | 다른 이름으로 저장          |

## 스크립트

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

# 버전 동기화 (저장소 루트에서 실행)
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## 기여하기

기여를 환영합니다! 아키텍처 세부 정보와 코드 스타일은 [CLAUDE.md](./CLAUDE.md)를 참고하세요.

1. 포크 후 클론
2. 버전 드리프트 검사 설정: `git config core.hooksPath .githooks`
3. 브랜치 생성: `git checkout -b feat/my-feature`
4. 검사 실행: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. [Conventional Commits](https://www.conventionalcommits.org/) 형식으로 커밋: `feat(canvas): add rotation snapping`
6. `main` 브랜치에 PR 생성

## 로드맵

- [x] CSS 동기화가 있는 디자인 변수 & 토큰
- [x] 컴포넌트 시스템 (인스턴스 & 오버라이드)
- [x] 오케스트레이터를 통한 AI 디자인 생성
- [x] 계층적 디자인 워크플로가 포함된 MCP 서버 통합
- [x] 멀티 페이지 지원
- [x] Figma `.fig` 가져오기
- [x] 불리언 연산 (합치기, 빼기, 교차)
- [x] 멀티 모델 역량 프로파일
- [x] 재사용 가능한 Rust crate 및 Web SDK 패키지를 포함한 Cargo workspace
- [x] 데스크톱 및 Web용 Rust 에디터
- [x] CLI 도구 (`op`) 터미널 제어
- [x] 내장 Rust Agent Runtime (멀티 제공자 지원)
- [x] i18n — 15개 언어
- [x] JavaScript, React 및 Vue용 wasm 기반 Viewer SDK
- [x] 태그 기반 매칭 및 MCP 도구를 제공하는 Style Guides
- [x] 위임 및 캔버스 표시기를 제공하는 Concurrent Agent Teams
- [x] Git 통합 (클론, 브랜치, 푸시/풀, 폴더 모드 3방향 병합)
- [x] 캔버스 내보내기 (SVG / PNG / JPEG / WEBP / PDF)
- [ ] 공동 편집
- [ ] 플러그인 시스템

## 기여자

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 스폰서

OpenPencil은 무료이며 오픈소스입니다. 개발은 이 도구를 유용하게 쓰는 분들의 후원으로 이어집니다 — 캔버스를 열어 두어 주셔서 감사합니다.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

**[MrQyun](https://github.com/mrqyun)** 님, 감사합니다 — 여기에 이름을 올리고 싶으신가요? **[스폰서 되기 →](https://github.com/sponsors/ZSeven-W)**

## 커뮤니티

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord에 참여하기</strong>
</a>
— 질문하기, 디자인 공유, 기능 제안.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## 보안 평가

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## 라이선스

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
