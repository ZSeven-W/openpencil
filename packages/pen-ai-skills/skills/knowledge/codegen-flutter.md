---
name: codegen-flutter
description: Flutter/Dart code generation rules — widget tree with BoxDecoration and EdgeInsets
phase: [generation]
trigger:
  flags: [isCodeGen]
priority: 20
budget: 2000
category: knowledge
---

# Flutter (Dart) 代码生成

生成使用 Material Design widgets 的 Flutter widget trees。

## 输出格式

- Dart file (`.dart`)
- 包含 `build()` method 并返回 widget tree 的 `StatelessWidget` class
- Import `package:flutter/material.dart`
- 为 path/polygon rendering 导入 `dart:math`

## layout 映射

- `layout: "vertical"` → `Column(children: [...])`
- `layout: "horizontal"` → `Row(children: [...])`
- No layout / stacked children → `Stack(children: [...])` with `Positioned()` wrappers
- `gap: N` → `SizedBox(height: N)` between children (Column) or `SizedBox(width: N)` between children (Row)
- `justifyContent: "start"` → `mainAxisAlignment: MainAxisAlignment.start`
- `justifyContent: "center"` → `mainAxisAlignment: MainAxisAlignment.center`
- `justifyContent: "end"` → `mainAxisAlignment: MainAxisAlignment.end`
- `justifyContent: "space_between"` → `mainAxisAlignment: MainAxisAlignment.spaceBetween`
- `justifyContent: "space_around"` → `mainAxisAlignment: MainAxisAlignment.spaceAround`
- `alignItems: "start"` → `crossAxisAlignment: CrossAxisAlignment.start`
- `alignItems: "center"` → `crossAxisAlignment: CrossAxisAlignment.center`
- `alignItems: "end"` → `crossAxisAlignment: CrossAxisAlignment.end`
- Always include `mainAxisSize: MainAxisSize.min` on Column/Row

## container 与 decoration

- Container nodes → `Container()` widget with named parameters
- `width: N` → `width: N`
- `height: N` → `height: N`
- `clipContent: true` → `clipBehavior: Clip.hardEdge`
- Styling via `decoration: BoxDecoration(...)` parameter

## color 与 fill 映射

- Solid fill `#RRGGBB` → `Color(0xFFRRGGBB)` (prefix FF for full alpha)
- 8-digit hex `#RRGGBBAA` → `Color(0xAARRGGBB)` (alpha moved to front)
- Variable ref `$name` → `Color(0x00000000) /* var(--name) */` (placeholder with comment)
- Text fill → `color: Color(0xFFhex)` in `TextStyle`
- Linear gradient → `gradient: LinearGradient(colors: [Color(...), Color(...)])`
- Radial gradient → `gradient: RadialGradient(colors: [Color(...), Color(...)])`

## border 与 stroke 映射

- `stroke.thickness + stroke.color` → `border: Border.all(color: Color(...), width: N)`
- Variable ref thickness → `/* var(--name) */ 1` placeholder

## cornerRadius

- Uniform → `borderRadius: BorderRadius.circular(N)`
- Per-corner → `borderRadius: BorderRadius.only(topLeft: Radius.circular(TL), topRight: Radius.circular(TR), bottomRight: Radius.circular(BR), bottomLeft: Radius.circular(BL))`

## effects

- Drop shadow → `boxShadow: [BoxShadow(color: Color(...), blurRadius: N, offset: Offset(X, Y))]`
- Blur → `BackdropFilter(filter: ImageFilter.blur(sigmaX: N, sigmaY: N), child: ...)`

## typography

- Text nodes → `Text('content', style: TextStyle(...))`
- `fontSize` → `fontSize: N`
- `fontWeight` → `fontWeight: FontWeight.wN00` (w100 through w900)
- `fontStyle: "italic"` → `fontStyle: FontStyle.italic`
- `fontFamily` → `fontFamily: 'Name'`
- `letterSpacing` → `letterSpacing: N`
- `lineHeight` → `height: lineHeight` (multiplier in TextStyle)
- `textAlign` → `textAlign: TextAlign.left|center|right|justify`
- `underline` → `decoration: TextDecoration.underline`
- `strikethrough` → `decoration: TextDecoration.lineThrough`
- Combined → `decoration: TextDecoration.combine([TextDecoration.underline, TextDecoration.lineThrough])`
- Fixed-size text → wrap in `SizedBox(width: N, height: N, child: Text(...))`

## padding

- Uniform → `padding: EdgeInsets.all(N)`
- Symmetric → `padding: EdgeInsets.symmetric(vertical: V, horizontal: H)`
- Per-side `[top, right, bottom, left]` → `padding: EdgeInsets.fromLTRB(left, top, right, bottom)`
- Variable ref → `EdgeInsets.all(/* var(--name) */ 0)` placeholder

## dimensions

- Fixed → `width: N, height: N` on Container
- Text sizing → wrap in `SizedBox`

## image 处理

- Network URL → `Image.network('url', width: N, height: N, fit: BoxFit.cover)`
- Asset → `Image.asset('path', width: N, height: N, fit: BoxFit.cover)`
- Data URI → `Image.memory(base64Decode('...'))`
- `objectFit: "fit"` → `BoxFit.contain`
- `objectFit: "crop"` → `BoxFit.cover`
- Corner radius on images → `ClipRRect(borderRadius: BorderRadius.circular(N), child: Image(...))`

## opacity 与 transform

- Opacity → `Opacity(opacity: N, child: widget)` wrapper
- Rotation → `Transform.rotate(angle: N * pi / 180, child: widget)` wrapper
- 作为 wrapper widgets 应用于 base widget 外层

## positioning

- Absolute children → `Positioned(left: X, top: Y, child: widget)` inside `Stack`

## ellipse

- Circle/ellipse → `Container` with `BoxDecoration(shape: BoxShape.circle)`

## icon 处理

- Icon font nodes → `Icon(LucideIcons.icon_name, size: N, color: Color(...))`
- Icon name: kebab-case converted to snake_case

## path 与 polygon

- Path nodes → `CustomPaint(size: Size(W, H), painter: _PathPainter(pathData, color))`
- Polygon nodes → `CustomPaint(size: Size(W, H), painter: _PolygonPainter(sides, color))`
- 在文件底部包含 helper `CustomPainter` classes

## 响应式设计

- Use `MediaQuery.of(context).size` for screen dimensions
- `LayoutBuilder` for parent-relative sizing
- `Flexible` and `Expanded` for proportional layouts
