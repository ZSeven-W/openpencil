<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Нажмите на изображение, чтобы посмотреть демо-видео</sub></p>

<br />

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

Веб-приложение + нативный десктоп на macOS, Windows и Linux через Electron. Автообновление из GitHub Releases. Ассоциация файлов `.op` — двойной клик для открытия.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Управляйте инструментом дизайна из терминала. `op design`, `op insert`, `op export` — пакетный DSL дизайна, манипуляция узлами, экспорт кода. Ввод через pipe из файлов или stdin. Работает с десктопным приложением или веб-сервером.

</td>
<td width="50%">

### 🎯 Мультиплатформенный экспорт кода

Экспорт из одного файла `.op` в React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Переменные дизайна превращаются в пользовательские свойства CSS.

</td>
</tr>
</table>

## Быстрый старт

```bash
# Установить зависимости
bun install

# Запустить сервер разработки на http://localhost:3000
bun --bun run dev
```

Или запустить как десктопное приложение:

```bash
bun run electron:dev
```

> **Требования:** [Bun](https://bun.sh/) >= 1.0 и [Node.js](https://nodejs.org/) >= 18

### Docker

Доступно несколько вариантов образов — выберите подходящий для ваших нужд:

| Образ | Размер | Содержит |
| --- | --- | --- |
| `openpencil:latest` | ~226 МБ | Только веб-приложение |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 ГБ | Все CLI-инструменты |

**Запуск (только веб):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Запуск с AI CLI (например, Claude Code):**

AI-чат использует OAuth-авторизацию Claude CLI. Используйте Docker-том для сохранения сессии авторизации:

```bash
# Шаг 1 — Авторизация (однократно)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Шаг 2 — Запуск
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Локальная сборка:**

```bash
# Базовый (только веб)
docker build --target base -t openpencil .

# С конкретным CLI
docker build --target with-claude -t openpencil-claude .

# Полный (все CLI)
docker build --target full -t openpencil-full .
```

## AI-нативный дизайн

**От запроса к UI**
- **Текст в дизайн** — опишите страницу и получите её на холсте в реальном времени со стриминговой анимацией
- **Оркестратор** — разбивает сложные страницы на пространственные подзадачи для параллельной генерации
- **Изменение дизайна** — выберите элементы и опишите изменения на естественном языке
- **Визуальный ввод** — прикрепляйте скриншоты или макеты в качестве референса для дизайна

**Поддержка нескольких агентов**

| Агент | Настройка |
| --- | --- |
| **Claude Code** | Без настройки — использует Claude Agent SDK с локальным OAuth |
| **Codex CLI** | Подключить в настройках агента (`Cmd+,`) |
| **OpenCode** | Подключить в настройках агента (`Cmd+,`) |
| **GitHub Copilot** | `copilot login`, затем подключить в настройках агента (`Cmd+,`) |
| **Gemini CLI** | Подключить в настройках агента (`Cmd+,`) |

**Профили возможностей моделей** — автоматически адаптирует промпты, режим thinking и таймауты для каждого уровня моделей. Модели полного уровня (Claude) получают полные промпты; стандартного уровня (GPT-4o, Gemini, DeepSeek) — с отключённым thinking; базового уровня (MiniMax, Qwen, Llama, Mistral) — упрощённые промпты с вложенным JSON для максимальной надёжности.

**MCP-сервер**
- Встроенный MCP-сервер — установка в один клик в Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLI
- Автоопределение Node.js — если не установлен, автоматический переход на HTTP-транспорт и автозапуск MCP HTTP-сервера
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
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Запустить десктопное приложение
op design @landing.txt       # Пакетный дизайн из файла
op insert '{"type":"RECT"}'  # Вставить узел
op export react --out .      # Экспорт в React + Tailwind
op import:figma design.fig   # Импортировать файл Figma
cat design.dsl | op design - # Передача через stdin
```

Поддерживает три метода ввода: строка, `@filepath` (чтение из файла) или `-` (чтение из stdin). Работает с десктопным приложением или веб-сервером разработки. Подробнее в [CLI README](./apps/cli/README.md).

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

**Импорт из Figma**
- Импорт файлов `.fig` с сохранением раскладки, заливок, обводок, эффектов, текста, изображений и векторов

**Десктопное приложение**
- Нативная поддержка macOS, Windows и Linux через Electron
- Ассоциация файлов `.op` — двойной клик для открытия, блокировка единственного экземпляра
- Автообновление из GitHub Releases
- Нативное меню приложения и диалоги файлов

## Технологический стек

| | |
| --- | --- |
| **Фронтенд** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Холст** | CanvasKit/Skia (WASM, GPU-ускорение) |
| **Состояние** | Zustand v5 |
| **Сервер** | Nitro |
| **Десктоп** | Electron 35 |
| **CLI** | `op` — управление из терминала, пакетный DSL дизайна, экспорт кода |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Среда выполнения** | Bun · Vite 7 |
| **Формат файла** | `.op` — на основе JSON, удобочитаемый, дружественный к Git |

## Структура проекта

```text
openpencil/
├── apps/
│   ├── web/                 Веб-приложение TanStack Start
│   │   ├── src/
│   │   │   ├── canvas/      Движок CanvasKit/Skia — рисование, синхронизация, раскладка
│   │   │   ├── components/  React UI — редактор, панели, общие диалоги, иконки
│   │   │   ├── services/ai/ AI-чат, оркестратор, генерация дизайна, стриминг
│   │   │   ├── stores/      Zustand — холст, документ, страницы, история, AI
│   │   │   ├── mcp/         Инструменты MCP-сервера для интеграции с внешними CLI
│   │   │   ├── hooks/       Горячие клавиши, перетаскивание файлов, вставка из Figma
│   │   │   └── uikit/       Система переиспользуемых наборов компонентов
│   │   └── server/
│   │       ├── api/ai/      Nitro API — стриминговый чат, генерация, валидация
│   │       └── utils/       Обёртки клиентов Claude CLI, OpenCode, Codex, Copilot
│   ├── desktop/             Десктопное приложение Electron
│   │   ├── main.ts          Окно, форк Nitro, нативное меню, автообновление
│   │   ├── ipc-handlers.ts  Нативные файловые диалоги, синхронизация темы, настройки IPC
│   │   └── preload.ts       IPC-мост
│   └── cli/                 CLI-инструмент — команда `op`
│       ├── src/commands/    Команды: дизайн, документ, экспорт, импорт, узлы, страницы, переменные
│       ├── connection.ts    WebSocket-соединение с запущенным приложением
│       └── launcher.ts      Автоопределение и запуск десктопного приложения или веб-сервера
├── packages/
│   ├── pen-types/           Определения типов для модели PenDocument
│   ├── pen-core/            Операции с деревом документа, движок раскладки, переменные
│   ├── pen-codegen/         Генераторы кода (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Парсер и конвертер файлов Figma .fig
│   ├── pen-renderer/        Автономный рендерер CanvasKit/Skia
│   └── pen-sdk/             Зонтичный SDK (реэкспортирует все пакеты)
└── .githooks/               Pre-commit синхронизация версий из имени ветки
```

## Горячие клавиши

| Клавиша | Действие | | Клавиша | Действие |
| --- | --- | --- | --- | --- |
| `V` | Выбор | | `Cmd+S` | Сохранить |
| `R` | Прямоугольник | | `Cmd+Z` | Отменить |
| `O` | Эллипс | | `Cmd+Shift+Z` | Повторить |
| `L` | Линия | | `Cmd+C/X/V/D` | Копировать/Вырезать/Вставить/Дублировать |
| `T` | Текст | | `Cmd+G` | Сгруппировать |
| `F` | Frame | | `Cmd+Shift+G` | Разгруппировать |
| `P` | Инструмент пера | | `Cmd+Shift+E` | Экспорт |
| `H` | Рука (панорама) | | `Cmd+Shift+C` | Панель кода |
| `Del` | Удалить | | `Cmd+Shift+V` | Панель переменных |
| `[ / ]` | Изменить порядок | | `Cmd+J` | AI-чат |
| Стрелки | Сдвиг на 1px | | `Cmd+,` | Настройки агента |
| `Cmd+Alt+U` | Булево объединение | | `Cmd+Alt+S` | Булево вычитание |
| `Cmd+Alt+I` | Булево пересечение | | | |

## Скрипты

```bash
bun --bun run dev          # Сервер разработки (порт 3000)
bun --bun run build        # Сборка для продакшена
bun --bun run test         # Запустить тесты (Vitest)
npx tsc --noEmit           # Проверка типов
bun run bump <version>     # Синхронизация версий во всех package.json
bun run electron:dev       # Разработка Electron
bun run electron:build     # Упаковка Electron
bun run cli:dev            # Запуск CLI из исходников
bun run cli:compile        # Компиляция CLI в dist
```

## Участие в разработке

Мы приветствуем вклад в проект! Подробности об архитектуре и стиле кода смотрите в [CLAUDE.md](./CLAUDE.md).

1. Сделайте форк и клонируйте репозиторий
2. Настройте синхронизацию версий: `git config core.hooksPath .githooks`
3. Создайте ветку: `git checkout -b feat/my-feature`
4. Запустите проверки: `npx tsc --noEmit && bun --bun run test`
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
- [x] Реструктуризация в монорепозиторий с переиспользуемыми пакетами
- [x] CLI-инструмент (`op`) для управления из терминала
- [ ] Совместное редактирование
- [ ] Система плагинов

## Участники

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Сообщество

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
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

## Лицензия

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
