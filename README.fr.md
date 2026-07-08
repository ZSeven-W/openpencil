<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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

> **Note :** Il existe un autre projet open-source portant le même nom — [OpenPencil](https://github.com/open-pencil/open-pencil), axé sur le design visuel compatible Figma avec collaboration en temps réel. Ce projet est axé sur les workflows AI-natifs de design vers code.

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

Application web + bureau natif sur macOS, Windows et Linux — un seul cœur Rust, un binaire unique autonome, sans moteur de navigateur. Association de fichiers `.op` — double-cliquez pour ouvrir.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Contrôlez l'outil de design depuis le terminal. `op design`, `op insert` — DSL de design par lots, manipulation de nœuds. Entrée par pipe depuis des fichiers ou stdin. Fonctionne avec l'app de bureau ou le serveur web.

</td>
<td width="50%">

### 🎯 Export de Code Multi-Plateforme

Exportez depuis un seul fichier `.op` vers React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Les variables de design deviennent des propriétés CSS personnalisées.

</td>
</tr>
</table>

## Démarrage rapide

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Ou lancer en tant qu'application de bureau :

```bash
cargo run -p op-host-desktop
```

> **Prérequis :** [Rust](https://www.rust-lang.org/) (stable) pour compiler le produit. [Bun](https://bun.sh/) >= 1.0 et [Node.js](https://nodejs.org/) >= 18 ne sont nécessaires que pour le SDK web dans `packages/`.

### Docker

Plusieurs variantes d'images sont disponibles — choisissez celle qui correspond à vos besoins :

| Image                        | Taille  | Contenu                    |
| ---------------------------- | ------- | -------------------------- |
| `openpencil:latest`          | ~226 Mo | Application web uniquement |
| `openpencil-claude:latest`   | —       | + Claude Code CLI          |
| `openpencil-codex:latest`    | —       | + Codex CLI                |
| `openpencil-opencode:latest` | —       | + OpenCode CLI             |
| `openpencil-copilot:latest`  | —       | + GitHub Copilot CLI       |
| `openpencil-gemini:latest`   | —       | + Gemini CLI               |
| `openpencil-full:latest`     | ~1 Go   | Tous les outils CLI        |

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

| Agent                         | Configuration                                                                                                           |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------- |
| **Intégré (9+ fournisseurs)** | Choisissez parmi les préréglages de fournisseurs avec sélecteur de région — Anthropic, OpenAI, Google, DeepSeek et plus |
| **Claude Code**               | Aucune configuration — utilise le Claude Agent SDK avec OAuth local                                                     |
| **Codex CLI**                 | Connecter dans les Paramètres de l'agent (`Cmd+,`)                                                                      |
| **OpenCode**                  | Connecter dans les Paramètres de l'agent (`Cmd+,`)                                                                      |
| **GitHub Copilot**            | `copilot login` puis connecter dans les Paramètres de l'agent (`Cmd+,`)                                                 |
| **Gemini CLI**                | Connecter dans les Paramètres de l'agent (`Cmd+,`)                                                                      |

**Profils de capacités des modèles** — adapte automatiquement les prompts, le mode de réflexion et les délais d'attente par niveau de modèle. Les modèles de niveau complet (Claude) reçoivent des prompts complets ; le niveau standard (GPT-4o, Gemini, DeepSeek) désactive la réflexion ; le niveau basique (MiniMax, Qwen, Llama, Mistral) reçoit des prompts JSON imbriqués simplifiés pour une fiabilité maximale.

**i18n** — Localisation complète de l'interface en 15 langues : English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**Serveur MCP**

- Serveur MCP intégré (crate `op-mcp`) — installation en un clic dans les CLI Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot
- Aucun besoin de Node.js — transport stdio via le binaire de bureau (`--mcp <path>`), plus un point de terminaison HTTP en direct (`127.0.0.1:<port>/mcp`) depuis l'application en cours d'exécution
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # Lancer l'app de bureau
op design @landing.txt       # Design par lots depuis un fichier
op insert '{"type":"RECT"}'  # Insérer un nœud
op import:figma design.fig   # Importer un fichier Figma
cat design.dsl | op design - # Pipe depuis stdin
```

Supporte trois méthodes d'entrée : chaîne en ligne, `@filepath` (lecture depuis un fichier), ou `-` (lecture depuis stdin). Fonctionne avec l'app de bureau ou le serveur de développement web. Voir le [README du CLI](./crates/op-cli) pour la référence complète des commandes.

**Compétence LLM** — installez le plugin [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) pour apprendre aux agents IA (Claude Code, Cursor, Codex, Gemini CLI, etc.) à concevoir avec `op`.

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
- UIKits réutilisables — import/export de kits de composants depuis des fichiers `.pen`

**IA et agents**

- Prompt-vers-canevas avec génération en streaming et décomposition spatiale pilotée par l'orchestrateur
- Équipes d'agents concurrentes — plusieurs designers travaillent sur différentes sections en parallèle, avec des indicateurs sur le canevas par membre
- Workflow en couches — `design_skeleton` → `design_content` → `design_refine` avec des prompts ciblés par phase
- Guides de style — plus de 50 styles intégrés (glassmorphism, brutalist, retro, etc.) avec appariement flou basé sur les tags, intégrés dans la planification et la génération
- Profils de capacités multi-modèles — adapte automatiquement le mode de réflexion, l'effort et la forme du prompt selon le niveau du modèle
- Runtime d'agent intégré (Rust) + fournisseurs Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Passthrough au format Anthropic pour les fournisseurs LLM chinois — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Intégration Git**

- Assistant de clonage avec authentification SSH / HTTPS et gestion des clés SSH
- Sélecteur de branches — créer, changer, supprimer, fusionner, tout depuis le panneau Git
- Cascades pull / push avec réessai d'authentification et gestion non-fast-forward
- Fusion à trois voies en mode dossier avec suivi d'état `MERGE_HEAD` sur disque
- Panneau de conflits avec cartes à trois voies par nœud / champ, éditeur JSON inline, actions groupées et bloc diff inline
- UI de paramètres distants et de clés SSH ; i18n en 15 langues sur toute la surface Git

**Export**

- Export du canevas — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Export de code — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Pipeline de codegen MCP incrémental — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Import Figma**

- Importer des fichiers `.fig` en préservant la mise en page, les remplissages, les contours, les effets, le texte, les images et les vecteurs

**Application de bureau**

- macOS, Windows et Linux natifs — un binaire unique autonome (winit + GPU Skia, sans Electron)
- Association de fichiers `.op` — double-cliquez pour ouvrir, verrouillage d'instance unique
- Vérification des mises à jour en arrière-plan auprès de GitHub Releases
- Menu d'application natif avec Enregistrer sous, Ouvrir les récents et une boîte de dialogue de modifications non enregistrées à la fermeture
- Persistance des fichiers récents

## Stack technique

|                       |                                                                                             |
| --------------------- | ------------------------------------------------------------------------------------------- |
| **Cœur**              | Workspace Rust (`crates/`) — état de l'éditeur, widgets, hôtes, MCP, IA, codegen             |
| **Rendu**             | GPU Skia partout — `skia-safe` (GL) en natif, CanvasKit (WASM/WebGL2) dans le navigateur     |
| **Framework UI** | jian — framework UI GPU-Skia en Rust pur vendored : widgets, layout, événements, rechargement à chaud (`vendor/jian`) |
| **Fenêtrage**         | winit (fork vendored `casement`)                                                             |
| **Bureau**            | Binaire natif `openpencil-desktop` — sans moteur de navigateur                               |
| **SDK web**           | `op-web-sdk` + adaptateurs React 19 / Vue 3 — visualiseur `.op` en lecture seule (TypeScript) |
| **CLI**               | `op` — contrôle depuis le terminal, DSL de design par lots                                   |
| **IA**                | Runtime d'agent Rust intégré · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**              | clippy · rustfmt (Rust) · oxlint · oxfmt (SDK web)                                           |
| **Format de fichier** | `.op` — basé sur JSON, lisible par l'humain, compatible Git                                 |

## Écosystème

OpenPencil fait partie d'une famille d'outils en Rust pur, natifs IA, développés par **[ZSeven-W](https://github.com/ZSeven-W)**. Ils se composent : `jian` effectue le rendu d'OpenPencil, `agent-rs` exécute ses agents, `noema` mémorise, et `zode` conçoit depuis le terminal.

| Projet | De quoi il s'agit |
| ------ | ----------------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Assistant de code open-source natif IA pour votre terminal — une TUI Rust rapide (`ratatui`) qui lit votre code, exécute des commandes, recherche des fichiers et gère git. Pilote OpenPencil via MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Un runtime asynchrone en Rust pur pour déployer des agents LLM — multi-fournisseurs, capable d'utiliser des outils de bout en bout, permissions structurées, vrai MCP, zéro `unsafe`. Alimente le runtime d'agent intégré d'OpenPencil (`vendor/agent`) et Zode. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework UI GPU-Skia en Rust pur — widgets, layout, événements et rechargement à chaud dans une seule stack. Transforme un document déclaratif `.op` en une app native, contrôlable par IA, sans runtime JS, sans DOM, sans Electron. Le framework UI d'OpenPencil (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Système de mémoire local-first et non vectoriel pour les agents de code. Mémoire durable sous forme de fichiers inspectables, une file de revue pour les nouvelles entrées, et un rappel lexical (sans embedding) — fonctionne avec Zode, Codex, Claude Code et les runtimes MCP. |

## Pourquoi Rust

OpenPencil a été réécrit intégralement en **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). La réécriture est terminée — l'éditeur TypeScript + Electron a été retiré en `v0.7.5`, et le workspace Rust de ce dépôt est le produit : un cœur natif unique, nettement plus léger et plus rapide, fonctionnant sur davantage de plateformes à partir d'une seule base de code.

|                            | TypeScript + Electron (retiré, `v0.7.5`)               | Rust (aujourd'hui)                                                         |
| -------------------------- | ----------------------------------------------------- | -------------------------------------------------------------------------- |
| **Exécution bureau**       | Electron — embarque Chromium + Node.js                | Fenêtre native (`winit` + GPU Skia), sans moteur de navigateur              |
| **Empreinte bureau**       | Runtime Chromium complet par installation             | Binaire unique autonome — **55.5 MB**                                      |
| **Charge web**             | Bundle JS + WASM                                      | **8.2 MB** wasm / **2.18 MB** gzip sur le réseau                           |
| **Rendu**                  | CanvasKit/Skia sur le web                             | Un seul backend Skia accéléré GPU sur **chaque** cible                     |
| **Mémoire**                | Pauses du GC JavaScript                               | Sans GC — ownership Rust, latence prévisible                               |
| **Base de code**           | Stack web + Electron                 | Un workspace Rust : éditeur · CLI · MCP · AI · codegen · Figma · Git       |
| **Cibles**                 | Web + bureau, deux stacks séparées                    | Bureau (macOS/Win/Linux) · mobile (iOS/Android) · navigateur — un seul cœur |

**Améliorations mesurées**

- **Empreinte minimale** — l'intégralité de l'application bureau tient en un seul binaire natif de **55.5 MB**, au lieu d'un moteur de navigateur embarqué et d'un runtime Node. La version web pèse **8.2 MB** brut / **2.18 MB** gzip après découpage du catalogue d'icônes (−48% sur le réseau).
- **Passage à l'échelle sur les grands documents** — un canevas en direct de **10,000 nœuds** (auto-layout imbriqué, quatre niveaux de profondeur) écrit, lit et capture l'état de mise en page **sans panique et avec ~0% de CPU en veille** ; un snapshot complet de la mise en page des 10k nœuds est retourné en **~0.68 s**.
- **Interactions fluides** — le panoramique/zoom ne re-sérialise plus le document à chaque frame (un correctif sur un point chaud critique a ramené le CPU lors du zoom molette de **~69% à ~0%**) ; les glissés appliquent les modifications de scène de façon incrémentielle, la mesure du texte est mise en cache, et les repeints sont fusionnés en un seul par frame.
- **Un cœur, chaque écran** — le même état de l'éditeur et le même backend de rendu compilent vers le bureau natif, le mobile et le navigateur via WASM — sans réimplémentations parallèles à synchroniser.
- **GPU Skia partout** — le natif effectue le rendu via `skia-safe` sur un contexte GL ; le navigateur effectue le rendu via CanvasKit sur WebGL2 — le même code de dessin, le même résultat.
- **Accessibilité native** — AccessKit sur macOS, Windows et Linux, plus un miroir DOM sur le web, plutôt que de s'appuyer sur l'arbre a11y d'un navigateur.
- **Un workspace avec vérification de types** — l'hôte MCP, le CLI, les fournisseurs AI, la génération de code, l'import Figma et l'intégration Git résident tous dans un seul workspace Rust, avec filtrage de la chaîne d'approvisionnement par `cargo-deny` en CI.

> **Statut :** l'éditeur TypeScript a été retiré en `v0.7.5` et ne subsiste que dans l'historique Git ; ce dépôt est le workspace Rust. La version Rust `v0.8.0` est en développement actif (voir la Feuille de route ci-dessous).

## Structure du projet

```text
openpencil/
├── crates/                   Workspace Rust — le produit
│   ├── op-editor-core/       État canonique de l'éditeur `.op` (PenDocument) + EditorCommand + variables de design
│   ├── op-editor-ui/         Widgets indépendants de la plateforme + façade RenderBackend (wasm32-clean)
│   ├── op-editor-host-core/  Machines à états d'hôte sans transport, partagées par tous les hôtes
│   ├── op-host-native/       Bibliothèque hôte native — winit + skia-safe GL (bureau + mobile)
│   ├── op-host-web/          Bundle navigateur — cdylib wasm32, rendu CanvasKit
│   ├── op-host-desktop/      Binaire de bureau `openpencil-desktop` ; aussi le daemon `--serve-web`
│   ├── op-host-services/     Bibliothèque daemon serve-web / MCP headless
│   ├── op-host-web-server/   Binaire web-server léger, sans GL
│   ├── op-cli/               Outil CLI — commande `op`
│   ├── op-mcp/               Serveur MCP — outils, design par lots, workflow en couches
│   ├── op-ai/                Fournisseurs IA, runtime de chat, streaming
│   ├── op-ai-skills/         Moteur de compétences de prompts IA (chargement piloté par phases)
│   ├── op-orchestrator/      Orchestration d'équipes d'agents concurrentes
│   ├── op-codegen/           Générateurs de code (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Parseur et convertisseur de fichiers Figma .fig
│   ├── op-git/               Intégration Git — clone, branche, push/pull, fusion
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Workspace SDK web (Bun)
│   ├── op-web-sdk/           SDK visualiseur web `.op` en lecture seule (enveloppe le bundle wasm)
│   ├── op-web-sdk-react/     Adaptateur React 19
│   └── op-web-sdk-vue/       Adaptateur Vue 3
├── vendor/                   Sous-systèmes vendored (sous-modules git)
│   ├── jian/                 Framework UI GPU-Skia — widgets/rendu/événements
│   ├── casement/             Fork de winit
│   └── agent/                Runtime d'agent Rust transversal (agent-rs)
└── .githooks/                Synchronisation de version pre-commit depuis le nom de branche
```

## Raccourcis clavier

| Touche      | Action                 |     | Touche        | Action                         |
| ----------- | ---------------------- | --- | ------------- | ------------------------------ |
| `V`         | Sélectionner           |     | `Cmd+S`       | Enregistrer                    |
| `R`         | Rectangle              |     | `Cmd+Z`       | Annuler                        |
| `O`         | Ellipse                |     | `Cmd+Shift+Z` | Rétablir                       |
| `L`         | Ligne                  |     | `Cmd+C/X/V/D` | Copier/Couper/Coller/Dupliquer |
| `T`         | Texte                  |     | `Cmd+G`       | Grouper                        |
| `F`         | Frame                  |     | `Cmd+Shift+G` | Dégrouper                      |
| `P`         | Outil plume            |     | `Cmd+Shift+P` | Exporter (PNG/JPG/WEBP/PDF)    |
| `H`         | Main (panoramique)     |     | `Cmd+Shift+C` | Panneau de code                |
| `Del`       | Supprimer              |     | `Cmd+Shift+V` | Panneau des variables          |
| `[ / ]`     | Réordonner             |     | `Cmd+J`       | Chat IA                        |
| Flèches     | Déplacer de 1px        |     | `Cmd+,`       | Paramètres de l'agent          |
| `Cmd+Alt+U` | Union booléenne        |     | `Cmd+Alt+S`   | Soustraction booléenne         |
| `Cmd+Alt+I` | Intersection booléenne |     | `Cmd+Shift+S` | Enregistrer sous               |

## Scripts

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
cd packages && bun run bump <version>             # Sync SDK package.json versions
```

## Contribuer

Les contributions sont les bienvenues ! Consultez [CLAUDE.md](./CLAUDE.md) pour les détails d'architecture et le style de code.

1. Forker et cloner
2. Configurer la synchronisation de version : `git config core.hooksPath .githooks`
3. Créer une branche : `git checkout -b feat/my-feature`
4. Exécuter les vérifications : `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] SDK agent AI intégré avec support multi-fournisseurs
- [x] i18n — 15 langues
- [x] Intégration Git (clone, branche, push/pull, fusion à trois voies en mode dossier)
- [x] Export raster du canevas (PNG / JPEG / WEBP / PDF)
- [ ] Édition collaborative
- [ ] Système de plugins

## Contributeurs

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Sponsors

OpenPencil est gratuit et open source. Le développement est financé par celles et ceux qui le trouvent utile — merci de garder le canevas ouvert.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Merci à **[MrQyun](https://github.com/mrqyun)** — vous voulez voir votre nom ici ? **[Devenir sponsor →](https://github.com/sponsors/ZSeven-W)**

## Communauté

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
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

## Évaluations

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Licence

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
