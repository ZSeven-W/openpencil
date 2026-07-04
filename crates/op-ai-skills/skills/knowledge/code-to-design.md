---
name: code-to-design
description: Convert an existing frontend codebase into OpenPencil designs - tokens, unified component library, screens - via idempotent MCP tools
phase: [planning]
trigger:
  keywords: [code-to-design, convert codebase, reverse engineer, import code]
priority: 20
budget: 3000
category: knowledge
---

# Converting Code to OpenPencil Designs

You are an agent working inside the user's codebase. OpenPencil will not parse
the code for you. You inspect the source, infer the design system and screens,
and emit design through idempotent MCP tools. Every tool below is safe to
re-run: identical calls should produce identical OpenPencil documents.

## The Five-step Method

1. Inventory

Scan the repo first. List token sources (`tailwind.config.*`, CSS custom
properties, theme files), exported components, and routes/screens. Call
`conversion_status` before writing anything so you can resume already converted
work instead of duplicating it.

2. Tokens first

Call `upsert_variables` once per token source file. Use
`key = "tokens:<file>"`. Colors are hex strings. Spacing, radius, shadow blur,
font sizes, and z/elevation scales are numbers or strings only when the source
uses a named semantic value. Theme variants use themed values plus `set_themes`
axes.

```json
{
  "key": "tokens:src/theme.css",
  "variables": {
    "color/primary": { "type": "color", "value": "#3366ff" },
    "space/4": { "type": "number", "value": 16 },
    "radius/card": { "type": "number", "value": 12 }
  },
  "sourcePath": "src/theme.css",
  "sourceHash": "sha256-or-git-hash"
}
```

3. Components bottom-up

Call `upsert_component` once per source component, leaf components before
composites. Use `key = "<file>#<ExportName>"`. Map flexbox to OpenPencil
auto-layout; map real controls to widget nodes (`text_input`, `select`,
`checkbox`, `switch`, `slider`, etc.) instead of drawing rectangles. Convert
enum variants into separate component masters (`Button/Primary`) or instance
overrides. Convert slots/children into clearly named placeholder frames inside
the master. Reference design tokens by variable names instead of hardcoded
values when a token exists and the target field accepts expressions. Resolve
radius tokens to numeric `cornerRadius` values because that field accepts
numbers or four-number arrays, not `$variable` strings.

```json
{
  "key": "src/components/Button.tsx#Button",
  "name": "Button/Primary",
  "node_json": {
    "type": "frame",
    "id": "button-primary",
    "name": "Button/Primary",
    "layout": "horizontal",
    "gap": 8,
    "padding": [10, 16],
    "cornerRadius": 8,
    "fill": [{ "type": "solid", "color": "$color/primary" }],
    "children": [
      { "type": "text", "id": "button-label", "content": "Button" }
    ]
  },
  "sourcePath": "src/components/Button.tsx",
  "sourceHash": "sha256-or-git-hash"
}
```

4. Screens per route

Call `upsert_screen` once per route. Use `key = "route:<path>"`. One screen is
one top-level frame. Reuse the converted component library with `ref` nodes.
Get master ids from `conversion_status` after components have been upserted.

```json
{
  "key": "route:/settings",
  "node_json": {
    "type": "frame",
    "id": "screen-settings",
    "name": "Settings",
    "layout": "vertical",
    "width": 1440,
    "height": 900,
    "children": [
      { "type": "ref", "id": "settings-save", "ref": "n42" }
    ]
  },
  "sourcePath": "src/routes/settings.tsx",
  "sourceHash": "sha256-or-git-hash"
}
```

5. Verify and converge

After each unit, call `lint_document` and fix reported issues. Use
`get_screenshot` to compare the OpenPencil result against the running app or
storybook page. The goal is structural, semantic, and token correctness, not
pixel-perfect equality.

```json
{ "kind": "component" }
```

```json
{ "nodeId": "n42" }
```

```json
{ "nodeId": "root" }
```

## Batching for Large Projects

