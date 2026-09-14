# ==============================================================================
# Beejs Production Docker Image
# Optimized for high-performance JavaScript/TypeScript runtime
# ==============================================================================

# 阶段 1: 构建阶段
FROM rust:1-slim-bookworm AS builder

# 安装构建依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    curl \
    python3 \
    git \
    && ln -sf /usr/bin/python3 /usr/bin/python \
    && rm -rf /var/lib/apt/lists/*

# 设置工作目录
WORKDIR /app

# Docker builds should be reliable on standard CI runners.
ENV CARGO_PROFILE_RELEASE_LTO=false
ENV CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16
ENV CARGO_BUILD_JOBS=1

# Manifest + benches must exist before `cargo fetch` (Cargo.toml lists [[bench]]).
COPY Cargo.toml Cargo.lock build.rs ./
COPY benches ./benches
COPY src ./src
# types_export.rs embed: include_str!("../types/beejs.d.ts")
COPY types ./types

RUN cargo fetch --locked
RUN cargo build --release --bin bee

# 阶段 2: 运行时阶段 - 最小化镜像
FROM debian:bookworm-slim AS runtime

# 安装运行时依赖
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    tzdata \
    && rm -rf /var/lib/apt/lists/*

# 创建非特权用户
RUN groupadd -r beejs && useradd -r -g beejs beejs

# 设置工作目录
WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/bee /usr/local/bin/bee

# 创建必要的目录
RUN mkdir -p /app/cache /app/logs /app/tmp && \
    chown -R beejs:beejs /app

# 复制示例和文档
COPY --chown=beejs:beejs README.md /app/
COPY --chown=beejs:beejs examples/ /app/examples/

# 切换到非特权用户
USER beejs

# 设置默认环境变量
ENV BEEJS_MODE=production
ENV BEEJS_LOG_LEVEL=info
ENV BEEJS_MAX_CONNECTIONS=10000
ENV BEEJS_BATCH_SIZE=100
ENV BEEJS_CACHE_DIR=/app/cache
ENV BEEJS_TMP_DIR=/app/tmp

# 暴露端口
EXPOSE 3000

# 健康检查
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD bee --version >/dev/null || exit 1

# ENTRYPOINT is `bee` so `docker run IMAGE --version` execs bee, not `--version`.
# A CMD-only image replaces the whole argv; GHCR smoke failed with exit 127:
# exec: "--version": executable file not found in $PATH
ENTRYPOINT ["bee"]
CMD ["serve", "--host", "0.0.0.0", "--port", "3000"]

# ==============================================================================
# 多阶段构建说明:
# - builder: 编译 Rust 代码
# - runtime: 最小化生产镜像
# ==============================================================================
