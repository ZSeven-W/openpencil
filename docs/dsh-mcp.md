# DeepSeek Harness MCP setup

OpenPencil supports the DSH MCP client plugin, `@deepseek-ai/dsh-mcp-client`.
Choose the live desktop integration to edit the canvas open in OpenPencil, or
configure a file-backed stdio server for a headless agent.

## Live desktop integration

Enable **DeepSeek Harness** in OpenPencil's **Settings → MCP** panel. Keep
OpenPencil running while DSH uses this connection. The integration writes the
current local server endpoint, including `/mcp/lean` when the lean tool profile
is enabled.

The DSH home is `DSH_HOME` from the OpenPencil process environment, or `~/.dsh`
when unset. If DSH uses a custom home, launch OpenPencil with the same value.
An environment variable set only inside a terminal may not reach an app
launched from the desktop.

For existing profiles, OpenPencil writes each profile's `cordis.patch.yml`:

```text
$DSH_HOME/profiles/<profile>/cordis.patch.yml
```

DSH's desktop app normally uses the `desktop` profile. OpenPencil discovers
initialized profile directories through their `package.json`; it does not
invent a profile or modify its manifest. If no profiles have been initialized,
OpenPencil uses the supported home-level `$DSH_HOME/cordis.patch.yml` instead.
After adding a profile, restart OpenPencil or re-enable the integration to
include it.

Existing OpenPencil-managed home entries migrate to the profile files. Only
the block between the `openpencil-mcp-begin` and `openpencil-mcp-end` markers
is managed; other entries and comments are preserved. Empty patch layers stay
valid YAML arrays (`[]`). A hand-written OpenPencil entry or damaged markers
require manual reconciliation before OpenPencil changes the integration.

Automatic editing supports block-style patch lists and empty arrays. Other
top-level YAML forms are refused instead of being reformatted or appended to
unsafely. All target files are checked before migration; a write failure rolls
back the files already changed. DSH may observe intermediate file changes
while its watcher reloads them.

Do not copy the same `insert` entry into both home and profile files: DSH
composes both layers and creates duplicate instances rather than deduplicating
them by ID.

## File-backed stdio integration

Create and save the `.op` document first, for example with **File → Save As** in
OpenPencil. `openpencil-desktop --mcp <path>` opens an existing document; it does
not create one. A missing document produces an actionable error on stderr and
the MCP process exits.

For a Windows DSH profile, a manual patch can look like this. Replace both
paths with the actual executable and an existing document:

```yaml
- insert:
    - id: mcp-openpencil
      name: '@deepseek-ai/dsh-mcp-client'
      config:
        serverName: openpencil
        transport: stdio
        command: 'C:/path/to/openpencil-desktop.exe'
        args: ['--mcp', 'C:/path/to/design.op']
```

On macOS/Linux, use the executable's absolute path and a POSIX document path.
Remove the managed live-server block before adding a manual stdio entry, and
keep only one OpenPencil insertion across the home and profile layers.
OpenPencil detects manual entries but refuses to overwrite or remove them.

## Diagnosing a missing server

1. Confirm the DSH home and the profile actually used by the running process.
2. Check that the patch is a top-level YAML array. An empty or comments-only
   file is not an empty array; use `[]` when no entries remain.
3. Check DSH's configuration/HMR errors and whether `mcp-openpencil` was loaded
   before diagnosing transport failures. A rejected patch can leave the last
   working configuration active.
4. For live HTTP, keep OpenPencil open and use its current endpoint. For stdio,
   check the executable/document paths and read the server's stderr.

DSH `0.2.0-rc.2` supports both home and profile layers and watches both for
changes; a restart is not inherently required by the home layer. Its
[layer composition](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.2.0-rc.2/packages/boot/app-boot/src/profile-context.ts)
and [HMR setup](https://github.com/deepseek-ai/deepseek-harness/blob/dsh-v0.2.0-rc.2/packages/boot/hmr/src/index.ts)
define this behavior. Profile targeting addresses the setup reported in
[#254](https://github.com/ZSeven-W/openpencil/issues/254); the packaged Windows
application still needs platform-specific verification.
