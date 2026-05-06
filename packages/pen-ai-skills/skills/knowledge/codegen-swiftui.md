---
name: codegen-swiftui
description: SwiftUI code generation rules — declarative views with modifier chains
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# SwiftUI 代码生成

生成使用 modifier chains 的 SwiftUI views。

## 输出格式

- Swift file (`.swift`)
- `struct ViewName: View` with `var body: some View { ... }`
- Import `SwiftUI`
- 在底部包含 `#Preview { ViewName() }`
- 当存在 path/polygon nodes 时，包含 helper shapes（SVGPath、PolygonShape）

## layout 映射

- `layout: "vertical"` → `VStack(alignment:, spacing:) { ... }`
- `layout: "horizontal"` → `HStack(alignment:, spacing:) { ... }`
- No layout / stacked children → `ZStack(alignment: .topLeading) { ... }`
- `gap: N` → `spacing: N` parameter on VStack/HStack
- `alignItems` in vertical layout:
  - `"start"` → `alignment: .leading`
  - `"center"` → `alignment: .center`
  - `"end"` → `alignment: .trailing`
- `alignItems` in horizontal layout:
  - `"start"` → `alignment: .top`
  - `"center"` → `alignment: .center`
  - `"end"` → `alignment: .bottom`

## modifier chain 模式

- SwiftUI 在 views 上使用点链式 modifiers
- 顺序很重要：`.padding()` 放在 `.background()` 之前会让 padding 进入 background 内部
- Common chain: view → `.padding()` → `.frame()` → `.background()` → `.clipShape()` → `.shadow()` → `.opacity()` → `.offset()`

## container styling

- Empty container with fill + corner radius → `RoundedRectangle(cornerRadius: N).fill(color)`
- Empty container with fill, no radius → `Rectangle().fill(color)`
- Container with children → Stack + modifiers (`.padding()`, `.frame()`, `.background()`)
- `clipContent: true` → `.clipped()`

## color 与 fill 映射

- Solid fill `#RRGGBB` → `Color(red: R/255, green: G/255, blue: B/255)` (normalized 0-1 floats)
- 8-digit hex `#RRGGBBAA` → `.opacity(A/255)` modifier chained on Color
- Variable ref `$name` → `Color("var(--name)") /* variable */` placeholder
- Background fill → `.background(Color(...))` modifier
- Shape fill → `.fill(Color(...))` modifier
- Linear gradient → `LinearGradient(stops: [.init(color:, location:)], startPoint:, endPoint:)`
- Radial gradient → `RadialGradient(stops: [...], center: .center, startRadius: 0, endRadius: 100)`
- Text color → `.foregroundColor(Color(...))`
- Gradient direction angles map to UnitPoints: 0->bottom/top, 90->leading/trailing, 180->top/bottom, 270->trailing/leading

## border 与 stroke 映射

- With corner radius → `.overlay(RoundedRectangle(cornerRadius: N).stroke(color, lineWidth: N))`
- Without corner radius → `.overlay(Rectangle().stroke(color, lineWidth: N))`

## cornerRadius

- Uniform → `.clipShape(RoundedRectangle(cornerRadius: N))`
- On shapes → `RoundedRectangle(cornerRadius: N)` as the shape itself

## effects

- Drop shadow → `.shadow(color: Color(...), radius: N, x: X, y: Y)`
- Blur → `.blur(radius: N)`

## typography

- Text nodes → `Text("content")` with modifier chain
- `fontSize + fontWeight` → `.font(.system(size: N, weight: .weightName))`
- `fontSize` only → `.font(.system(size: N))`
- Font weights: `.ultraLight`, `.thin`, `.light`, `.regular`, `.medium`, `.semibold`, `.bold`, `.heavy`, `.black`
- `fontStyle: "italic"` → `.italic()`
- Text color → `.foregroundColor(Color(...))`
- `textAlign` → `.multilineTextAlignment(.leading|.center|.trailing)`
- Fixed-size text → `.frame(width: N, height: N, alignment: .leading|.trailing)`
- `letterSpacing` → `.kerning(N)`
- `lineHeight` (with fontSize) → `.lineSpacing(lineHeight * fontSize - fontSize)`
- `underline` → `.underline()`
- `strikethrough` → `.strikethrough()`

## padding

- Uniform → `.padding(N)`
- Symmetric → `.padding(.vertical, V)` + `.padding(.horizontal, H)`
- Per-side → `.padding(.top, T)` + `.padding(.trailing, R)` + `.padding(.bottom, B)` + `.padding(.leading, L)`
- Variable ref → `.padding(/* var(--name) */ 0)` placeholder

## dimensions

- Fixed → `.frame(width: N, height: N)`
- Width only → `.frame(width: N)`
- Height only → `.frame(height: N)`

## image 处理

- Local asset → `Image("name")` with `.resizable()` + `.aspectRatio(contentMode:)` + `.frame()`
- Network URL → `AsyncImage(url: URL(string: "url")) { image in image.modifiers } placeholder: { ProgressView() }`
- Data URI → decode base64 at runtime with `UIImage(data:)` → `Image(uiImage:)`
- `objectFit: "fit"` → `.aspectRatio(contentMode: .fit)`
- `objectFit: "crop"` → `.aspectRatio(contentMode: .fill)`
- Corner radius on images → `.clipShape(RoundedRectangle(cornerRadius: N))`

## opacity 与 transform

- Opacity → `.opacity(N)` modifier
- Rotation → `.rotationEffect(.degrees(N))` modifier
- Variable ref opacity → `.opacity(/* var(--name) */ 1.0)` placeholder

## positioning

- Absolute children → `.offset(x: X, y: Y)` modifier

## ellipse

- Ellipse node → `Ellipse()` with `.fill()`, `.frame()`, `.stroke()` modifiers

## icon 处理

- Icon font nodes → `Image("icon.name")` with `.resizable()` + `.frame(width: N, height: N)`
- Color → `.foregroundColor(Color(hex: "..."))`
- Icon name: kebab-case converted to dot.notation

## path 与 polygon

- Path nodes → `SVGPath("path-data").fill(color)` (include SVGPath helper shape)
- Polygon nodes → `PolygonShape(sides: N).fill(color)` (include PolygonShape helper)

## 响应式设计

- Use `GeometryReader { geometry in ... }` for parent-relative sizing
- `geometry.size.width` and `geometry.size.height` for dynamic dimensions
- Use `.frame(maxWidth: .infinity)` for full-width containers

## line nodes

- Line → `Rectangle()` with `.frame(width: N, height: 1)` + `.background(color)`
