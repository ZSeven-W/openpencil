# Native motion graphics

Open [examples/3d-motion.op](examples/3d-motion.op) in the desktop editor and
press `M` to open the timeline. Play, pause and seek sample the camera and
effects at the playhead. Export as MP4 to render the same sampled motion.

Three shader-fill presets are available through `.op` files and `batch_design`:

| Preset | Controls |
| --- | --- |
| `candlesticks_3d` | OHLC bars, perspective camera, per-face lighting, neon edges, wicks and trails |
| `spark_burst` | Start time, lifetime, origin, speed, gravity and tint |
| `iridescent_glass` | Sphere position, radius, intensity, rainbow bands and specular reflection |

For continuous camera motion set `pathSmoothness:1`. Fractional candle indices
progressively reveal the next bar; they do not snap the camera to whole bars.
Candlestick lighting follows physical face edges, with smoothed silhouettes
and upper/lower wicks drawn outside the body.
Set `stage:[95,1.45,1.35,1]` for a perspective grid, brighter bevels and
trails that land on the floor. The chart and stage share the same camera.

These are procedural surfaces, with camera controls for the candlestick scene.
They do not import mesh models or provide a general 3D scene editor. Text and
event overlays can remain ordinary editable nodes. Presets are expanded by
the shared loader; the saved document retains its parameters, not generated
shader source. See the parameter examples in
[motion-graphics.md](../crates/op-ai-skills/skills/knowledge/motion-graphics.md).

Native runtime uniforms now support flattened float arrays and matrices, using
Skia's reflected offsets. `size` is supplied automatically when the program
declares `uniform float2 size;`. An open timeline supplies `time` from the
playhead; MP4 supplies the exact frame time. Static images sample time zero.
The motion presets also accept `sampleTime` for a fixed poster frame.
Charts with a camera or view path and bursts scheduled after zero default
to the timeline clock, so they stay still when opening the file. Set
`motionClock:1` to opt other motion presets into that behavior, or
`motionClock:0` to run a preset from the live canvas clock.

MP4 supports finite clips up to ten minutes. Resolution and shader complexity
affect export time; software rendering can be slow for complex scenes. Audio
tracks can be imported, generated, mixed and embedded using the desktop
timeline or MCP. See [op-audio](../crates/op-audio/README.md) for natural voice
setup and supported playback/export paths.

Validation uses real native Skia compilation and pixel comparisons for motion,
array offsets, deterministic sampling and node-local coordinates. The loader
rejects malformed, oversized or unordered chart/camera arrays. The example is
self-contained and needs no downloaded visual assets.
