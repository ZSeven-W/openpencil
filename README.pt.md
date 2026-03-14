<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>A primeira ferramenta de design vetorial open-source nativa com IA do mundo.</strong><br />
  <sub>Equipes de Agentes Concorrentes &bull; Design-as-Code &bull; Servidor MCP Integrado &bull; Inteligência Multi-modelo</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md">Español</a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md"><b>Português</b></a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Formato de arquivo** | `.op` — baseado em JSON, legível por humanos, compatível com Git |

## Estrutura do Projeto

```text
src/
  canvas/          Motor CanvasKit/Skia — desenho, sincronização, layout, guias, ferramenta caneta
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
  utils/           Wrappers de cliente Claude CLI, OpenCode, Codex, Copilot
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
| `Cmd+Alt+U` | União booleana | | `Cmd+Alt+S` | Subtração booleana |
| `Cmd+Alt+I` | Interseção booleana | | | |

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
- [x] Integração com servidor MCP e fluxo de design em camadas
- [x] Suporte a múltiplas páginas
- [x] Importação do Figma `.fig`
- [x] Operações booleanas (união, subtração, interseção)
- [x] Perfis de capacidade multi-modelo
- [ ] Edição colaborativa
- [ ] Sistema de plugins

## Contribuidores

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Comunidade

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
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
