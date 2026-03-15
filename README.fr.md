<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Le premier outil de design vectoriel open-source natif IA au monde.</strong><br />
  <sub>Equipes d'agents concurrents &bull; Design-as-Code &bull; Serveur MCP intégré &bull; Intelligence multi-modèles</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md"><b>Français</b></a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat&color=cfb537" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil?color=64748b" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/KwXp6BJD"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white&color=5865F2" alt="Discord" /></a>
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

**Profils de capacités des modèles** — adapte automatiquement les prompts, le mode de réflexion et les délais d'attente par niveau de modèle. Les modèles de niveau complet (Claude) reçoivent des prompts complets ; le niveau standard (GPT-4o, Gemini, DeepSeek) désactive la réflexion ; le niveau basique (MiniMax, Qwen, Llama, Mistral) reçoit des prompts JSON imbriqués simplifiés pour une fiabilité maximale.

**Serveur MCP**
- Serveur MCP intégré — installation en un clic dans les CLI Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot
- Détection automatique de Node.js — si non installé, bascule automatiquement vers le transport HTTP et démarre le serveur MCP HTTP
- Automatisation du design depuis le terminal : lire, créer et modifier des fichiers `.op` via tout agent compatible MCP
- **Workflow de design en couches** — `design_skeleton` → `design_content` → `design_refine` pour des designs multi-sections de plus haute fidélité
- **Récupération segmentée des prompts** — chargez uniquement les connaissances de design nécessaires (schéma, layout, rôles, icônes, planification, etc.)
- Support multi-pages — créer, renommer, réordonner et dupliquer des pages via les outils MCP

**Génération de code**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

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
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Format de fichier** | `.op` — basé sur JSON, lisible par l'humain, compatible Git |

## Structure du projet

```text
src/
  canvas/          Moteur CanvasKit/Skia — dessin, sync, mise en page, guides, outil plume
  components/      Interface React — éditeur, panneaux, boîtes de dialogue partagées, icônes
  services/ai/     Chat IA, orchestrateur, génération de design, streaming
  services/figma/  Pipeline d'import binaire Figma .fig
  services/codegen Générateurs de code React+Tailwind et HTML+CSS
  stores/          Zustand — canevas, document, pages, historique, IA, paramètres
  variables/       Résolution des tokens de design et gestion des références
  mcp/             Outils serveur MCP pour l'intégration CLI externe
  uikit/           Système de kits de composants réutilisables
server/
  api/ai/          API Nitro — chat en streaming, génération, validation
  utils/           Enveloppes client Claude CLI, OpenCode, Codex, Copilot
electron/
  main.ts          Fenêtre, fork Nitro, menu natif, mise à jour automatique
  preload.ts       Pont IPC
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
bun run electron:dev       # Développement Electron
bun run electron:build     # Packaging Electron
```

## Contribuer

Les contributions sont les bienvenues ! Consultez [CLAUDE.md](./CLAUDE.md) pour les détails d'architecture et le style de code.

1. Forker et cloner
2. Créer une branche : `git checkout -b feat/my-feature`
3. Exécuter les vérifications : `npx tsc --noEmit && bun --bun run test`
4. Commiter avec [Conventional Commits](https://www.conventionalcommits.org/) : `feat(canvas): add rotation snapping`
5. Ouvrir une PR contre `main`

## Feuille de route

- [x] Variables de design & tokens avec synchronisation CSS
- [x] Système de composants (instances & substitutions)
- [x] Génération de design IA avec orchestrateur
- [x] Intégration du serveur MCP avec workflow de design en couches
- [x] Support multi-pages
- [x] Import Figma `.fig`
- [x] Opérations booléennes (union, soustraction, intersection)
- [x] Profils de capacités multi-modèles
- [ ] Édition collaborative
- [ ] Système de plugins

## Contributeurs

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>


## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Communauté

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Rejoindre notre Discord</strong>
</a>
— Posez des questions, partagez vos designs, suggérez des fonctionnalités.


