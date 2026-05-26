# 构建阶段
FROM rust:1.80-bookworm AS builder

WORKDIR /app

# 安装依赖
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# 复制依赖文件
COPY Cargo.toml Cargo.lock ./

# 创建 dummy src 以缓存依赖
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo build --release
RUN rm -rf src

# 复制源代码
COPY src ./src
COPY config.toml ./

# 构建
RUN touch src/main.rs
RUN cargo build --release

# 运行阶段
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# 从构建阶段复制二进制文件
COPY --from=builder /app/target/release/galaxy-proxy .

# 创建数据目录
RUN mkdir -p data logs

# 暴露端口
EXPOSE 8080

# 运行
CMD ["./galaxy-proxy"]
