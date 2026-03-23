<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>이미지를 클릭하여 데모 영상 보기</sub></p>

<br />

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

웹 앱 + Electron을 통한 macOS, Windows, Linux 네이티브 데스크톱. GitHub Releases에서 자동 업데이트. `.op` 파일 연결 — 더블 클릭으로 열기.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

터미널에서 디자인 도구 제어. `op design`, `op insert`, `op export` — 배치 디자인 DSL, 노드 조작, 코드 내보내기. 파일이나 stdin에서 파이프 입력 지원. 데스크톱 앱 또는 웹 서버와 연동.

</td>
<td width="50%">

### 🎯 멀티 플랫폼 코드 내보내기

하나의 `.op` 파일에서 React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native로 내보내기. 디자인 변수는 CSS 커스텀 프로퍼티로 변환.

</td>
</tr>
</table>

## 빠른 시작

```bash
# 의존성 설치
bun install

# http://localhost:3000 에서 개발 서버 시작
bun --bun run dev
```

또는 데스크톱 앱으로 실행:

```bash
bun run electron:dev
```

> **필수 조건:** [Bun](https://bun.sh/) >= 1.0 및 [Node.js](https://nodejs.org/) >= 18

### Docker

여러 이미지 변형을 사용할 수 있습니다 — 필요에 맞는 것을 선택하세요:

| 이미지 | 크기 | 포함 내용 |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | 웹 앱만 |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | 모든 CLI 도구 |

**실행 (웹만):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**AI CLI와 함께 실행 (예: Claude Code):**

AI 채팅은 Claude CLI OAuth 로그인에 의존합니다. Docker 볼륨을 사용하여 로그인 세션을 유지하세요:

```bash
# 1단계 — 로그인 (최초 1회)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# 2단계 — 시작
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**로컬 빌드:**

```bash
# 기본 (웹만)
docker build --target base -t openpencil .

# 특정 CLI 포함
docker build --target with-claude -t openpencil-claude .

# 전체 (모든 CLI)
docker build --target full -t openpencil-full .
```

## AI 네이티브 디자인

**프롬프트에서 UI로**
- **텍스트-투-디자인** — 페이지를 설명하면 스트리밍 애니메이션으로 실시간으로 캔버스에 생성
- **오케스트레이터** — 복잡한 페이지를 공간적 서브태스크로 분해하여 병렬 생성
- **디자인 수정** — 요소를 선택하고 자연어로 변경 사항을 설명
- **비전 입력** — 스크린샷이나 목업을 참조로 첨부하여 디자인

**멀티 에이전트 지원**

| 에이전트 | 설정 방법 |
| --- | --- |
| **Claude Code** | 설정 불필요 — 로컬 OAuth로 Claude Agent SDK 사용 |
| **Codex CLI** | 에이전트 설정에서 연결 (`Cmd+,`) |
| **OpenCode** | 에이전트 설정에서 연결 (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` 후 에이전트 설정에서 연결 (`Cmd+,`) |
| **Gemini CLI** | 에이전트 설정에서 연결 (`Cmd+,`) |

**모델 역량 프로파일** — 모델 티어에 따라 프롬프트, 사고 모드, 타임아웃을 자동 조정합니다. 풀 티어 모델(Claude)은 완전한 프롬프트를 받고, 스탠다드 티어(GPT-4o, Gemini, DeepSeek)는 사고 모드를 비활성화하며, 베이직 티어(MiniMax, Qwen, Llama, Mistral)는 최대 안정성을 위해 단순화된 중첩 JSON 프롬프트를 받습니다.

**MCP 서버**
- 내장 MCP 서버 — Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI에 원클릭 설치
- Node.js 자동 감지 — 설치되지 않은 경우 HTTP 전송 모드로 자동 대체하고 MCP HTTP 서버를 자동 시작
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
npm install -g @zseven-w/openpencil
```

```bash
op start                     # 데스크톱 앱 실행
op design @landing.txt       # 파일에서 배치 디자인
op insert '{"type":"RECT"}'  # 노드 삽입
op export react --out .      # React + Tailwind로 내보내기
op import:figma design.fig   # Figma 파일 가져오기
cat design.dsl | op design - # stdin에서 파이프 입력
```

세 가지 입력 방식을 지원합니다: 인라인 문자열, `@filepath` (파일에서 읽기), `-` (stdin에서 읽기). 데스크톱 앱 또는 웹 개발 서버와 연동됩니다. 전체 명령어 레퍼런스는 [CLI README](./apps/cli/README.md)를 참고하세요.

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

**Figma 가져오기**
- 레이아웃, 채우기, 선, 효과, 텍스트, 이미지, 벡터를 유지하며 `.fig` 파일 가져오기

**데스크톱 앱**
- Electron을 통한 네이티브 macOS, Windows, Linux 지원
- `.op` 파일 연결 — 더블 클릭으로 열기, 단일 인스턴스 잠금
- GitHub Releases에서 자동 업데이트
- 네이티브 애플리케이션 메뉴와 파일 다이얼로그

## 기술 스택

| | |
| --- | --- |
| **프론트엔드** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **캔버스** | CanvasKit/Skia (WASM, GPU 가속) |
| **상태 관리** | Zustand v5 |
| **서버** | Nitro |
| **데스크톱** | Electron 35 |
| **CLI** | `op` — 터미널 제어, 배치 디자인 DSL, 코드 내보내기 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **런타임** | Bun · Vite 7 |
| **파일 형식** | `.op` — JSON 기반, 사람이 읽을 수 있는, Git 친화적 |

## 프로젝트 구조

```text
openpencil/
├── apps/
│   ├── web/                 TanStack Start 웹 앱
│   │   ├── src/
│   │   │   ├── canvas/      CanvasKit/Skia 엔진 — 드로잉, 동기화, 레이아웃
│   │   │   ├── components/  React UI — 에디터, 패널, 공유 다이얼로그, 아이콘
│   │   │   ├── services/ai/ AI 채팅, 오케스트레이터, 디자인 생성, 스트리밍
│   │   │   ├── stores/      Zustand — 캔버스, 문서, 페이지, 히스토리, AI
│   │   │   ├── mcp/         외부 CLI 통합용 MCP 서버 도구
│   │   │   ├── hooks/       키보드 단축키, 파일 드롭, Figma 붙여넣기
│   │   │   └── uikit/       재사용 가능한 컴포넌트 킷 시스템
│   │   └── server/
│   │       ├── api/ai/      Nitro API — 스트리밍 채팅, 생성, 유효성 검사
│   │       └── utils/       Claude CLI, OpenCode, Codex, Copilot 래퍼
│   ├── desktop/             Electron 데스크톱 앱
│   │   ├── main.ts          윈도우, Nitro 포크, 네이티브 메뉴, 자동 업데이터
│   │   ├── ipc-handlers.ts  네이티브 파일 대화상자, 테마 동기화, 환경설정 IPC
│   │   └── preload.ts       IPC 브리지
│   └── cli/                 CLI 도구 — `op` 명령어
│       ├── src/commands/    디자인, 문서, 내보내기, 가져오기, 노드, 페이지, 변수 명령어
│       ├── connection.ts    실행 중인 앱과의 WebSocket 연결
│       └── launcher.ts      데스크톱 앱 또는 웹 서버 자동 감지 및 실행
├── packages/
│   ├── pen-types/           PenDocument 모델 타입 정의
│   ├── pen-core/            문서 트리 연산, 레이아웃 엔진, 변수
│   ├── pen-codegen/         코드 생성기 (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Figma .fig 파일 파서 및 변환기
│   ├── pen-renderer/        독립형 CanvasKit/Skia 렌더러
│   └── pen-sdk/             통합 SDK (모든 패키지 재export)
└── .githooks/               브랜치 이름에서 버전 동기화를 위한 pre-commit
```

## 키보드 단축키

| 키 | 동작 | | 키 | 동작 |
| --- | --- | --- | --- | --- |
| `V` | 선택 | | `Cmd+S` | 저장 |
| `R` | 사각형 | | `Cmd+Z` | 실행 취소 |
| `O` | 타원 | | `Cmd+Shift+Z` | 다시 실행 |
| `L` | 직선 | | `Cmd+C/X/V/D` | 복사/잘라내기/붙여넣기/복제 |
| `T` | 텍스트 | | `Cmd+G` | 그룹화 |
| `F` | Frame | | `Cmd+Shift+G` | 그룹 해제 |
| `P` | 펜 툴 | | `Cmd+Shift+E` | 내보내기 |
| `H` | 핸드(팬) | | `Cmd+Shift+C` | 코드 패널 |
| `Del` | 삭제 | | `Cmd+Shift+V` | 변수 패널 |
| `[ / ]` | 순서 변경 | | `Cmd+J` | AI 채팅 |
| 화살표 키 | 1px 이동 | | `Cmd+,` | 에이전트 설정 |
| `Cmd+Alt+U` | 불리언 합치기 | | `Cmd+Alt+S` | 불리언 빼기 |
| `Cmd+Alt+I` | 불리언 교차 | | | |

## 스크립트

```bash
bun --bun run dev          # 개발 서버 (포트 3000)
bun --bun run build        # 프로덕션 빌드
bun --bun run test         # 테스트 실행 (Vitest)
npx tsc --noEmit           # 타입 검사
bun run bump <version>     # 모든 package.json에 버전 동기화
bun run electron:dev       # Electron 개발 모드
bun run electron:build     # Electron 패키징
bun run cli:dev            # 소스에서 CLI 실행
bun run cli:compile        # CLI를 dist로 컴파일
```

## 기여하기

기여를 환영합니다! 아키텍처 세부 정보와 코드 스타일은 [CLAUDE.md](./CLAUDE.md)를 참고하세요.

1. 포크 후 클론
2. 버전 동기화 설정: `git config core.hooksPath .githooks`
3. 브랜치 생성: `git checkout -b feat/my-feature`
4. 검사 실행: `npx tsc --noEmit && bun --bun run test`
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
- [x] 재사용 가능한 패키지를 포함한 모노레포 구조 변경
- [x] CLI 도구 (`op`) 터미널 제어
- [ ] 공동 편집
- [ ] 플러그인 시스템

## 기여자

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 커뮤니티

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
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

## 라이선스

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
