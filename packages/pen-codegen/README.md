# @zseven-w/pen-codegen

Multi-platform code generators for [OpenPencil](https://github.com/nicepkg/openpencil) designs. Turn your design files into production-ready code for 8 frameworks.

## Install

```bash
npm install @zseven-w/pen-codegen
```

## Supported Platforms

| Platform | Generator | Output |
|---|---|---|
| React + Tailwind | `generateReactCode` | `.tsx` with Tailwind classes |
| HTML + CSS | `generateHTMLCode` | Vanilla HTML/CSS |
| Vue 3 | `generateVueCode` | `.vue` SFC |
| Svelte | `generateSvelteCode` | `.svelte` component |
| Flutter | `generateFlutterCode` | Dart widget |
| SwiftUI | `generateSwiftUICode` | Swift view |
| Jetpack Compose | `generateComposeCode` | Kotlin composable |
| React Native | `generateReactNativeCode` | `.tsx` with StyleSheet |

## Usage

Generate code from a single node:

```ts
import { generateReactCode } from '@zseven-w/pen-codegen'

const code = generateReactCode(node, { indent: 2 })
```

Generate from an entire document (resolves variables, computes layout):

```ts
import { generateReactFromDocument } from '@zseven-w/pen-codegen'

const code = generateReactFromDocument(document)
```

### CSS Variables

Extract design variables as CSS custom properties:

```ts
import { generateCSSVariables } from '@zseven-w/pen-codegen'

const css = generateCSSVariables(variables, themes)
// :root { --color-primary: #3b82f6; ... }
```

## License

MIT
