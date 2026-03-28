// CSS Variables
export { variableNameToCSS, generateCSSVariables } from './css-variables-generator.js'

// React + Tailwind
export { generateReactCode, generateReactFromDocument } from './react-generator.js'

// HTML + CSS
export { generateHTMLCode, generateHTMLFromDocument } from './html-generator.js'

// Vue 3
export { generateVueCode, generateVueFromDocument } from './vue-generator.js'

// Svelte
export { generateSvelteCode, generateSvelteFromDocument } from './svelte-generator.js'

// Flutter / Dart
export { generateFlutterCode, generateFlutterFromDocument } from './flutter-generator.js'

// SwiftUI
export { generateSwiftUICode, generateSwiftUIFromDocument } from './swiftui-generator.js'

// Android Jetpack Compose
export { generateComposeCode, generateComposeFromDocument } from './compose-generator.js'

// React Native
export { generateReactNativeCode, generateReactNativeFromDocument } from './react-native-generator.js'

// Utilities
export { varOrLiteral, sanitizeName, nodeTreeToSummary, isVariableRef } from './utils.js'

// Types
export type {
  Framework,
  PlannedChunk,
  CodePlanFromAI,
  ExecutableChunk,
  CodeExecutionPlan,
  ChunkContract,
  PropDef,
  SlotDef,
  ImportDef,
  ChunkResult,
  ChunkStatus,
  CodeGenProgress,
  ContractValidationResult,
} from './codegen-types.js'
export { FRAMEWORKS, validateContract } from './codegen-types.js'
