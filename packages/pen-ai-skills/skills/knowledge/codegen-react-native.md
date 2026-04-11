---
name: codegen-react-native
description: React Native code generation rules — View/Text/Image with StyleSheet.create()
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# React Native Code Generation

Generate React Native components with `StyleSheet.create()` style objects. No CSS, no web-specific styling.

## Output Format

- TypeScript/JavaScript (`.tsx` or `.jsx`)
- Functional components with `export function ComponentName()`
- Import from `react-native`: `View`, `Text`, `Image`, `StyleSheet`
- Import from `react-native-svg` when path/polygon nodes exist: `Svg`, `Path as SvgPath`, `Polygon as SvgPolygon`
- All styling via inline style objects or `StyleSheet.create()` for performance

## Layout Mapping

- `layout: "vertical"` → `flexDirection: 'column'`
- `layout: "horizontal"` → `flexDirection: 'row'`
- No layout → default column (React Native default)
- `gap: N` → `gap: N` (numeric, no units)
- `padding: N` → `padding: N`
- `padding: [vertical, horizontal]` → `paddingVertical: V, paddingHorizontal: H`
- `padding: [top, right, bottom, left]` → `paddingTop: T, paddingRight: R, paddingBottom: B, paddingLeft: L`
- `justifyContent: "start"` → `justifyContent: 'flex-start'`
- `justifyContent: "center"` → `justifyContent: 'center'`
- `justifyContent: "end"` → `justifyContent: 'flex-end'`
- `justifyContent: "space_between"` → `justifyContent: 'space-between'`
- `justifyContent: "space_around"` → `justifyContent: 'space-around'`
- `alignItems: "start"` → `alignItems: 'flex-start'`
- `alignItems: "center"` → `alignItems: 'center'`
- `alignItems: "end"` → `alignItems: 'flex-end'`
- `clipContent: true` → `overflow: 'hidden'`

## Components

- Container/frame/rectangle/group → `<View style={...} />`
- Text → `<Text style={...}>content</Text>`
- Image → `<Image source={...} style={...} />`
- Line → `<View style={{ width: N, height: thickness, backgroundColor: color }} />`
- Ellipse → `<View style={{ borderRadius: min(w,h)/2, ...fill }} />`

## Color & Fill Mapping

- Solid fill `#hex` → `backgroundColor: '#hex'`
- Variable ref `$name` → `/* var(--name) */ backgroundColor: '#000000'` (placeholder with comment)
- Text fill → `color: '#hex'`
- Gradients: not natively supported in React Native — use `react-native-linear-gradient` library or note as comment

## Border & Stroke Mapping

- `stroke.thickness` → `borderWidth: N` (numeric, no units)
- `stroke.color` → `borderColor: '#hex'`
- Variable ref → `/* var(--name) */ borderWidth: 1` placeholder

## Corner Radius

- Uniform → `borderRadius: N`
- Per-corner → `borderTopLeftRadius: TL, borderTopRightRadius: TR, borderBottomRightRadius: BR, borderBottomLeftRadius: BL`
- Ellipse → `borderRadius: min(width, height) / 2`

## Effects (Shadows)

- Shadow color → `shadowColor: '#color'`
- Shadow offset → `shadowOffset: { width: X, height: Y }`
- Shadow opacity → `shadowOpacity: 1`
- Shadow radius → `shadowRadius: blur`
- Android elevation → `elevation: Math.round(blur / 2)` (minimum 1)

## Typography

- `fontSize` → `fontSize: N` (numeric, no units)
- `fontWeight` → `fontWeight: 'N'` (string: '100' through '900')
- `fontStyle: "italic"` → `fontStyle: 'italic'`
- `fontFamily` → `fontFamily: 'Name'`
- `letterSpacing` → `letterSpacing: N`
- `lineHeight` → `lineHeight: Math.round(fontSize * lineHeight)` (computed absolute value)
- `textAlign` → `textAlign: 'left'|'center'|'right'`
- `underline` → `textDecorationLine: 'underline'`
- `strikethrough` → `textDecorationLine: 'line-through'`
- Both → `textDecorationLine: 'underline line-through'`

## Dimensions

- Fixed → `width: N, height: N` (numeric, no units — React Native uses density-independent pixels)
- `fill_container` → `flex: 1` or `width: '100%'`

## Image Handling

- Network URL → `<Image source={{ uri: 'url' }} style={...} />`
- Data URI → `<Image source={{ uri: 'data:image/...' }} style={...} />`
- Local asset → `<Image source={require('./path')} style={...} />`
- `objectFit: "fit"` → `resizeMode: 'contain'`
- `objectFit: "crop"` → `resizeMode: 'cover'`
- `objectFit: "fill"` → `resizeMode: 'stretch'`
- Corner radius applied directly in style object

## Opacity & Transform

- `opacity: N` → `opacity: N` (numeric 0-1)
- Variable ref → `/* var(--name) */ opacity: 1` placeholder
- `rotation: N` → `transform: [{ rotate: 'Ndeg' }]` (string with deg suffix)

## Positioning

- Absolute children → `position: 'absolute', left: X, top: Y`
- Container → `position: 'relative'` (default in RN, usually not needed)

## SVG Elements (via react-native-svg)

- Path nodes → `<Svg width={W} height={H} viewBox="0 0 W H"><SvgPath d="..." fill="color" /></Svg>`
- Polygon nodes → `<Svg><SvgPolygon points="x1,y1 x2,y2 ..." fill="color" /></Svg>`
- Wrap in `<View style={positionStyles}>` if positioned

## Icon Handling

- Icon font nodes → `<IconName size={N} color="color" />` (kebab-to-PascalCase)

## Style Values — Key Differences from CSS

- All numeric values are unitless (no `px`, `em`, `rem`)
- String values must be quoted: `'center'`, `'row'`, `'absolute'`
- `borderRadius` does not support shorthand — use per-corner properties for different values
- No `box-shadow` — use `shadowColor` + `shadowOffset` + `shadowOpacity` + `shadowRadius`
- No CSS gradients — use third-party libraries
- `transform` takes an array of objects: `[{ rotate: '45deg' }, { scale: 2 }]`

## Responsive Design

- Use `Dimensions.get('window')` for screen width/height
- Use `useWindowDimensions()` hook for reactive screen size
- Use `flex: 1` and percentage widths for adaptive layouts
- `ScrollView` for scrollable content
