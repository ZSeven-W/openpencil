<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Первый в мире AI-нативный инструмент векторного дизайна с открытым исходным кодом.</strong><br />
  <sub>Параллельные команды агентов &bull; Дизайн как код &bull; Встроенный MCP-сервер &bull; Мультимодельный интеллект</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Нажмите на изображение, чтобы посмотреть демо-видео</sub></p>

<br />

> **Примечание:** Существует другой проект с открытым исходным кодом с таким же названием — [OpenPencil](https://github.com/open-pencil/open-pencil), ориентированный на визуальный дизайн, совместимый с Figma, с совместной работой в реальном времени. Этот проект ориентирован на AI-нативные рабочие процессы «дизайн в код».

## Почему OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Запрос → Холст

Опишите любой интерфейс на естественном языке. Наблюдайте, как он появляется на бесконечном холсте в реальном времени со стриминговой анимацией. Изменяйте существующие дизайны, выбирая элементы и общаясь в чате.

</td>
<td width="50%">

### 🤖 Параллельные команды агентов

Оркестратор декомпозирует сложные страницы на пространственные подзадачи. Несколько AI-агентов работают над разными секциями одновременно — hero, features, footer — всё стримится параллельно.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Мультимодельный интеллект

Автоматически адаптируется к возможностям каждой модели. Claude получает полные промпты с thinking; GPT-4o/Gemini — без thinking; маленькие модели (MiniMax, Qwen, Llama) получают упрощённые промпты для надёжного вывода.

</td>
<td width="50%">

### 🔌 MCP-сервер

Установка в один клик в Claude Code, Codex, Gemini, OpenCode, Kiro или Copilot CLI. Дизайн из терминала — чтение, создание и изменение файлов `.op` через любой MCP-совместимый агент.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Дизайн как код

Файлы `.op` — это JSON: удобочитаемые, дружественные к Git, поддерживают diff. Переменные дизайна генерируют CSS custom properties. Экспорт кода в React + Tailwind или HTML + CSS.

</td>
<td width="50%">

### 🖥️ Работает везде

Веб-приложение + нативный десктоп на macOS, Windows и Linux — единое Rust-ядро, один самодостаточный бинарный файл, без браузерного движка. Ассоциация файлов `.op` — двойной клик для открытия.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Управляйте инструментом дизайна из терминала. `op design`, `op insert` — пакетный DSL дизайна, манипуляция узлами. Ввод через pipe из файлов или stdin. Работает с десктопным приложением или веб-сервером.

</td>
<td width="50%">

### 🎯 Мультиплатформенный экспорт кода

Экспорт из одного файла `.op` в React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Переменные дизайна превращаются в пользовательские свойства CSS.

</td>
</tr>
</table>

## Быстрый старт

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Или запустить как десктопное приложение:

```bash
cargo run -p op-host-desktop
```

> **Требования:** [Rust](https://www.rust-lang.org/) (stable) для сборки продукта. [Bun](https://bun.sh/) >= 1.0 и [Node.js](https://nodejs.org/) >= 18 нужны только для веб-SDK в `packages/`.

### Docker

Тегированные Rust-релизы публикуют один образ web host. Устаревшие TypeScript-образы со встроенными AI CLI больше не публикуются.

| Образ | Содержит |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:v0.8.1` | Rust web host, wasm bundle и ресурсы CanvasKit |

Веб-интерфейс показывает только встроенные профили агентов; инструменты Claude/Codex/OpenCode/Copilot/Gemini CLI не входят в Docker-образы.

**Запуск:**

```bash
docker run -d -p 3100:3100 ghcr.io/zseven-w/openpencil-web:v0.8.1
```

Затем откройте `http://localhost:3100/`.

**Локальная сборка:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## AI-нативный дизайн

**От запроса к UI**

- **Текст в дизайн** — опишите страницу и получите её на холсте в реальном времени со стриминговой анимацией
- **Оркестратор** — разбивает сложные страницы на пространственные подзадачи для параллельной генерации
- **Изменение дизайна** — выберите элементы и опишите изменения на естественном языке
- **Визуальный ввод** — прикрепляйте скриншоты или макеты в качестве референса для дизайна

**Поддержка нескольких агентов**

| Агент                           | Настройка                                                                                                      |
| ------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| **Встроенный (9+ провайдеров)** | Выбор из предустановленных провайдеров с переключателем региона — Anthropic, OpenAI, Google, DeepSeek и другие |
| **Claude Code**                 | Без настройки — использует Claude Agent SDK с локальным OAuth                                                  |
| **Codex CLI**                   | Подключить в настройках агента (`Cmd+,`)                                                                       |
| **OpenCode**                    | Подключить в настройках агента (`Cmd+,`)                                                                       |
| **GitHub Copilot**              | `copilot login`, затем подключить в настройках агента (`Cmd+,`)                                                |
| **Gemini CLI**                  | Подключить в настройках агента (`Cmd+,`)                                                                       |

**Профили возможностей моделей** — автоматически адаптирует промпты, режим thinking и таймауты для каждого уровня моделей. Модели полного уровня (Claude) получают полные промпты; стандартного уровня (GPT-4o, Gemini, DeepSeek) — с отключённым thinking; базового уровня (MiniMax, Qwen, Llama, Mistral) — упрощённые промпты с вложенным JSON для максимальной надёжности.

**i18n** — Полная локализация интерфейса на 15 языках: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**MCP-сервер**

- Встроенный MCP-сервер (крейт `op-mcp`) — установка в один клик в Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- Node.js не требуется — stdio-транспорт через десктопный бинарный файл (`--mcp <path>`), плюс работающий HTTP-эндпоинт (`127.0.0.1:<port>/mcp`) из запущенного приложения
- Автоматизация дизайна из терминала: чтение, создание и изменение файлов `.op` через любой MCP-совместимый агент
- **Послойный рабочий процесс** — `design_skeleton` → `design_content` → `design_refine` для дизайнов высокого качества с несколькими секциями
- **Сегментированное получение промптов** — загружайте только нужные знания о дизайне (schema, layout, roles, icons, planning и т.д.)
- Поддержка нескольких страниц — создание, переименование, переупорядочивание и дублирование страниц через инструменты MCP

**Генерация кода**

- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Установите глобально и управляйте инструментом дизайна из терминала:

```bash
brew install zseven-w/openpencil/op
```

```bash
op start                     # Запустить десктопное приложение
op start --headless --file design.op # Запустить headless-сервер
op design @landing.txt       # Пакетный дизайн из файла
op design @ui.js             # Изолированный JavaScript с циклами
op insert '{"type":"rectangle"}' # Вставить узел
op import:figma design.fig   # Импортировать файл Figma
cat design.dsl | op design - # Передача через stdin
```

Поддерживает строку, `@filepath` и stdin (`-`). Работает с десктопным приложением, веб-сервером или файловым headless-сервером. Все команды перечислены в [справочнике CLI](./crates/op-cli/src/usage.txt).

**LLM-навык** — установите плагин [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill), чтобы научить ИИ-агентов проектировать с помощью `op`. Используйте `op install` для обнаруженных агентов или `op install --target codex` для конкретной цели.

## Возможности

**Холст и рисование**

- Бесконечный холст с панорамированием, масштабированием, умными направляющими и привязкой
- Прямоугольник, Эллипс, Линия, Многоугольник, Перо (Безье), Frame, Текст
- Булевы операции — объединение, вычитание, пересечение с контекстной панелью инструментов
- Выбор иконок (Iconify) и импорт изображений (PNG/JPEG/SVG/WebP/GIF)
- Авто-раскладка — вертикальная/горизонтальная с gap, padding, justify, align
- Многостраничные документы с навигацией по вкладкам

**Система дизайна**

- Переменные дизайна — цветовые, числовые и строковые токены со ссылками `$variable`
- Поддержка нескольких тем — несколько осей, каждая с вариантами (Светлая/Тёмная, Компактная/Комфортная)
- Система компонентов — переиспользуемые компоненты с экземплярами и переопределениями
- CSS-синхронизация — автоматически генерируемые пользовательские свойства, `var(--name)` в выводе кода
- Переиспользуемые UIKit — импорт/экспорт наборов компонентов из файлов `.pen`

**AI и агенты**

- Prompt-to-canvas с потоковой генерацией и разбиением пространства под управлением оркестратора
- Параллельные команды агентов — несколько дизайнеров одновременно работают над разными секциями с индикаторами на холсте по каждому участнику
- Послойный рабочий процесс — `design_skeleton` → `design_content` → `design_refine` со сфокусированными промптами на каждой фазе
- Руководства по стилю — 50+ встроенных стилей (glassmorphism, brutalist, retro и т. д.) с нечётким сопоставлением по тегам, интегрированным в планирование и генерацию
- Профили возможностей мультимодели — автоматически подстраивает режим размышления, уровень усилий и форму промпта в зависимости от уровня модели
- Встроенный рантайм агентов (Rust) + провайдеры Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Проброс формата Anthropic для китайских провайдеров LLM — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Интеграция с Git**

- Мастер клонирования с аутентификацией SSH / HTTPS и управлением SSH-ключами
- Выбор ветки — создание, переключение, удаление, слияние — всё прямо из панели Git
- Каскады pull / push с повтором аутентификации и обработкой non-fast-forward
- Трёхстороннее слияние в режиме папки с отслеживанием состояния `MERGE_HEAD` на диске
- Панель конфликтов с трёхсторонними карточками по узлам/полям, встроенным редактором JSON, массовыми действиями и встроенным блоком diff
- Интерфейс настроек удалённых репозиториев и SSH-ключей; i18n на 15 языков по всей поверхности Git

**Экспорт**

- Экспорт холста — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Экспорт кода — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Инкрементальный конвейер генерации кода MCP — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Импорт из Figma**

- Импорт файлов `.fig` с сохранением раскладки, заливок, обводок, эффектов, текста, изображений и векторов

**Десктопное приложение**

- Нативная поддержка macOS, Windows и Linux — единый самодостаточный бинарный файл (winit + GPU Skia, без Electron)
- Ассоциация файлов `.op` — двойной клик для открытия, блокировка единственного экземпляра
- Фоновая проверка обновлений через GitHub Releases
- Нативное меню приложения с пунктами «Сохранить как», «Открыть недавние» и диалогом несохранённых изменений при закрытии
- Сохранение списка недавних файлов

## Технологический стек

|                      |                                                                                  |
| -------------------- | -------------------------------------------------------------------------------- |
| **Ядро**              | Rust-пространство (`crates/`) — состояние редактора, виджеты, хосты, MCP, AI, кодген |
| **Рендеринг**         | GPU Skia везде — `skia-safe` (GL) на нативных платформах, CanvasKit (WASM/WebGL2) в браузере |
| **UI-фреймворк**      | jian — вендоренный чистый Rust GPU-Skia UI-фреймворк: виджеты, раскладка, события, горячая перезагрузка (`vendor/jian`) |
| **Оконная система**   | winit (вендоренный форк `casement`)                                             |
| **Десктоп**           | Нативный бинарный файл `openpencil-desktop` — без браузерного движка            |
| **Веб-SDK**           | `op-web-sdk` + адаптеры React 19 / Vue 3 — SDK для просмотра `.op` только для чтения (TypeScript) |
| **CLI**               | `op` — управление из терминала, пакетный DSL дизайна                             |
| **AI**                | Встроенный рантайм агентов (Rust) · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Линтинг**           | clippy · rustfmt (Rust) · oxlint · oxfmt (веб-SDK)                               |
| **Формат файла**      | `.op` — на основе JSON, удобочитаемый, дружественный к Git                       |

## Экосистема

OpenPencil — часть семейства AI-нативных инструментов на чистом Rust от **[ZSeven-W](https://github.com/ZSeven-W)**. Они дополняют друг друга: `jian` отрисовывает OpenPencil, `agent-rs` запускает его агентов, `noema` запоминает, а `zode` проектирует из терминала.

| Проект | Что это такое |
| ------ | ------------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Open-source AI-нативный ассистент для программирования в вашем терминале — быстрый Rust TUI (`ratatui`), который читает ваш код, выполняет команды, ищет файлы и управляет git. Управляет OpenPencil через MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Асинхронный рантайм на чистом Rust для выпуска LLM-агентов — мультипровайдерный, со сквозной поддержкой инструментов, структурированными разрешениями, настоящим MCP и нулём `unsafe`. Питает встроенный рантайм агентов OpenPencil (`vendor/agent`) и Zode. |
| **[jian](https://github.com/ZSeven-W/jian)** | UI-фреймворк на чистом Rust с GPU-Skia — виджеты, раскладка, события и горячая перезагрузка в одном стеке. Превращает декларативный документ `.op` в нативное, управляемое ИИ приложение без JS-рантайма, без DOM, без Electron. UI-фреймворк OpenPencil (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Локальная невекторная система памяти для агентов программирования. Долговременная память в виде инспектируемых файлов, очередь на проверку новых записей и лексический (без эмбеддингов) поиск — работает в Zode, Codex, Claude Code и рантаймах MCP. |

## Почему Rust

OpenPencil переписан с нуля на **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). Переписывание завершено — редактор на TypeScript + Electron был выведен из эксплуатации в `v0.7.5`, и Rust-пространство в этом репозитории — это и есть продукт: единое нативное ядро, значительно меньшее и быстрее, работающее на большем количестве платформ из единой кодовой базы.

|                          | TypeScript + Electron (устарел, `v0.7.5`)      | Rust (сегодня)                                                                |
| ------------------------ | ---------------------------------------------- | ----------------------------------------------------------------------------- |
| **Рантайм на десктопе**  | Electron — включает Chromium + Node.js         | Нативное окно (`winit` + GPU Skia), без браузерного движка                    |
| **Размер на десктопе**   | Полный рантайм Chromium при каждой установке   | Один самодостаточный бинарный файл — **55.5 MB**                              |
| **Объём для веба**       | JS + WASM бандл                                | **8.2 MB** wasm / **2.18 MB** gzip по сети                                   |
| **Рендеринг**            | CanvasKit/Skia на вебе                         | Единый GPU-ускоренный Skia бэкенд на **каждой** платформе                     |
| **Память**               | Паузы JavaScript GC                            | Без GC — владение Rust, предсказуемая задержка                                |
| **Кодовая база**         | Веб-стек + Electron           | Одно Rust-пространство: редактор · CLI · MCP · AI · кодген · Figma · Git     |
| **Платформы**            | Веб + десктоп, два раздельных стека            | Десктоп (macOS/Win/Linux) · мобильные (iOS/Android) · браузер — одно ядро    |

**Измеренные улучшения**

- **Минимальный размер** — всё десктопное приложение представляет собой один нативный бинарный файл **55.5 MB** вместо встроенного браузерного движка плюс Node-рантайм. Веб-сборка составляет **8.2 MB** в несжатом виде / **2.18 MB** gzip после разбивки каталога иконок (−48% по сети).
- **Масштабируется на больших документах** — холст с **10,000 узлами** в реальном времени (вложенный авто-layout, четыре уровня глубины) записывает, читает и делает снимок раскладки **без паник и при ~0% CPU в режиме ожидания**; полный снимок раскладки всех 10k узлов возвращается за **~0.68 с**.
- **Быстрое взаимодействие** — панорамирование/зум больше не сериализует документ заново на каждом кадре (одно исправление горячего пути снизило CPU при зуме колесом с **~69% до ~0%**); перетаскивание обновляет сцену инкрементально, измерение текста кэшируется, а перерисовки объединяются в одну на кадр.
- **Одно ядро, каждый экран** — одно и то же состояние редактора и один и тот же рендер-бэкенд компилируются в нативный десктоп, мобильные устройства и браузер через WASM — без параллельных реализаций, которые нужно синхронизировать.
- **GPU Skia везде** — нативный рендеринг через `skia-safe` на GL-контексте; браузер рендерит через CanvasKit на WebGL2 — один и тот же код рисования, один и тот же вывод.
- **Нативная доступность** — AccessKit на macOS, Windows и Linux, плюс DOM-зеркало на вебе, вместо опоры на дерево доступности браузера.
- **Одно типизированное пространство** — хост MCP, CLI, AI-провайдеры, генерация кода, импорт Figma и интеграция с Git — всё в одном Rust-пространстве с контролем цепочки поставок через `cargo-deny` в CI.

> **Статус:** редактор на TypeScript был выведен из эксплуатации в `v0.7.5` и сохранился только в истории git; этот репозиторий — это Rust-пространство. Релиз `v0.8.1` на Rust находится в активной разработке (см. [Дорожная карта](#дорожная-карта) ниже).

## Структура проекта

```text
openpencil/
├── crates/                   Rust-пространство — продукт
│   ├── op-editor-core/       Каноническое состояние редактора `.op` (PenDocument) + EditorCommand + переменные дизайна
│   ├── op-editor-ui/         Платформонезависимые виджеты + фасад RenderBackend (wasm32-clean)
│   ├── op-editor-host-core/  Транспортонезависимые state machine хостов, общие для всех хостов
│   ├── op-host-native/       Библиотека нативного хоста — winit + skia-safe GL (десктоп + мобильные)
│   ├── op-host-web/          Браузерный бандл — wasm32 cdylib, рендерер CanvasKit
│   ├── op-host-desktop/      Десктопный бинарный файл `openpencil-desktop`; также демон `--serve-web`
│   ├── op-host-services/     Библиотека headless-демона serve-web / MCP
│   ├── op-host-web-server/   Тонкий веб-серверный бинарный файл без GL
│   ├── op-cli/               CLI-инструмент — команда `op`
│   ├── op-mcp/               MCP-сервер — инструменты, пакетный дизайн, послойный рабочий процесс
│   ├── op-ai/                AI-провайдеры, рантайм чата, стриминг
│   ├── op-ai-skills/         Движок AI-навыков (фазовая загрузка промптов)
│   ├── op-orchestrator/      Оркестрация параллельных команд агентов
│   ├── op-codegen/           Генераторы кода (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Парсер и конвертер файлов Figma .fig
│   ├── op-git/               Интеграция с Git — клонирование, ветки, push/pull, слияние
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Пространство веб-SDK (Bun)
│   ├── op-web-sdk/           SDK веб-просмотрщика `.op` только для чтения (обёртка wasm-бандла)
│   ├── op-web-sdk-react/     Адаптер React 19
│   └── op-web-sdk-vue/       Адаптер Vue 3
├── vendor/                   Вендоренные подсистемы (git submodules)
│   ├── jian/                 GPU-Skia UI-фреймворк — виджеты/рендеринг/события
│   ├── casement/             Форк winit
│   └── agent/                Кроссплатформенный Rust-рантайм агентов (agent-rs)
└── .githooks/                Pre-commit синхронизация версий из имени ветки
```

## Горячие клавиши

| Клавиша     | Действие           |     | Клавиша       | Действие                                 |
| ----------- | ------------------ | --- | ------------- | ---------------------------------------- |
| `V`         | Выбор              |     | `Cmd+S`       | Сохранить                                |
| `R`         | Прямоугольник      |     | `Cmd+Z`       | Отменить                                 |
| `O`         | Эллипс             |     | `Cmd+Shift+Z` | Повторить                                |
| `L`         | Линия              |     | `Cmd+C/X/V/D` | Копировать/Вырезать/Вставить/Дублировать |
| `T`         | Текст              |     | `Cmd+G`       | Сгруппировать                            |
| `F`         | Frame              |     | `Cmd+Shift+G` | Разгруппировать                          |
| `P`         | Инструмент пера    |     | `Cmd+Shift+P` | Экспорт (PNG/JPG/WEBP/PDF)               |
| `H`         | Рука (панорама)    |     | `Cmd+Shift+C` | Панель кода                              |
| `Del`       | Удалить            |     | `Cmd+Shift+V` | Панель переменных                        |
| `[ / ]`     | Изменить порядок   |     | `Cmd+J`       | AI-чат                                   |
| Стрелки     | Сдвиг на 1px       |     | `Cmd+,`       | Настройки агента                         |
| `Cmd+Alt+U` | Булево объединение |     | `Cmd+Alt+S`   | Булево вычитание                         |
| `Cmd+Alt+I` | Булево пересечение |     | `Cmd+Shift+S` | Сохранить как                            |

## Скрипты

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
cd packages && bun run sync-version               # Sync SDK versions from root Cargo.toml
cd packages && bun run sync-version:check         # Verify SDK versions match root Cargo.toml
```

## Участие в разработке

Мы приветствуем вклад в проект! Подробности об архитектуре и стиле кода смотрите в [CLAUDE.md](./CLAUDE.md).

1. Сделайте форк и клонируйте репозиторий
2. Настройте синхронизацию версий: `git config core.hooksPath .githooks`
3. Создайте ветку: `git checkout -b feat/my-feature`
4. Запустите проверки: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
5. Сделайте коммит в формате [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. Откройте PR в ветку `main`

## Дорожная карта

- [x] Переменные и токены дизайна с CSS-синхронизацией
- [x] Система компонентов (экземпляры и переопределения)
- [x] Генерация дизайна с помощью AI и оркестратора
- [x] Интеграция с MCP-сервером и послойный рабочий процесс
- [x] Поддержка нескольких страниц
- [x] Импорт Figma `.fig`
- [x] Булевы операции (объединение, вычитание, пересечение)
- [x] Мультимодельные профили возможностей
- [x] Workspace Cargo с переиспользуемыми Rust-крейтами и пакетами веб-SDK
- [x] Rust-редактор для настольных систем и веба
- [x] CLI-инструмент (`op`) для управления из терминала
- [x] Встроенная среда выполнения Rust-агентов с поддержкой нескольких провайдеров
- [x] i18n — 15 языков
- [x] SDK для просмотра на базе wasm для JavaScript, React и Vue
- [x] Style Guides с сопоставлением по тегам и инструментами MCP
- [x] Параллельные Agent Teams с делегированием и индикаторами на холсте
- [x] Интеграция с Git (клон, ветки, push/pull, трёхстороннее слияние в режиме папки)
- [x] Экспорт холста (SVG / PNG / JPEG / WEBP / PDF)
- [ ] Совместное редактирование
- [ ] Система плагинов

## Участники

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Спонсоры

OpenPencil бесплатен и open source. Разработка финансируется теми, кому он полезен — спасибо, что держите холст открытым.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Спасибо **[MrQyun](https://github.com/mrqyun)** — хотите видеть здесь своё имя? **[Стать спонсором →](https://github.com/sponsors/ZSeven-W)**

## Сообщество

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
  <strong> Присоединяйтесь к нашему Discord</strong>
</a>
— Задавайте вопросы, делитесь дизайнами, предлагайте функции.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Оценки

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Лицензия

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
