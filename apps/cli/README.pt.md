# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [Français](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [**Português**](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI para o [OpenPencil](https://github.com/ZSeven-W/openpencil) — controle a ferramenta de design pelo seu terminal.

## Instalar

```bash
npm install -g @zseven-w/openpencil
```

## Suporte a Plataformas

A CLI detecta e inicia automaticamente o aplicativo desktop OpenPencil em todas as plataformas:

| Plataforma  | Caminhos de instalacao detectados                                                                   |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS por usuario (`%LOCALAPPDATA%`), por maquina (`%PROGRAMFILES%`), portatil                        |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Uso

```bash
op <comando> [opcoes]
```

### Metodos de Entrada

Argumentos que aceitam JSON ou DSL podem ser passados de tres formas:

```bash
op design '...'              # String inline (payloads pequenos)
op design @design.txt        # Ler de arquivo (recomendado para designs grandes)
cat design.txt | op design - # Ler da entrada padrao (piping)
```

### Controle do Aplicativo

```bash
op start [--desktop|--web]   # Iniciar o OpenPencil (desktop por padrao)
op stop                      # Parar a instancia em execucao
op status                    # Verificar se esta em execucao
```

### Design (DSL em Lote)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Operacoes de Documento

```bash
op open [file.op]            # Abrir arquivo ou conectar ao canvas ativo
op save <file.op>            # Salvar o documento atual
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Obter a selecao atual do canvas
```

### Manipulacao de Nos

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Exportacao de Codigo

```bash
op export <format> [--out file]
# Formatos: react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variaveis e Temas

```bash
op vars                      # Obter variaveis
op vars:set <json>           # Definir variaveis
op themes                    # Obter temas
op themes:set <json>         # Definir temas
op theme:save <file.optheme> # Salvar preset de tema
op theme:load <file.optheme> # Carregar preset de tema
op theme:list [dir]          # Listar presets de temas
```

### Paginas

```bash
op page list                 # Listar paginas
op page add [--name N]       # Adicionar uma pagina
op page remove <id>          # Remover uma pagina
op page rename <id> <name>   # Renomear uma pagina
op page reorder <id> <index> # Reordenar uma pagina
op page duplicate <id>       # Duplicar uma pagina
```

### Importacao

```bash
op import:svg <file.svg>     # Importar arquivo SVG
op import:figma <file.fig>   # Importar arquivo .fig do Figma
```

### Layout

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Flags Globais

```text
--file <path>     Arquivo .op alvo (padrao: canvas ativo)
--page <id>       ID da pagina alvo
--pretty          Saida JSON legivel
--help            Mostrar ajuda
--version         Mostrar versao
```

## Licenca

MIT
