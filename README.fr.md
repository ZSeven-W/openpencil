<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Le premier outil de design vectoriel open-source natif IA au monde.</strong><br />
  <sub>Equipes d'agents concurrents &bull; Design-as-Code &bull; Serveur MCP intégré &bull; Intelligence multi-modèles</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — cliquez pour regarder la démo" width="100%" />
  </a>
</p>
<p align="center"><sub>Cliquez sur l'image pour regarder la vidéo de démonstration</sub></p>

<br />

## Pourquoi OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canevas

Décrivez n'importe quelle interface en langage naturel. Regardez-la apparaître sur le canevas infini en temps réel avec une animation en streaming. Modifiez des designs existants en sélectionnant des éléments et en conversant.

</td>
<td width="50%">

### 🤖 Équipes d'agents concurrents

L'orchestrateur décompose les pages complexes en sous-tâches spatiales. Plusieurs agents IA travaillent simultanément sur différentes sections — hero, fonctionnalités, pied de page — le tout en streaming parallèle.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Intelligence multi-modèles

S'adapte automatiquement aux capacités de chaque modèle. Claude obtient des prompts complets avec réflexion ; GPT-4o/Gemini désactivent la réflexion ; les modèles plus petits (MiniMax, Qwen, Llama) reçoivent des prompts simplifiés pour une sortie fiable.

</td>
<td width="50%">

### 🔌 Serveur MCP

Installation en un clic dans les CLI Claude Code, Codex, Gemini, OpenCode, Kiro ou Copilot. Designez depuis votre terminal — lisez, créez et modifiez des fichiers `.op` via tout agent compatible MCP.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

Les fichiers `.op` sont du JSON — lisibles par l'humain, compatibles Git, comparables. Les variables de design génèrent des propriétés personnalisées CSS. Export de code vers React + Tailwind ou HTML + CSS.

</td>
<td width="50%">

### 🖥️ Fonctionne partout

Application web + bureau natif sur macOS, Windows et Linux via Electron. Mises à jour automatiques depuis GitHub Releases. Association de fichiers `.op` — double-cliquez pour ouvrir.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Contrôlez l'outil de design depuis le terminal. `op design`, `op insert`, `op export` — DSL de design par lots, manipulation de nœuds, export de code. Entrée par pipe depuis des fichiers ou stdin. Fonctionne avec l'app de bureau ou le serveur web.

</td>
<td width="50%">

### 🎯 Export de Code Multi-Plateforme

Exportez depuis un seul fichier `.op` vers React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Les variables de design deviennent des propriétés CSS personnalisées.

</td>
</tr>
</table>

## Démarrage rapide

```bash
# Installer les dépendances
bun install

# Démarrer le serveur de développement sur http://localhost:3000
bun --bun run dev
```

Ou lancer en tant qu'application de bureau :

```bash
bun run electron:dev
```

