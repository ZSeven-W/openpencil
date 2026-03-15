<p align="center">
  <img src="./electron/icon.png" alt="OpenPencil" width="120" />
</p>

<h1 align="center">OpenPencil</h1>

<p align="center">
  <strong>La primera herramienta de diseño vectorial de código abierto nativa de IA del mundo.</strong><br />
  <sub>Equipos de Agentes Concurrentes &bull; Diseño como Código &bull; Servidor MCP Integrado &bull; Inteligencia Multimodelo</sub>
</p>

<p align="center">
  <a href="./README.md">English</a> · <a href="./README.zh.md">简体中文</a> · <a href="./README.zh-TW.md">繁體中文</a> · <a href="./README.ja.md">日本語</a> · <a href="./README.ko.md">한국어</a> · <a href="./README.fr.md">Français</a> · <a href="./README.es.md"><b>Español</b></a> · <a href="./README.de.md">Deutsch</a> · <a href="./README.pt.md">Português</a> · <a href="./README.ru.md">Русский</a> · <a href="./README.hi.md">हिन्दी</a> · <a href="./README.tr.md">Türkçe</a> · <a href="./README.th.md">ไทย</a> · <a href="./README.vi.md">Tiếng Việt</a> · <a href="./README.id.md">Bahasa Indonesia</a>
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
    <img src="./screenshot/op-cover.png" alt="OpenPencil — haz clic para ver la demostración" width="100%" />
  </a>
</p>
<p align="center"><sub>Haz clic en la imagen para ver el video de demostración</sub></p>

<br />

## Por Qué OpenPencil

<table>
<tr>
<td width="50%">

### 🎨 Prompt → Lienzo

Describe cualquier interfaz en lenguaje natural. Obsérvala aparecer en el lienzo infinito en tiempo real con animación de transmisión. Modifica diseños existentes seleccionando elementos y chateando.

</td>
<td width="50%">

### 🤖 Equipos de Agentes Concurrentes

El orquestador descompone páginas complejas en subtareas espaciales. Múltiples agentes de IA trabajan en diferentes secciones simultáneamente — hero, características, footer — todos transmitiendo en paralelo.

</td>
</tr>
<tr>
<td width="50%">

### 🧠 Inteligencia Multimodelo

Se adapta automáticamente a las capacidades de cada modelo. Claude recibe prompts completos con pensamiento; GPT-4o/Gemini desactivan el pensamiento; modelos más pequeños (MiniMax, Qwen, Llama) reciben prompts simplificados para una salida confiable.

</td>
<td width="50%">

### 🔌 Servidor MCP

Instalación con un clic en Claude Code, Codex, Gemini, OpenCode, Kiro o Copilot CLIs. Diseña desde tu terminal — lee, crea y modifica archivos `.op` a través de cualquier agente compatible con MCP.

</td>
</tr>
<tr>
<td width="50%">

### 📦 Diseño como Código

Los archivos `.op` son JSON — legibles por humanos, compatibles con Git, comparables. Las variables de diseño generan propiedades personalizadas CSS. Exportación de código a React + Tailwind o HTML + CSS.

</td>
<td width="50%">

### 🖥️ Funciona en Todas Partes

Aplicación web + escritorio nativo en macOS, Windows y Linux mediante Electron. Actualizaciones automáticas desde GitHub Releases. Asociación de archivos `.op` — doble clic para abrir.

</td>
</tr>
</table>

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
| **GitHub Copilot** | `copilot login` y luego conectar en Configuración de Agente (`Cmd+,`) |

**Perfiles de Capacidad de Modelos** — adapta automáticamente los prompts, el modo de pensamiento y los tiempos de espera según el nivel del modelo. Los modelos de nivel completo (Claude) reciben prompts completos; los de nivel estándar (GPT-4o, Gemini, DeepSeek) desactivan el pensamiento; los de nivel básico (MiniMax, Qwen, Llama, Mistral) reciben prompts simplificados de JSON anidado para máxima fiabilidad.

**Servidor MCP**
- Servidor MCP integrado — instalación con un clic en Claude Code / Codex / Gemini / OpenCode / Kiro / Copilot CLIs
- Detección automática de Node.js — si no está instalado, recurre automáticamente al transporte HTTP e inicia el servidor MCP HTTP
- Automatización de diseño desde la terminal: leer, crear y modificar archivos `.op` a través de cualquier agente compatible con MCP
- **Flujo de diseño por capas** — `design_skeleton` → `design_content` → `design_refine` para diseños multisección de mayor fidelidad
- **Recuperación segmentada de prompts** — carga solo el conocimiento de diseño que necesitas (schema, layout, roles, icons, planning, etc.)
- Soporte multipágina — crear, renombrar, reordenar y duplicar páginas mediante herramientas MCP

**Generación de Código**
- React + Tailwind CSS, HTML + CSS, CSS Variables
- Vue, Svelte, Flutter, SwiftUI, Jetpack Compose, React Native

## Características

**Lienzo y Dibujo**
- Lienzo infinito con panorámica, zoom, guías de alineación inteligentes y ajuste
- Rectángulo, Elipse, Línea, Polígono, Pluma (Bezier), Frame, Texto
- Operaciones booleanas — unión, resta, intersección con barra de herramientas contextual
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
- Asociación de archivos `.op` — doble clic para abrir, bloqueo de instancia única
- Actualización automática desde GitHub Releases
- Menú de aplicación nativo y diálogos de archivo

## Stack Tecnológico

| | |
| --- | --- |
| **Frontend** | React 19 · TanStack Start · Tailwind CSS v4 · shadcn/ui |
| **Lienzo** | CanvasKit/Skia (WASM, acelerado por GPU) |
| **Estado** | Zustand v5 |
| **Servidor** | Nitro |
| **Escritorio** | Electron 35 |
| **IA** | Anthropic SDK · Claude Agent SDK · OpenCode SDK · Copilot SDK |
| **Runtime** | Bun · Vite 7 |
| **Formato de archivo** | `.op` — basado en JSON, legible por humanos, compatible con Git |

## Estructura del Proyecto

```text
src/
  canvas/          Motor CanvasKit/Skia — dibujo, sincronización, diseño, guías, herramienta pluma
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
  utils/           Wrappers de cliente Claude CLI, OpenCode, Codex, Copilot
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
| `Cmd+Alt+U` | Unión booleana | | `Cmd+Alt+S` | Resta booleana |
| `Cmd+Alt+I` | Intersección booleana | | | |

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
- [x] Integración con servidor MCP con flujo de diseño por capas
- [x] Soporte multipágina
- [x] Importación de Figma `.fig`
- [x] Operaciones booleanas (unión, sustracción, intersección)
- [x] Perfiles de capacidad multimodelo
- [ ] Edición colaborativa
- [ ] Sistema de plugins

## Colaboradores

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

## Comunidad

<a href="https://discord.gg/KwXp6BJD">
  <img src="./public/logo-discord.svg" alt="Discord" width="16" />
  <strong> Únete a nuestro Discord</strong>
</a>
— Haz preguntas, comparte diseños y sugiere funciones.


