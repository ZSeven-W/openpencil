---
name: motion-graphics
description: Perspective candlestick scenes, animated camera paths, particles and iridescent glass
phase: [generation, maintenance]
trigger:
  keywords: [candlestick, candlesticks_3d, spark_burst, iridescent_glass, motion graphics, 3d, 粒子, 三维, 镜头, 特效, 虹彩]
priority: 30
budget: 650
category: domain
---

Use these presets for cinematic charts, particles and rainbow glass surfaces.
They work in native Skia and CanvasKit:

```
fill: [{ type: "shader", preset: "candlesticks_3d", uniforms: {
  bars: [100, 110, 95, 108, 108, 115, 104, 106],
  viewPath: [0, 0, 14, 2, 500, 900, 3, 1, 12, 2, 500, 800],
  cameraPath: [0, 0.2, 0.1, 1250, 1, 3, 0.5, 0.3, 1000, 1.5]
} }]
```

`bars` contains 1–256 `[open,high,low,close]` rows flattened into one array.
`viewPath` rows are `[seconds,index,spacing,priceScale,cursorX,cursorY]`.
`cameraPath` rows are `[seconds,yawRadians,pitchRadians,focalPx,glow]`.
Both paths hold their endpoints and accept 1–256 rows
with strictly increasing nonnegative times. Coordinates use a 720×1280
scene fitted to the node. Camera angles are limited to ±1.2 radians.
Set `pathSmoothness:1` for bounded cubic interpolation with continuous
velocity; omit it for linear paths. Fractional indices progressively form
the next candle and move the camera continuously. `edgeSoftness` controls
subpixel silhouette smoothing (default 0.8 document pixels).
Static alternatives: `view:[index,spacing,priceScale,cursorX]`,
`lens:[cursorY,yaw,pitch,glow]`, `focal:1250`, `baseline:100`.

`spark_burst` uses `origin:[x,y]` (normalized),
`burst:[startSeconds,lifetimeSeconds,speedPxPerSecond,gravity]`, and
`tint:[r,g,b]`. It emits 40 deterministic moving streaks on transparency.
`iridescent_glass` uses `sphere:[centerX,centerY,radius,intensity]`;
the center is normalized and radius is a fraction of the smaller node side.
It paints a translucent sphere with Fresnel edges, rainbow bands and a
specular highlight. Both effects need their own rectangle layer.
All three accept `sampleTime` to freeze a poster at a specific second;
omit it for animation. Keep text and event cards as ordinary editable nodes.
These presets render procedural surfaces, not imported 3D meshes. Shader
render cost increases with resolution, especially in software MP4 export.

