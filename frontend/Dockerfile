# ── Build ─────────────────────────────────────────────────────────────────────
FROM node:22-bookworm-slim AS build

WORKDIR /app

# corepack ativa o pnpm na versão fixada em package.json (packageManager).
RUN corepack enable

# Camada de dependências primeiro (cache enquanto só o código muda).
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
RUN pnpm install --frozen-lockfile

# Código e build de produção (gera .output com o servidor Nitro/Node).
COPY . .
RUN pnpm build

# ── Runtime ───────────────────────────────────────────────────────────────────
FROM node:22-bookworm-slim AS runtime

WORKDIR /app
ENV NODE_ENV=production \
    HOST=0.0.0.0 \
    PORT=3001

# Só o output autossuficiente do Nuxt é necessário em runtime.
COPY --from=build /app/.output ./.output

USER node
EXPOSE 3001
CMD ["node", ".output/server/index.mjs"]
