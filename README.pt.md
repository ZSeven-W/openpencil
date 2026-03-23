<p align="center">
  <img src="./apps/desktop/build/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>A primeira ferramenta de design vetorial open-source nativa com IA do mundo.</strong><br />
  <sub>Equipes de Agentes Concorrentes &bull; Design-as-Code &bull; Servidor MCP Integrado &bull; Inteligência Multi-modelo</sub>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — clique para assistir ao demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Clique na imagem para assistir ao vídeo de demonstração</sub></p>

<br />

## Por que OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Canvas

Descreva qualquer UI em linguagem natural. Veja-a aparecer no canvas infinito em tempo real com animação de streaming. Modifique designs existentes selecionando elementos e conversando.

</td>
<td width="50%">

### 🤖 Equipes de Agentes Concorrentes

O orquestrador decompõe páginas complexas em sub-tarefas espaciais. Vários agentes de IA trabalham em diferentes seções simultaneamente — hero, features, footer — tudo em streaming paralelo.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Inteligência Multi-Modelo

Adapta-se automaticamente às capacidades de cada modelo. Claude recebe prompts completos com thinking; GPT-4o/Gemini desativam thinking; modelos menores (MiniMax, Qwen, Llama) recebem prompts simplificados para saída confiável.

</td>
<td width="50%">

### 🔌 Servidor MCP

Instalação com um clique no Claude Code, Codex, Gemini, OpenCode, Kiro ou Copilot CLIs. Faça design pelo seu terminal — leia, crie e modifique arquivos `.op` através de qualquer agente compatível com MCP.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Design-as-Code

Arquivos `.op` são JSON — legíveis por humanos, compatíveis com Git, com diff. Variáveis de design geram propriedades CSS personalizadas. Exportação de código para React + Tailwind ou HTML + CSS.

</td>
<td width="50%">

### 🖥️ Roda em Qualquer Lugar

App web + desktop nativo no macOS, Windows e Linux via Electron. Atualização automática a partir do GitHub Releases. Associação de arquivos `.op` — clique duplo para abrir.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Controle a ferramenta de design pelo terminal. `op design`, `op insert`, `op export` — DSL de design em lote, manipulação de nós, exportação de código. Entrada por pipe de arquivos ou stdin. Funciona com o app desktop ou servidor web.

</td>
<td width="50%">

### 🎯 Exportação de Código Multiplataforma

Exporte de um único arquivo `.op` para React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Variáveis de design se tornam propriedades CSS customizadas.

</td>
</tr>
</table>

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

### Docker

Várias variantes de imagem estão disponíveis — escolha a que se adequa às suas necessidades:

| Imagem | Tamanho | Inclui |
| --- | --- | --- |
| `openpencil:latest` | ~226 MB | Apenas aplicação web |
| `openpencil-claude:latest` | — | + Claude Code CLI |
| `openpencil-codex:latest` | — | + Codex CLI |
| `openpencil-opencode:latest` | — | + OpenCode CLI |
| `openpencil-copilot:latest` | — | + GitHub Copilot CLI |
| `openpencil-gemini:latest` | — | + Gemini CLI |
| `openpencil-full:latest` | ~1 GB | Todas as ferramentas CLI |

**Executar (apenas web):**

```bash
docker run -d -p 3000:3000 ghcr.io/zseven-w/openpencil:latest
```

**Executar com AI CLI (ex. Claude Code):**

O chat de IA depende do login OAuth do Claude CLI. Use um volume Docker para persistir a sessão de login:

```bash
# Passo 1 — Login (apenas uma vez)
docker volume create openpencil-claude-auth
docker run -it --rm \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest claude login

# Passo 2 — Iniciar
docker run -d -p 3000:3000 \
  -v openpencil-claude-auth:/root/.claude \
  ghcr.io/zseven-w/openpencil-claude:latest
```

**Compilar localmente:**

```bash
# Base (apenas web)
docker build --target base -t openpencil .

# Com um CLI específico
docker build --target with-claude -t openpencil-claude .

# Completo (todos os CLIs)
docker build --target full -t openpencil-full .
```

## Design Nativo com IA

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
| **GitHub Copilot** | `copilot login` e depois conectar nas Configurações do Agente (`Cmd+,`) |
| **Gemini CLI** | Conectar nas Configurações do Agente (`Cmd+,`) |

**Perfis de Capacidade de Modelo** — adapta automaticamente prompts, modo de thinking e timeouts por nível de modelo. Modelos de nível completo (Claude) recebem prompts completos; nível padrão (GPT-4o, Gemini, DeepSeek) desativam thinking; nível básico (MiniMax, Qwen, Llama, Mistral) recebem prompts simplificados de JSON aninhado para máxima confiabilidade.

**Servidor MCP**
- Servidor MCP integrado — instalação com um clique no Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Detecção automática de Node.js — se não instalado, recurso automático para transporte HTTP e início automático do servidor MCP HTTP
- Automação de design pelo terminal: leia, crie e modifique arquivos `.op` via qualquer agente compatível com MCP
- **Fluxo de design em camadas** — `design_skeleton` → `design_content` → `design_refine` para designs multi-seção de maior fidelidade
- **Recuperação segmentada de prompts** — carregue apenas o conhecimento de design necessário (schema, layout, roles, icons, planning, etc.)
- Suporte a múltiplas páginas — crie, renomeie, reordene e duplique páginas via ferramentas MCP

**Geração de Código**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## CLI — `op`

Instale globalmente e controle a ferramenta de design pelo terminal:

```bash
npm install -g @zseven-w/openpencil
```

```bash
op start                     # Iniciar app desktop
op design @landing.txt       # Design em lote a partir de arquivo
op insert '{"type":"RECT"}'  # Inserir um nó
op export react --out .      # Exportar para React + Tailwind
op import:figma design.fig   # Importar arquivo Figma
cat design.dsl | op design - # Entrada por pipe via stdin
```