> **Prérequis :** [Bun](https://bun.sh/) >= 1.0 et [Node.js](https://nodejs.org/) >= 18

### Docker

Plusieurs variantes d'images sont disponibles — choisissez celle qui correspond à vos besoins :

| Image | Taille | Contenu |
| --- | --- | --- |
| `openpencil:latest` | ~226 Mo | Application web uniquement |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 Go | Tous les outils CLI |

**Exécuter (web uniquement) :**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Exécuter avec un CLI IA (ex. Claude Code) :**

Le chat IA repose sur la connexion OAuth de Claude CLI. Utilisez un volume Docker pour conserver la session de connexion :

```bash
# Étape 1 — Connexion (une seule fois)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Étape 2 — Démarrer
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Compiler localement :**

```bash
# Base (web uniquement)
docker build --target base -t openpencil .

# Avec un CLI spécifique
docker build --target with-claude -t openpencil-claude .

# Complet (tous les CLIs)
docker build --target full -t openpencil-full .
```

## Design natif IA

**Du prompt à l'interface**
- **Texte vers design** — décrivez une page, elle est générée en temps réel sur le canevas avec une animation en streaming
- **Orchestrateur** — décompose les pages complexes en sous-tâches spatiales pour une génération parallèle
- **Modification de design** — sélectionnez des éléments, puis décrivez les modifications en langage naturel
- **Entrée vision** — joignez des captures d'écran ou des maquettes pour un design basé sur des références

**Support multi-agents**

| Agent | Configuration |
| --- | --- |
| **Claude Code** | Aucune configuration — utilise le Claude Agent SDK avec OAuth local |
| **Codex CLI** | Connecter dans les Paramètres de l'agent (`Cmd+,`) |
| **OpenCode** | Connecter dans les Paramètres de l'agent (`Cmd+,`) |
| **GitHub Copilot** | `copilot login` puis connecter dans les Paramètres de l'agent (`Cmd+,`) |
| **Gemini CLI** | Connecter dans les Paramètres de l'agent (`Cmd+,`) |

**Profils de capacités des modèles** — adapte automatiquement les prompts, le mode de réflexion et les délais d'attente par niveau de modèle. Les modèles de niveau complet (Claude) reçoivent des prompts complets ; le niveau standard (GPT-4o, Gemini, DeepSeek) désactive la réflexion ; le niveau basique (MiniMax, Qwen, Llama, Mistral) reçoit des prompts JSON imbriqués simplifiés pour une fiabilité maximale.

**Serveur MCP**
- Serveur MCP intégré — installation en un clic dans les CLI Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot
- Détection automatique de Node.js — si non installé, bascule vers le transport HTTP et démarre automatiquement le serveur MCP HTTP
- Automatisation du design depuis le terminal : lire, créer et modifier des fichiers `.op` via tout agent compatible MCP
- **Workflow de design en couches** — `design_skeleton` → `design_content` → `design_refine` pour des designs multi-sections de plus haute fidélité
- **Récupération segmentée des prompts** — chargez uniquement les connaissances de design nécessaires (schéma, layout, rôles, icônes, planification, etc.)
- Support multi-pages — créer, renommer, réordonner et dupliquer des pages via les outils MCP

**Génération de code**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Installez globalement et contrôlez l'outil de design depuis votre terminal :

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Lancer l'app de bureau
op design @landing.txt       # Design par lots depuis un fichier
op insert '{"type":"RECT"}'  # Insérer un nœud
op export react --out .      # Exporter en React + Tailwind
op import:figma design.fig   # Importer un fichier Figma
cat design.dsl | op design - # Pipe depuis stdin
```

Supporte trois méthodes d'entrée : chaîne en ligne, `@filepath` (lecture depuis un fichier), ou `-` (lecture depuis stdin). Fonctionne avec l'app de bureau ou le serveur de développement web. Voir le [README du CLI](./apps/cli/README.md) pour la référence complète des commandes.

## Fonctionnalités

**Canevas et dessin**
- Canevas infini avec panoramique, zoom, guides d'alignement intelligents et magnétisme
- Rectangle, Ellipse, Ligne, Polygone, Plume (Bézier), Frame, Texte
- Opérations booléennes — union, soustraction, intersection avec barre d'outils contextuelle
- Sélecteur d'icônes (Iconify) et import d'images (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — vertical/horizontal avec gap, padding, justify, align
- Documents multi-pages avec navigation par onglets

**Système de design**
- Variables de design — tokens de couleur, nombre et chaîne avec références `$variable`
- Support multi-thèmes — plusieurs axes, chacun avec des variantes (Clair/Sombre, Compact/Confortable)
- Système de composants — composants réutilisables avec instances et substitutions
- Synchronisation CSS — propriétés personnalisées auto-générées, `var(--name)` dans la sortie de code

**Import Figma**
- Importer des fichiers `.fig` en préservant la mise en page, les remplissages, les contours, les effets, le texte, les images et les vecteurs

**Application de bureau**
- macOS, Windows et Linux natifs via Electron
- Association de fichiers `.op` — double-cliquez pour ouvrir, verrouillage d'instance unique
- Mise à jour automatique depuis GitHub Releases
- Menu d'application natif et boîtes de dialogue de fichiers

## Stack technique

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canevas** | CanvasKit/Skia (WASM, accélération GPU) |
| **État** | Zustand v5 |
| **Serveur** | Nitro |
| **Bureau** | Electron 35 |
| **CLI** | `op` — contrôle depuis le terminal, DSL de design par lots, export de code |
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Format de fichier** | `.op` — basé sur JSON, lisible par l'humain, compatible Git |

## Structure du projet

```text
openpencil/
├── apps/
│   ├── web/                 Application web TanStack Start
│   │   ├── src/
│   │   │   ├── canvas/      Moteur CanvasKit/Skia — dessin, sync, mise en page
│   │   │   ├── components/  Interface React — éditeur, panneaux, boîtes de dialogue partagées, icônes
│   │   │   ├── services/ai/ Chat IA, orchestrateur, génération de design, streaming
│   │   │   ├── stores/      Zustand — canevas, document, pages, historique, IA
│   │   │   ├── mcp/         Outils serveur MCP pour l'intégration CLI externe
│   │   │   ├── hooks/       Raccourcis clavier, dépôt de fichiers, collage Figma
│   │   │   └── uikit/       Système de kits de composants réutilisables
│   │   └── server/
│   │       ├── api/ai/      API Nitro — chat en streaming, génération, validation
│   │       └── utils/       Enveloppes Claude CLI, OpenCode, Codex, Copilot
│   ├── desktop/             Application de bureau Electron
│   │   ├── main.ts          Fenêtre, fork Nitro, menu natif, mise à jour automatique
│   │   ├── ipc-handlers.ts  Dialogues fichiers natifs, sync thème, préférences IPC
│   │   └── preload.ts       Pont IPC
│   └── cli/                 Outil CLI — commande `op`
│       ├── src/commands/    Commandes design, document, export, import, nœud, page, variable
│       ├── connection.ts    Connexion WebSocket à l'app en cours d'exécution
│       └── launcher.ts      Détection automatique et lancement de l'app de bureau ou du serveur web
├── packages/
│   ├── pen-types/           Définitions de types pour le modèle PenDocument
│   ├── pen-core/            Opérations sur l'arbre du document, moteur de mise en page, variables
│   ├── pen-codegen/         Générateurs de code (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Parseur et convertisseur de fichiers Figma .fig
│   ├── pen-renderer/        Moteur de rendu CanvasKit/Skia autonome
│   └── pen-sdk/             SDK parapluie (réexporte tous les packages)
└── .githooks/               Synchronisation de version pre-commit depuis le nom de branche
```

## Raccourcis clavier

| Touche | Action | | Touche | Action |
| --- | --- | --- | --- | --- |
| `V` | Sélectionner | | `Cmd+S` | Enregistrer |
| `R` | Rectangle | | `Cmd+Z` | Annuler |
| `O` | Ellipse | | `Cmd+Shift+Z` | Rétablir |
| `L` | Ligne | | `Cmd+C/X/V/D` | Copier/Couper/Coller/Dupliquer |
| `T` | Texte | | `Cmd+G` | Grouper |
| `F` | Frame | | `Cmd+Shift+G` | Dégrouper |
| `P` | Outil plume | | `Cmd+Shift+E` | Exporter |
| `H` | Main (panoramique) | | `Cmd+Shift+C` | Panneau de code |
| `Del` | Supprimer | | `Cmd+Shift+V` | Panneau des variables |
| `[ / ]` | Réordonner | | `Cmd+J` | Chat IA |
| Flèches | Déplacer de 1px | | `Cmd+,` | Paramètres de l'agent |
| `Cmd+Alt+U` | Union booléenne | | `Cmd+Alt+S` | Soustraction booléenne |
| `Cmd+Alt+I` | Intersection booléenne | | | |

## Scripts

```bash
bun --bun run dev          # Serveur de développement (port 3000)
bun --bun run build        # Build de production
bun --bun run test         # Lancer les tests (Vitest)
npx tsc --noEmit           # Vérification des types
bun run bump <version>     # Synchroniser la version dans tous les package.json
bun run electron:dev       # Développement Electron
bun run electron:build     # Packaging Electron
bun run cli:dev            # Exécuter le CLI depuis les sources
bun run cli:compile        # Compiler le CLI vers dist
```

## Contribuer

Les contributions sont les bienvenues ! Consultez [CLAUDE.md](./CLAUDE.md) pour les détails d'architecture et le style de code.

1. Forker et cloner
2. Configurer la synchronisation de version : `git config core.hooksPath .githooks`
3. Créer une branche : `git checkout -b feat/my-feature`
4. Exécuter les vérifications : `npx tsc --noEmit && bun --bun run test`
5. Commiter avec [Conventional Commits](https://www.conventionalcommits.org/) : `feat(canvas): add rotation snapping`
6. Ouvrir une PR contre `main`

## Feuille de route

- [x] Variables de design & tokens avec synchronisation CSS
- [x] Système de composants (instances & substitutions)
- [x] Génération de design IA avec orchestrateur
- [x] Intégration du serveur MCP avec workflow de design en couches
- [x] Support multi-pages
- [x] Import Figma `.fig`
- [x] Opérations booléennes (union, soustraction, intersection)
- [x] Profils de capacités multi-modèles
- [x] Restructuration en monorepo avec packages réutilisables
- [x] Outil CLI (`op`) pour le contrôle depuis le terminal
- [ ] Édition collaborative
- [ ] Système de plugins

## Contributeurs

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Communauté

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Rejoindre notre Discord</strong>
</a>
— Posez des questions, partagez vos designs, suggérez des fonctionnalités.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Licence

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
