# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [**Español**](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI para [OpenPencil](https://github.com/ZSeven-W/openpencil) — controla la herramienta de diseno desde tu terminal.

## Instalacion

```bash
npm install -g @zseven-w/openpencil
```

## Soporte de plataformas

El CLI detecta y lanza automaticamente la aplicacion de escritorio OpenPencil en todas las plataformas:

| Plataforma  | Rutas de instalacion detectadas                                                                     |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS por usuario (`%LOCALAPPDATA%`), por maquina (`%PROGRAMFILES%`), portable                       |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Uso

```bash
op <comando> [opciones]
```

### Metodos de entrada

Los argumentos que aceptan JSON o DSL se pueden pasar de tres maneras:

```bash
op design '...'              # Cadena en linea (cargas pequenas)
op design @design.txt        # Leer desde archivo (recomendado para disenos grandes)
cat design.txt | op design - # Leer desde stdin (tuberia)
```

### Control de la aplicacion

```bash
op start [--desktop|--web]   # Iniciar OpenPencil (escritorio por defecto)
op stop                      # Detener la instancia en ejecucion
op status                    # Verificar si esta en ejecucion
```

### Diseno (DSL por lotes)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Operaciones de documento

```bash
op open [file.op]            # Abrir archivo o conectar al lienzo activo
op save <file.op>            # Guardar el documento actual
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Obtener la seleccion actual del lienzo
```

### Manipulacion de nodos

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Exportacion de codigo

```bash
op export <format> [--out file]
# Formatos: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variables y temas

```bash
op vars                      # Obtener variables
op vars:set <json>           # Establecer variables
op themes                    # Obtener temas
op themes:set <json>         # Establecer temas
op theme:save <file.optheme> # Guardar preset de tema
op theme:load <file.optheme> # Cargar preset de tema
op theme:list [dir]          # Listar presets de tema
```

### Paginas

```bash
op page list                 # Listar paginas
op page add [--name N]       # Agregar una pagina
op page remove <id>          # Eliminar una pagina
op page rename <id> <name>   # Renombrar una pagina
op page reorder <id> <index> # Reordenar una pagina
op page duplicate <id>       # Duplicar una pagina
```

### Importacion

```bash
op import:svg <file.svg>     # Importar archivo SVG
op import:figma <file.fig>   # Importar archivo Figma .fig
```

### Disposicion

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Opciones globales

```text
--file <path>     Archivo .op de destino (por defecto: lienzo activo)
--page <id>       ID de la pagina de destino
--pretty          Salida JSON legible
--help            Mostrar ayuda
--version         Mostrar version
```

## Licencia

MIT
