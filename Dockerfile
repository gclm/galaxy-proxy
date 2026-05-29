FROM debian:bookworm-slim
ARG TARGETARCH

RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY dist/linux-${TARGETARCH}/galaxy-router /app/galaxy-router
COPY config.docker.toml /app/config.toml
RUN chmod +x /app/galaxy-router && mkdir -p data logs

EXPOSE 8080
CMD ["./galaxy-router", "--config", "config.toml"]
