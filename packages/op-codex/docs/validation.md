# Validation

## v0.3.13 runtime pin and publication — 2026-10-17

The plugin now pins `runtime-v0.8.5-d155cd0`, built from upstream
`d155cd08c09cdb79f4c0a2c733dc5f9376765abd`. The downloaded catalog's digest
and all six archive sizes and SHA-256 values match GitHub's published asset
metadata. Runtime publication [run 39027593112](https://github.com/ZSeven-W/codex-openpencil/actions/runs/39027593112)
passed the dependency audit, production Web/WASM gate, and all six native
build/edit/save/reopen jobs. The audit includes reviewed exceptions for the
unchanged build-only Copilot SDK ZIP dependency; it does not mean every locked
package has been upgraded.

The runtime workflow now marks future runtime-only releases as `latest=false`.
The user-facing plugin release owns GitHub Latest. English and Chinese opening
instructions describe the verified plus-menu → current-chat → in-app-browser
handoff and its copy fallback. Package checks and all 87 plugin tests passed.
Fresh public installation against this exact pin passed on macOS ARM64/x64,
Windows ARM64/x64 and Linux ARM64/x64 in
[run 39046299744](https://github.com/ZSeven-W/codex-openpencil/actions/runs/39046299744).
Each job used empty plugin/runtime state and public Codex CLI 0.162.1, verified
native panel/edit/save/independent reopen, exercised the installed plugin and
terminal fallback, and repeated setup. POSIX jobs also exercised a real PTY.
All six Node.js 22/24 package jobs passed in
[run 39046294372](https://github.com/ZSeven-W/codex-openpencil/actions/runs/39046294372).
These installation checks used commit `6242af71`; later release-documentation
changes do not alter the tested plugin or runtime payload. Bare-OS provisioning
and other hosts' MCP Apps presentation are separate from this runner coverage.
A local empty-runtime-state setup on macOS ARM64 with Node.js 26.7 also downloaded
the exact published package, verified its manifest and passed native panel,
edit/save/independent reopen. Initial download attempts hit a TLS interruption;
a later normal setup through the existing system proxy succeeded without a
code or security-setting change.

## In-app browser handoff — 2026-10-17

The user confirmed that v0.3.12 opened Chrome. That default-browser behavior
was not the intended Codex in-app experience and is superseded by v0.3.13.
The OS browser launcher has been removed. App-only `canvas_prepare_browser`
returns the existing managed canvas URL and a two-minute ready receipt without
opening anything. Where the host advertises text messaging, an explicit click
sends one standard `ui/message` to the current chat, requesting its built-in
browser interface. Otherwise the launcher copies an opening instruction to send
in that chat. Message acknowledgments are never treated as editor connections;
uncertain messages are not automatically resent. Failed and unsupported paths
never switch to an external browser.

Package validation and all 87 tests passed, including current-chat capability
checks, no external-browser action, duplicate-message prevention, foreign URL
rejection, clipboard denial and expired/mismatched receipts. Setup verified
native edit/save/independent reopen and installed v0.3.13; Codex's actual plugin
loader discovered 193 tools including the new app-only preparation control.
An isolated native document opened in the existing Codex in-app browser visibly
showed “画布已连接” and returned its matching ready receipt. The user's existing
blank managed canvas was also opened and visibly connected in the in-app browser,
without editing its contents. Screenshots remain under ignored
`artifacts/final-check-20261017/`.

Read-only local inspection found a Codex browser deep-link route, but Browser Use
security rejected a test navigation. That route was abandoned; no deep-link
execution or security-policy change is included in the implementation. Opening
a normal local HTTP page through the built-in browser is the supported path.
After the user restarted Codex, a real click in the refreshed v0.3.13 MCP Apps
panel delivered its `ui/message` to this chat. The exact existing canvas URL,
including theme and launch receipt, was opened through Codex's browser target.
The built-in browser visibly showed “画布已连接”, an enabled Save button and
the native editor. The originating MCP Apps panel independently showed
“画布已连接。” after receiving the matching native-ready receipt. This verifies
the real button → current chat → in-app browser → editor-ready → launcher
receipt chain. The blank document was not edited or recreated. The acceptance
screenshot is retained locally as
`artifacts/final-check-20261017/real-handoff-in-app-connected.png`.

## Browser launch button correction — 2026-10-17

After reconnection, the real MCP Apps launcher rendered, but clicking its
`ui/open-link` action returned an acknowledgment without opening a tab. Read-only
inspection of the running Codex bundle found its external-link payload schema
accepts HTTPS/Codex schemes, not the local HTTP canvas URL. An acknowledgment
therefore did not establish browser navigation.

v0.3.12 replaces that action with app-only `canvas_launch_browser`: an explicit
click opens only the current controller-owned loopback canvas in the system
browser using an argument array, without a shell. Neither URLs nor command paths
are accepted from the app. Repeated pending clicks do not launch another tab.
The wrapper polls `canvas_launch_status`; success requires a matching receipt
sent by the browser panel after its native editor is ready. Missing browsers,
request errors, timeouts and unconfirmed receipts expose a copyable URL without
retrying the launch. Clipboard denial selects the URL for manual copying.
The browser starts with the launching host's theme. Sandbox policy is unchanged.

Package checks and all 87 plugin tests passed. Native setup verified panel load,
edit, save and independent reopen. The bundled Codex loader loaded installed
v0.3.12 with 193 tools, including both app-only launch controls. A real click on
the shipped launcher in an isolated local host fixture opened a new tab in the
existing default Chrome profile; the native canvas displayed “画布已连接” and
the launcher received its ready receipt. Screenshots are retained under ignored
`artifacts/final-check-20261017/launcher-*-connected.png`. The fixture used its
own document and did not modify the user's live canvas. This verifies the local
button-to-browser chain; acceptance from the refreshed real MCP Apps panel still
requires reconnecting that chat's plugin, and is not claimed by installation.

## Codex local-frame restriction and browser entrypoint — 2026-10-17

The real Codex MCP Apps entrypoint opened, but its nested local canvas displayed
Chromium's blocked-content page. Inspection of the running desktop host's bundled
CSP normalization confirmed that HTTP frame origins, including loopback, are
filtered from the effective allowlist. Declaring `frameDomains` on our resource
therefore cannot enable this iframe in the current Codex host.

v0.3.10 adds a model-callable `canvas_open_browser` tool without UI-entrypoint
metadata. It opens the same isolated managed session and returns its panel URL.
The MCP Apps wrapper shows a browser launcher instead of navigating an iframe
when running under Codex's sandbox or when the effective host frame allowlist
excludes the local origin. The launcher uses the advertised `ui/open-link`
capability and reports rejected or uncertain delivery without claiming success.
The normal browser canvas supports chat-driven selection edits and saves; Code
Generate still requires saying “继续转代码” in chat in this mode.

All 78 plugin regression tests and package checks passed. New coverage checks
zero forbidden iframe navigations, theme propagation to the launcher URL,
host-open rejection and foreign-URL rejection. The real native canvas smoke
opened through `canvas_open_browser`, edited, saved and independently reopened
the document. HTML/React/Vue native source-return and stale-design tests passed.
The same live panel URL was opened in Codex's existing in-app browser and visibly
rendered “画布已连接”; its blank document was left unchanged. This proves browser
rendering, not updated launcher presentation or automatic MCP Apps chat handoff.

Separately, the previous v0.3.9 public pin passed fresh installation on all six
supported OS/architecture combinations with Node 22 and public Codex CLI 0.162.1
in [run 39025882661](https://github.com/ZSeven-W/codex-openpencil/actions/runs/39025882661).
Each job started from empty plugin/runtime state and exercised repeat setup.
This terminal-based verification does not establish MCP Apps presentation.
Local evidence is retained under ignored `artifacts/final-check-20261017/`.

## Publication rehearsal — 2026-10-17

Three new independent documents were exercised through the installed v0.3.9
controller and its configured public Rust/WASM runtime: a fashion shopping phone
screen (59 nodes), a studio dashboard (77), and a brand landing page (41).
Each used real native MCP operations, a scoped selection change, comparison of
an untouched neighboring region, native rendering, save, and independent-session
reopen with a deep-equal document check. The phone retained one complete canonical
status bar and three embedded existing photographs. No image generation or new
stock-search validation was performed in this rehearsal.

Native previews were inspected. Heuristic lint warnings remain for intentionally
similar container fills, spacer/style differences, and a rotated artwork shape;
this is not a zero-warning claim. One low-contrast landing-page wordmark was
darkened within its existing palette, then saved and independently reopened; the
whole document comparison confirmed that only this fill changed.

In the user's existing Chrome profile, real pointer actions selected the phone
board, opened Code, chose HTML, and clicked Generate. The request captured that
board, page, framework and version. Its deterministic baseline contained incomplete
layout, so it was used as a starting point for a separate runnable HTML
implementation, rather than treated as a finished page. Actual browser actions
verified category filtering, adding the knit product to the bag, a total of 599,
and persistence after reload. The matching source completed the same request and
visibly appeared under “Codex 已回填 · 项目见对话”. Clicking the panel's download
button produced a 255,964-byte HTML file whose SHA-256 exactly matched the primary
implementation. This closes the earlier unverified UI-download step for HTML.
Later pointer selection reached the headline container and, with a double-click,
its text node; the controller read the corresponding IDs while the completed
code request remained bound to the original board. UI Save displayed “已保存”.

The actual bundled Codex loader again discovered all 190 tools, including both
code-status and completion controls. Package checks and all 74 plugin regression
tests passed. The current running chat retained an older
tool inventory, so the installed controller's terminal path was used for this
rehearsal; it does not establish that the chat reconnected. Fresh-machine
installation, live MCP Apps automatic message delivery, other code frameworks,
and narrow physical-device acceptance were not repeated here. The existing
dependency-audit limitations below are unchanged. Evidence and generated designs
remain local under ignored `artifacts/prepublish-20261017/`.

## Code-panel handoff and source return — 2026-10-17

v0.3.9 routes the existing embedded Code panel's explicit Generate action through
its owning canvas controller. The captured framework, page, node IDs and document
version remain fixed. The stable Rust runtime now exposes read-only deterministic
`codegen_export` for its eight existing panel frameworks and CSS variables. This
provides a baseline preview without configuring a second model provider.

The MCP Apps wrapper sends one `ui/message` only when the host advertises text
messages. An acknowledgement means accepted delivery, not completed code. Missing
capability preserves the baseline and asks the user to say “继续转代码” in Codex.
Timeouts and uncertain delivery are not replayed. The model-only status/completion
tools check request identity, design version, cancellation, source extension,
UTF-8 and the 2 MiB source limit. Completion returns the primary source to the
existing preview/cache/copy/download path; other project files stay in the workspace.

Package checks, all 74 plugin regressions and skill validation passed. Rust checks
passed full workspace format/Clippy on the stable base, all 766 Web host tests,
three deterministic-export tests, the read-only MCP wire test and focused bridge/UI
regressions. Production WASM passed all seven bundle checks with zero `env.*`
imports and a 5,320,374-byte gzip payload. The bridge was also mirrored to the
v0.8.5 development branch, where both host-request tests passed; that checkout's
full gate separately fails in pre-existing `op-orchestrator` callers of absent
`op-design-lint` helpers. No unrelated working-tree changes were edited.

Upstream native CI found an exact-count catalog assertion that needed to move
from 135 to 136 after registering `codegen_export`. A test-only follow-up corrects
that assertion and isolates the starter-document test from saved developer
settings (`d92f32458`). All 1,251 host-services tests and the full workspace
format/Clippy gate then passed locally. The production runtime remains built from
`fd2fac91`; these follow-up changes affect tests only.
The separate upstream dependency audit still fails on existing `rustls`/`ruzstd`
advisories and a yanked `zip` version. This change adds one internal `op-codegen`
dependency edge and changes no locked dependency versions; it does not establish
a passing dependency audit.

The newly built native/WASM runtime passed isolated HTML, React and Vue export,
primary-source completion/readback and changed-design rejection. The full native
bridge smoke also passed create/read/layout/lint/PNG/save/reopen with 184 tools;
the managed canvas smoke passed edit/save/independent reopen on a freshly staged
runtime. In the existing Chrome profile, actual pointer actions selected the board,
opened the right Code
tab, selected HTML and clicked Generate. The controller received the same captured
board/framework/version, and a completed HTML source visibly replaced the baseline
with “Codex 已回填 · 项目见对话”. A local runnable result was opened in Codex's existing
in-app browser; clicking its button displayed “Your project is ready.” UI download
verification was interrupted by concurrent browser interaction and is not claimed.
The cloud-built macOS ARM archive was independently downloaded, checksum-verified
and passed the same HTML/React/Vue source-return and stale-design smoke locally.
Local evidence is retained under `artifacts/v0.3.9-codegen/`.

The six Node.js 22/24 plugin CI jobs passed in
[run 38925916437](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38925916437).
The matching public runtime passed its Web job, all six native build/smoke jobs
and publication in
[run 38925966825](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38925966825).
The plugin pins `runtime-v0.8.5-fd2fac9` with all six release-provided archive
sizes and checksums; their sizes match the published assets.
Normal setup then downloaded and verified that release into the current local
profile, passed native edit/save/independent reopen, installed v0.3.9 and verified
190 tools through the bundled Codex plugin loader, including
`canvas_codegen_status` and `canvas_codegen_complete`. The installed plugin tree
matches the repository package exactly. Existing running canvases need a plugin
reconnect and reopen to load the new bridge/runtime.
Live MCP Apps presentation and automatic chat delivery remain unverified; the
host bridge tests simulate the capability/acknowledgement contract. No claim of
automatic chat submission is made for the ordinary in-app browser.

## Embedded sidebar and live slide previews — 2026-10-16

v0.3.8 passes an explicit `chat=host` capability to the embedded Rust editor.
Only an embedded host can enable it; the hint is transient and leaves standalone
Agent behavior and stored settings intact. A stale Agent selection falls back to
Layers. The browser now paints visible slide thumbnails with the same scene
renderer, images and fonts as its main canvas, using the matching board ID and
aspect ratio. Edits, board removal and page switches use the current scene.

The stable editor base passed all 764 Web host tests, 67 shared sidebar tests and
66 plugin regressions. Production WASM bundle checks passed with zero `env.*`
imports and a 5,319,155-byte gzip payload. Content regressions cover distinct
portrait/landscape boards, text, deletion, clipping and page changes. The actual
canvas was inspected in the existing Chrome profile: only Layers/Slides remained,
both board previews included their content, and a card click framed its board.
A matching native/WASM package built from clean stable revision
`175844ede42a47268f753d331f0eeae32b010324` passed full workspace format/Clippy,
managed canvas edit/save/independent-reopen, and the isolated native bridge smoke
(create/read/layout/lint/PNG/save/reopen; 178 advertised tools). Its complete plugin
panel was inspected in dark mode in the existing Chrome profile: Agent was hidden,
portrait and landscape previews matched, and clicking the phone card centered it.
The earlier light-mode inspection covered the same scene-rendering change.
Local logs are retained in `artifacts/v0.3.8-sidebar/`.

Public runtime workflow `38904587152` passed its Web job and all six native
build/smoke jobs, then published `runtime-v0.8.5-175844e`. The plugin now pins the
published source revision and all six release-provided archive sizes/checksums.

## Default Codex image generation — 2026-10-16

v0.3.7 replaces v0.3.6's opt-in image policy: new raster assets use the host's
built-in `image_gen` by default. Codex announces the intended assets and possible
quota use in chat, then continues without a separate per-call permission question.
Explicit no-generation/stock-only instructions and host-required approvals take
precedence. Unavailable/failed generation or unimportable output uses scoped
stock search. Unknown generation results are inspected rather than blindly
retried, and successful outputs are reused after import/save failures.

Both README languages now show a prominent quota notice before the one-prompt
install example. The skill, image/node references, manifest and architecture
documentation agree on the new default, stock fallback and unchanged portable
import/save flow. The Chinese selection documentation also matches the existing
selection-only canvas. No canvas switch or prompt form was added.

Package checks, all 66 regression tests and skill validation passed. The required
isolated bridge smoke passed create/read/layout/lint/native PNG/save/reopen. Setup
installed an identical local v0.3.7 plugin cache and verified actual bundled Codex
host loading with 185 tools, plus native edit/save/reopen. The configured runtime
and user designs were preserved. Logs are in
`artifacts/v0.3.7-default-generation/`.

This policy/documentation update did not call image generation or repeat stock
search; it establishes no new host generation or quota-accounting result. The
public-runtime search failures recorded for v0.3.6 remain applicable evidence,
and the pinned runtime catalog is unchanged.

## Image consent and portable imports — 2026-10-16

v0.3.6 defaults new image needs to stock search. The design skill requires a
chat disclosure of quota use, intended assets and call count, with explicit
task-scoped permission before Codex's built-in generation. Missing permission,
unavailable generation or failed generation uses scoped search fallback; search
failure does not grant generation permission. This is skill guidance, not a
host-enforced quota gate or an accounting/refund implementation. The canvas UI
adds no toggle or prompt form.

The model-only `canvas_import_image` imports local raster bytes into an existing
image node, checks version/page/target, preserves geometry and unrelated nodes,
and confirms readback without replaying uncertain writes. The panel RPC allowlist
does not expose it. All 66 regression tests, package checks and skill validation
passed, including stale/wrong targets, format/size limits and denied panel access.

An isolated local development runtime imported the existing plugin icon, converted
one Generate-only hint into a scoped Search slot and resolved it through the real
stock pipeline (one target, one result). Native PNGs were inspected. Guarded save
and independent reopen preserved the embedded bytes after deleting the original
asset. The required native bridge smoke also passed create/read/layout/lint/PNG/
save/reopen. Setup installed the identical v0.3.6 cache and verified actual bundled
Codex host loading with 185 tools, including the new import control. The user's
configured runtime and live designs were preserved.

The existing cached public runtime at the catalog's pinned revision
`40902a3fa595a78b6f97e9409875cef823b0e191` was reverified against all manifest
hashes. Its image import, native rendering, guarded save and independent reopen
after original-asset deletion passed. Stock search returned one target and one
failure for each of two independent fixtures, including a simpler query; no
public-runtime stock-search success is claimed. Its reopened PNG was inspected
and showed the imported icon and the runtime's explicit missing-image fallback.
The smoke reports search failure separately from passed import/reopen checks.
This turn did not complete a new public-runtime download or change the catalog.

No image-generation call was made: tests used an existing local PNG and native
stock search. Host generation, quota accounting and real consent-dialog execution
are not established by these checks. Evidence is retained locally under
`artifacts/v0.3.6-images/`, including `native/run-gek6wD/report.json`,
`public-native/run-zQUmpn/report.json`, the native PNGs and setup/check/test logs.

## Selection-only canvas — 2026-10-16

v0.3.5 leaves instructions, progress and results in the current Codex chat. The
canvas shows only selected count/names in its existing 28 px footer; the extra
32 px selection row, guide, optional prompt form, message submission and edit
request lifecycle are removed. Selection context still synchronizes silently
to capable hosts. `canvas_selection` remains available for verified edit targets;
`canvas_prepare_edit` and `canvas_edit_status` are no longer advertised.

Package validation, all 59 remaining regression tests and the design skill's
validator passed. Coverage includes selection transitions, stale targets,
context deduplication/recovery, removed controls, save/conflict warnings and
checkpoint debouncing. The isolated native selection smoke passed single,
nested, multiple and empty selection cases, preserved an unrelated sibling and
the originally resolved target after a later selection change, and saved and
reopened the result. The selected-subtree PNG was visually inspected.

In the existing Chrome profile, actual pointer single-selection and Shift-click
multi-selection updated the footer; a separate `canvas_selection` read confirmed
both selected IDs. The real Rust/WASM panel had no prompt form or extra selection
row. An owned host fixture was inspected at 360 px in light mode, then at 720 px
after a live switch to dark; the selected targets persisted. An agent edit made
the design dirty, clicking Save restored saved status, and independent reopen
confirmed the persisted edit. These checks do not claim live embedded Codex UI
presentation or an update to an already-open panel.

The required bridge smoke passed native create/read/layout/lint/PNG/save/reopen.
Setup installed the local v0.3.5 cache and verified actual bundled Codex host
loading with 184 tools, plus native edit/save/reopen. This turn used the existing
configured local OpenPencil 0.8.5 development runtime at revision
`baf279c07ae71741358bcebacd25ee126c5df7df`; it does not establish a new public
runtime installation result. The public runtime catalog is unchanged.

Local logs, browser selection readback and the host fixture are in
`artifacts/v0.3.5-selection/`. Native selection evidence is in
`artifacts/v0.3.5-selection/native/run-wT9tSF/`; the pointer/save fixture is
`artifacts/canvas-4WTSMu/`. All fixtures were isolated from user designs.

## Editor theme alignment — 2026-10-16

v0.3.4 replaces the plugin's warm gray/olive chrome with the actual light/dark
palette from `crates/op-editor-ui/src/theme.rs` at the pinned runtime revision
`40902a3fa595a78b6f97e9409875cef823b0e191`. The toolbar, selection strip, loading
wrapper, buttons, inputs and dialogs now use the editor's neutral surfaces and
blue accent. The collapsed selection strip retains its 32 px layout.

In the existing Chrome profile, an isolated panel using the real Rust/WASM
editor was inspected in dark mode, including Save As. An owned embedded fixture
then verified light mode, the expanded optional input, its focus ring and a live
host-theme switch back to dark. No user chat message was sent. A native edit
made the fixture dirty, clicking Save restored its saved status, and a separate
Rust session reopened the persisted change. These checks do not claim an update
to an already-open Codex panel.

Package validation and all 69 regression tests passed. Setup installed the local
v0.3.4 cache and passed native edit/save/reopen plus the bundled Codex host check,
exposing 186 tools including the canvas controls. Local logs and the owned host
fixture are in `artifacts/v0.3.4-theme/`; the saved design is in
`artifacts/canvas-QD4225/`.

## Compact selection strip — 2026-10-16

v0.3.3 puts selection, chat guidance and the optional shortcut on one row.
In the existing Chrome profile, the previous empty strip measured 62.2 px;
the new collapsed strip measured 32 px at 720 px and 360 px widths. Light/dark
themes, long selected names, the shortcut and edit progress stayed on one row
without horizontal overflow. Opening the optional input expanded to 121 px.
These measurements use the actual panel markup and CSS in isolated fixtures;
they do not claim an appearance change in an already-open Codex panel.

Package validation and all 69 regression tests passed, including selection
synchronization, shortcut capability gating, progress and draft recovery.
The local fixture and logs are in `artifacts/v0.3.3-compact/`.
The local cached v0.3.3 plugin passed the bundled Codex host check (186 tools)
and the isolated native edit/save/reopen installation check.

## Desktop plugin startup — v0.3.2

v0.3.2 fixes a reproduced desktop-only startup failure. The plugin was installed
and enabled, while the desktop log repeatedly reported `No such file or directory`
for `codex_openpencil`. An isolated profile using the same bundled Codex app-server
(`0.162.0-alpha.17.2`) reproduced that error. A contained diagnostic launcher
confirmed its child PATH could not find `node`, despite a working terminal.
The separately configured native `openpencil` HTTP server is not the same server;
no namespace collision was established, and its configuration was preserved.

The executable launcher now uses the Node executable atomically registered by
setup outside the package, with version validation and local fallbacks. Setup
also checks the actual Codex plugin loader before reporting success. The same
isolated bundled host now loads the canvas controls successfully. The user's
local v0.3.2 installation passed runtime edit/save/reopen and a fresh Codex host
check, exposing 186 tools including `open_canvas`, `canvas_status`,
`canvas_selection` and `canvas_save`. All 69 Node tests and package checks passed.
The launcher regression uses a restricted PATH and a private executable path
containing spaces and an ampersand. Local logs: `artifacts/host-activation/`,
`artifacts/v0.3.2-setup.log`, `artifacts/v0.3.2-tests.log`.

The [six-platform fresh install matrix](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38886578454)
passed with the public Codex CLI, including actual host loading after setup,
installed-bridge native edit/save/reopen, terminal PNG/save flow and repeat setup.
This also verifies Windows resolves the `.cmd` launcher for the portable command.
The [Node 22/24 matrix](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38886577603)
passed all six jobs, including the restricted-PATH launcher regression. A final
host-check guard also requires the expected plugin identity, so a manually
configured server with the same name cannot be mistaken for a loaded plugin;
its negative cases and the user's actual host check passed locally.

This proves a new host connection can load the installed plugin. The current
chat's catalog still lacks those controls. Native computer-use access to the
Codex application was rejected by the tool, so no desktop UI reconnect, embedded
canvas presentation, or actual shortcut message delivery is claimed here.
The existing user design was not modified.

# Chat-first selection edits — 2026-10-16

v0.3.1 makes the current Codex chat the default edit input. The standalone panel
shows the current selection and a chat guide, with no prompt form or copy dialog.
A host advertising `message.text` exposes an optional collapsed shortcut. Its
action is “修改选中内容”. Drafts survive failed sends, capability revocation and
selection changes; uncertain deliveries direct users to check chat before retrying.

Submitted is established by the host acknowledgement. Working and completed
require a matching agent progress report. The model-only control rejects unknown
or finished requests and refuses completion without a document change and a
guarded save of the current version. The design workflow also requires inspecting
the screenshot and verifying the saved design before reporting completion. The
panel cannot report its own completion, and unrelated changes cannot establish it.

Package checks and all 67 Node tests passed locally. The isolated published macOS
ARM64 runtime passed selected-subtree edits, a later selection change without
retargeting, nested and multiple selections, empty/stale rejection, progress/save
guards, native PNG capture, guarded save and reopen. Native PNGs were inspected:
the selected card became navy with white text while its sibling stayed unchanged.
Local report: `artifacts/selection-chat-first/run-p0Aglt/report.json`.

In the existing Chrome profile, an isolated panel visibly showed the compact chat
guide without the old copy button. A real pointer click selected `Accent`; both
the panel label and `canvas_selection` resolved `canvas-card`. A native edit to
that exact node, guarded save, saved readback and independent reopen passed.
Local report: `artifacts/canvas-24dHYC/browser-report.json`. Browser screenshots
were inspected for the default layout and selection guide. Browser extension
automation subsequently timed out, so no live host shortcut/delivery result is
claimed. Embedded Codex presentation and actual `ui/message` follow-up delivery
remain unverified; the 67 tests exercise those protocol boundaries with a
deterministic harness.

# One-prompt installation — 2026-10-16

The repository is public. v0.3.0 adds a complete runtime installer, an agent-facing
`INSTALL.md`, verified setup before plugin registration, and a persistent terminal
canvas for the installation chat before newly installed MCP tools reconnect.
Managed upgrades preserve custom runtime choices and retain the previous
configuration when acquiring a new release fails.

From empty runtime and Codex configuration directories, setup downloaded the
published macOS ARM64 runtime without an override, verified it, completed the
isolated edit/save/reopen check, and installed the actual cached v0.3.0 plugin.
That installed bridge exposed 178 tools and passed a separate edit/save/reopen
smoke. Both pipe and real-PTY terminal fallbacks passed long Chinese input,
inspectable PNG preview and saved readback. Repeating setup reused the verified
runtime and registered marketplace successfully. The installer also creates a
requested, previously absent `CODEX_HOME` before invoking Codex CLI.
Local logs are retained at `artifacts/fresh-published-setup-2.log` and
`artifacts/fresh-published-exercise.log`. Package checks and all 60 Node tests
passed; all six published asset sizes and GitHub SHA-256 digests match the
bundled runtime catalog.

The [fresh public install matrix](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38785451951)
passed on all six native runners. Each starts with empty runtime and Codex state,
installs the public Codex CLI, downloads the public runtime asset, verifies a real
native edit/save/reopen, exercises the actual installed plugin bridge and terminal
PNG/save flow, and repeats setup. The four POSIX runners also pass the real-PTY
long-input regression. System prerequisites come from the runner or the explicit
Linux library step; these are empty application states, not bare OS images.
The [Node 22/24 matrix](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38785444832)
also passes package checks and all 60 tests on macOS, Windows and Linux.

Separately, an unauthenticated HTTPS clone/fetch of the public repository was
updated to the published main commit with Git's credential helper disabled.
Setup, the installed bridge and the real-PTY fallback passed from that checkout
with a new Codex profile, reusing the independently verified runtime cache.

The runtime pins the clean public OpenPencil revision
`40902a3fa595a78b6f97e9409875cef823b0e191` (0.8.5). The
[runtime build](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38778552317)
passed native edit/save/reopen smoke tests on macOS, Windows and Linux for both
ARM64 and x64. The macOS ARM64 archive was independently downloaded and its
archive hash, file manifest and native mobile behavior were checked locally.
Phone widths 375, 390 and 430 retain exactly one complete canonical status bar
through create, edit, save and reopen; a 1440px desktop has none.

A first-use single-page task app was created through the persistent terminal,
visually inspected, saved and reopened in a separate native session. It has one
root, 58 editable nodes and one complete status bar. The before/reopened PNGs
have the same SHA-256:
`9b90faad85e5b64caecc3c10743e643582821edaf10e7bd6729b7f53a700e559`.
The ignored local report is `artifacts/first-use-design-report.json`.
Layout and lint were inspected; nonblocking warnings for tonal card surfaces and
mixed heading/progress geometry were reviewed against the actual PNG.

This test exposed canonical PTY input truncating long JSON batches on macOS.
The terminal entry point now uses raw input and restores the terminal mode on
exit. The regression sends a long Chinese update through a real POSIX PTY,
checks the saved text and inspectable PNG, then closes the owned session.
The PTY test's relay uses Python 3's standard library only; production setup and
canvas remain dependency-free Node.js. Ctrl-C shutdown was also checked locally.

Actual embedded Codex host UI rendering and automatic `ui/message` follow-up
delivery remain separate from these native/terminal tests. A newly installed
plugin may need reconnect or a new chat; the first-chat terminal fallback does
not claim that the MCP tools have already loaded. This public GitHub release has
not been submitted to OpenAI's public plugin directory.

# Selection warning recovery — 2026-10-15

Selection/page changes during a poll now carry a typed read error instead of
being shown as a lost canvas connection. The prompt bar temporarily disables its
target and shows a syncing state; the next stable read restores the selection.
Failed reads also clear the host's stale selection context. Preparation warnings
clear after a valid selection read, while save failures and sync conflicts remain
visible. Error banners have an accessible dismiss button; identical background
failures do not reopen a dismissed banner. Prompts remain intact and no write is
automatically replayed.

`npm run check`, all 45 Node tests, and `git diff --check` passed. New regressions
exercise a transition followed by a stable selection on another page, legacy
untyped bridge responses, typed errors over HTTP/MCP, connection recovery, save
and sync warning retention, draft retention, and dismissal across repeated polls.
The isolated Rust selection/edit/save/reopen smoke passed again; its report is
`artifacts/selection-recovery-20261015/run-iYclf1/report.json`. This validates the
native edit flow separately from the simulated UI recovery tests. Live browser
recovery verification was not completed: native Chrome actions twice yielded to
concurrent user input. The owned test server was closed; the user's live canvas
was not reloaded.

# Selection prompt editing — 2026-10-15

The managed canvas now exposes typed selection context (`canvas_selection`) and
a prompt bar. A submitted request fixes its file/recovery path, page ID, document
version and node IDs. Read-only preparation rejects empty/changed selections and
unsynced browser snapshots. It never applies an edit itself. The design skill
directs Codex to read the captured nodes, modify their subtrees and preview them.

`npm run check` and all 39 Node tests passed. Regression coverage includes nested
and multi-selection, page changes, deleted nodes, empty/stale selection rejection,
snapshot matching, origin/source checks, duplicate message suppression, host
rejection/timeout draft retention, clipboard denial, opaque sandbox origins,
selection context deduplication/clearing, document-image caching and recovery
checkpoints under frequent selection polling.

The isolated Rust smoke passed single-card/descendant edits, a nested title,
multi-selected titles, empty/stale rejection, unchanged siblings, and save/reopen
document equality. Its report and PNGs are retained locally in
`artifacts/selection-edit-20261015/run-OX4XTw/`. The reopened PNG was inspected:
the selected card is blue with white text, the adjacent card retains its styling,
and both headings remain readable. This smoke selects through the native MCP
selection tool; browser gestures were verified separately below.

Actual browser testing used the existing authenticated Chrome profile through
native Computer Use, because its extension connection returned a fetch error.
In a separate fixture at `artifacts/canvas-eEi7hk/design.op`:

- Clicking the Headline layer displayed “已选 1 个元素 · Headline”.
- Entering a prompt enabled “复制修改请求”; clicking it flushed the browser snapshot
  and displayed successful copy feedback while preserving the draft.
- Pasting the copied text back into the fixture's input verified the exact
  `canvas-headline` ID, active page `0`, source/recovery paths and user prompt.
- The agent applied the requested headline edit through that fixture's managed
  controller. The browser visibly showed the new headline and retained the other
  content. UI Save displayed “已保存”; saved readback and a new Rust session both
  retained the edit. Only the test tab and owned test processes were closed.

Direct `ui/message` delivery is capability-gated and regression-tested against a
simulated MCP Apps host. It has not been verified as a live embedded Codex turn.
Standalone browser panels intentionally offer copy/paste; no automatic chat
submission or completed edit is claimed when a request is merely copied.
The bridge follows [OpenAI's MCP Apps UI guidance](https://developers.openai.com/plugins/build/chatgpt-ui)
and [the MCP Apps message/context contract](https://github.com/modelcontextprotocol/ext-apps/blob/main/src/spec.types.ts).

# Mobile status bar validation — 2026-10-15

The previously packaged runtime at `442c484` does not contain the mobile
normalization change `cbcfd14`. An isolated managed-canvas smoke reproduced the missing system
chrome: new phone frames had no status bar after creation, a subsequent edit, or
save and reopen. The test loads mobile guidelines and creates only the root;
it never authors replacement status-bar nodes.

The replacement full runtime was built locally from `baf279c07`, including
working-tree changes and a minimal host `--config-root` compatibility patch.
The same smoke passed with canonical `Time` and `Levels` containers, one clock,
cellular and Wi-Fi vector paths, and a complete battery subtree.

| Viewport | Previous runtime: create / edit / reopen | Replacement runtime: create / edit / reopen |
| --- | --- | --- |
| Phone 375 × 812 | 0 / 0 / 0 status bars — FAIL | 1 / 1 / 1 — PASS |
| Phone 390 × 844 | 0 / 0 / 0 status bars — FAIL | 1 / 1 / 1 — PASS |
| Phone 430 × 932 | 0 / 0 / 0 status bars — FAIL | 1 / 1 / 1 — PASS |
| Desktop 1440 × 900 | 0 / 0 / 0 status bars — PASS | 0 / 0 / 0 — PASS |

Each case used its own managed session and run-local state, then a new independent
session for reopening. Readbacks, PNGs, saved files, and per-stage checks are
retained in ignored local artifacts. The replacement report is
`artifacts/mobile-status-20261015/new-runtime/run-WTgKN2/report.json`.
`scripts/mobile-smoke.mjs` takes explicit absolute `--runtime` and `--output`
paths, never targets a live editor, and never retries a mutation.

The generic managed-canvas smoke also passed, as did package checks, all 27 Node
tests, 29 host argument tests, and the WASM gate check.

An existing tall phone design was repaired through its owning canvas controller
and saved, preserving its 97 existing nodes. Its original rendered height of 1433px
was extended by the canonical 62px status bar to an explicit 1495px height;
this avoids the mobile normalizer treating `fit_content` as an 812px viewport
and clipping the design. This saved migration is separate from the isolated
fresh-document smoke results above. A new session on the replacement runtime
reopened the saved file with an identical document snapshot and one status bar.
Repeating the same root-height update left the normalized document unchanged.
The reopened 393 × 1495 PNG matched the independent preview byte-for-byte, with
the bottom navigation retained at y=1417, height=78, and no content clipping.

This is local runtime validation, not a formal release. Newly created canvases
use the replacement runtime; already-running daemons retain their original
runtime until their sessions are reopened. No hot update of all existing
sessions is claimed.

# Theme synchronization fix — 2026-10-15

The panel previously forced `theme=light` and fixed light CSS, while the MCP App
wrapper ignored host theme context. The panel now follows browser media changes;
the MCP App passes initial and updated host themes through to the same panel.
Both routes update the Rust editor with its existing `op-bridge/theme` message,
without navigating the iframe or changing document state.

The installed Codex 26.1007.30912 code maps the app's resolved theme to IAB
`prefers-color-scheme` emulation at page attachment and on theme changes. This is
read-only installed-source evidence, not a live Codex appearance toggle.

`npm run check` and all 27 tests pass. Six theme regressions execute the shipped
UI scripts and cover dark initial load, both switch directions, host priority,
media fallback, startup handshake, origin/source checks, no iframe reload, and
preservation of dirty revision state. The pre-existing save/context tests pass.
The UI tests simulate host/browser events; live embedded visual acceptance remains
separate because Computer Use access to this Codex surface is restricted.

Protocol references: [OpenAI UI context](https://developers.openai.com/plugins/reference)
and [MCP Apps host context notifications](https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/2026-01-26/apps.mdx).

# Interactive canvas validation — 2026-10-15

Validated v0.2.0 on macOS with the complete Rust 0.8.5 runtime. The main Rust
repository and the user's existing desktop document were not modified.

## Actual interactive editing

The local panel was opened in the user's existing authenticated Chrome profile.
The complete Rust editor visibly rendered the fixture, layers, toolbar and zoom.

- A keyboard move changed the selected frame from x=80 to x=82; native MCP
  `batch_get` read that same edit from the managed daemon.
- `update_node` changed the headline through the packaged stdio connection; the
  interactive canvas immediately displayed the updated text.
- UI Save flushed its browser snapshot and displayed “已保存”. File readback
  retained both x=82 and the agent-edited headline.
- UI Save As created a different `.op` file and updated the displayed source path.
- A new independent Rust session reopened that saved file and verified both edits
  and the active page. The automatic canvas smoke also saved and reopened its file.
- A real panel screenshot and source files are retained only in ignored local
  `artifacts/canvas-*` output. They are not bundled example user documents.

This validation found and repaired a save mismatch between disk snapshots
(`editorMeta`, compact image tables) and runtime JSON. A regression checks that
normalization accepts equivalent images while still rejecting edited text,
changed active pages and changes hidden in free-form state.

## Native Codex protocol and installation

- `codex plugin add openpencil@zseven-openpencil --json` installed v0.2.0.
- Native `plugin/read` and `mcpServerStatus/list` loaded the cached plugin and
  discovered 183 tools (179 existing desktop tools plus four canvas controls).
- Native `mcpServer/resource/read` returned `text/html;profile=mcp-app` and the
  canvas tool retained both `ui.resourceUri` and its `openai/ui` thread entrypoint.
- Native `mcpServer/tool/call` opened the saved file with `open_canvas`. Subsequent
  `batch_get` and `canvas_status` read that managed document through the same client.
- This used an ephemeral app-server test context, with zero model turns and no
  user-visible new task. Only owned test processes were stopped.
- Direct managed-runtime tool discovery returned 176 tools (172 native tools in
  the configured runtime plus four controls). Codex's status inventory remained
  cached at 183 after its list-changed notification. Runtime tool availability
  follows the configured release; the UI and core edit/read/save routing were verified.

The current Computer Use tool rejects access to the Codex app. Consequently,
**native desktop embedded presentation remains unverified**. Resource registration
and successful browser interaction do not claim that additional UI check passed.
Reconnect the upgraded plugin or open a new chat to load its entrypoint.

## Checks

`npm run check` and all 21 tests passed. Coverage includes existing discovery and
transport safeguards, external-save conflict protection, non-overwriting Save As,
runtime validation, shared-daemon routing, private lifecycle tokens, ordered
no-retry controls, narrow panel origins, offline resource discovery, disk/live
snapshot equivalence and the executable MCP Apps handshake/context bridge.

Both isolated Rust smoke paths passed: existing-editor create/read/layout/lint/
PNG/save/reopen and managed-canvas native edit/save/new-session reopen. Updated
existing-editor discovery returned 183 tools and lint reported zero issues.
Cross-platform CI runs are available in [GitHub Actions](https://github.com/ZSeven-W/codex-openpencil/actions).
Clean-machine runtime acquisition and public directory publication remain separate.

---

# Initial validation — 2026-10-14

Validated locally with Codex CLI 0.162.0, Node.js 26.7.0, and the Rust
OpenPencil 0.8.5 host on macOS.

## Package and native Codex loading

- `npm run check`: identity/version, relative launch paths, skill references,
  package license, machine-path exclusion, and file-size checks passed.
- Both skills passed the Skill Creator frontmatter validator.
- `codex plugin marketplace add` accepted the repository marketplace.
- `codex plugin add openpencil@zseven-openpencil --json` installed version 0.1.0.
- `codex plugin list` reported the plugin installed and enabled.
- Native app-server `plugin/read` resolved the portable manifest, interface,
  onboarding skill, both namespaced skills, and bundled MCP server.
- Native `skills/list` loaded both skills from the installed plugin cache.
- Native `mcpServerStatus/list` connected the plugin's `codex_openpencil` stdio
  server and discovered 179 Rust tools.

## Protocol regression tests

All 14 tests passed: port validation; token-bound discovery; stale-record refusal;
missing/legacy records; foreign listener rejection; argument, Unicode and image
passthrough; notifications; native errors; single-send writes; timeout handling;
redirect/malformed response rejection; size bounds; invalid requests; and a real
stdio child process receiving split input and multiple messages.

## Isolated Rust smoke test

The smoke runner started its own file-backed HTTP host, then exercised the
packaged stdio bridge through initialize, notification, tools discovery, design
creation, readback, layout inspection, lint, native PNG rendering, save, and reopen.

- Server version: 0.8.5; 179 tools discovered.
- The saved document retained the headline and description after reopening.
- Lint: zero issues.
- PNG: 480 × 320; 12,210 bytes. The image was visually inspected for text and shape
  placement, with no clipping or overlap observed.
- PNG SHA-256: `a4ca5f0d66c3dd72479ed28e6ae445b69ce6a4dcdeb4e9d60765c7a8fcc5d3ae`.
- The smoke run terminated only its own processes and did not target the live canvas.

Local artifacts are intentionally ignored by Git. Run the smoke command to
reproduce them on a machine with a compatible Rust host.

## Cross-platform CI

[CI run #2](https://github.com/ZSeven-W/codex-openpencil/actions/runs/38529138631)
passed all six jobs for Node.js 22/24 on Linux, macOS and Windows at commit
`14d27a5`. Each job ran package checks and all 14 protocol tests. Windows exposed
a CRLF compatibility issue in the skill validator; the fix also passed an
isolated checkout converted entirely to CRLF before it was pushed.

These results do not assert a completed desktop UI workflow.
Clean-machine installation, absent-runtime onboarding, embedded interactive
canvas, and public-directory submission remain separate validation/release work.
