# Voiceover and audio tracks

The desktop motion timeline offers **Import audio**, **System voiceover** and
**Natural voice (online)**. Select the text layers to narrate. A worker
generates the audio while the editor stays available. System voiceover on macOS
selects Tingting for Chinese text; Windows uses installed system speech;
Linux uses `espeak-ng`.

The natural voice option uses Microsoft Edge's online speech service and sends
the selected text when invoked. It requires the `edge-tts` Python package
(tested with 7.3.1). Set `OPENPENCIL_TTS_PYTHON` to the absolute Python
executable for a virtual environment, or install the package for the Python on
the desktop launch path. Chinese text defaults to Yunxi. For MCP calls, choose
`provider: "neural"`; the default provider remains `system`.

Tracks live in `ContainerProps.audioTracks`, embedded as audio data URIs, so
save/reopen, copying a board and undo/redo retain the recording. Each track has
an id, name, source duration, timeline start, input trim, gain, mute state and
mount/in-view/manual trigger. Generated tracks also retain their narration text,
voice and rate. There are no machine-specific file paths in the saved track.

Desktop timeline playback prepares a PCM mix off-thread, seeks from the current
playhead and stops on pause, scrubbing, loop restart, scope or document change.
Audio preparation is scoped to the document identity and soundtrack contents.
macOS playback uses `afplay`, Windows uses system sound playback, and Linux uses
`ffplay`. Standalone prototype Play and HTML export do not yet play these tracks.

MP4 export includes matching, unmuted tracks and retains the native video frames.
The shared native export path muxes AAC after video encoding. Browser MP4 exports
with embedded audio use the daemon route; silent browser clips still encode
locally. GIF/APNG/PNG sequences remain picture-only formats.

`ffmpeg` and `ffprobe` must be installed for audio validation, preview preparation
and MP4 mixing. Homebrew locations are discovered for Finder launches on macOS.
Media processes have deadlines, run without a shell, and use temporary private
files with explicit media formats and file/pipe-only input protocols. A failed
audio mix fails the export instead of returning a silent movie.

MCP tools: `list_audio_tracks`, `import_audio`, `generate_voiceover`,
`update_audio_track`, `remove_audio_track`. Timing, trim, gain and mute can be
edited through `update_audio_track`; replacing a recording preserves existing
timing, name and gain unless an override is provided. File import and speech
generation are unavailable to shared deployments and require write scope.

Validation:

```sh
cargo test -p op-editor-core audio_tracks --lib
cargo test -p op-audio --lib -- --include-ignored
cargo test -p op-editor-ui motion_timeline --lib
```

The ignored media integration check needs FFmpeg. It verifies that a WAV imports
without path references, that an MP4 receives an audio stream, and that the
authored delay remains silent before audible speech begins.
