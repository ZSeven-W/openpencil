<p align="center">
  <img src="./docs/images/dsh-openpencil-logo.png" alt="DSH OpenPencil" width="120" />
</p>

<h1 align="center">DSH OpenPencil</h1>

<p align="center">
  <strong>OpenPencil용 DeepSeek Harness 플러그인 — 대화 안에서 실제 <code>.op</code> 문서를 미리 보고, 검사하고, 편집할 수 있습니다.</strong><br />
  <sub>정확한 멀티 프레임 미리보기 &bull; 인터랙티브 캔버스 &bull; 관리형 편집기 &bull; 에이전트 네이티브 디자인 도구</sub>
</p>

<p align="center">
  <sub>npm: <a href="https://www.npmjs.com/package/@zseven-w/dsh-openpencil"><code>@zseven-w/dsh-openpencil</code></a> · 현재 플러그인 릴리스: <code>0.1.0-rc.7</code> · DSH <code>0.1.1-rc.2</code>까지 테스트됨</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md"><b>한국어</b></a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
  <img src="./docs/images/dsh-openpencil-overview.png" alt="DSH OpenPencil — 멀티 프레임 미리보기 및 사이드바 편집기" width="100%" />
</p>
<p align="center"><sub>인터랙티브 캔버스와 관리형 편집기 워크벤치를 갖춘 정확한 멀티 프레임 <code>.op</code> 미리보기</sub></p>

## 왜 DSH OpenPencil인가

