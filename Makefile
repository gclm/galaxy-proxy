# Galaxy Router Makefile

.PHONY: all build frontend-build dev dev-rust dev-frontend dev-stop \
        test fmt clippy clean check doc watch watch-test db-reset \
        release release-archive release-all \
        release-linux-amd64 release-linux-arm64 \
        release-darwin-arm64 release-darwin-x86_64 \
        release-windows-amd64 \
        docker docker-run help

VERSION ?= $(shell git describe --tags --abbrev=0 2>/dev/null || echo 'dev')
COMMIT  := $(shell git rev-parse --short HEAD 2>/dev/null || echo 'unknown')
APP     := galaxy-router
DIST    := build/dist

# 默认目标
all: build

# ===== 开发 =====

frontend-build:
	cd frontend && pnpm install && pnpm build

build: frontend-build
	cargo build

dev-rust:
	cargo run

dev-frontend:
	cd frontend && pnpm dev

dev:
	@echo "启动开发环境..."
	@trap 'kill 0; exit 0' INT TERM; \
	cargo run & \
	cd frontend && npx vite; \
	wait

dev-stop:
	@lsof -ti :8080 2>/dev/null | xargs kill 2>/dev/null; \
	lsof -ti :5173 2>/dev/null | xargs kill 2>/dev/null; \
	lsof -ti :5174 2>/dev/null | xargs kill 2>/dev/null; \
	echo "开发环境已停止"

# ===== 测试 / 检查 =====

test:
	cargo test

fmt:
	cargo fmt

clippy:
	cargo clippy -- -D warnings

clean:
	cargo clean
	rm -rf data/galaxy.db
	rm -rf /tmp/galaxy_test*
	rm -rf $(DIST)

check: fmt clippy test

doc:
	cargo doc --open

watch:
	cargo watch -x build

watch-test:
	cargo watch -x test

db-reset:
	rm -f data/galaxy.db
	@echo "Database reset. Run 'make run' to recreate."

# ===== 发布构建 =====

release:
	cargo build --release

# 单目标交叉编译（需要对应 toolchain 已安装）
release-linux-amd64:
	cross build --release --target x86_64-unknown-linux-gnu

release-linux-arm64:
	cross build --release --target aarch64-unknown-linux-gnu

release-darwin-arm64:
	cargo build --release --target aarch64-apple-darwin

release-darwin-x86_64:
	cargo build --release --target x86_64-apple-darwin

release-windows-amd64:
	cross build --release --target x86_64-pc-windows-gnu

# 打 zip 包（在交叉编译完成后调用）
# 用法: make release-archive TARGET=aarch64-apple-darwin
release-archive:
	@mkdir -p $(DIST)
	@target=$(TARGET); \
	os=$${target%-*-*}; \
	arch=$${target##*-}; \
	case "$$os" in \
	  x86_64-pc-windows-gnu) ext=".exe"; name="windows-amd64";; \
	  aarch64-apple-darwin) name="darwin-arm64";; \
	  x86_64-apple-darwin) name="darwin-x86_64";; \
	  x86_64-unknown-linux-gnu) name="linux-amd64";; \
	  aarch64-unknown-linux-gnu) name="linux-arm64";; \
	  *) name="$$os-$$arch";; \
	esac; \
	bin="target/$$target/release/$(APP)$${ext}"; \
	if [ -f "$$bin" ]; then \
	  cp "$$bin" "$(DIST)/$(APP)"; \
	  cd $(DIST) && zip -q "$(APP)-$$name.zip" $(APP) && rm $(APP); \
	  echo "已打包: $(DIST)/$(APP)-$$name.zip"; \
	else \
	  echo "二进制不存在: $$bin" >&2; exit 1; \
	fi

# CI 用：全平台构建 + 打包（由 GitHub Actions 按矩阵调用单个目标）
# 本地全量构建需要所有 toolchain 就绪
release-all: release-darwin-arm64 release-darwin-x86_64 \
             release-linux-amd64 release-linux-arm64

# ===== Docker =====

docker:
	docker build -t galaxy-router:latest .

docker-run:
	docker run -p 8080:8080 -v $(PWD)/data:/app/data galaxy-router:latest

# ===== 帮助 =====

help:
	@echo "Galaxy Router - AI 协议互转代理网关"
	@echo ""
	@echo "用法: make [target]"
	@echo ""
	@echo "开发:"
	@echo "  build              开发构建"
	@echo "  dev                启动开发环境（前后端同时运行）"
	@echo "  dev-stop           停止残留的开发进程"
	@echo ""
	@echo "测试 / 检查:"
	@echo "  test               运行测试"
	@echo "  fmt                格式化代码"
	@echo "  clippy             代码检查"
	@echo "  check              完整检查（格式+检查+测试）"
	@echo ""
	@echo "发布:"
	@echo "  release            本机 release 构建"
	@echo "  release-darwin-arm64    macOS ARM64"
	@echo "  release-darwin-x86_64   macOS x86_64"
	@echo "  release-linux-amd64     Linux AMD64"
	@echo "  release-linux-arm64     Linux ARM64"
	@echo "  release-windows-amd64   Windows AMD64"
	@echo "  release-archive TARGET=<triple>  打 zip 包"
	@echo ""
	@echo "运维:"
	@echo "  db-reset           重置数据库"
	@echo "  clean              清理构建产物"
