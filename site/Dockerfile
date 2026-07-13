# syntax=docker/dockerfile:1.7
#
# Boinc portal — production image.
#
# Multi-stage build:
#   1. `builder` runs `npm ci && npm run build` to produce dist/.
#   2. `runtime` is nginx-alpine serving the static site (SPA fallback +
#      /healthz probe endpoint, see nginx.conf).
#
# Build context is the site/ directory (the deploy script does this).

ARG NODE_VERSION=20
ARG NGINX_VERSION=1.27

# ---------------------------------------------------------------------------
# Stage 1: builder
# ---------------------------------------------------------------------------
FROM node:${NODE_VERSION}-alpine AS builder

WORKDIR /build

# Install deps first so this layer is cached when only source changes.
COPY package.json package-lock.json ./
RUN npm ci --no-audit --no-fund

COPY . ./
RUN npm run build

# ---------------------------------------------------------------------------
# Stage 2: runtime
# ---------------------------------------------------------------------------
FROM nginx:${NGINX_VERSION}-alpine AS runtime

RUN rm /etc/nginx/conf.d/default.conf
COPY nginx.conf /etc/nginx/conf.d/boinc.conf

COPY --from=builder /build/dist /usr/share/nginx/html

EXPOSE 80
STOPSIGNAL SIGQUIT
