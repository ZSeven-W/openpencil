<p align="center">
  <img src="./crates/op-host-desktop/assets/icon.png" alt="OpenPencil" width="120" />
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
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/rust-check.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/rust-check.yml?label=CI" alt="CI" /></a>
  <a href="https://discord.gg/h9Fmyy6pVh"><img src="https://img.shields.io/badge/Discord-Join%20chat-5865F2?logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — clique para assistir ao demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Clique na imagem para assistir ao vídeo de demonstração</sub></p>

<br />

> **Nota:** Existe outro projeto de código aberto com o mesmo nome — [OpenPencil](https://github.com/open-pencil/open-pencil), focado em design visual compatível com Figma com colaboração em tempo real. Este projeto foca em fluxos de trabalho AI-nativos de design para código.

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

App web + desktop nativo no macOS, Windows e Linux — um único núcleo Rust, um binário único autocontido, sem motor de navegador. Associação de arquivos `.op` — clique duplo para abrir.

</td>
</tr>
<tr>
<td width="50%">

### ⌨️ CLI — `op`

Controle a ferramenta de design pelo terminal. `op design`, `op insert` — DSL de design em lote, manipulação de nós. Entrada por pipe de arquivos ou stdin. Funciona com o app desktop ou servidor web.

</td>
<td width="50%">

### 🎯 Exportação de Código Multiplataforma

Exporte de um único arquivo `.op` para React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native. Variáveis de design se tornam propriedades CSS customizadas.

</td>
</tr>
</table>

## Início Rápido

```bash
# Web dev server (builds the CanvasKit wasm bundle, then runs the headless web host)
bash scripts/start-web-rust.sh
```

Ou executar como aplicativo desktop:

```bash
cargo run -p op-host-desktop
```

> **Pré-requisitos:** [Rust](https://www.rust-lang.org/) (stable) para compilar o produto. [Bun](https://bun.sh/) >= 1.0 e [Node.js](https://nodejs.org/) >= 18 são necessários apenas para o SDK web em `packages/`.

### Docker

Releases Rust com tag publicam uma única imagem de web host. As imagens TypeScript aposentadas com CLIs de IA incluídos não são mais publicadas.

| Imagem | Inclui |
| --- | --- |
| `ghcr.io/zseven-w/openpencil-web:v0.8.1` | Rust web host, wasm bundle e assets do CanvasKit |

A UI web expõe apenas perfis de agente integrados; as ferramentas Claude/Codex/OpenCode/Copilot/Gemini CLI não são incluídas nas imagens Docker.

**Executar:**

```bash
docker run -d -p 3100:3100 ghcr.io/zseven-w/openpencil-web:v0.8.1
```

Depois abra `http://localhost:3100/`.

**Compilar localmente:**

```bash
docker build -f Dockerfile.web-rust -t openpencil-web-rust .
docker run -p 3100:3100 openpencil-web-rust
```

## Design Nativo com IA

**Do Prompt à UI**

- **Texto para design** — descreva uma página e ela será gerada no canvas em tempo real com animação de streaming
- **Orquestrador** — decompõe páginas complexas em sub-tarefas espaciais para geração paralela
- **Modificação de design** — selecione elementos e descreva as alterações em linguagem natural
- **Entrada de visão** — anexe capturas de tela ou mockups para design baseado em referência

**Suporte Multi-Agente**

| Agente                        | Configuração                                                                                             |
| ----------------------------- | -------------------------------------------------------------------------------------------------------- |
| **Integrado (9+ provedores)** | Selecione entre presets de provedores com seletor de região — Anthropic, OpenAI, Google, DeepSeek e mais |
| **Claude Code**               | Sem configuração — usa o Claude Agent SDK com OAuth local                                                |
| **Codex CLI**                 | Conectar nas Configurações do Agente (`Cmd+,`)                                                           |
| **OpenCode**                  | Conectar nas Configurações do Agente (`Cmd+,`)                                                           |
| **GitHub Copilot**            | `copilot login` e depois conectar nas Configurações do Agente (`Cmd+,`)                                  |
| **Gemini CLI**                | Conectar nas Configurações do Agente (`Cmd+,`)                                                           |

**Perfis de Capacidade de Modelo** — adapta automaticamente prompts, modo de thinking e timeouts por nível de modelo. Modelos de nível completo (Claude) recebem prompts completos; nível padrão (GPT-4o, Gemini, DeepSeek) desativam thinking; nível básico (MiniMax, Qwen, Llama, Mistral) recebem prompts simplificados de JSON aninhado para máxima confiabilidade.

**i18n** — Localização completa da interface em 15 idiomas: English, 简体中文, 繁體中文, 日本語, 한국어, Français, Español, Deutsch, Português, Русский, हिन्दी, Türkçe, ไทย, Tiếng Việt, Bahasa Indonesia.

**Servidor MCP**

- Servidor MCP integrado (crate `op-mcp`) — instalação com um clique no Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Não requer Node.js — transporte stdio via o binário desktop (`--mcp <path>`), além de um endpoint HTTP ativo (`127.0.0.1:<port>/mcp`) a partir do app em execução
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
brew install zseven-w/openpencil/op
```

```bash
op start                     # Iniciar app desktop
op start --headless --file design.op # Iniciar servidor headless
op design @landing.txt       # Design em lote a partir de arquivo
op design @ui.js             # JavaScript isolado com loops
op insert '{"type":"rectangle"}' # Inserir um nó
op import:figma design.fig   # Importar arquivo Figma
cat design.dsl | op design - # Entrada por pipe via stdin
```

Suporta strings inline, `@filepath` e stdin (`-`). Funciona com o app desktop, servidor web ou servidor headless baseado em arquivo. Consulte a [referência de comandos CLI](./crates/op-cli/src/usage.txt).

**Habilidade LLM** — instale o plugin [OpenPencil Skill](https://github.com/ZSeven-W/openpencil-skill) para ensinar agentes IA a projetar com `op`. Use `op install` para agentes detectados ou `op install --target codex` para um destino específico.

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
- UIKits reutilizáveis — importe/exporte kits de componentes a partir de arquivos `.pen`

**IA e Agentes**

- Prompt-para-canvas com geração em streaming e decomposição espacial orientada pelo orquestrador
- Equipes de agentes concorrentes — vários designers trabalham em seções diferentes em paralelo, com indicadores no canvas por membro
- Fluxo em camadas — `design_skeleton` → `design_content` → `design_refine` com prompts focados por fase
- Guias de estilo — mais de 50 estilos integrados (glassmorphism, brutalist, retro, etc.) com correspondência fuzzy baseada em tags, integrados ao planejamento e geração
- Perfis de capacidade multi-modelo — adapta automaticamente o modo de pensamento, o esforço e a forma do prompt conforme o nível do modelo
- Runtime de agente integrado (Rust) + provedores Anthropic, Claude Agent SDK, OpenCode, Codex, Copilot, Gemini
- Passthrough no formato Anthropic para provedores de LLMs chineses — Kimi, Zhipu, GLM, DouBao, Ark, Bailian/DashScope, ModelScope, Coding Plans

**Integração com Git**

- Assistente de clone com autenticação SSH / HTTPS e gestão de chaves SSH
- Seletor de branches — criar, trocar, excluir, mesclar, tudo a partir do painel Git
- Cascatas de pull / push com retry de autenticação e tratamento non-fast-forward
- Merge a três vias em modo pasta com rastreamento do estado `MERGE_HEAD` em disco
- Painel de conflitos com cards de três vias por nó / campo, editor JSON inline, ações em lote e bloco de diff inline
- UI de configurações remotas e chaves SSH; i18n em 15 idiomas em toda a superfície Git

**Exportação**

- Exportação do canvas — PNG, JPEG, WEBP, PDF (`Cmd+Shift+P`)
- Exportação de código — React + Tailwind, HTML + CSS, Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native
- Pipeline incremental de codegen MCP — `codegen_plan`, `codegen_submit_chunk`, `codegen_assemble`, `codegen_clean`

**Importação do Figma**

- Importe arquivos `.fig` preservando layout, preenchimentos, traços, efeitos, texto, imagens e vetores

**Aplicativo Desktop**

- macOS, Windows e Linux nativos — um único binário autocontido (winit + GPU Skia, sem Electron)
- Associação de arquivos `.op` — clique duplo para abrir, bloqueio de instância única
- Verificação de atualização em segundo plano a partir do GitHub Releases
- Menu de aplicativo nativo com Salvar como, Abrir recentes e um diálogo de alterações não salvas ao fechar
- Persistência de arquivos recentes

## Stack Tecnológica

|                        |                                                                                       |
| ---------------------- | ------------------------------------------------------------------------------------- |
| **Núcleo**             | Workspace Rust (`crates/`) — estado do editor, widgets, hosts, MCP, IA, codegen       |
| **Renderização**       | GPU Skia em todo lugar — `skia-safe` (GL) no nativo, CanvasKit (WASM/WebGL2) no navegador |
| **Framework de UI**    | jian — framework de UI GPU-Skia puro-Rust vendorizado: widgets, layout, eventos, hot reload (`vendor/jian`) |
| **Janelamento**        | winit (fork `casement` vendorizado)                                                   |
| **Desktop**            | Binário nativo `openpencil-desktop` — sem motor de navegador                          |
| **SDK Web**            | `op-web-sdk` + adaptadores React 19 / Vue 3 — visualizador `.op` somente leitura (TypeScript) |
| **CLI**                | `op` — controle pelo terminal, DSL de design em lote                                  |
| **IA**                 | Runtime de agente Rust integrado · Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Lint**               | clippy · rustfmt (Rust) · oxlint · oxfmt (SDK web)                                     |
| **Formato de arquivo** | `.op` — baseado em JSON, legível por humanos, compatível com Git                      |

## Ecossistema

O OpenPencil faz parte de uma família de ferramentas puro-Rust, nativas com IA da **[ZSeven-W](https://github.com/ZSeven-W)**. Elas se compõem: `jian` renderiza o OpenPencil, `agent-rs` executa seus agentes, `noema` memoriza, e `zode` projeta a partir do terminal.

| Projeto | O que é |
| ------- | ------- |
| **[Zode](https://github.com/ZSeven-W/zode)** | Assistente de código open-source e nativo com IA para o seu terminal — uma TUI Rust rápida (`ratatui`) que lê seu código, executa comandos, busca arquivos e gerencia o git. Controla o OpenPencil via MCP. |
| **[agent-rs](https://github.com/ZSeven-W/agent-rs)** | Um runtime assíncrono puro-Rust para entregar agentes LLM — multi-provedor, com suporte a ferramentas de ponta a ponta, permissões estruturadas, MCP real, zero `unsafe`. Alimenta o runtime de agente integrado do OpenPencil (`vendor/agent`) e o Zode. |
| **[jian](https://github.com/ZSeven-W/jian)** | Framework de UI puro-Rust, GPU-Skia — widgets, layout, eventos e hot reload em uma única stack. Transforma um documento `.op` declarativo em um app nativo e controlável por IA, sem runtime JS, sem DOM, sem Electron. O framework de UI do OpenPencil (`vendor/jian`). |
| **[noema](https://github.com/ZSeven-W/noema)** | Sistema de memória local-first e não vetorial para agentes de código. Memória durável como arquivos inspecionáveis, uma fila de revisão para novas entradas e recuperação léxica (sem embeddings) — funciona no Zode, Codex, Claude Code e runtimes MCP. |

## Por que Rust

O OpenPencil foi reescrito do zero em **Rust** ([#129](https://github.com/ZSeven-W/openpencil/issues/129)). A reescrita está completa — o editor TypeScript + Electron foi descontinuado na `v0.7.5`, e o workspace Rust deste repositório é o produto: um núcleo nativo drasticamente menor e mais rápido, que roda em mais plataformas a partir de uma única base de código.

|                          | TypeScript + Electron (descontinuado, `v0.7.5`)   | Rust (hoje)                                                              |
| ------------------------ | ------------------------------------------------- | ------------------------------------------------------------------------ |
| **Runtime desktop**      | Electron — empacota Chromium + Node.js            | Janela nativa (`winit` + GPU Skia), sem motor de navegador               |
| **Pegada desktop**       | Runtime completo do Chromium por instalação       | Binário único autocontido — **55.5 MB**                                  |
| **Payload web**          | Bundle JS + WASM                                  | **8.2 MB** wasm / **2.18 MB** gzip transferido pela rede                 |
| **Renderização**         | CanvasKit/Skia na web                             | Um único backend Skia com aceleração GPU em **todos** os alvos           |
| **Memória**              | Pausas do JavaScript GC                           | Sem GC — ownership Rust, latência previsível                             |
| **Base de código**       | Web stack + Electron             | Um workspace Rust: editor · CLI · MCP · AI · codegen · Figma · Git       |
| **Plataformas**          | Web + desktop, duas stacks separadas              | Desktop (macOS/Win/Linux) · mobile (iOS/Android) · navegador — um núcleo |

**Melhorias mensuradas**

- **Pegada mínima** — o aplicativo desktop completo é um único binário nativo de **55.5 MB** em vez de um motor de navegador empacotado mais um runtime Node. A versão web tem **8.2 MB** bruto / **2.18 MB** gzip após a divisão do catálogo de ícones (−48% transferido pela rede).
- **Escala para documentos grandes** — um canvas ativo com **10,000 nós** (auto-layout aninhado, quatro níveis de profundidade) grava, lê e captura o layout **sem pânicos e com ~0% de CPU ociosa**; um snapshot completo do layout de todos os 10k nós retorna em **~0.68 s**.
- **Interação rápida** — pan/zoom não mais reserializa o documento a cada frame (uma única correção no hot-path reduziu o uso de CPU no zoom com scroll de **~69% para ~0%**); arrastar atualiza a cena de forma incremental, a medição de texto é armazenada em cache e os redesenhos são consolidados em um por frame.
- **Um núcleo, cada tela** — o mesmo estado do editor e o mesmo backend de renderização compilam para desktop nativo, mobile e o navegador via WASM — sem reimplementações paralelas para manter sincronizadas.
- **GPU Skia em todo lugar** — o nativo renderiza via `skia-safe` em um contexto GL; o navegador renderiza via CanvasKit no WebGL2 — o mesmo código de desenho, a mesma saída.
- **Acessibilidade nativa** — AccessKit no macOS, Windows e Linux, mais um espelho DOM na web, em vez de depender da árvore de acessibilidade de um navegador.
- **Um workspace com verificação de tipos** — o host MCP, CLI, provedores de AI, geração de código, importação do Figma e integração com Git vivem em um único workspace Rust, com o `cargo-deny` fazendo controle da cadeia de suprimentos na CI.

> **Status:** o editor TypeScript foi descontinuado na `v0.7.5` e existe apenas no histórico do Git; este repositório é o workspace Rust. O lançamento Rust `v0.8.1` está em desenvolvimento ativo (consulte o Roadmap abaixo).

## Estrutura do Projeto

```text
openpencil/
├── crates/                   Workspace Rust — o produto
│   ├── op-editor-core/       Estado canônico do editor `.op` (PenDocument) + EditorCommand + variáveis de design
│   ├── op-editor-ui/         Widgets independentes de plataforma + fachada RenderBackend (wasm32-clean)
│   ├── op-editor-host-core/  Máquinas de estado de host sem transporte, compartilhadas por todos os hosts
│   ├── op-host-native/       Lib de host nativo — winit + skia-safe GL (desktop + mobile)
│   ├── op-host-web/          Bundle para navegador — cdylib wasm32, renderizador CanvasKit
│   ├── op-host-desktop/      Binário desktop `openpencil-desktop`; também o daemon `--serve-web`
│   ├── op-host-services/     Lib do daemon headless serve-web / MCP
│   ├── op-host-web-server/   Binário web-server enxuto, sem GL
│   ├── op-cli/               Ferramenta CLI — comando `op`
│   ├── op-mcp/               Servidor MCP — ferramentas, design em lote, fluxo em camadas
│   ├── op-ai/                Provedores de IA, runtime de chat, streaming
│   ├── op-ai-skills/         Engine de skills de IA (carregamento de prompts por fases)
│   ├── op-orchestrator/      Orquestração de equipes de agentes concorrentes
│   ├── op-codegen/           Geradores de código (React, HTML, Vue, Flutter, ...)
│   ├── op-figma/             Parser e conversor de arquivos .fig do Figma
│   ├── op-git/               Integração com Git — clone, branch, push/pull, merge
│   └── ...                   op-opmerge / op-pen-loader / op-design-lint / op-i18n /
│                             op-config-store / op-process-io / op-acp / op-smoke / ...
├── packages/                 Workspace do SDK web (Bun)
│   ├── op-web-sdk/           SDK web viewer `.op` somente leitura (envolve o bundle wasm)
│   ├── op-web-sdk-react/     Adaptador React 19
│   └── op-web-sdk-vue/       Adaptador Vue 3
├── vendor/                   Subsistemas vendorizados (submódulos git)
│   ├── jian/                 Framework de UI GPU-Skia — widgets/renderização/eventos
│   ├── casement/             Fork do winit
│   └── agent/                Runtime de agente Rust compartilhado entre produtos (agent-rs)
└── .githooks/                Verificação de consistência de versão no pre-commit
```

## Atalhos de Teclado

| Tecla       | Ação                |     | Tecla         | Ação                           |
| ----------- | ------------------- | --- | ------------- | ------------------------------ |
| `V`         | Selecionar          |     | `Cmd+S`       | Salvar                         |
| `R`         | Retângulo           |     | `Cmd+Z`       | Desfazer                       |
| `O`         | Elipse              |     | `Cmd+Shift+Z` | Refazer                        |
| `L`         | Linha               |     | `Cmd+C/X/V/D` | Copiar/Recortar/Colar/Duplicar |
| `T`         | Texto               |     | `Cmd+G`       | Agrupar                        |
| `F`         | Frame               |     | `Cmd+Shift+G` | Desagrupar                     |
| `P`         | Ferramenta caneta   |     | `Cmd+Shift+P` | Exportar (PNG/JPG/WEBP/PDF)    |
| `H`         | Mão (pan)           |     | `Cmd+Shift+C` | Painel de código               |
| `Del`       | Excluir             |     | `Cmd+Shift+V` | Painel de variáveis            |
| `[ / ]`     | Reordenar           |     | `Cmd+J`       | Chat IA                        |
| Setas       | Mover 1px           |     | `Cmd+,`       | Configurações do agente        |
| `Cmd+Alt+U` | União booleana      |     | `Cmd+Alt+S`   | Subtração booleana             |
| `Cmd+Alt+I` | Interseção booleana |     | `Cmd+Shift+S` | Salvar como                    |

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
scripts/sync-version.sh                            # Sync all managed versions from root Cargo.toml
tools/check-version-sync.sh                        # Verify all managed versions match root Cargo.toml
```

## Contribuindo

Contribuições são bem-vindas! Consulte o [CLAUDE.md](./CLAUDE.md) para detalhes de arquitetura e estilo de código.

1. Faça fork e clone
2. Configure a sincronização de versão: `git config core.hooksPath .githooks`
3. Crie uma branch: `git checkout -b feat/my-feature`
4. Execute as verificações: `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
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
- [x] Workspace Cargo com crates Rust e pacotes de SDK web reutilizáveis
- [x] Editor Rust para desktop e web
- [x] Ferramenta CLI (`op`) para controle pelo terminal
- [x] Runtime de agentes Rust integrado com suporte multi-provedor
- [x] i18n — 15 idiomas
- [x] SDKs de visualização baseados em wasm para JavaScript, React e Vue
- [x] Style Guides com correspondência por tags e ferramentas MCP
- [x] Agent Teams concorrentes com delegação e indicadores no canvas
- [x] Integração com Git (clone, branch, push/pull, merge a três vias em modo pasta)
- [x] Exportação do canvas (SVG / PNG / JPEG / WEBP / PDF)
- [ ] Edição colaborativa
- [ ] Sistema de plugins

## Contribuidores

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Patrocinadores

OpenPencil é gratuito e de código aberto. O desenvolvimento é financiado por quem o acha útil — obrigado por manter o canvas aberto.

<a href="https://github.com/mrqyun" title="MrQyun">
  <img src="https://wsrv.nl/?url=github.com/mrqyun.png&w=128&h=128&mask=circle&maxage=7d" width="64" height="64" alt="MrQyun" />
</a>

Obrigado a **[MrQyun](https://github.com/mrqyun)** — quer ver seu nome aqui? **[Torne-se um patrocinador →](https://github.com/sponsors/ZSeven-W)**

## Comunidade

<a href="https://discord.gg/h9Fmyy6pVh">
  <img src="./screenshot/logo-discord.svg" alt="Discord" width="16" />
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

## Avaliações

[![MseeP.ai Security Assessment Badge](https://mseep.net/pr/zseven-w-openpencil-badge.png)](https://mseep.ai/app/zseven-w-openpencil)

## Licença

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
