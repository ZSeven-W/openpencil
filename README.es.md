<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>Herramienta de diseño de código abierto nativa de IA. Diseño como Código.</strong><br />
  De prompt a interfaz en el lienzo. Orquestación multiagente. Servidor MCP integrado. Generación de código.
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md"><b>Español</b></a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
</p>

<p align="center">
  <a href="https://github.com/ZSeven-W/openpencil/stargazers"><img src="https://img.shields.io/github/stars/ZSeven-W/openpencil?style=flat" alt="Stars" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/blob/main/LICENSE"><img src="https://img.shields.io/github/license/ZSeven-W/openpencil" alt="License" /></a>
  <a href="https://github.com/ZSeven-W/openpencil/actions/workflows/ci.yml"><img src="https://img.shields.io/github/actions/workflow/status/ZSeven-W/openpencil/ci.yml?branch=main&label=CI" alt="CI" /></a>
  <a href="https://discord.gg/fE9STbMG"><img src="https://img.shields.io/discord/1476517942949580952?label=Discord&logo=discord&logoColor=white" alt="Discord" /></a>
</p>

<p align="center">
  <a href="#inicio-rápido">Inicio Rápido</a> ·
  <a href="#diseño-nativo-de-ia">IA</a> ·
  <a href="#características">Características</a> ·
  <a href="https://discord.gg/fE9STbMG">Discord</a> ·
  <a href="#contribuir">Contribuir</a>
</p>

<br />

<p align="center">
  <a href="https://oss.ioa.tech/zseven/openpencil/a46e24733239ce24de36702342201033.mp4">
    <img src="./screenshot/op-cover.png" alt="OpenPencil — click to watch demo" width="100%" />
  </a>
</p>
<p align="center"><sub>Haz clic en la imagen para ver el video de demostración</sub></p>

<br />

## Inicio Rápido

```bash
# Instalar dependencias
bun install

# Iniciar el servidor de desarrollo en http://localhost:3000
bun --bun run dev
```

O ejecutar como aplicación de escritorio:

```bash
bun run electron:dev
```

