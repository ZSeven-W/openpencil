# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [**Русский**](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI для [OpenPencil](https://github.com/ZSeven-W/openpencil) — управляйте инструментом дизайна из терминала.

## Установка

```bash
npm install -g @zseven-w/openpencil
```

## Поддержка платформ

CLI автоматически обнаруживает и запускает настольное приложение OpenPencil на всех платформах:

| Платформа   | Обнаруживаемые пути установки                                                                       |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS для пользователя (`%LOCALAPPDATA%`), для машины (`%PROGRAMFILES%`), портативная версия          |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Использование

```bash
op <команда> [параметры]
```

### Методы ввода

Аргументы, принимающие JSON или DSL, можно передать тремя способами:

```bash
op design '...'              # Встроенная строка (небольшие данные)
op design @design.txt        # Чтение из файла (рекомендуется для больших дизайнов)
cat design.txt | op design - # Чтение из stdin (через конвейер)
```

### Управление приложением

```bash
op start [--desktop|--web]   # Запустить OpenPencil (по умолчанию — настольное приложение)
op stop                      # Остановить запущенный экземпляр
op status                    # Проверить, запущено ли приложение
```

### Дизайн (пакетный DSL)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Операции с документом

```bash
op open [file.op]            # Открыть файл или подключиться к активному холсту
op save <file.op>            # Сохранить текущий документ
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Получить текущее выделение на холсте
```

### Работа с узлами

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Экспорт кода

```bash
op export <format> [--out file]
# Форматы: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Переменные и темы

```bash
op vars                      # Получить переменные
op vars:set <json>           # Задать переменные
op themes                    # Получить темы
op themes:set <json>         # Задать темы
op theme:save <file.optheme> # Сохранить пресет темы
op theme:load <file.optheme> # Загрузить пресет темы
op theme:list [dir]          # Список пресетов тем
```

### Страницы

```bash
op page list                 # Список страниц
op page add [--name N]       # Добавить страницу
op page remove <id>          # Удалить страницу
op page rename <id> <name>   # Переименовать страницу
op page reorder <id> <index> # Изменить порядок страницы
op page duplicate <id>       # Дублировать страницу
```

### Импорт

```bash
op import:svg <file.svg>     # Импортировать SVG-файл
op import:figma <file.fig>   # Импортировать файл Figma .fig
```

### Макет

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Глобальные флаги

```text
--file <path>     Целевой файл .op (по умолчанию: активный холст)
--page <id>       ID целевой страницы
--pretty          Читаемый вывод JSON
--help            Показать справку
--version         Показать версию
```

## Лицензия

MIT
