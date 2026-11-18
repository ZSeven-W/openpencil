# OpenPencil node and layout reference

Adapted from the MIT-licensed OpenPencil bundled skill. The connected server
is authoritative: inspect its current schema and design guidelines before edits.

## PenNode Schema

### Common Properties

```json
{
  "type": "frame|rectangle|text|ellipse|line|polygon|path|image|icon_font|group|ref",
  "name": "Display Name",
  "role": "semantic-role",
  "x": 0, "y": 0,
  "rotation": 0, "opacity": 1, "visible": true
}
```

### Container Properties (frame, rectangle, group, ellipse)

```json
{
  "width": 400,              // number | "fill_container" | "fit_content"
  "height": 300,
  "layout": "vertical",      // "none" | "vertical" | "horizontal"
  "gap": 16,
  "padding": [16, 24],       // number | [v, h] | [top, right, bottom, left]
  "justifyContent": "center", // "start" | "center" | "end" | "space_between" | "space_around"
  "alignItems": "center",    // "start" | "center" | "end"
  "clipContent": true,
  "cornerRadius": 12,        // number | [tl, tr, br, bl]
  "fill": [{ "type": "solid", "color": "#FFFFFF" }],
  "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": "#E5E7EB" }], "align": "inside", "dashPattern": [5, 3] },
  "effects": [{ "type": "shadow", "offsetX": 0, "offsetY": 4, "blur": 12, "spread": 0, "color": "rgba(0,0,0,0.08)" }],
  "children": []
}
```

### Text

```json
{
  "type": "text",
  "content": "Hello",          // string or StyledTextSegment[]
  "fontSize": 16, "fontFamily": "Inter", "fontWeight": 600,
  "textAlign": "center",       // "left" | "center" | "right"
  "textGrowth": "fixed-width", // "auto" | "fixed-width" | "fixed-width-height"
  "lineHeight": 1.5, "letterSpacing": 0,
  "fill": [{ "type": "solid", "color": "#111111" }]
}
```

Rich text: `"content": [{ "text": "Bold ", "fontWeight": "bold" }, { "text": "normal" }]`

### Icons — Two Options

#### Option A: `icon_font` (RECOMMENDED — renders directly, no post-processing needed)

```json
{ "type": "icon_font", "name": "Lock Icon", "iconFontName": "lock",
  "width": 20, "height": 20,
  "fill": [{ "type": "solid", "color": "#6B7280" }] }
```

**Field is `iconFontName` (NOT `iconName`, NOT `icon`).** Values are lowercase kebab-case Lucide names: `mail`, `lock`, `eye`, `eye-off`, `chrome`, `apple`, `message-circle`, `x`, `arrow-right`, `search`, `heart`, `star`, `check`, `plus`, `bell`, `home`, `user`, `settings`, `chevron-right`, `download`, `globe`, `layers`, `zap`, `shield`, `play`.

Works in ALL contexts: CLI, MCP tools, or direct `.op` files — no `design:refine` required.

#### Option B: `path` (requires post-processing)

```json
{ "type": "path", "name": "HeartIcon", "width": 24, "height": 24,
  "fill": [{ "type": "solid", "color": "#111111" }] }
```

PascalCase + "Icon" suffix. Auto-resolved from Lucide set during post-processing.

> **Path icons need post-processing.** After inserting path nodes, run `op design:refine --root-id <id>` or use `op insert --post-process`. Without this, path icons won't render visually. The standalone MCP server (used by ACP agents) does NOT have hook implementations registered, so path icons will NOT resolve there — **prefer `icon_font` in MCP contexts.**

### Image

```json
{ "type": "image", "src": "https://example.com/photo.jpg", "width": 400, "height": 300,
  "objectFit": "crop", "cornerRadius": 12 }
```

For a new raster asset, use Codex's built-in `image_gen` by default, then import
its output into the image node. `imagePrompt` alone marks Generate intent;
it does not invoke the host's tool automatically. Both acquisition fields mean
Auto. Follow the [image workflow](images.md) for generation and portable imports.

Search fallback or an explicit stock-only request (resolve with scoped
`enrich_images` after structural repair):

```json
{ "type": "image", "width": 400, "height": 300,
  "imageSearchQuery": "modern office workspace natural light wide interior" }
```

`design_refine` repairs structure; it is not proof that a real image was acquired.

Image adjustments (all -100 to 100): `exposure`, `contrast`, `saturation`, `temperature`, `tint`, `highlights`, `shadows`.

