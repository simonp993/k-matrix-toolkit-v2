# ── Stage 1: Build Rust backend ──────────────────────────────────────
FROM rust:1.85-slim AS backend-build
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY backend/ ./backend/
WORKDIR /app/backend
RUN cargo build --release --bin kmatrix-server

# ── Stage 2: Build Next.js frontend ─────────────────────────────────
FROM node:20-alpine AS frontend-build
WORKDIR /app
COPY frontend/package*.json ./
RUN npm ci
COPY frontend/ .
# The API URL must be set at build time for Next.js static optimization
ARG NEXT_PUBLIC_API_URL=http://localhost:3001
ENV NEXT_PUBLIC_API_URL=${NEXT_PUBLIC_API_URL}
RUN npm run build

# ── Stage 3: Backend runtime ────────────────────────────────────────
FROM debian:bookworm-slim AS backend
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=backend-build /app/backend/target/release/kmatrix-server /usr/local/bin/
RUN mkdir -p /data
ENV RUST_LOG=info
EXPOSE 3001
CMD ["kmatrix-server"]

# ── Stage 4: Frontend runtime ───────────────────────────────────────
FROM node:20-alpine AS frontend
WORKDIR /app
ENV NODE_ENV=production
# Copy standalone server + static assets
COPY --from=frontend-build /app/.next/standalone ./
COPY --from=frontend-build /app/.next/static ./.next/static
COPY --from=frontend-build /app/public ./public
EXPOSE 3000
CMD ["node", "server.js"]
