<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI 네이티브 오픈소스 디자인 툴. 디자인-애즈-코드.</strong><br />
  프롬프트에서 캔버스 UI로. 멀티 에이전트 오케스트레이션. 내장 MCP 서버. 코드 생성.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md"><b>한국어</b></a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">빠른 시작</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">기능</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">기여하기</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>이미지를 클릭하여 데모 영상 보기</sub></p>

<br />

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

## AI 네이티브 디자인

OpenPencil은 AI를 플러그인이 아닌 핵심 워크플로로서 처음부터 구축했습니다.

**프롬프트에서 UI로**
- **텍스트-투-디자인** — 페이지를 설명하면 스트리밍 애니메이션으로 실시간으로 캔버스에 생성
- **오케스트레이터** — 복잡한 페이지를 공간적 서브태스크로 분해하여 병렬 생성 지원
- **디자인 수정** — 요소를 선택하고 자연어로 변경 내용을 설명
- **비전 입력** — 스크린샷이나 목업을 참조로 첨부하여 디자인

**멀티 에이전트 지원**

| 에이전트 | 설정 방법 |
| --- | --- |
| **Claude Code** | 설정 불필요 — 로컬 OAuth로 Claude Agent SDK 사용 |
| **Codex CLI** | 에이전트 설정에서 연결 (`Cmd+,`) |
| **OpenCode** | 에이전트 설정에서 연결 (`Cmd+,`) |

**MCP 서버**
- 내장 MCP 서버 — Claude Code / Codex / Gemini / OpenCode / Kiro CLI에 원클릭 설치
- 터미널에서 디자인 자동화: MCP 호환 에이전트를 통해 `.op` 파일 읽기, 생성, 편집 가능

**코드 생성**
- React + Tailwind CSS
- HTML + CSS
- 디자인 토큰에서 CSS Variables 생성

## 기능

**캔버스 & 드로잉**
- 팬, 줌, 스마트 정렬 가이드, 스냅 지원의 무한 캔버스
- 사각형, 타원, 직선, 다각형, 펜(베지어), Frame, 텍스트
- 아이콘 피커(Iconify)와 이미지 가져오기(PNG/JPEG/SVG/WebP/GIF)
- 오토 레이아웃 — 수직/수평 방향, 갭·패딩·justify·align 지원
- 탭 내비게이션이 있는 멀티 페이지 문서

**디자인 시스템**
- 디자인 변수 — 컬러·숫자·문자열 토큰, `$variable` 참조 지원
- 멀티 테마 지원 — 여러 테마 축, 각 축에 여러 변형(라이트/다크, 컴팩트/컴포터블)
- 컴포넌트 시스템 — 인스턴스와 오버라이드를 가진 재사용 가능한 컴포넌트
- CSS 동기화 — 커스텀 프로퍼티 자동 생성, 코드 출력에 `var(--name)` 사용

**Figma 가져오기**
- 레이아웃, 채우기, 선, 효과, 텍스트, 이미지, 벡터를 유지하며 `.fig` 파일 가져오기

**데스크톱 앱**
- Electron을 통한 네이티브 macOS·Windows·Linux 지원
- GitHub Releases에서 자동 업데이트
- 네이티브 애플리케이션 메뉴와 파일 다이얼로그

## 기술 스택

| | |
| --- | --- |
| **프론트엔드** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **캔버스** | Fabric.js v7 |
| **상태 관리** | Zustand v5 |
| **서버** | Nitro |
| **데스크톱** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **런타임** | Bun · Vite 7 |
| **파일 형식** | `.op` — JSON 기반, 사람이 읽을 수 있는, Git 친화적 |

## 프로젝트 구조

```text
src/
  canvas/          Fabric.js 엔진 — 드로잉, 동기화, 레이아웃, 가이드, 펜 툴
  components/      React UI — 에디터, 패널, 공유 다이얼로그, 아이콘
  services/ai/     AI 채팅, 오케스트레이터, 디자인 생성, 스트리밍
  services/figma/  Figma .fig 바이너리 가져오기 파이프라인
  services/codegen React+Tailwind 및 HTML+CSS 코드 생성기
  stores/          Zustand — 캔버스, 문서, 페이지, 히스토리, AI, 설정
  variables/       디자인 토큰 해석 및 참조 관리
  mcp/             외부 CLI 통합용 MCP 서버 툴
  uikit/           재사용 가능한 컴포넌트 킷 시스템
server/
  api/ai/          Nitro API — 스트리밍 채팅, 생성, 유효성 검사
  utils/           Claude CLI, OpenCode, Codex 클라이언트 래퍼
electron/
  main.ts          윈도우, Nitro 포크, 네이티브 메뉴, 자동 업데이터
  preload.ts       IPC 브리지
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

## 스크립트

```bash
bun --bun run dev          # 개발 서버 (포트 3000)
bun --bun run build        # 프로덕션 빌드
bun --bun run test         # 테스트 실행 (Vitest)
npx tsc --noEmit           # 타입 검사
bun run electron:dev       # Electron 개발 모드
bun run electron:build     # Electron 패키징
```

## 기여하기

기여를 환영합니다! 아키텍처 세부 정보와 코드 스타일은 [CLAUDE.md](./CLAUDE.md)를 참고하세요.

1. 포크 후 클론
2. 브랜치 생성: `git checkout -b feat/my-feature`
3. 검사 실행: `npx tsc --noEmit && bun --bun run test`
4. [Conventional Commits](https://www.conventionalcommits.org/) 형식으로 커밋: `feat(canvas): add rotation snapping`
5. `main` 브랜치에 PR 생성

## 로드맵

- [x] CSS 동기화가 있는 디자인 변수 & 토큰
- [x] 컴포넌트 시스템(인스턴스 & 오버라이드)
- [x] 오케스트레이터를 통한 AI 디자인 생성
- [x] MCP 서버 통합
- [x] 멀티 페이지 지원
- [x] Figma `.fig` 가져오기
- [ ] 불리언 연산(결합, 빼기, 교차)
- [ ] 공동 편집
- [ ] 플러그인 시스템

## 기여자

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## 커뮤니티

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord에 참여하기</strong>
</a>
— 질문하기, 디자인 공유, 기능 제안.

## 라이선스

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
