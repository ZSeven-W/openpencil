# OpenPencilPlayer (Android)

A thin Android host for the OpenPencil engine, consuming `crates/op-engine-jni`'s
`tech.zseven.openpencil.OpNative` C-ABI surface. The engine renders through
EGL/GLES onto a `SurfaceView`; the shell owns lifecycle, insets, touch, and
the frame pump. Gestures are interpreted by the engine: single-finger tap
selects the topmost node under the finger, single-finger drag pans, two-finger
pinch zooms around the pinch midpoint. The engine paints the document's
active page with the exact painter the desktop editor canvas uses.

## Build + install

Requires the Android SDK + NDK and `cargo-ndk` (`cargo install cargo-ndk`).
Uses the Gradle wrapper (Gradle 8.14.3); point Gradle at a JDK 17+ (Android
Studio's bundled JBR works):

```bash
export JAVA_HOME="/Applications/Android Studio.app/Contents/jbr/Contents/Home"
export ANDROID_NDK_HOME="$HOME/Library/Android/sdk/ndk/<version>"

# rquickjs-sys (QuickJS behind op-mcp's `script` feature, pulled in by the
# editor's design pipeline) generates its FFI bindings at build time on
# Android (no pregenerated bindings for these triples). cargo-ndk exports
# CC/CFLAGS, but bindgen's own libclang reads neither — without these it
# parses the QuickJS headers against the HOST sysroot and fails with
# "stdio.h not found". Adjust the prebuilt dir for a Linux host.
NDK_SYSROOT="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/sysroot"
export BINDGEN_EXTRA_CLANG_ARGS_aarch64_linux_android="--target=aarch64-linux-android26 --sysroot=$NDK_SYSROOT"
export BINDGEN_EXTRA_CLANG_ARGS_x86_64_linux_android="--target=x86_64-linux-android26 --sysroot=$NDK_SYSROOT"

# Build the cdylib into the Debug-only jniLibs source set (arm64-v8a + x86_64),
# then install. Release has a separate empty source set by default:
# `editor` forwards to op-engine-ffi/editor (full desktop chrome).
cargo ndk -t arm64-v8a -t x86_64 -o packaging/android/app/src/debug/jniLibs \
  build -p op-engine-jni --features gl,editor
cd packaging/android && ./gradlew installDebug && cd -
```

For local embedded-login work, link an external ABI-v2 or ABI-v3 archive into
the Debug `.so` through the explicit development feature:

```bash
scripts/build-mobile-auth-dev.sh \
  --platform android-arm64 \
  --archive /absolute/path/to/libop_auth.a \
  --abi 3

cd packaging/android && ./gradlew installDebug && cd -
```

The bridge accepts that override only in Cargo's `debug` profile. The archive
is linked into `libop_engine_jni.so`; it is never packaged as an Android asset
or standalone `.a`. Both the archive and generated `.so` stay ignored by Git.
The Gradle Release variant reads only `app/src/release/jniLibs`, so a local
Debug auth library cannot leak into Release. Production consumes the adopted
ten-target ABI-v3 matrix under `crates/op-auth-bridge/prebuilt/`. Its signatures
and source-owned `AUTH-RELEASE-POLICY` pin the exact matrix and private build
identity; the matrix's signed version and public revision are provenance, not
requirements that every consuming OpenPencil commit must match.

Private release CI rebuilds, audits, and signs all ten immutable ABI-v3
candidates together: both Android targets, both iOS targets, and all six
desktop targets. The candidate artifacts are never production link inputs. An
authenticated Android Release first verifies the adopted policy and complete
signed matrix, then builds both JNI ABIs in Cargo's release profile into
`app/src/release/jniLibs` and proves Cargo actually selected signed ABI 3. Only
the resulting `libop_engine_jni.so` files enter the APK; the `.a` archives are
never packaged as assets. Any policy, matrix, provenance, ABI, hardening,
signature, or digest failure stops the release instead of falling back to
unsigned code. OpenPencil-only commits and version bumps reuse the adopted
matrix; private implementation or ABI changes require a new signed matrix and
reviewed policy adoption.

## Production GitHub release

The canonical `Rust release artifacts` workflow calls the reusable
`.github/workflows/android-release.yml` lane for stable `vX.Y.Z` tags. Android
does not have a separate dispatch trigger and CI does not upload to Google Play.
The lane publishes these formally signed files into the same GitHub Release as
the desktop artifacts:

- `OpenPencil-<version>-android.apk` for direct installation and testing;
- `OpenPencil-<version>-android.aab` for a later manual Play Console upload;
- `SHA256SUMS.android.txt`, which is also covered by the release's unified
  checksums and provenance.

The unsigned build runner receives only the two collaboration bootstrap
secrets. It verifies the complete signed Auth matrix, builds both Android JNI
ABIs, and hands off an unsigned APK/AAB pair by immutable Actions artifact ID.
It never receives the Android keystore. A fresh runner in the protected
`release-production` environment downloads that exact handoff and receives the
signing values only for one sign-and-verify step. That runner does not invoke
Cargo or Gradle.

Configure these `release-production` environment secrets:

- `ANDROID_RELEASE_KEYSTORE_BASE64` — the complete stable JKS or PKCS#12
  keystore encoded as one-line base64;
- `ANDROID_RELEASE_KEYSTORE_PASSWORD`;
- `ANDROID_RELEASE_KEY_ALIAS`;
- `ANDROID_RELEASE_KEY_PASSWORD`.

Also configure the non-secret environment variable
`ANDROID_RELEASE_CERT_SHA256` as exactly 64 lowercase hexadecimal characters.
It is the SHA-256 digest of the signing certificate's DER bytes. The workflow
checks the keystore certificate, APK signer, and AAB signer against this value,
so replacing the release key cannot silently create an update-incompatible
release. Keep this keystore stable and backed up; the runtime Android Keystore
used for user credentials and the macOS/iOS login keychain are unrelated.

The build installs Android SDK Platform 36 revision 2, Build-Tools 36.0.0, and
NDK r28c (28.2.13676358) from repository-owned SHA-256 pins instead of
`sdkmanager`. Gradle 8.14.3, Android Gradle Plugin 8.13.2, Maven artifacts,
Android Skia archives, bundletool 1.18.3, and the signing JDK are pinned too.
Release checks require target API 36, 16 KB ELF LOAD alignment, APK zip
alignment, and bundletool's `PAGE_ALIGNMENT_16K` declaration.

## Versioning

Gradle resolves `versionName` during configuration by running the repository's
`scripts/android-version.sh` against the root `Cargo.toml`; Android therefore
uses the same canonical workspace version as the Rust crates. The corresponding
`versionCode` is `major * 1,000,000 + minor * 1,000 + patch`. Minor and patch
are each limited to three digits, so every supported SemVer increase is also a
strictly increasing Android version code; on the current `0.x` line this
reduces to `minor * 1,000 + patch`. Android builds reject pre-release/build
suffixes, zero, component overflow, and values above Android's version-code
ceiling instead of publishing colliding codes.

Run `scripts/sync-version.sh` after changing the workspace version. The
repository guard and `scripts/android-version.test.sh` verify the Gradle wiring
and the monotonic mapping. To inspect the resolved values directly:

```bash
cd packaging/android
./gradlew -q :app:printOpenPencilVersion
```

## Run

```bash
adb shell am start -n tech.zseven.openpencil/.MainActivity
adb logcat -s OpenPencilPlayer:V OpJni:V AndroidRuntime:E libEGL:W
```

## Background generation

User-started AI generation starts a same-process `dataSync` foreground service
while the editor Activity is still visible. After Android removes the render
surface, a single non-main worker advances the render-free engine tick and an
ongoing notification returns to the editor. The worker stops at completion and
posts a result notification when completion happened in the background.

Lock-screen work uses a bounded partial wake lock only while the render surface
is suspended. The lock is renewed in short segments while work remains active
and is released on resume, completion, failure, service timeout, and teardown.
Android 13+ may hide drawer notifications when the user denies notification
permission, although the foreground service remains visible in Task Manager.

This keeps ordinary Home/app-switcher and lock-screen transitions running, but
it cannot override Android or vendor power policy. A force-stop, process death,
device shutdown, OEM task killer, or the Android 15+ `dataSync` quota can still
end background execution; OpenPencil never claims guaranteed execution across
those terminal OS events.

The full editor starts with a new untitled `.op` document instead of loading
demo content. Pass an asset name without the `.op` suffix in the existing
`doc` intent extra to load a bundled document. For example, load the six-slide,
16:9 PowerPoint demo at `assets/ppt-demo.op` with:

```bash
adb shell am start -n tech.zseven.openpencil/.MainActivity --es doc ppt-demo
```

The demo is derived from
`crates/op-editor-core/assets/scene_templates/slide-deck.op` and pinned to the
`corporate-blue-light` style guide. Use `--es doc sample` to load the bundled
sample instead. A viewer-only launch (`--ez editor false`) still falls back to
`ppt-demo.op` when no `doc` override is supplied.

In editor mode, the engine-painted **Export** action renders PNG, JPEG, SVG,
or PDF into an app-private staging file (Rust writes it directly, so large
payloads never cross JNI as a byte array) and then presents the system
create-document UI; the chosen destination receives a plain copy and the
staging directory is removed on every terminal path. WebP is hidden on mobile
because the pinned Skia archive does not include its encoder.
The desktop-class **Code** panel reuses that action and staging flow for
framework source files and generated/AI bundle ZIPs.

Sign-in is platform-native: the engine's device-login flow hands its
verification URL to a programmatic-View overlay that performs email/password
sign-in against the regional SSO JSON API and approves the pairing directly,
while third-party providers, registration, and password recovery open the
regional web pages in the system browser — the running pairing is approved
there instead. The Account tile opens a native account center backed by the
engine's read-only profile snapshot. The third-party buttons are
region-accurate: the login overlay fetches
`GET /api/v1/auth/providers?channel=web_mobile` from the pairing's origin, so
the mainland site lists WeChat / Alipay / Douyin and the global site lists
Apple / GitHub / Google without a hardcoded table. The sign-in region (Mainland China
`sso.zseven.cn` / Global `sso.zseven.tech`) resolves from a persisted user
choice, then an IP-informed probe of the global gateway's mainland redirect,
then the device locale; changes apply on the next launch because the auth
runtime initializes once per process.

## What the shell does / does not own

- `OpSurfaceView` — create/attach/resume/suspend/resize on
  `SurfaceHolder` events, Choreographer frame pump driven by the
  `onNeedsRedraw` upcall, one suspend→resume GPU-error recovery per
  surface generation, touch forwarding in logical px (top-left origin),
  inset replay after create/attach/resize.
- `MainActivity` — transparent edge-to-edge window with a continuous dark
  backdrop; four-sided system-bar/cutout insets move only interactive editor
  chrome while IME occlusion remains a separate logical-pixel channel. This
  covers portrait/landscape cutouts plus gesture and three-button navigation
  without padding or resizing the `SurfaceView`; teardown releases the View's
  process-owned engine lease, while active generation or an unread background
  result keeps that engine alive for bounded foreground recovery.
- `OpNative` / `OpCallbacks` — the JNI surface contract with
  `crates/op-engine-jni` (engine-thread upcalls, blocking barriers for
  lifecycle calls).

Everything else — document loading/layout, painting, gesture
interpretation, fit-to-view, selection — lives in the engine
(`crates/op-engine-ffi`).
