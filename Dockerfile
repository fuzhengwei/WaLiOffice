# ── Stage 1: 构建前端 ──
FROM node:20-slim AS frontend
WORKDIR /fe
COPY frontend/package.json frontend/package-lock.json ./
RUN npm ci
COPY frontend/ .
RUN npm run build

# ── Stage 2: 构建后端 ──
FROM rust:1.80-bookworm AS server
WORKDIR /srv
# 先复制 Cargo.toml 利用 docker 缓存
COPY server/Cargo.toml server/Cargo.lock* ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && cargo build --release 2>/dev/null || true
# 复制源码
COPY server/src ./src
COPY migrations ../migrations
# 复制前端构建产物（rust-embed 在编译时读取）
COPY --from=frontend /fe/dist ../frontend/dist
RUN touch src/main.rs && cargo build --release

# ── Stage 3: 运行时 ──
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=server /srv/target/release/walioffice /usr/local/bin/walioffice

ENV AIPPT_HOST=0.0.0.0
ENV AIPPT_PORT=8000
EXPOSE 8000
VOLUME ["/app/data", "/app/outputs"]

CMD ["walioffice"]
