# ── Stage 1: 构建前端 ──
FROM node:20-slim AS frontend
WORKDIR /fe
COPY frontend/package.json frontend/package-lock.json ./
RUN npm config set registry https://registry.npmmirror.com && npm ci
COPY frontend/ .
RUN npm run build

# ── Stage 2: 构建后端 ──
FROM rust:1.88-bookworm AS server
WORKDIR /srv
# 使用国内 crates.io 镜像加速
RUN mkdir -p /usr/local/cargo && \
    echo '[source.crates-io]' > /usr/local/cargo/config.toml && \
    echo 'replace-with = "ustc"' >> /usr/local/cargo/config.toml && \
    echo '[source.ustc]' >> /usr/local/cargo/config.toml && \
    echo 'registry = "sparse+https://mirrors.ustc.edu.cn/crates.io-index/"' >> /usr/local/cargo/config.toml
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
# 不换 apt 源，不改 sources.list，避免国内镜像源超时
# ffmpeg 用静态二进制，不走 apt
FROM ubuntu:22.04
ENV DEBIAN_FRONTEND=noninteractive
# 只装 ca-certificates 和 libssl3（小包，默认源即使慢也能装上）
# 如果默认源也超时，加 --fix-missing 重试
RUN apt-get update && \
    apt-get install -y --no-install-recommends --fix-missing \
    ca-certificates libssl3 curl xz-utils && \
    rm -rf /var/lib/apt/lists/*
# 下载 ffmpeg 静态二进制（不走 apt，避免大量依赖包）
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then FFMPEG_ARCH="amd64"; \
    else FFMPEG_ARCH="arm64"; fi && \
    curl -fsSL --retry 3 "https://johnvansickle.com/ffmpeg/releases/ffmpeg-release-${FFMPEG_ARCH}-static.tar.xz" \
    -o /tmp/ffmpeg.tar.xz && \
    mkdir -p /tmp/ffmpeg && \
    tar xf /tmp/ffmpeg.tar.xz -C /tmp/ffmpeg && \
    cp /tmp/ffmpeg/*/ffmpeg /usr/local/bin/ffmpeg && \
    cp /tmp/ffmpeg/*/ffprobe /usr/local/bin/ffprobe && \
    chmod +x /usr/local/bin/ffmpeg /usr/local/bin/ffprobe && \
    rm -rf /tmp/ffmpeg*
WORKDIR /app
COPY --from=server /srv/target/release/walioffice /usr/local/bin/walioffice

ENV AIPPT_HOST=0.0.0.0
ENV AIPPT_PORT=8000
EXPOSE 8000
VOLUME ["/app/data", "/app/outputs"]

CMD ["walioffice"]
