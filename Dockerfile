# 语法=docker/dockerfile:1 ──
# Stage 1: Build Web 应用程序 ──
FROM oven/bun:1 AS builder

# Install Zig。 agent-native postinstall 更喜欢下载与 ZSeven-W/agent 版本的子模块提交匹配的预构建
# .node，但当不存在匹配资产时会回退到 `zig build napi`（例如，为我们尚未发布的拱门构建）。 Pin 0.15.2，因为
# Zig 源使用 0.15 中引入的非托管 ArrayList / std.process.getEnvVarOwned 形状。
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
# agent-native 是一个 git 子模块，带有嵌套工作区包 (napi/) 和 postinstall 挂钩所需的 Zig 源 -
# 将其整个复制。
COPY packages/agent-native ./packages/agent-native
RUN bun install --frozen-lockfile
COPY . .
ARG VITE_SUPABASE_URL
ARG VITE_SUPABASE_ANON_KEY
ENV VITE_SUPABASE_URL=$VITE_SUPABASE_URL
ENV VITE_SUPABASE_ANON_KEY=$VITE_SUPABASE_ANON_KEY
ENV NODE_OPTIONS="--max-old-space-size=4096"
RUN bun --bun run build

# ── Stage 2：Base（仅限网页，无 CLI）──
FROM oven/bun:1-slim AS base

WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./

ENV NODE_ENV=production
ENV NITRO_HOST=0.0.0.0
ENV NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

# ── CLI 变种 ──

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

# ── Full：所有 CLI 工具 ──
FROM oven/bun:1 AS full
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @anthropic-ai/claude-code @openai/codex opencode-ai @github/copilot @google/gemini-cli
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]
