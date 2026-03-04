<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>AI-нативный инструмент дизайна с открытым исходным кодом. Дизайн как код.</strong><br />
  От текстового запроса к UI на холсте. Многоагентная оркестрация. Встроенный MCP-сервер. Генерация кода.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md"><b>Русский</b></a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#quick-start">Быстрый старт</a> ·
  <a href="#ai-native-design">AI</a> ·
  <a href="#features">Возможности</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contributing">Участие в разработке</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Нажмите на изображение, чтобы посмотреть демо-видео</sub></p>

<br />

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

## AI-нативный дизайн

OpenPencil построен вокруг AI с самого начала — не как плагин, а как основной рабочий процесс.

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

**MCP-сервер**
- Встроенный MCP-сервер — установка в один клик в Claude Code / Codex / Gemini / OpenCode / Kiro CLI
- Автоматизация дизайна из терминала: чтение, создание и изменение файлов `.op` через любой MCP-совместимый агент

**Генерация кода**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables из дизайн-токенов

## Возможности

**Холст и рисование**
- Бесконечный холст с панорамированием, масштабированием, умными направляющими и привязкой
- Прямоугольник, Эллипс, Линия, Многоугольник, Перо (Безье), Frame, Текст
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
- Автообновление из GitHub Releases
- Нативное меню приложения и диалоги файлов

## Технологический стек

| | |
| --- | --- |
| **Фронтенд** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Холст** | Fabric.js v7 |
| **Состояние** | Zustand v5 |
| **Сервер** | Nitro |
| **Десктоп** | Electron 35 |
| **AI** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Среда выполнения** | Bun · Vite 7 |
| **Формат файла** | `.op` — на основе JSON, удобочитаемый, дружественный к Git |

## Структура проекта

```text
src/
  canvas/          Движок Fabric.js — рисование, синхронизация, раскладка, направляющие, инструмент пера
  components/      React UI — редактор, панели, общие диалоги, иконки
  services/ai/     AI-чат, оркестратор, генерация дизайна, стриминг
  services/figma/  Пайплайн бинарного импорта Figma .fig
  services/codegen Генераторы кода React+Tailwind и HTML+CSS
  stores/          Zustand — холст, документ, страницы, история, AI, настройки
  variables/       Разрешение дизайн-токенов и управление ссылками
  mcp/             Инструменты MCP-сервера для интеграции с внешними CLI
  uikit/           Система переиспользуемых наборов компонентов
server/
  api/ai/          Nitro API — стриминговый чат, генерация, валидация
  utils/           Обёртки клиентов Claude CLI, OpenCode, Codex
electron/
  main.ts          Окно, форк Nitro, нативное меню, автообновление
  preload.ts       IPC-мост
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

## Скрипты

```bash
bun --bun run dev          # Сервер разработки (порт 3000)
bun --bun run build        # Сборка для продакшена
bun --bun run test         # Запустить тесты (Vitest)
npx tsc --noEmit           # Проверка типов
bun run electron:dev       # Разработка Electron
bun run electron:build     # Упаковка Electron
```

## Участие в разработке

Мы приветствуем вклад в проект! Подробности об архитектуре и стиле кода смотрите в [CLAUDE.md](./CLAUDE.md).

1. Сделайте форк и клонируйте репозиторий
2. Создайте ветку: `git checkout -b feat/my-feature`
3. Запустите проверки: `npx tsc --noEmit && bun --bun run test`
4. Сделайте коммит в формате [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Откройте PR в ветку `main`

## Дорожная карта

- [x] Переменные и токены дизайна с CSS-синхронизацией
- [x] Система компонентов (экземпляры и переопределения)
- [x] Генерация дизайна с помощью AI и оркестратора
- [x] Интеграция с MCP-сервером
- [x] Поддержка нескольких страниц
- [x] Импорт Figma `.fig`
- [ ] Булевы операции (объединение, вычитание, пересечение)
- [ ] Совместное редактирование
- [ ] Система плагинов

## Участники

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Сообщество

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Присоединяйтесь к нашему Discord</strong>
</a>
— Задавайте вопросы, делитесь дизайнами, предлагайте функции.

## Лицензия

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
