---
name: shader-fill
description: Native SkSL shader fills — generative noise / aurora / glow backgrounds via the shader fill type
phase: [generation]
trigger:
  keywords: [shader, glsl, sksl, generative, noise, aurora]
priority: 28
budget: 700
category: knowledge
---

SHADER FILL (advanced — render-only):

A node fill can be a native SkSL shader. Use it only for genuinely
procedural surfaces (generative noise, aurora, animated-looking glow)
where `linear_gradient` / `radial_gradient` / `mesh_gradient` can't get
the look. For ordinary multi-hue panels, PREFER `mesh_gradient` — it is
simpler, safer, and renders everywhere. WHEN UNSURE, fall back to
`linear_gradient` or `mesh_gradient`. A shader that fails to compile
degrades to a flat solid (first colour uniform, else gray), so a bad
shader silently looks worse than a gradient.

SHAPE (one fill entry):

```
fill: [{ type: "shader", sksl: "<SkSL source>", uniforms: { name: value, ... } }]
```

- `sksl` — RAW SkSL (Skia's GLSL dialect). The entrypoint MUST be the
  exact signature `half4 main(float2 fragCoord)` and return a `half4`
  RGBA colour. `fragCoord` is the pixel position inside the node box.
- `uniforms` — OPTIONAL map of named uniforms. A shader may take none.
  Supported value types:
  - number → `float` uniform (declare `uniform float name;`)
  - number array `[a,b]` / `[a,b,c]` / `[a,b,c,d]` → `vec2`/`vec3`/`vec4`
  - hex string `"#rrggbb"` → `vec4` colour (premultiplied RGBA); declare
    it `uniform half4 name;`. The first colour uniform also doubles as
    the visible fallback if compilation fails — so always include one.
- Optional `opacity` (0..1) folds into the fill alpha.

KNOWN-GOOD SNIPPETS (copy verbatim, tweak colours via uniforms):

1) Vertical two-colour fade (top → bottom):

```
fill: [{ type: "shader",
  sksl: "uniform half4 top; uniform half4 bottom; uniform float2 size; half4 main(float2 p){ float t = clamp(p.y/size.y, 0.0, 1.0); return mix(top, bottom, t); }",
  uniforms: { top: "#1e1b4b", bottom: "#0b0614", size: [400, 600] } }]
```

2) Radial glow (bright centre → dark edges):

```
fill: [{ type: "shader",
  sksl: "uniform half4 core; uniform half4 edge; uniform float2 size; half4 main(float2 p){ float2 c = p/size - 0.5; float d = clamp(length(c)*1.6, 0.0, 1.0); return mix(core, edge, d); }",
  uniforms: { core: "#7c3aed", edge: "#0b0614", size: [600, 600] } }]
```

3) Subtle value noise (gentle film grain over a base colour):

```
fill: [{ type: "shader",
  sksl: "uniform half4 base; uniform float2 size; float hash(float2 p){ return fract(sin(dot(p, float2(127.1, 311.7))) * 43758.5453); } half4 main(float2 p){ float n = (hash(floor(p)) - 0.5) * 0.06; return half4(base.rgb + n, base.a); }",
  uniforms: { base: "#111827", size: [800, 600] } }]
```

RULES:

- Keep `main`'s signature EXACTLY `half4 main(float2 fragCoord)` (any
  param name is fine). Any other signature fails to compile.
- Pass the node's pixel size as a `float2 size` uniform when you need to
  normalise `fragCoord` (SkSL has no built-in resolution).
- No external textures / images in v1 — colours + math only.
- Do NOT hand-author exotic shaders for routine UI. Reach for a shader
  only on explicit "generative / noise / aurora / shader" intent;
  otherwise `mesh_gradient` or a gradient is the right tool.
