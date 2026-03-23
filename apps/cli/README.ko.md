# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [**한국어**](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

[OpenPencil](https://github.com/ZSeven-W/openpencil)용 CLI — 터미널에서 디자인 도구를 제어합니다.

## 설치

```bash
npm install -g @zseven-w/openpencil
```

## 플랫폼 지원

CLI는 모든 플랫폼에서 OpenPencil 데스크톱 앱을 자동으로 감지하고 실행합니다:

| 플랫폼      | 감지되는 설치 경로                                                                                    |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS 사용자별 (`%LOCALAPPDATA%`), 시스템 전체 (`%PROGRAMFILES%`), 포터블                               |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## 사용법

```bash
op <command> [options]
```

### 입력 방식

JSON 또는 DSL을 받는 인자는 세 가지 방법으로 전달할 수 있습니다:

```bash
op design '...'              # 인라인 문자열 (작은 페이로드)
op design @design.txt        # 파일에서 읽기 (대규모 디자인에 권장)
cat design.txt | op design - # 표준 입력에서 읽기 (파이핑)
```

### 앱 제어

```bash
op start [--desktop|--web]   # OpenPencil 실행 (기본값: 데스크톱)
op stop                      # 실행 중인 인스턴스 중지
op status                    # 실행 상태 확인
```

### 디자인 (배치 DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### 문서 작업

```bash
op open [file.op]            # 파일 열기 또는 라이브 캔버스에 연결
op save <file.op>            # 현재 문서 저장
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # 현재 캔버스 선택 항목 가져오기
```

### 노드 조작

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### 코드 내보내기

```bash
op export <format> [--out file]
# 형식: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### 변수 및 테마

```bash
op vars                      # 변수 가져오기
op vars:set <json>           # 변수 설정
op themes                    # 테마 가져오기
op themes:set <json>         # 테마 설정
op theme:save <file.optheme> # 테마 프리셋 저장
op theme:load <file.optheme> # 테마 프리셋 불러오기
op theme:list [dir]          # 테마 프리셋 목록 보기
```

### 페이지

```bash
op page list                 # 페이지 목록 보기
op page add [--name N]       # 페이지 추가
op page remove <id>          # 페이지 제거
op page rename <id> <name>   # 페이지 이름 변경
op page reorder <id> <index> # 페이지 순서 변경
op page duplicate <id>       # 페이지 복제
```

### 가져오기

```bash
op import:svg <file.svg>     # SVG 파일 가져오기
op import:figma <file.fig>   # Figma .fig 파일 가져오기
```

### 레이아웃

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### 전역 플래그

```text
--file <path>     대상 .op 파일 (기본값: 라이브 캔버스)
--page <id>       대상 페이지 ID
--pretty          사람이 읽기 쉬운 JSON 출력
--help            도움말 표시
--version         버전 표시
```

## 라이선스

MIT