DSH OpenPencil은 [DeepSeek Harness](https://github.com/deepseek-ai/DSH)와 [OpenPencil](https://github.com/ZSeven-W/openpencil)을 연결하여, 에이전트가 생성된 이미지를 반환하는 대신 실제로 편집 가능한 인터랙티브 디자인 캔버스를 다루도록 합니다.

<table>
<tr>
<td width="50%">

### 🖼️ 정확한 멀티 프레임 미리보기

설치된 OpenPencil 헤드리스 내보내기가 디자인에 충실한 미리보기를 렌더링합니다. 첫 번째 최상위 프레임은 재생해도 안전한 대형 PNG로, 나머지는 가로로 스크롤 가능한 썸네일 트랙으로 제공되며, 클릭 선택 및 이전/다음 탐색을 통해 멀티 프레임 문서를 살펴볼 수 있습니다.

</td>
<td width="50%">

### 🗺️ 인터랙티브 캔버스

"인터랙티브 캔버스 열기"는 팬, 줌, 화면 맞춤이 지원되는 읽기 전용 OpenPencil 웹 SDK를 지연 마운트합니다 — 대화를 떠나지 않고 모든 페이지, 중첩 노드, 비활성 페이지를 검사할 수 있습니다.

</td>
</tr>
<tr>
<td width="50%">

### ✏️ 관리형 편집기

`editable: true`로 설정하면 편집 작업이 관리형 OpenPencil 편집기를 엽니다 — 선택, 레이어, 속성, 그리기 도구, 실행 취소/다시 실행, 명시적 저장 의미론 — 전체 화면 옵션이 있는 크기 조절 가능한 오른쪽 워크벤치에서 제공됩니다.

</td>
<td width="50%">

### 🤖 에이전트 네이티브 디자인 도구

직접 캔버스를 다루는 다섯 가지 도구와 여섯 가지 `openpencil_pipeline_*` 도구를 통해 에이전트는 관리형 OpenPencil 런타임에서 실제 캔버스를 생성, 검사, 개선, 게시, 수정, 읽을 수 있습니다.

</td>
</tr>
<tr>
<td width="50%">

### 🔐 권한 기반 승인

이미지 및 문서 승인은 서명되고 해시에 바인딩된 권한입니다. 브라우저 메타데이터는 임의의 호스트 경로를 절대 노출하지 않으며, 서명된 미리보기/편집기 권한은 정식 도구 결과나 모델 컨텍스트에 절대 들어가지 않습니다.

</td>
<td width="50%">

### ⚡ 트랜잭션 안전성

전체 파이프라인 문서는 모든 네이티브 및 DSH 품질 게이트를 통과할 때까지 비공개 미게시 초안에 머뭅니다. 게시 시 기존 경로를 덮어쓰지 않으며, 중단되거나 실패한 배치는 빈 대상 파일을 남기지 않습니다.

</td>
</tr>
<tr>
<td width="50%">

### 🌍 DSH의 룩앤필 준수

도구 카드와 관리형 편집기는 편집 세션을 다시 로드하지 않고도 DSH의 중국어/영어 로케일과 라이트/다크 테마를 따릅니다.

</td>
<td width="50%">

### 🎯 하나의 완결된 워크플로

"요구사항 → 사이드바의 private live canvas → 사용자에게 정확한 PNG를 보여 주는 두 개의 의미 단위 배치 → 네이티브/DSH 결정론적 품질 게이트 → 원자적 게시" — DSH 안에서 완결되는 하나의 루프입니다.

</td>
</tr>
</table>

## DSH에 설치하기

DSH는 별도 패키지입니다. 아직 없다면 한 번 설치하세요:

```sh
npm install -g @deepseek-ai/dsh@latest
```

그다음 플러그인을 프로필에 추가하고 웹 앱을 실행합니다:

```sh
dsh plugin --profile web add @zseven-w/dsh-openpencil@next
dsh web
```

로컬 개발에서는 이 체크아웃을 빌드하고 절대 경로를 Web 프로필에 링크한 다음 DSH를 완전히 다시 시작하세요:

```sh
pnpm run build
dsh plugin --profile web add link:/absolute/path/to/dsh-openpencil
dsh web
```

`link:` 의존성은 이후 재빌드 결과를 이 체크아웃에서 바로 반영합니다. 단, 제공되는 Web 프로필은 기본적으로 호스트 번들을 핫 리로드하지 않으므로 프로필 의존성을 교체한 뒤에는 DSH를 완전히 다시 시작해야 합니다.

DSH를 전역 설치하고 싶지 않다면 같은 두 단계를 `pnpm dlx`로 실행하세요:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh plugin --profile web add @zseven-w/dsh-openpencil@next
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

> OpenPencil 플러그인은 공개되어 npm 토큰이 필요 없습니다. DSH 프리릴리스 자체가 레지스트리 인증을 요구하는 경우, 해당 자격 증명은 체크아웃 외부의 사용자 수준 또는 임시 npm 구성에 보관하세요. 이 저장소에는 의도적으로 레지스트리 자격 증명이 포함되어 있지 않습니다.

## 디자인 도구

선택적 스타일 흐름은 `openpencil_style_plan` → `openpencil_style_confirm` → `openpencil_pipeline_begin`입니다. 원래 brief와 `style_selection`을 전달합니다. 카탈로그와 특정 가이드 선택에는 모델 추천이 필요 없으며 추천 제공자는 기본적으로 꺼져 있습니다. 이 전달은 script를 지원하며 app-agent와 상거래에는 지원되지 않습니다. 일반 app-agent 생성은 계속 사용할 수 있습니다.

| 도구 | 역할 |
| --- | --- |
| `openpencil_new` | 단순 작업을 위한 호환 빠른 경로입니다. 하나의 트랜잭션 QuickJS `batch_design` 스크립트를 실행하고 대상이 없을 때만 게시한 뒤 편집 가능 프레젠테이션을 반환합니다. 프로덕션 디자인에는 아래 전체 파이프라인을 우선 사용하세요. |
| `openpencil_pipeline_begin` | 소유 세션 전용 비공개 초안과 유일한 root를 시작하고, 같은 live canvas를 사이드바에서 즉시 엽니다. 대상 `.op`는 게시되지 않은 상태로 유지됩니다. |
| `openpencil_pipeline_context` | begin contract에 실제로 빠진 guideline, style, theme, UI kit 세부 정보 하나만 제한적으로 불러옵니다. 시작 시 refresh loop가 아닙니다. |
| `openpencil_pipeline_batch` | 직접 QuickJS 생성 스크립트를 최대 두 개 실행합니다. 성공한 각 트랜잭션은 사용자에게 정확한 PNG 표시만 시도하며, 도구가 반환되면 즉시 `next`를 따릅니다. |
| `openpencil_pipeline_inspect` | 사용자가 명시적으로 요청한 경우에만 수동 진단을 제공합니다. 일반 생성에서는 미리보기나 모델의 이미지 검사 단계로 사용하지 않습니다. |
| `openpencil_pipeline_finish` | 네이티브 및 DSH 결정론적 게이트를 실행하고 최종화 후 정확한 PNG를 렌더링한 뒤 `createIfAbsent`로 원자 게시합니다. `needs_correction`, `canContinue: true`, 완전한 비어 있지 않은 `repairTargets`, `omitted: 0`이 모두 있을 때만 U-only 수정 1회와 마지막 finish 1회를 허용하며, 그 밖의 미게시 결과는 terminal입니다. |
| `openpencil_pipeline_abort` | 대상 파일을 만들지 않고 미게시 초안을 버립니다. |
| `openpencil_create` | 트랜잭션 `batch_design` 프로그램을 적용하여 기존 라이브 캔버스에서 노드를 생성하거나 재구성합니다. |
| `openpencil_edit` | 명시된 노드 또는 사용자가 선택한 단일 노드를 수정합니다. |
| `openpencil_render` | 변경 불가능하고 콘텐츠 주소 지정 방식의 `.op` 스냅샷을 만들고 활성 페이지의 모든 최상위 프레임을 렌더링합니다 — 선택적 `scale` 및 `editable`. |
| `openpencil_selection` | 라이브 편집기 캔버스에서 선택된 정확한 노드를 읽습니다. |

## 에이전트 디자인 워크플로

일반 생성에는 고정된 짧은 경로를 사용합니다. `openpencil_pipeline_begin` → 직접 QuickJS `openpencil_pipeline_batch` 두 번 → `openpencil_pipeline_finish` 한 번입니다. Begin은 유일한 root를 만들고 비공개 live canvas를 사이드바에서 즉시 엽니다. 게시가 성공하기 전에는 요청한 워크스페이스 경로가 존재하지 않습니다. begin 또는 batch가 성공할 때마다 설명, 계획, 비교, 검사, 무관한 도구 호출 없이 다음 필수 호출을 즉시 실행합니다. 기본적으로 전체 디자인에는 이미지 하나만 사용하고, 각 Hero/Product/Art/Media frame에는 주 시각 요소 하나만 두며 이미지와 placeholder icon을 함께 두지 않습니다.

성공한 두 배치는 모두 사용자에게 정확한 PNG 미리보기를 표시하려고 시도합니다. 도구가 반환되면 즉시 `next`를 따르세요. `next`가 `previewUnavailable`을 보고해도 스크립트는 이미 live canvas에 커밋된 상태입니다. 배치를 다시 실행하지 말고 `openpencil_pipeline_inspect`나 `read_image`도 호출하지 마세요. `openpencil_pipeline_inspect`는 사용자가 명시적으로 요청한 진단에만 사용합니다.

Finish는 OpenPencil 네이티브 최종화, lint, 대비, 레이아웃 검사와 DSH 결정론적 품질 게이트를 실행하고, 정상 호출 하나에서 정확한 최종 PNG를 렌더링해 대상을 원자 게시합니다. 결과에 `stage: "needs_correction"`, `canContinue: true`, 완전한 비어 있지 않은 `repairTargets`와 `omitted: 0`이 동시에 있고 각 target에 `operation: "U"`, 정확한 비어 있지 않은 `nodeId`, 비어 있지 않은 `patch`가 있을 때만 수정을 허용합니다. 모든 target을 정확히 하나의 U-only batch로 함께 적용한 뒤 설명 없이 finish를 마지막으로 한 번만 호출합니다. 다른 미게시 결과, error, `canContinue: false`는 terminal입니다. 한 번만 보고하고 retry, inspect, image/context 읽기, abort, 대체 draft 시작을 하지 않습니다. 게이트 실패나 `openpencil_pipeline_abort` 시 대상은 생성되지 않습니다. published result의 정확한 최종 PNG와 live editor는 이미 authoritative하므로 이를 반환하고 즉시 종료합니다. 사이드바는 idle 상태일 때만 자동으로 열며 명시적 전환을 위한 **캔버스 편집**을 항상 유지합니다.

동일하게 실행 중인 DSH 서비스 안에서는 브라우저를 바꾸거나 다시 로드한 뒤에도 엄격히 파싱된 `openpencil_new` 또는 `openpencil_pipeline_finish`의 영속 publication을 정확한 PNG와 명시적인 **캔버스 편집** 작업으로 복원할 수 있습니다. 과거 카드는 사이드바를 자동으로 열지 않으므로 사용자가 해당 작업을 클릭해야 합니다. 일반 과거 `openpencil_render`는 읽기 전용으로 유지되며, 비 loopback 연결에는 편집기 권한이 발급되지 않습니다.

번들된 `openpencil-design` skill은 계속 스크립팅 및 품질 가이드를 제공하며, 관리형 런타임은 데스크톱 바이너리에 의존하지 않습니다. `openpencil_new`는 호환 가능한 단일 배치 빠른 경로로 유지되지만 프로덕션 품질 생성에는 전체 파이프라인을 우선 사용해야 합니다.

`openpencil_create`와 `openpencil_edit`은 기존 라이브 캔버스에만 사용하세요. 이들의 편집 내용은 편집기의 저장(Save) 작업 전까지 저장되지 않은 상태로 유지됩니다.

## 웹 뷰어 에셋

DSH는 클라이언트 플러그인에 대해 `client.js`만 제공하므로, OpenPencil ESM SDK, 해당 WASM 및 CanvasKit은 명시적인 동일 출처 에셋으로 준비됩니다:

```sh
pnpm run sync:viewer-assets
```

동기화 명령은 기본적으로 상위 OpenPencil 모노레포 루트(`../..`)를 사용하며 형제 `../openpencil` 체크아웃으로 폴백합니다. `OPENPENCIL_ROOT` 또는 `--openpencil-root`로 재정의할 수 있습니다. `DSHPLUGIN_OPENPENCIL_VIEWER_SOURCE`로 완전한 사전 빌드 에셋 디렉터리를 선택할 수 있으며, `DSHPLUGIN_OPENPENCIL_VIEWER_ASSET_DIR`로 런타임 조회를 재정의할 수 있습니다.

뷰어 에셋은 사용자가 캔버스를 연 후에만 지연 로드됩니다. 에셋이 없거나 유효하지 않으면 PNG 미리보기를 계속 사용할 수 있으며 캔버스 버튼은 표시되지 않습니다.

## 관리형 편집기

편집 가능한 세션은 OpenPencil의 관리형 웹 호스트를 사용합니다 — `op-vscode`에서 사용하는 것과 동일한 아키텍처입니다. 플러그인은 인증된 사용자 작업 후에만 호스트를 시작하고, 데몬 토큰을 메모리에 보관하며, iframe 소스와 출처를 검증하고, 편집기 세션이 끝나면 프로세스를 종료합니다. 편집기 표면은 점진적으로 선택됩니다: 호스트가 해당 접합부(seam)를 선언하면 기본 Tool 세부 정보를, 그렇지 않으면 크기 조절 및 전체 화면 컨트롤이 있는 플러그인의 오른쪽 워크벤치를 사용합니다.

시작 시 느린 마운트에도 안전한 listening handshake를 사용하며, 번들 호스트가 바인딩 주소를 알린 뒤에만 readiness probe를 시작합니다. 데스크톱 OpenPencil 설치는 필요하지 않습니다.

게시된 설치 패키지는 `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-arm64`, `win32-x64`의 여섯 개 네이티브 플랫폼 패키지 중 현재 OS/CPU에 맞는 패키지를 선택하며, 두 Linux 패키지는 glibc를 대상으로 합니다. 루트 패키지는 이를 정확한 버전의 `optionalDependencies`로 선언하여 패키지 관리자가 올바른 변형(예: `@zseven-w/dsh-openpencil-darwin-arm64`)을 선택하도록 합니다. 이 패키지는 서로 호환되는 `op-host-web-server`, 편집기 웹 번들, CanvasKit을 하나의 런타임으로 함께 제공합니다. 따라서 관리형 편집기는 `/Applications/OpenPencil.app`, `PATH`의 `openpencil-desktop` 또는 OpenPencil 소스 체크아웃에 의존하지 않습니다.

캔버스가 더티 상태인 동안 DSH가 플러그인을 다시 로드하거나 언로드하면, 호스트는 최대 7일 동안 불투명한 로컬 복구 초안을 보관합니다. 동일한 소스를 다시 열면 라이브 캔버스에 복원하기 전에 확인을 요청하며, 복구는 사용자가 명시적으로 저장하기 전까지 `.op` 파일을 절대 덮어쓰지 않습니다.

공식 6개 플랫폼 패키지는 보호된 release 빌드에서 중국/글로벌 협업 bootstrap 엔드포인트를 주입하고 검증한 뒤에만 게시됩니다. 이 주입 없이 로컬에서 직접 빌드한 경우 DSH를 시작하기 전에 `OPENPENCIL_COLLAB_BOOTSTRAP_URL=https://<your-host>/api/v1/collaboration/bootstrap`으로 bootstrap을 재정의할 수 있습니다. 값은 반드시 `https`를 사용해야 하며 경로는 정확히 `/api/v1/collaboration/bootstrap`이어야 합니다.

기기 간 캔버스 동기화를 사용하려면 PC/DSH 네이티브 런타임과 모바일 앱을 모두 현재 협업 큐 수정이 포함된 동일한 OpenPencil 릴리스 계열로 업데이트해야 합니다. 이전 모바일 앱과 새 PC 런타임을 함께 사용하면 원격 커서는 보이지만 캔버스 커밋을 받지 못할 수 있습니다.

이 저장소에서 개발할 때는 DSH를 시작하기 전에 편집기 Web bundle, 네이티브 호스트 순서로 빌드한 다음 서로 일치하는 런타임을 스테이징하세요.

`pnpm run build:editor-web`은 OpenPencil이 공식 지원하는 WASM bundle gate를 실행합니다. Bash, `wasm32-unknown-unknown` target이 설치된 Cargo/Rust, `wasm-bindgen` CLI, Binaryen의 `wasm-opt`, Node.js, `gzip`이 필요하며 CanvasKit에는 EMSDK가 필요하지 않습니다. Web 빌드는 협업 bootstrap 빌드 변수를 사용하지 않습니다. `pnpm run build:editor-runtime` 전에 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_CN`과 `OPENPENCIL_BUILD_COLLAB_BOOTSTRAP_URL_GLOBAL`을 모두 설정해야 합니다. 이 변수들은 네이티브 Cargo 빌드에만 사용되며 하나라도 없으면 fail closed로 실패합니다. 두 빌드가 모두 성공한 뒤 마지막 명령으로 런타임을 스테이징하세요.

```sh
pnpm run build:editor-web
pnpm run build:editor-runtime
pnpm run stage:editor-runtime
```

런타임을 명시적으로 재정의할 때는 서로 일치하는 다음 세 항목을 완전한 한 세트로 제공해야 합니다:

- `op-host-web-server`용 `DSHPLUGIN_OPENPENCIL_EDITOR_BINARY`;
- 빌드된 편집기 웹 번들용 `DSHPLUGIN_OPENPENCIL_EDITOR_WEB_BUNDLE_DIR`;
- CanvasKit 에셋용 `DSHPLUGIN_OPENPENCIL_EDITOR_CANVASKIT_DIR`.

세트의 일부만 제공하면 잘못된 구성입니다. 플러그인은 사용자 지정 경로와 패키지에 포함된 런타임 에셋을 혼합하지 않습니다.

저장은 낙관적 소스 해시, 원자적 교체, 후속 권한(successor capability)을 사용합니다. 소스가 편집기 외부에서 변경되면 플러그인은 덮어쓰는 대신 충돌을 보고합니다.

## 결과 메타데이터

모델에 표시되는 결과는 일반 JSON으로 유지됩니다. 브라우저 전용 `presentationMeta.$dshOpenPencil`은 다음에 대한 추가 권한을 전달합니다:

- `image`: PNG 경로, 미리보기/다운로드 URL 및 실제 너비/높이;
- `frames`: 활성 페이지 순서대로 정확히 렌더링된 모든 최상위 프레임(노드 id/이름/인덱스 및 서명된 PNG URL 포함);
- `document`: 소스 작업 경로와 변경 불가능한 스냅샷 URL, 바이트 및 SHA-256;
- `viewer`: 에셋 경로가 연결된 경우 버전이 지정된 SDK/WASM/CanvasKit URL;
- `editor`: `editable: true`가 승인된 경우 범위가 지정된 실행/새로고침 권한.

결과에는 `renderer`, `rendererBinary`, `fidelity` 및 모든 경고도 기록됩니다. 기존의 PNG 전용 schema-v1 메시지는 계속 렌더링할 수 있습니다.

DSH `0.1.1-rc.2`은 PTC/Code Mode 아래에 중첩된 도구에 대한 브라우저 프레젠테이션 메타데이터를 유지하지 않습니다. 플러그인은 동일 출처의 세션 바인딩 엔드포인트를 통해 해당 UI-only 투영(projection)을 복구합니다: 브라우저는 세션 id, 호출 id 및 변경 불가능한 문서 SHA-256만 전송하고, 호스트는 영속적인 DSH 세션 로그에서 권위 있는 결과를 확인하며, 최근 라이브 편집을 승인하는 용도로만 수명이 짧은 인프로세스 마커를 사용합니다. 서명된 미리보기/편집기 권한은 정식 도구 결과나 모델 컨텍스트에 절대 들어가지 않습니다. 일반 `openpencil_render`의 영속 기록은 읽기 전용으로 유지됩니다. 엄격히 파싱된 `openpencil_new` 또는 `openpencil_pipeline_finish`의 영속 publication은 loopback 연결에서 사용자가 명시적으로 클릭한 뒤에만 편집기 권한을 받을 수 있습니다. 사이드바 자동 열기는 최근의 신뢰할 수 있는 라이브 결과에만 허용됩니다.

제한된 재생을 위해 중첩 메타데이터 복구는 최대 128개의 최상위 프레임을 허용하며, 더 큰 Code Mode 결과는 정식 JSON 폴백을 통해 계속 사용할 수 있습니다.

## 현재 제한 사항

- 기존 캔버스에 대한 후속 편집에는 이미 열려 있는 관리형 편집기가 필요합니다. 변경 사항은 사용자가 저장(Save) 작업을 호출할 때까지 저장되지 않은 상태로 유지됩니다.
- 경량 웹 SDK 캔버스는 읽기 전용입니다. 전체 편집은 별도의 관리형 편집기 표면을 사용합니다. DSH `0.1.1-rc.2`에서 플러그인은 전체 화면 옵션이 있는 크기 조절 가능한 오른쪽 워크벤치를 사용합니다.
- 정확한 갤러리는 활성 페이지의 최상위 프레임을 다룹니다. 비활성 페이지와 중첩 노드를 검사하는 방법은 인터랙티브 캔버스입니다.
- 렌더 및 스냅샷 캐시에는 여전히 제품 수준의 보존 정책이 필요합니다.

## 프로젝트 구조

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

## 빌드 및 검증

```sh
pnpm run sync:viewer-assets
pnpm run build
pnpm run test:viewer-assets
pnpm run test:client
pnpm run test:host /absolute/path/to/design.op 375 1091
```

빌드에는 Node 24.11 이상과 pnpm이 필요합니다. DSH 호스트/클라이언트 패키지는 대상 DSH 프로필이 제공하는 피어 종속성입니다. 빌드 도구는 로컬 개발 종속성, 활성 연결된 DSH 체크아웃 또는 설치된 DSH 소스 번들에서 확인되며, `DSH_SOURCE_ROOT`로 소스 체크아웃을 명시적으로 선택할 수 있습니다. 잠금 파일(lockfile)은 해당 환경이 별도로 프로비저닝되는 경우 독립형 공개 빌드 도구를 고정합니다.

비공개 DSH 프리릴리스의 경우, 발급된 npm 자격 증명을 이 저장소 외부에 보관하고(예: 사용자 수준 또는 임시 `.npmrc`) 요청된 버전을 직접 실행하세요:

```sh
pnpm dlx --package=@deepseek-ai/dsh@latest dsh web
```

`.npmrc`, `NPM_TOKEN` 또는 복사된 레지스트리 자격 증명을 절대 커밋하지 마세요. 이 저장소는 기본적으로 로컬 npm 구성을 무시합니다.

`test:host`는 실제 정확한 렌더링을 수행하고, PNG IHDR 지오메트리와 SHA-256을 검증하며, HTTP를 통해 변경 불가능한 이미지/문서 권한을 실행하고, 뷰어 에셋에 권한을 부여할 수 있는지 확인합니다. 예상 치수는 픽스처별로 다릅니다.

## 에코시스템

DSH OpenPencil은 세계 최초의 오픈소스 AI 네이티브 벡터 디자인 도구인 **[OpenPencil](https://github.com/ZSeven-W/openpencil)**용 DeepSeek Harness 플러그인이자, 순수 Rust 기반 AI 네이티브 도구 모음인 **[ZSeven-W](https://github.com/ZSeven-W)** 제품군의 일부입니다.

| 프로젝트 | 설명 |
| ------- | ---------- |
| **[OpenPencil](https://github.com/ZSeven-W/openpencil)** | 이 플러그인이 구동하는 디자인 도구 — 프롬프트-투-캔버스 생성, 동시 에이전트 팀, 디자인-애즈-코드 `.op` 파일, 내장 MCP 서버를 제공합니다. 여기의 정확한 미리보기, 인터랙티브 캔버스, 관리형 편집기는 모두 OpenPencil 자체로 구동됩니다. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | LLM 에이전트를 출시하기 위한 순수 Rust 비동기 런타임 — 멀티 프로바이더, 엔드투엔드 도구 지원, 구조화된 권한, 실제 MCP, `unsafe` 제로. OpenPencil의 내장 에이전트 런타임을 구동합니다. |
| **[jian](https://github.com/ZSeven-W/jian)** | 순수 Rust, GPU-Skia UI 프레임워크 — 위젯, 레이아웃, 이벤트, 핫 리로드를 하나의 스택에 담았습니다. OpenPencil의 UI 프레임워크이자 이 플러그인 폴백 렌더러의 소스입니다. |
| **[Zode](https://github.com/ZSeven-W/zode)** | 터미널용 오픈소스 AI 네이티브 코딩 어시스턴트 — 코드를 읽고, 명령을 실행하며, MCP를 통해 OpenPencil을 구동합니다. |
| **[noema](https://github.com/ZSeven-W/noema)** | 코딩 에이전트를 위한 로컬 우선, 비벡터 메모리 시스템 — 검사 가능한 파일 형태의 영속 메모리로, 여러 런타임에서 작동합니다. |
| **[openpencil-skill](https://github.com/ZSeven-W/openpencil-skill)** | AI 에이전트에게 `op`로 디자인하는 방법을 가르치는 LLM 스킬 플러그인 — 이 DSH 플러그인의 동반자입니다. |

같은 계열의 DSH 플러그인:

- [DSH Android](https://github.com/ZSeven-W/dsh-android) — 대화 안에서 동작하는 Android 에뮬레이터와 USB 연결 기기 — 전부 adb로 구동
- [DSH Crew](https://github.com/ZSeven-W/dsh-crew) — Claude Code / Codex에서 DSH 에이전트로 작업 위임
- [DSH iOS](https://github.com/ZSeven-W/dsh-ios) — 대화 안에서 동작하는 iOS 시뮬레이터와 USB 연결 iPhone
- [DSH Noema](https://github.com/ZSeven-W/dsh-noema) — DSH를 위한 장기 기억

## 기여하기

기여를 환영합니다! 포크하고 클론한 뒤 브랜치를 만들고, `pnpm run build`와 테스트 스위트를 실행하고, [Conventional Commits](https://www.conventionalcommits.org/)으로 커밋한 후 `main`에 PR을 열어 주세요.

## 커뮤니티

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="https://raw.githubusercontent.com/ZSeven-W/openpencil/main/screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Discord 커뮤니티에 참여하세요</strong>
</a>
— 질문하고, 디자인을 공유하고, 기능을 제안하세요.

**공식 커뮤니티: [LINUX DO](https://linux.do/)**

## 라이선스

[MIT](./LICENSE) — 저작권 (c) 2026 ZSeven-W

타사 구성 요소는 [THIRD_PARTY_NOTICES.md](./THIRD_PARTY_NOTICES.md)에 나열되어 있습니다.
