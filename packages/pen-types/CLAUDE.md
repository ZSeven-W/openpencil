# pen-types

TypeScript type definitions for the PenDocument model and all shared interfaces.

## Structure

- `src/pen.ts` — Core document model: `PenDocument`, `PenPage`, `PenNode` union (frame, group, rectangle, ellipse, line, polygon, path, text, image, icon_font, ref), `PenNodeBase`, `ContainerProps`, `SizingBehavior`, `PenPathAnchor`
- `src/styles.ts` — Visual style types: `PenFill` (solid, linear_gradient, radial_gradient, image), `PenStroke`, `PenEffect` (blur, shadow), `BlendMode`, `StyledTextSegment`, `GradientStop`
- `src/canvas.ts` — Canvas interaction types: `ToolType`, `ViewportState`, `SelectionState`, `CanvasInteraction`
- `src/variables.ts` — Design variable types: `VariableDefinition`, `VariableValue`, `ThemedValue`
- `src/engine.ts` — Engine option/event types: `DesignEngineOptions`, `DesignEngineEvents`, `CodePlatform`, `CodeResult`, `TextEditState`, `AgentIndicatorEntry`, `InsertionIndicator`, `IconLookupFn`
- `src/codegen.ts` — Code generation types: `Framework`, `FRAMEWORKS` (only runtime export), `PlannedChunk`, `CodePlanFromAI`, `ExecutableChunk`, `CodeExecutionPlan`, `ChunkContract`, `ChunkResult`, `CodeGenProgress`, `NodeSnapshot`, `ExecutableChunkPayload`
- `src/layout.ts` — Layout type: `Padding`
- `src/uikit.ts` — UI kit types: `UIKit`, `KitComponent`, `ComponentCategory`
- `src/theme-preset.ts` — Theme preset types: `ThemePreset`, `ThemePresetFile`
- `src/design-md.ts` — Design spec types: `DesignMdSpec`, `DesignMdColor`, `DesignMdTypography`
- `src/index.ts` — Barrel re-export of all types

## Key exports

All exports are `type`-only except `FRAMEWORKS` (a runtime `Framework[]` constant). Consumed by every other package in the monorepo.

## Testing

```bash
bun --bun vitest run packages/pen-types/src/__tests__/
```
