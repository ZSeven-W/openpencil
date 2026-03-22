# ── Stage 1: Build web app ──
FROM oven/bun:1 AS builder

WORKDIR /app
COPY package.json bun.lock ./
COPY packages/pen-types/package.json packages/pen-types/
COPY packages/pen-core/package.json packages/pen-core/
COPY packages/pen-codegen/package.json packages/pen-codegen/
COPY packages/pen-figma/package.json packages/pen-figma/
COPY packages/pen-renderer/package.json packages/pen-renderer/
COPY packages/pen-sdk/package.json packages/pen-sdk/
COPY apps/web/package.json apps/web/
COPY apps/desktop/package.json apps/desktop/
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
RUN bun install -g @anthropic-ai/gemini-cli
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]

# ── Full: all CLI tools ──
FROM oven/bun:1 AS full
WORKDIR /app
COPY --from=builder /app/out/web ./out/web
COPY --from=builder /app/package.json ./
RUN bun install -g @anthropic-ai/claude-code @openai/codex opencode-ai @github/copilot @anthropic-ai/gemini-cli
ENV NODE_ENV=production NITRO_HOST=0.0.0.0 NITRO_PORT=3000
EXPOSE 3000
CMD ["bun", "run", "./out/web/server/index.mjs"]
