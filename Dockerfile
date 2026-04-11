# syntax=docker/dockerfile:1
# ── Stage 1: Build web app ──
FROM oven/bun:1 AS builder

# Install Zig. agent-native postinstall prefers downloading a prebuilt .node
# matching the submodule commit from the ZSeven-W/agent release, but falls
# back to `zig build napi` when no matching asset exists (e.g. building for
# an arch we don't publish yet). Pin 0.15.2 because the Zig source uses the
# unmanaged ArrayList / std.process.getEnvVarOwned shape introduced in 0.15.
RUN apt-get update && apt-get install -y --no-install-recommends curl xz-utils ca-certificates \
    && ARCH="$(uname -m)" \
    && case "$ARCH" in \
        x86_64) ZIG_ARCH=x86_64 ;; \
        aarch64) ZIG_ARCH=aarch64 ;; \
        *) echo "Unsupported arch: $ARCH" && exit 1 ;; \
       esac \
    && curl -fsSL "https://ziglang.org/download/0.15.2/zig-${ZIG_ARCH}-linux-0.15.2.tar.xz" \
       | tar -xJ -C /usr/local \
    && ln -sf "/usr/local/zig-${ZIG_ARCH}-linux-0.15.2/zig" /usr/local/bin/zig \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY package.json bun.lock ./
COPY --parents packages/*/package.json apps/*/package.json ./
# agent-native is a git submodule with a nested workspace package (napi/)
# and Zig sources needed by the postinstall hook — copy it whole.
COPY packages/agent-native ./packages/agent-native
RUN bun install --frozen-lockfile
COPY . .
ENV NODE_OPTIONS="--max-old-space-size=4096"
RUN bun --bun run build

# ── Stage 2: Base (web only, no CLI) ──
FROM oven/bun:1-slim AS base

WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./

ENV NODE_ENV=production
ENV NITRO_HOST=0.0.0.0
ENV NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

# ── CLI variants ──

FROM oven/bun:1 AS with-claude
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @anthropic-ai/claude-code
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

FROM oven/bun:1 AS with-codex
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @openai/codex
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

FROM oven/bun:1 AS with-opencode
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g opencode-ai
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

FROM oven/bun:1 AS with-copilot
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @github/copilot
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

FROM oven/bun:1 AS with-gemini
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @google/gemini-cli
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

# ── Full: all CLI tools ──
FROM oven/bun:1 AS full
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @anthropic-ai/claude-code @openai/codex opencode-ai @github/copilot @google/gemini-cli
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]
