# @zseven-w/openpencil

[English](./README.md) · [简体中文](./README.zh.md) · [繁體中文](./README.zh-TW.md) · [日本語](./README.ja.md) · [한국어](./README.ko.md) · [**Français**](./README.fr.md) · [Español](./README.es.md) · [Deutsch](./README.de.md) · [Português](./README.pt.md) · [Русский](./README.ru.md) · [हिन्दी](./README.hi.md) · [Türkçe](./README.tr.md) · [ไทย](./README.th.md) · [Tiếng Việt](./README.vi.md) · [Bahasa Indonesia](./README.id.md)

CLI pour [OpenPencil](https://github.com/ZSeven-W/openpencil) — controlez l'outil de design depuis votre terminal.

## Installation

```bash
npm install -g @zseven-w/openpencil
```

## Plateformes supportees

Le CLI detecte et lance automatiquement l'application de bureau OpenPencil sur toutes les plateformes :

| Plateforme  | Chemins d'installation detectes                                                                     |
| ----------- | --------------------------------------------------------------------------------------------------- |
| **macOS**   | `/Applications/OpenPencil.app`, `~/Applications/OpenPencil.app`                                     |
| **Windows** | NSIS par utilisateur (`%LOCALAPPDATA%`), par machine (`%PROGRAMFILES%`), portable                    |
| **Linux**   | `/usr/bin`, `/usr/local/bin`, `~/.local/bin`, AppImage (`~/Applications`, `~/Downloads`), Snap, Flatpak |

## Utilisation

```bash
op <commande> [options]
```

### Methodes de saisie

Les arguments acceptant du JSON ou du DSL peuvent etre passes de trois manieres :

```bash
op design '...'              # Chaine en ligne (petites charges)
op design @design.txt        # Lecture depuis un fichier (recommande pour les grands designs)
cat design.txt | op design - # Lecture depuis stdin (pipe)
```

### Controle de l'application

```bash
op start [--desktop|--web]   # Lancer OpenPencil (bureau par defaut)
op stop                      # Arreter l'instance en cours
op status                    # Verifier si l'application est en cours d'execution
```

### Design (DSL par lot)

```bash
op design <dsl|@file|-> [--post-process] [--canvas-width N]
op design:skeleton <json|@file|->
op design:content <section-id> <json|@file|->
op design:refine --root-id <id>
```

### Operations sur les documents

```bash
op open [file.op]            # Ouvrir un fichier ou se connecter au canevas actif
op save <file.op>            # Enregistrer le document actuel
op get [--type X] [--name Y] [--id Z] [--depth N]
op selection                 # Obtenir la selection actuelle du canevas
```

### Manipulation des noeuds

```bash
op insert <json> [--parent P] [--index N] [--post-process]
op update <id> <json> [--post-process]
op delete <id>
op move <id> --parent <P> [--index N]
op copy <id> [--parent P]
op replace <id> <json> [--post-process]
```

### Export de code

```bash
op export <format> [--out file]
# Formats : react, html, vue, svelte, flutter, swiftui, compose, rn, css
```

### Variables et themes

```bash
op vars                      # Obtenir les variables
op vars:set <json>           # Definir les variables
op themes                    # Obtenir les themes
op themes:set <json>         # Definir les themes
op theme:save <file.optheme> # Enregistrer un preset de theme
op theme:load <file.optheme> # Charger un preset de theme
op theme:list [dir]          # Lister les presets de theme
```

### Pages

```bash
op page list                 # Lister les pages
op page add [--name N]       # Ajouter une page
op page remove <id>          # Supprimer une page
op page rename <id> <name>   # Renommer une page
op page reorder <id> <index> # Reordonner une page
op page duplicate <id>       # Dupliquer une page
```

### Importation

```bash
op import:svg <file.svg>     # Importer un fichier SVG
op import:figma <file.fig>   # Importer un fichier Figma .fig
```

### Mise en page

```bash
op layout [--parent P] [--depth N]
op find-space [--direction right|bottom|left|top]
```

### Options globales

```text
--file <path>     Fichier .op cible (par defaut : canevas actif)
--page <id>       ID de la page cible
--pretty          Sortie JSON lisible
--help            Afficher l'aide
--version         Afficher la version
```

## Licence

MIT
