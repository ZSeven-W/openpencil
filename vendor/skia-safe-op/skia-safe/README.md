# <img alt="" width="48" align="top"  src="https://raw.githubusercontent.com/rust-skia/rust-skia/master/artwork/rust-skia-icon_512x512.png"/> Safe Rust bindings to the [Skia Graphics Library](https://skia.org/).

This packages contains safe Rust wrappers for Skia and uses [skia-bindings](https://crates.io/crates/skia-bindings) to build and interface with the Skia C++ library.

For information about the supported build targets and how to run the examples, please visit the [github page of the rust-skia project](https://github.com/rust-skia/rust-skia).

## Documentation

Function level documentation is [not yet](https://github.com/rust-skia/rust-skia/issues/23) available. To get started, take a look at the [Rust examples](https://github.com/rust-skia/rust-skia/tree/master/skia-org/src/) or the [Skia documentation](https://skia.org).

## Bindings & Wrappers

Skia-safe wraps most parts of the public Skia C++ APIs:

- [x] Vector Geometry: Matrix, Rect, Point, Size, etc.
- [x] Most drawing related classes and functions: Surface, Canvas, Paint, Path
- [x] Effects and Shaders
- [x] Utility classes we think are useful
- [x] PDF & SVG rendering
- [ ] Skia Modules
  - [x] Text shaping with [Harfbuzz](https://www.freedesktop.org/wiki/Software/HarfBuzz/) and [ICU](http://site.icu-project.org/home).
  - [x] Text layout (skparagraph)
  - [x] Animation via [Skottie](https://skia.org/docs/user/modules/skottie/)
- [ ] GPU Backends
  - [ ] Ganesh
    - [x] Vulkan
    - [x] OpenGL
    - [x] Metal
    - [x] Direct3D
    - [ ] WebGPU [Dawn](https://dawn.googlesource.com/dawn/)
  - [ ] Graphite

Wrappers for functions that take callbacks and virtual classes are not supported right now. While we think they should be wrapped, the use cases related seem to be rather special, so we postponed that for now.

## Codecs

By default, full builds and prebuilt binaries of all platforms support the following image formats[^1]:

| Decoding                       | Encoding  |
| ------------------------------ | --------- |
| BMP, GIF, ICO, JPEG, PNG, WBMP | JPEG, PNG |

[^1]: skia-safe versions before 0.34.1 had no support for decoding GIF images.

In addition to that, support for the WEBP image format can be enabled through the features `webp-encode`, `webp-decode`, and `webp` explained below.

## Features

Skia-safe supports the following features that can be configured [via cargo](https://doc.rust-lang.org/cargo/reference/manifest.html#the-features-section):

### `gl`

Platform support for OpenGL or OpenGL ES can be enabled by adding the feature `gl`. Since version `0.25.0`, rust-skia is configured by default to enable CPU rendering only. Before that, OpenGL support was included in every feature configuration. To render the examples with OpenGL, use

```bash
(cd skia-org && cargo run --features gl [OUTPUT_DIR] --driver opengl)
```

#### `egl`, `x11`, `wayland`

These features are configure the Window manager integration. They are supported on Linux based platforms and enable the `gl` feature implicitly:

`egl` uses EGL instead of GLX to set up OpenGL contexts, a prerequisite for enabling `wayland`.

`x11` enables support for setting up X11 OpenGL contexts.

`wayland` enables support for the [Wayland display server protocol](<https://en.wikipedia.org/wiki/Wayland_(display_server_protocol)>) and implicitly enables `egl`.

### `vulkan`

Vulkan support can be enabled by adding the feature `vulkan`. To render the examples with Vulkan, use

```bash
(cd skia-org && cargo run --features vulkan [OUTPUT_DIR] --driver vulkan)
```

Note that Vulkan drivers need to be available. On Windows, they are most likely available already, on Linux [this article on linuxconfig.org](https://linuxconfig.org/install-and-test-vulkan-on-linux) might get you started, and on macOS with Metal support, [install the Vulkan SDK](https://vulkan.lunarg.com/sdk/home) for Mac and configure MoltenVK by setting the `DYLD_LIBRARY_PATH`, `VK_LAYER_PATH`, and `VK_ICD_FILENAMES` environment variables as described in `Documentation/getting_started_macos.html`.

### `metal`

Support for Metal on macOS and iOS targets can be enabled by adding the feature `metal`.

### `d3d`

The Direct3D backend can be enabled for Windows targets by adding the feature `d3d`.

### `textlayout`

The Cargo feature `textlayout` enables text shaping with Harfbuzz and ICU by providing bindings to the Skia modules skshaper and skparagraph.

The skshaper module can be accessed through `skia_safe::Shaper` and the Rust bindings for skparagraph are in the `skia_safe::textlayout` module.

### `svg`

This feature enables support for rendering SVG files (`svg::Dom`).

### `skottie`

The Cargo feature `skottie` enables support for rendering Lottie animations through Skia's Skottie module. Skottie provides an API to load, parse, and render animations exported in the [Lottie JSON format](https://lottiefiles.github.io/lottie-docs/). The bindings are available in the `skia_safe::skottie` module.

### `webp-encode`, `webp-decode`, `webp`

`webp-encode` enables support for encoding Skia bitmaps and images to the [WEBP](https://en.wikipedia.org/wiki/WebP) image format, and `webp-decode` enables support for decoding WEBP to Skia bitmaps and images. The `webp` feature can be used as a shorthand to enable the `webp-encode` and `webp-decode` features.

### `binary-cache` (enabled by default)

`binary-cache` enables download pre-built skia binaries instead of building them locally.

### `embed-icudtl` (enabled by default)

Usually when Skia is used on **Windows**, the file `icudtl.dat` must be available in your executable's directory. But if this default feature is enabled, the `icudtl.dat` file is directly embedded in Rust and is automatically initialized before any of the `textlayout` features are used.

If this feature is disabled, the `icudtl.dat` file needs to be copied from the build's output directory to the executable's directory. If your executable directory is writable, this can be done by calling the function `skia_safe::icu::init()` before the `skia_safe::textlayout` module is used.

The output directory is displayed when skia-bindings is compiled with `cargo build -vv | grep "ninja: Entering directory"`,

Simple examples of how to use the `skshaper` and `skparagraph` module bindings can be found [in the skia-org example command line application](https://github.com/rust-skia/rust-skia/blob/master/skia-org/src/).

### `embed-freetype`

On most platforms, Skia builds and runs well with the [FreeType](https://freetype.org/) version that is already installed. If you encounter FreeType related build errors on older platforms (like with for example Debian 9 "stretch"), `embed-freetype` makes sure that FreeType is built and embedded alongside with Skia.

## Multithreading

Conflicting with Rust philosophy, we've decided to fully support Skia's reference counting semantics, which means that all reference counted types can be cloned and modified from within the same thread. To send a reference counted type to another thread, its reference count must be 1, and must be wrapped with the `Sendable` type and then unwrapped in the receiving thread. The following functions support the sending mechanism:

Every mutable reference counted type implements the following two functions:

`can_send(&self) -> bool`

returns `true` if the handle can be sent to another thread right now.

`wrap_send(self) -> Result<Sendable<Self>, Self>`

wraps the handle into a `Sendable` type that implements `Send`.

And the `Sendable` type implements:

`unwrap(self)`

which unwraps the original handle.

For more information about the various wrapper types, take a look [at the rust-skia wiki](https://github.com/rust-skia/rust-skia/wiki/Wrapper-Types).
