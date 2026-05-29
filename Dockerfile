# 构建阶段
FROM rust:1.80-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 缓存 Rust 依赖
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 构建前端
FROM node:22-slim AS frontend

WORKDIR /app/frontend
RUN npm install -g pnpm
COPY frontend/package.json frontend/pnpm-lock.yaml ./
RUN pnpm install --frozen-lockfile
COPY frontend/ ./
RUN pnpm build

# 最终构建
FROM builder AS final

COPY src ./src
COPY config.toml ./
COPY --from=frontend /app/frontend/dist ./frontend/dist
RUN touch src/main.rs && cargo build --release

# 运行阶段
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=final /app/target/release/galaxy-router .
RUN mkdir -p data logs

EXPOSE 8080
CMD ["./galaxy-router", "--config", "config.toml"]
