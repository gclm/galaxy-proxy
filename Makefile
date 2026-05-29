# Galaxy Proxy Makefile

.PHONY: build run test clean fmt clippy release docker help frontend-build dev dev-stop

# 默认目标
all: build

# 前端构建
frontend-build:
	cd frontend && pnpm install && pnpm build

# 构建（包含前端）
build: frontend-build
	cargo build

# ===== 开发模式：同时启动前后端 =====

# 运行rust 后端
dev-rust:
	cargo run

# 启动前端开发环境
dev-frontend:
	cd frontend && pnpm dev

# 启动开发环境（后端 + 前端 dev server，Ctrl+C 停止全部）
dev:
	@echo "启动开发环境..."
	@trap 'kill 0; exit 0' INT TERM; \
	cargo run & \
	cd frontend && npx vite; \
	wait

# 停止残留的开发进程
dev-stop:
	@lsof -ti :8080 2>/dev/null | xargs kill 2>/dev/null; \
	lsof -ti :5173 2>/dev/null | xargs kill 2>/dev/null; \
	lsof -ti :5174 2>/dev/null | xargs kill 2>/dev/null; \
	echo "开发环境已停止"

# ===== 测试 =====

# 运行测试
test:
	cargo test

# 格式化代码
fmt:
	cargo fmt

# 代码检查
clippy:
	cargo clippy -- -D warnings

# 清理构建产物
clean:
	cargo clean
	rm -rf data/galaxy.db
	rm -rf /tmp/galaxy_test*

# 发布构建
release:
	cargo build --release

# 交叉编译（可选）
release-linux:
	cross build --release --target x86_64-unknown-linux-gnu

release-macos:
	cargo build --release --target aarch64-apple-darwin

# Docker 构建
docker:
	docker build -t galaxy-proxy:latest .

# Docker 运行
docker-run:
	docker run -p 8080:8080 -v $(PWD)/data:/app/data galaxy-proxy:latest

# 检查代码（格式 + 检查 + 测试）
check: fmt clippy test

# 生成文档
doc:
	cargo doc --open

# 监听文件变化自动构建
watch:
	cargo watch -x build

# 监听文件变化自动测试
watch-test:
	cargo watch -x test

# 数据库重置
db-reset:
	rm -f data/galaxy.db
	@echo "Database reset. Run 'make run' to recreate."

# 显示帮助
help:
	@echo "Galaxy Proxy - AI 协议互转代理网关"
	@echo ""
	@echo "用法: make [target]"
	@echo ""
	@echo "目标:"
	@echo "  build          开发构建"
	@echo "  run            运行服务"
	@echo "  dev            启动开发环境（前后端同时运行，Ctrl+C 停止）"
	@echo "  dev-stop       停止残留的开发进程"
	@echo "  test           运行测试"
	@echo "  fmt            格式化代码"
	@echo "  clippy         代码检查"
	@echo "  clean          清理构建产物"
	@echo "  release        发布构建"
	@echo "  docker         Docker 构建"
	@echo "  docker-run     Docker 运行"
	@echo "  check          完整检查（格式+检查+测试）"
	@echo "  doc            生成文档"
	@echo "  watch          监听自动构建"
	@echo "  watch-test     监听自动测试"
	@echo "  db-reset       重置数据库"
	@echo "  help           显示帮助"
