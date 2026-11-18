---
name: motion
description: Editable keyframes, named interaction clips, and screen transitions
phase: [generation, maintenance]
trigger:
  keywords: [motion, animation, animate, animated, transition, keyframes, 动效, 动画, 转场, 关键帧]
priority: 29
budget: 650
category: domain
---

Use editable node animations for requested motion. Keep each node's authored
layout and visible end state. When available, use `apply_motion_preset` for
standard effects, inspect timing with `get_motion`, and sample the start,
midpoint, and end of the clip with `get_motion_frames`. For a subtle
entrance use a 200–500 ms `mount` or `inView` clip with opacity and a small
translation. The latter starts when
the node enters the viewport. A 60–100 ms stagger separates repeated items.
Keep each screen under 32 motion tracks. Use `iterations:"infinite"` only for
an intentional ambient loop, and respect reduced motion.

For a user-triggered sequence, set `trigger:"manual"` and the same nonempty
`id` on every participating layer. Use `delayMs` on each clip to stagger it.
The trigger targets the authored id of the container or layer and runs
matching manual clips throughout that subtree.

```json
{"onTap":[{"play_motion":{"target":"card-stack","animation":"reveal","mode":"restart"}}]}
```

The playback modes are `play`, `restart`, `reverse`, and `stop`. Preserve
the resolved node ids when applying the action with `batch_design`.

A navigation transition can specify `kind`, `durationMs` and `easing`.
Kinds are `slideLeft`, `slideRight`, `slideUp`, `slideDown`, `fade` and
`none`. Start route paths as quoted string expressions.

```json
{"onTap":[{"push":{"path":"'/details'","transition":{"kind":"slideUp","durationMs":300,"easing":"ease_out"}}}]}
```

`replace` accepts the same body. `{"pop":null}` reuses the entering
transition. Use `{"pop":{"transition":{"kind":"none"}}}` for an immediate return.

For neon chart films, use shader fill `preset:"candlesticks_3d"` with OHLC
`bars`, a timed `viewPath` (progress, spacing, scale, cursor), and optional
`cameraPath` (yaw, pitch, focal length, glow). Use `spark_burst` for moving
particles and `iridescent_glass` for translucent rainbow spheres. These are
procedural surfaces with sampled camera paths; keep labels as editable text.
See `motion-graphics` for parameters. They follow the timeline playhead and MP4
frame clock. Add a finite root animation to establish the clip duration.
Audio can be imported or generated with `generate_voiceover` using
`provider:"neural"` for online natural speech, then mixed into MP4.
