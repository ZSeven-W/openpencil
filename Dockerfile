# ── Stage 1: Build ──
FROM oven/bun:1 AS builder

WORKDIR /app

# Install dependencies first (cache layer)
COPY package.json bun.lock ./
RUN bun install --frozen-lockfile

# Copy source and build
COPY . .
RUN bun --bun run build

# ── Stage 2: Production ──
FROM oven/bun:1-slim

WORKDIR /app

COPY --from=builder /app/.output ./.output
COPY --from=builder /app/package.json ./

ENV NODE_ENV=production
ENV NITRO_HOST=0.0.0.0
ENV NITRO_PORT=3000

EXPOSE 3000

CMD ["bun", "run", "./.output/server/index.mjs"]
