---
name: codegen-compose
description: Jetpack Compose (Kotlin) code generation rules — composable functions with Modifier chains
phase: [generation]
trigger:
  keywords: [compose, jetpack, kotlin, android]
priority: 20
budget: 2000
category: knowledge
---

# Jetpack Compose (Kotlin) Code Generation

Generate Kotlin composable functions using Jetpack Compose UI toolkit.

## Output Format
- Kotlin file (`.kt`)
- `@Composable fun ComponentName() { ... }`
- Standard Compose imports: `androidx.compose.foundation.*`, `androidx.compose.material3.*`, `androidx.compose.ui.*`
- Use `dp` for dimensions, `sp` for font sizes

## Layout Mapping
- `layout: "vertical"` → `Column(modifier, verticalArrangement, horizontalAlignment) { ... }`
- `layout: "horizontal"` → `Row(modifier, horizontalArrangement, verticalAlignment) { ... }`
- No layout / stacked children → `Box(modifier) { ... }`
- `gap: N` → `verticalArrangement = Arrangement.spacedBy(N.dp)` (Column) or `horizontalArrangement = Arrangement.spacedBy(N.dp)` (Row)
- `alignItems` in Column (horizontal alignment):
  - `"start"` → `horizontalAlignment = Alignment.Start`
  - `"center"` → `horizontalAlignment = Alignment.CenterHorizontally`
  - `"end"` → `horizontalAlignment = Alignment.End`
- `alignItems` in Row (vertical alignment):
  - `"start"` → `verticalAlignment = Alignment.Top`
  - `"center"` → `verticalAlignment = Alignment.CenterVertically`
  - `"end"` → `verticalAlignment = Alignment.Bottom`

## Modifier Chain Pattern
- Compose uses `Modifier` chains: `Modifier.size().background().border().padding()`
- Order matters: modifiers apply outside-in
- Size before background: `.size(width = N.dp, height = N.dp)`
- Width only: `.width(N.dp)`, height only: `.height(N.dp)`
- `clipContent: true` → `.clipToBounds()`
- Position offset: `.offset(x = N.dp, y = N.dp)`
- Rotation: `.rotate(Nf)`
- Opacity: `.alpha(Nf)`

## Color & Fill Mapping
- Solid fill `#RRGGBB` → `Color(0xFFRRGGBB)` (uppercase hex, FF alpha prefix)
- 8-digit hex `#RRGGBBAA` → `Color(0xAARRGGBB)` (alpha moved to front)
- Variable ref `$name` → `Color.Unspecified /* var(--name) */` placeholder
- Background with shape: `.background(Color(...), RoundedCornerShape(N.dp))`
- Background without shape: `.background(Color(...))`
- Text color → `color = Color(0xFFhex)` parameter on `Text()`
- Linear gradient → `Brush.linearGradient(listOf(Color(...), Color(...)))` as background
- Radial gradient → `Brush.radialGradient(listOf(Color(...), Color(...)))` as background

## Border & Stroke Mapping
- `stroke` → `.border(N.dp, Color(...), shape)`
- Shape defaults to `RectangleShape` if no corner radius
- With corner radius → `.border(N.dp, Color(...), RoundedCornerShape(N.dp))`
- Variable ref thickness → `/* var(--name) */ 1.dp` placeholder

## Corner Radius
- Uniform → `RoundedCornerShape(N.dp)`
- Per-corner → `RoundedCornerShape(topStart = TL.dp, topEnd = TR.dp, bottomEnd = BR.dp, bottomStart = BL.dp)`
- Applied via `.clip(shape)` or as parameter in `.background(color, shape)` / `.border()`

## Effects
- Shadow → `.shadow(elevation = N.dp, shape = RoundedCornerShape(0.dp))`
- Blur → `// .blur(radius = N.dp) — requires custom implementation` (not natively supported as modifier)

## Typography
- Text nodes → `Text(text = "content", fontSize, fontWeight, color, ...)`
- `fontSize` → `fontSize = N.sp`
- `fontWeight` → `fontWeight = FontWeight.Thin|ExtraLight|Light|Normal|Medium|SemiBold|Bold|ExtraBold|Black`
- `fontStyle: "italic"` → `fontStyle = FontStyle.Italic`
- `fontFamily` → `fontFamily = FontFamily(Font(R.font.name))`
- `letterSpacing` → `letterSpacing = N.sp`
- `lineHeight` → `lineHeight = (lineHeight * fontSize).sp`
- `textAlign` → `textAlign = TextAlign.Start|Center|End|Justify`
- `underline` → `textDecoration = TextDecoration.Underline`
- `strikethrough` → `textDecoration = TextDecoration.LineThrough`
- Combined → `textDecoration = TextDecoration.combine(listOf(TextDecoration.Underline, TextDecoration.LineThrough))`
- Short param list (2 or fewer) → inline single-line `Text(text, fontSize)`
- Long param list → multi-line with indentation

## Padding
- Uniform → `.padding(N.dp)`
- Symmetric → `.padding(vertical = V.dp, horizontal = H.dp)`
- Per-side → `.padding(start = L.dp, top = T.dp, end = R.dp, bottom = B.dp)`
- Variable ref → `.padding(/* var(--name) */ 0.dp)` placeholder

## Dimensions
- Both → `.size(width = N.dp, height = N.dp)`
- Width only → `.width(N.dp)`
- Height only → `.height(N.dp)`

## Image Handling
- Network URL → `AsyncImage(model = "url", contentDescription = "name", modifier, contentScale)`
- Local resource → `Image(painter = painterResource(id = R.drawable.name), contentDescription, modifier, contentScale)`
- Data URI → decode at runtime with `BitmapFactory.decodeByteArray()` → `Image(bitmap = bitmap.asImageBitmap())`
- `objectFit: "fit"` → `contentScale = ContentScale.Fit`
- `objectFit: "crop"` → `contentScale = ContentScale.Crop`
- `objectFit: "fill"` → `contentScale = ContentScale.FillBounds`
- Corner radius on images → `.clip(RoundedCornerShape(N.dp))`

## Ellipse
- Ellipse node → `Box(modifier = Modifier.size(...).clip(CircleShape).background(color))`

## Icon Handling
- Icon font nodes → `Icon(LucideIcons.IconName, contentDescription = "name", modifier = Modifier.size(N.dp), tint = Color(...))`
- Icon name: kebab-case converted to PascalCase

## Line Nodes
- Line → `Divider(color = Color(...), thickness = N.dp, modifier = Modifier.width(W.dp))`

## Path & Polygon
- Path nodes → `Canvas(modifier) { drawPath(PathParser().parsePathString(data).toPath(), color) }`
- Polygon nodes → `Canvas(modifier) { ... }` with polygon path calculation using trigonometry

## Responsive Design
- Use `BoxWithConstraints { ... }` for parent-relative sizing
- `maxWidth` and `maxHeight` constraints available
- Use `Modifier.fillMaxWidth()` / `.fillMaxHeight()` for full-size containers