### Polygon

```json
{ "type": "polygon", "polygonCount": 6, "width": 80, "height": 80, "cornerRadius": 4,
  "fill": [{ "type": "solid", "color": "#6366F1" }] }
```

### Icon Font

```json
{ "type": "icon_font", "iconFontName": "lucide:home", "width": 24, "height": 24,
  "fill": [{ "type": "solid", "color": "#111111" }] }
```

### Line

```json
{ "type": "line", "x2": 200, "y2": 0,
  "stroke": { "thickness": 1, "fill": [{ "type": "solid", "color": "#E5E7EB" }] } }
```

### Fill Types

```json
{ "type": "solid", "color": "#3B82F6" }
{ "type": "linear_gradient", "angle": 135,
  "stops": [{ "offset": 0, "color": "#6366F1" }, { "offset": 1, "color": "#8B5CF6" }] }
{ "type": "radial_gradient", "cx": 0.5, "cy": 0.5, "radius": 0.5,
  "stops": [{ "offset": 0, "color": "#FFF" }, { "offset": 1, "color": "#000" }] }
{ "type": "image", "url": "https://example.com/texture.jpg", "mode": "fill" }
```

Image fill modes: `fill`, `fit`, `crop`, `tile`, `stretch`. Image fill also supports adjustment filters (`exposure`, `contrast`, `saturation`, etc.).

### Ref Node (Component Instance)

```json
{ "type": "ref", "ref": "reusable-frame-id",
  "descendants": { "child-id": { "content": "Override text" } } }
```

References a `frame` with `reusable: true`. Override specific descendant properties via `descendants`.

### Design Variables

Reference with `$` prefix: `"color": "$primaryColor"`, `"gap": "$spacing"`.

## Semantic Roles

Roles declare intent — the engine applies smart defaults. Always prefer roles over manual styling.

| Category | Roles |
|----------|-------|
| **Layout** | `section`, `row`, `column`, `centered-content`, `divider`, `spacer` |
| **Navigation** | `navbar`, `nav-links`, `nav-link` |
| **Phone system chrome** | `status-bar` (preserve the runtime-generated subtree) |
| **Interactive** | `button`, `icon-button`, `badge`, `tag`, `pill`, `input`, `form-input`, `search-bar` |
| **Cards** | `card`, `feature-card`, `stat-card`, `pricing-card`, `image-card` |
| **Content** | `hero`, `feature-grid`, `cta-section`, `footer`, `testimonial`, `stats-section` |
| **Typography** | `heading`, `subheading`, `body-text`, `caption`, `label` |
| **Media** | `avatar`, `icon`, `phone-mockup`, `screenshot-frame` |
| **Table** | `table`, `table-row`, `table-header`, `table-cell` |
| **Form** | `form-group` |

Key defaults:
- `navbar` → height: 56-72, horizontal, space_between, center-aligned
- `button` → padding: [12, 24], cornerRadius: 8, centered
- `card` → vertical, gap: 12, cornerRadius: 12, padding: 24
- `heading` → lineHeight: 1.2, letterSpacing: -0.5
- `body-text` → fill_container, textGrowth: fixed-width, lineHeight: 1.5

### Phone Status Bar

For a new phone screen, load `get_guidelines` with `topic: "mobile"`, create the
root, then read back its runtime-generated `role: "status-bar"` before adding
content. Preserve its structure and indicator children; do not substitute
hand-drawn or emoji indicators. Missing canonical chrome requires runtime
diagnosis, not a claim of successful native setup. Visually verify one complete
set of time, cellular signal, Wi-Fi, and battery after saving and reopening.
Respect existing custom bars and an explicit no-status-bar brief; a narrow frame
alone does not establish phone-screen intent.

## Layout Rules

1. **NEVER set x/y on children inside layout containers** — engine positions them
2. **Siblings must use same width strategy** — all `fill_container` or all fixed
3. **NEVER `fill_container` inside `fit_content` parent** — circular dependency
4. Cards in horizontal row: ALL `width: "fill_container"`, `height: "fill_container"`

### Sizing Decision

| Question | Answer |
|----------|--------|
| Stretch to fill? | `"fill_container"` |
| Shrink to content? | `"fit_content"` |
| Exact size? | number (px) |

### Design Type Sizing

| Type | Width | Height |
|------|-------|--------|
| Landing page | 1200 | 0 (auto) |
| Mobile screen | 375 | 812 |
| Dashboard | 1200 | 0 (auto) |
