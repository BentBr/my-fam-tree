# syntax=docker/dockerfile:1.7
# Stage 1: Build the SPA. The output is architecture-independent, so we run
# the Node build on the BUILDPLATFORM and only nginx ships per-arch.
#
# OCI image metadata (populated by CI via --build-arg). Surfaces on the GHCR
# package page.
ARG OCI_SOURCE="https://github.com/BentBr/my-family"
ARG OCI_REVISION=""
ARG OCI_CREATED=""

FROM --platform=$BUILDPLATFORM node:22-alpine AS builder
WORKDIR /app

# Install pnpm via corepack at the workspace-pinned version. Keep deps install
# isolated so a source-only change doesn't bust the npm cache layer.
RUN corepack enable && corepack prepare pnpm@10.33.4 --activate

COPY fe/package.json fe/pnpm-lock.yaml fe/.npmrc ./
RUN pnpm install --frozen-lockfile

# Bring in the rest of the FE source. `fe/openapi.json` is the source of truth
# for the TS types; `fe/src/api/schema.d.ts` is gitignored and regenerated here.
COPY fe/ ./
RUN pnpm openapi-codegen
RUN pnpm build

# Stage 2: Serve the built assets with nginx.
FROM nginx:alpine AS production

ARG OCI_SOURCE
ARG OCI_REVISION
ARG OCI_CREATED

LABEL org.opencontainers.image.title="my-family-fe" \
      org.opencontainers.image.description="my-family frontend served by nginx" \
      org.opencontainers.image.source="${OCI_SOURCE}" \
      org.opencontainers.image.revision="${OCI_REVISION}" \
      org.opencontainers.image.created="${OCI_CREATED}"

# nginx writes temp files under /var/cache/nginx. Ensure the directories exist
# with the right ownership so the unprivileged worker can use them.
RUN mkdir -p /var/cache/nginx/client_temp /var/cache/nginx/proxy_temp \
    /var/cache/nginx/fastcgi_temp /var/cache/nginx/uwsgi_temp /var/cache/nginx/scgi_temp \
    && chown -R nginx:nginx /var/cache/nginx \
    && chmod -R 755 /var/cache/nginx

COPY --from=builder --chown=nginx:nginx /app/dist /usr/share/nginx/html
COPY .docker/fe/nginx.conf /etc/nginx/conf.d/default.conf

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]