> **Requisitos previos:** [Bun](https://bun.sh/) >= 1.0 y [Node.js](https://nodejs.org/) >= 18

## Diseño Nativo de IA

OpenPencil está construido desde cero con IA en su núcleo — no como un plugin, sino como un flujo de trabajo central.

**De Prompt a Interfaz**
- **Texto a diseño** — describe una página y se genera en el lienzo en tiempo real con animación de transmisión
- **Orquestador** — descompone páginas complejas en subtareas espaciales para generación en paralelo
- **Modificación de diseño** — selecciona elementos y describe los cambios en lenguaje natural
- **Entrada visual** — adjunta capturas de pantalla o bocetos como referencia para el diseño

**Soporte Multiagente**

| Agente | Configuración |
| --- | --- |
| **Claude Code** | Sin configuración — usa Claude Agent SDK con OAuth local |
| **Codex CLI** | Conectar en Configuración de Agente (`Cmd+,`) |
| **OpenCode** | Conectar en Configuración de Agente (`Cmd+,`) |

**Servidor MCP**
- Servidor MCP integrado — instalación con un clic en Claude Code / Codex / Gemini / OpenCode / Kiro CLIs
- Automatización de diseño desde la terminal: leer, crear y modificar archivos `.op` a través de cualquier agente compatible con MCP

**Generación de Código**
- React + Tailwind CSS
- HTML + CSS
- CSS Variables a partir de tokens de diseño

## Características

**Lienzo y Dibujo**
- Lienzo infinito con panorámica, zoom, guías de alineación inteligentes y ajuste
- Rectángulo, Elipse, Línea, Polígono, Pluma (Bezier), Frame, Texto
- Selector de iconos (Iconify) e importación de imágenes (PNG/JPEG/SVG/WebP/GIF)
- Diseño automático — vertical/horizontal con gap, padding, justify, align
- Documentos multipágina con navegación por pestañas

**Sistema de Diseño**
- Variables de diseño — tokens de color, número y texto con referencias `$variable`
- Soporte multitema — múltiples ejes, cada uno con variantes (Claro/Oscuro, Compacto/Cómodo)
- Sistema de componentes — componentes reutilizables con instancias y sobreescrituras
- Sincronización CSS — propiedades personalizadas autogeneradas, `var(--name)` en la salida de código

**Importación de Figma**
- Importa archivos `.fig` conservando diseño, rellenos, trazos, efectos, texto, imágenes y vectores

**Aplicación de Escritorio**
- Compatible de forma nativa con macOS, Windows y Linux mediante Electron
- Actualización automática desde GitHub Releases
- Menú de aplicación nativo y diálogos de archivo

## Stack Tecnológico

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Lienzo** | Fabric.js v7 |
| **Estado** | Zustand v5 |
| **Servidor** | Nitro |
| **Escritorio** | Electron 35 |
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK |
| **Runtime** | Bun · Vite 7 |
| **Formato de archivo** | `.op` — basado en JSON, legible por humanos, compatible con Git |

## Estructura del Proyecto

```text
src/
  canvas/          Motor Fabric.js — dibujo, sincronización, diseño, guías, herramienta pluma
  components/      Interfaz React — editor, paneles, diálogos compartidos, iconos
  services/ai/     Chat de IA, orquestador, generación de diseño, transmisión
  services/figma/  Pipeline de importación binaria de Figma .fig
  services/codegen Generadores de código React+Tailwind y HTML+CSS
  stores/          Zustand — lienzo, documento, páginas, historial, IA, configuración
  variables/       Resolución de tokens de diseño y gestión de referencias
  mcp/             Herramientas del servidor MCP para integración con CLI externas
  uikit/           Sistema de kit de componentes reutilizables
server/
  api/ai/          API Nitro — chat en streaming, generación, validación
  utils/           Wrappers de cliente Claude CLI, OpenCode, Codex
electron/
  main.ts          Ventana, fork Nitro, menú nativo, actualizador automático
  preload.ts       Puente IPC
```

## Atajos de Teclado

| Tecla | Acción | | Tecla | Acción |
| --- | --- | --- | --- | --- |
| `V` | Seleccionar | | `Cmd+S` | Guardar |
| `R` | Rectángulo | | `Cmd+Z` | Deshacer |
| `O` | Elipse | | `Cmd+Shift+Z` | Rehacer |
| `L` | Línea | | `Cmd+C/X/V/D` | Copiar/Cortar/Pegar/Duplicar |
| `T` | Texto | | `Cmd+G` | Agrupar |
| `F` | Frame | | `Cmd+Shift+G` | Desagrupar |
| `P` | Herramienta pluma | | `Cmd+Shift+E` | Exportar |
| `H` | Mano (panorámica) | | `Cmd+Shift+C` | Panel de código |
| `Del` | Eliminar | | `Cmd+Shift+V` | Panel de variables |
| `[ / ]` | Reordenar | | `Cmd+J` | Chat de IA |
| Flechas | Mover 1px | | `Cmd+,` | Configuración de agente |

## Scripts

```bash
bun --bun run dev          # Servidor de desarrollo (puerto 3000)
bun --bun run build        # Compilación de producción
bun --bun run test         # Ejecutar pruebas (Vitest)
npx tsc --noEmit           # Verificación de tipos
bun run electron:dev       # Desarrollo con Electron
bun run electron:build     # Empaquetado de Electron
```

## Contribuir

¡Las contribuciones son bienvenidas! Consulta [CLAUDE.md](./CLAUDE.md) para detalles sobre la arquitectura y el estilo de código.

1. Haz fork y clona el repositorio
2. Crea una rama: `git checkout -b feat/my-feature`
3. Ejecuta las verificaciones: `npx tsc --noEmit && bun --bun run test`
4. Haz commit con [Conventional Commits](https://www.conventionalcommits.org/): `feat(canvas): add rotation snapping`
5. Abre un PR contra `main`

## Hoja de Ruta

- [x] Variables de diseño y tokens con sincronización CSS
- [x] Sistema de componentes (instancias y sobreescrituras)
- [x] Generación de diseño con IA y orquestador
- [x] Integración con servidor MCP
- [x] Soporte multipágina
- [x] Importación de Figma `.fig`
- [ ] Operaciones booleanas (unión, sustracción, intersección)
- [ ] Edición colaborativa
- [ ] Sistema de plugins

## Colaboradores

<a href="https://github.com/ZSeven-W/openpencil/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=ZSeven-W/openpencil" alt="Contributors" />
</a>

## Comunidad

<a href="https://discord.gg/fE9STbMG">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Únete a nuestro Discord</strong>
</a>
— Haz preguntas, comparte diseños y sugiere funciones.

## Licencia

[MIT](./LICENSE) — Copyright (c) 2026 ZSeven-W