Convert in batches by route or directory. `conversion_status` is your
checkpoint: entries include `sourcePath` and `sourceHash`. An `orphaned` entry
means the design node was deleted; re-run that unit's upsert.

## Tool Reference

### upsert_variables

Use for design tokens.

Required: `key`, `variables`.
Optional: `sourcePath`, `sourceHash`.

The `variables` object is `name -> { "type": "color|number|boolean|string",
"value": ... }`.

### upsert_component

Use for reusable component masters.

Required: `key`, `name`, `node_json`.
Optional: `sourcePath`, `sourceHash`.

The root must be a `frame`, `group`, or `rectangle`. For UI components, prefer
`frame` roots with auto-layout and semantically named children.

### upsert_screen

Use for route-level screens.

Required: `key`, `node_json`.
Optional: `sourcePath`, `sourceHash`.

The root must be a `frame`. Use `ref` nodes for converted components.

### conversion_status

Use before and after every batch.

```json
{}
```

```json
{ "kind": "screen" }
```

### lint_document

Use after every upsert. With no args it lints the active document. With
`nodeId`, it filters to that node and descendants.

```json
{}
```

```json
{ "nodeId": "n42" }
```

### get_screenshot

Use for visual verification after lint is clean.

```json
{ "nodeId": "root" }
```

## Node Schema Quick Reference

Frame:

```json
{
  "type": "frame",
  "id": "card",
  "name": "Card",
  "layout": "vertical",
  "gap": 12,
  "padding": [16, 16],
  "children": []
}
```

Text:

```json
{ "type": "text", "id": "title", "content": "Title", "fontSize": 24 }
```

Image:

```json
{ "type": "image", "id": "hero-image", "src": "https://example.com/image.png" }
```

Widget:

```json
{ "type": "text_input", "id": "email", "placeholder": "Email" }
```

Reference:

```json
{ "type": "ref", "id": "save-button", "ref": "n42" }
```

## Optional DOM Snapshot Reference

Use this only as a geometry and style hint while inspecting a running app. It
does not replace reading source code, props, route files, and token definitions.
Paste into DevTools or run with Playwright `page.evaluate`.

```js
(() => {
  const visualProps = [
    "display",
    "position",
    "flexDirection",
    "gap",
    "padding",
    "margin",
    "background",
    "backgroundColor",
    "color",
    "fontSize",
    "fontFamily",
    "fontWeight",
    "lineHeight",
    "borderRadius",
    "border",
    "boxShadow",
    "opacity",
    "overflow",
  ];
  const cssVars = {};
  const rootStyle = getComputedStyle(document.documentElement);
  for (const name of rootStyle) {
    if (name.startsWith("--")) cssVars[name] = rootStyle.getPropertyValue(name).trim();
  }
  const selectorFor = (el) => {
    if (el.id) return `#${CSS.escape(el.id)}`;
    const testId = el.getAttribute("data-testid") || el.getAttribute("data-test");
    if (testId) return `[data-testid="${CSS.escape(testId)}"]`;
    const cls = [...el.classList].slice(0, 3).map((c) => `.${CSS.escape(c)}`).join("");
    return `${el.tagName.toLowerCase()}${cls}`;
  };
  const elements = [...document.querySelectorAll("*")]
    .map((el) => {
      const rect = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      const visible =
        rect.width > 0 &&
        rect.height > 0 &&
        style.display !== "none" &&
        style.visibility !== "hidden" &&
        Number(style.opacity || 1) > 0;
      if (!visible) return null;
      const picked = {};
      for (const prop of visualProps) picked[prop] = style[prop];
      return {
        sel: selectorFor(el),
        tag: el.tagName.toLowerCase(),
        text: (el.innerText || el.textContent || "").trim().slice(0, 120),
        rect: {
          x: Math.round(rect.x),
          y: Math.round(rect.y),
          w: Math.round(rect.width),
          h: Math.round(rect.height),
        },
        style: picked,
      };
    })
    .filter(Boolean);
  const json = JSON.stringify({ url: location.href, cssVars, elements }, null, 2);
  if (typeof copy === "function") copy(json);
  console.log(json);
  return json;
})();
```