Suporta três métodos de entrada: string inline, `@filepath` (ler de arquivo) ou `-` (ler de stdin). Funciona com o app desktop ou servidor web de desenvolvimento. Veja o [CLI README](./apps/cli/README.md) para referência completa de comandos.

## Funcionalidades

**Canvas e Desenho**
- Canvas infinito com pan, zoom, guias de alinhamento inteligentes e snapping
- Retângulo, Elipse, Linha, Polígono, Caneta (Bezier), Frame, Texto
- Operações booleanas — união, subtração, interseção com barra de ferramentas contextual
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
- Associação de arquivos `.op` — clique duplo para abrir, bloqueio de instância única
- Atualização automática a partir do GitHub Releases
- Menu de aplicativo nativo e diálogos de arquivo

## Stack Tecnológica

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Canvas** | CanvasKit/Skia (WASM, acelerado por GPU) |
| **Estado** | Zustand v5 |
| **Servidor** | Nitro |
| **Desktop** | Electron 35 |
| **CLI** | `op` — controle pelo terminal, DSL de design em lote, exportação de código |
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Formato de arquivo** | `.op` — baseado em JSON, legível por humanos, compatível com Git |

## Estrutura do Projeto

```text
openpencil/
├── apps/
│   ├── web/                 Aplicação web TanStack Start
│   │   ├── src/
│   │   │   ├── canvas/      Motor CanvasKit/Skia — desenho, sincronização, layout
│   │   │   ├── components/  UI React — editor, painéis, diálogos compartilhados, ícones
│   │   │   ├── services/ai/ Chat IA, orquestrador, geração de design, streaming
│   │   │   ├── stores/      Zustand — canvas, documento, páginas, histórico, IA
│   │   │   ├── mcp/         Ferramentas do servidor MCP para integração com CLI externo
│   │   │   ├── hooks/       Atalhos de teclado, soltar arquivos, colar do Figma
│   │   │   └── uikit/       Sistema de kit de componentes reutilizáveis
│   │   └── server/
│   │       ├── api/ai/      API Nitro — chat em streaming, geração, validação
│   │       └── utils/       Wrappers de cliente Claude CLI, OpenCode, Codex, Copilot
│   ├── desktop/             Aplicativo desktop Electron
│   │   ├── main.ts          Janela, fork do Nitro, menu nativo, atualizador automático
│   │   ├── ipc-handlers.ts  Diálogos de arquivo nativos, sincronização de tema, preferências IPC
│   │   └── preload.ts       Ponte IPC
│   └── cli/                 Ferramenta CLI — comando `op`
│       ├── src/commands/    Comandos de design, documento, exportação, importação, nó, página, variável
│       ├── connection.ts    Conexão WebSocket com o app em execução
│       └── launcher.ts      Detecção automática e inicialização do app desktop ou servidor web
├── packages/
│   ├── pen-types/           Definições de tipos para o modelo PenDocument
│   ├── pen-core/            Operações de árvore de documento, motor de layout, variáveis
│   ├── pen-codegen/         Geradores de código (React, HTML, Vue, Flutter, ...)
│   ├── pen-figma/           Parser e conversor de arquivos .fig do Figma
│   ├── pen-renderer/        Renderizador CanvasKit/Skia independente
│   └── pen-sdk/             SDK guarda-chuva (re-exporta todos os pacotes)
└── .githooks/               Sincronização de versão no pre-commit a partir do nome da branch
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
| `Cmd+Alt+U` | União booleana | | `Cmd+Alt+S` | Subtração booleana |
| `Cmd+Alt+I` | Interseção booleana | | | |

## Scripts

```bash
bun --bun run dev          # Servidor de desenvolvimento (porta 3000)
bun --bun run build        # Build de produção
bun --bun run test         # Executar testes (Vitest)
npx tsc --noEmit           # Verificação de tipos
bun run bump <version>     # Sincronizar versão em todos os package.json
bun run electron:dev       # Desenvolvimento com Electron
bun run electron:build     # Empacotamento do Electron
bun run cli:dev            # Executar CLI a partir do código-fonte
bun run cli:compile        # Compilar CLI para dist
```

## Contribuindo

Contribuições são bem-vindas! Consulte o [CLAUDE.md](./CLAUDE.md) para detalhes de arquitetura e estilo de código.

1. Faça fork e clone
2. Configure a sincronização de versão: `git config core.hooksPath .githooks`
3. Crie uma branch: `git checkout -b feat/my-feature`
4. Execute as verificações: `npx tsc --noEmit && bun --bun run test`
5. Faça commit com [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
6. Abra um PR contra `main`

## Roadmap

- [x] Variáveis de design e tokens com sincronização CSS
- [x] Sistema de componentes (instâncias e substituições)
- [x] Geração de design com IA e orquestrador
- [x] Integração com servidor MCP e fluxo de design em camadas
- [x] Suporte a múltiplas páginas
- [x] Importação do Figma `.fig`
- [x] Operações booleanas (união, subtração, interseção)
- [x] Perfis de capacidade multi-modelo
- [x] Reestruturação em monorepo com pacotes reutilizáveis
- [x] Ferramenta CLI (`op`) para controle pelo terminal
- [ ] Edição colaborativa
- [ ] Sistema de plugins

## Contribuidores

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Comunidade

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./apps/web/public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Entre no nosso Discord</strong>
</a>
— Faça perguntas, compartilhe designs, sugira funcionalidades.

## Star History

<a href="https://star-history.com/#ZSeven-W/openpencil&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=ZSeven-W/openpencil&type=Date" width="100%" />
 </picture>
</a>

## Licença

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
