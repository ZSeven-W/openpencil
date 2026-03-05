<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Ferramenta de design open-source nativa com IA. Design-as-Code.</strong><br />
  Do prompt à UI no canvas. Orquestração multi-agente. Servidor MCP integrado. Geração de código.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md"><b>Português</b></a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#início-rápido">Início Rápido</a> ·
  <a href="#design-nativo-com-ia">IA</a> ·
  <a href="#funcionalidades">Funcionalidades</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contribuindo">Contribuindo</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Clique na imagem para assistir ao vídeo de demonstração</sub></p>

<br />

## Início Rápido

```bash
# Instalar dependências
bun install

# Iniciar servidor de desenvolvimento em http://localhost:3000
bun --bun run dev
```

Ou executar como aplicativo desktop:

```bash
bun run electron:dev
```

> **Pré-requisitos:** [Bun](https://bun.sh/) >= 1.0 e [Node.js](https://nodejs.org/) >= 18

## Design Nativo com IA

O OpenPencil é construído com IA desde o início — não como um plugin, mas como um fluxo de trabalho central.

**Do Prompt à UI**
- **Texto para design** — descreva uma página e ela será gerada no canvas em tempo real com animação de streaming
- **Orquestrador** — decompõe páginas complexas em sub-tarefas espaciais para geração paralela
- **Modificação de design** — selecione elementos e descreva as alterações em linguagem natural
- **Entrada de visão** — anexe capturas de tela ou mockups para design baseado em referência

**Suporte Multi-Agente**

| Agente | Configuração |
| --- | --- |
| **Claude Code** | Sem configuração — usa o Claude Agent SDK com OAuth local |
| **Codex CLI** | Conectar nas Configurações do Agente (`Cmd+,`) |
| **OpenCode** | Conectar nas Configurações do Agente (`Cmd+,`) |

**Servidor MCP**
- Servidor MCP integrado — instalação com um clique no Claude Code / Codex / Gemini / OpenCode / Kiro CLIs
- Automação de design pelo terminal: leia, crie e modifique arquivos `.op` via qualquer agente compatível com MCP

**Geração de Código**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables a partir de tokens de design

## Funcionalidades

**Canvas e Desenho**
- Canvas infinito com pan, zoom, guias de alinhamento inteligentes e snapping
- Retângulo, Elipse, Linha, Polígono, Caneta (Bezier), Frame, Texto
- Seletor de ícones (Iconify) e importação de imagens (PNG/JPEG/SVG/WebP/GIF)
- Auto-layout — vertical/horizontal com gap, padding, justify, align
- Documentos com múltiplas páginas e navegação por abas

**Sistema de Design**
- Variáveis de design — tokens de cor, número e string com referências `$variable`
- Suporte a múltiplos temas — vários eixos, cada um com variantes (Claro/Escuro, Compacto/Confortável)
- Sistema de componentes — componentes reutilizáveis com instâncias e substituições
- Sincronização CSS — propriedades personalizadas geradas automaticamente, `var(--name)` na saída de código

**Importação do Figma**
- Importe arquivos `.fig` preservando layout, preenchimentos, traços, efeitos, texto, imagens e vetores

**Aplicativo Desktop**
- macOS, Windows e Linux nativos via Electron
- Atualização automática a partir do GitHub Releases
- Menu de aplicativo nativo e diálogos de arquivo

## Stack Tecnológica

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canvas** | Fabric.js v7 |
| **Estado** | Zustand v5 |
| **Servidor** | Nitro |
| **Desktop** | Electron 35 |
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Runtime** | Bun · Vite 7 |
| **Formato de arquivo** | `.op` — baseado em JSON, legível por humanos, compatível com Git |

## Estrutura do Projeto

```text
src/
  canvas/          Motor Fabric.js — desenho, sincronização, layout, guias, ferramenta caneta
  components/      UI React — editor, painéis, diálogos compartilhados, ícones
  services/ai/     Chat IA, orquestrador, geração de design, streaming
  services/figma/  Pipeline de importação binária do Figma .fig
  services/codegen Geradores de código React+Tailwind e HTML+CSS
  stores/          Zustand — canvas, documento, páginas, histórico, IA, configurações
  variables/       Resolução de tokens de design e gerenciamento de referências
  mcp/             Ferramentas do servidor MCP para integração com CLI externo
  uikit/           Sistema de kit de componentes reutilizáveis
server/
  api/ai/          API Nitro — chat em streaming, geração, validação
  utils/           Wrappers de cliente Claude CLI, OpenCode, Codex
electron/
  main.ts          Janela, fork do Nitro, menu nativo, atualizador automático
  preload.ts       Ponte IPC
```

## Atalhos de Teclado

| Tecla | Ação | | Tecla | Ação |
| --- | --- | --- | --- | --- |
| `V` | Selecionar | | `Cmd+S` | Salvar |
| `R` | Retângulo | | `Cmd+Z` | Desfazer |
| `O` | Elipse | | `Cmd+Shift+Z` | Refazer |
| `L` | Linha | | `Cmd+C/X/V/D` | Copiar/Recortar/Colar/Duplicar |
| `T` | Texto | | `Cmd+G` | Agrupar |
| `F` | Frame | | `Cmd+Shift+G` | Desagrupar |
| `P` | Ferramenta caneta | | `Cmd+Shift+E` | Exportar |
| `H` | Mão (pan) | | `Cmd+Shift+C` | Painel de código |
| `Del` | Excluir | | `Cmd+Shift+V` | Painel de variáveis |
| `[ / ]` | Reordenar | | `Cmd+J` | Chat IA |
| Setas | Mover 1px | | `Cmd+,` | Configurações do agente |

## Scripts

```bash
bun --bun run dev          # Servidor de desenvolvimento (porta 3000)
bun --bun run build        # Build de produção
bun --bun run test         # Executar testes (Vitest)
npx tsc --noEmit           # Verificação de tipos
bun run electron:dev       # Desenvolvimento com Electron
bun run electron:build     # Empacotamento do Electron
```

## Contribuindo

Contribuições são bem-vindas! Consulte o [CLAUDE.md](./CLAUDE.md) para detalhes de arquitetura e estilo de código.

1. Faça fork e clone
2. Crie uma branch: `git checkout -b feat/my-feature`
3. Execute as verificações: `npx tsc --noEmit && bun --bun run test`
4. Faça commit com [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Abra um PR contra `main`

## Roadmap

- [x] Variáveis de design e tokens com sincronização CSS
- [x] Sistema de componentes (instâncias e substituições)
- [x] Geração de design com IA e orquestrador
- [x] Integração com servidor MCP
- [x] Suporte a múltiplas páginas
- [x] Importação do Figma `.fig`
- [ ] Operações booleanas (união, subtração, interseção)
- [ ] Edição colaborativa
- [ ] Sistema de plugins

## Contribuidores

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Comunidade

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Entre no nosso Discord</strong>
</a>
— Faça perguntas, compartilhe designs, sugira funcionalidades.

## Licença

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
