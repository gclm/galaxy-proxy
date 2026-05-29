# 安装指南

Galaxy Router 支持多种安装方式，根据你的环境选择最合适的一种。

## Homebrew（推荐 macOS 用户）

```bash
# 添加 tap
brew tap gclm/tap

# 安装
brew install gclm/tap/galaxy-router

# 启动服务
brew services start gclm/tap/galaxy-router
```

安装完成后访问 `http://127.0.0.1:29088` 进入管理界面。

**管理文件路径：**

| 文件 | 路径 |
|---|---|
| 配置文件 | `$(brew --prefix)/etc/galaxy-router/config.toml` |
| 数据库 | `$(brew --prefix)/var/lib/galaxy-router/galaxy.db` |
| 日志 | `$(brew --prefix)/var/log/galaxy-router/` |

**常用命令：**

```bash
# 查看服务状态
brew services info gclm/tap/galaxy-router

# 停止服务
brew services stop gclm/tap/galaxy-router

# 重启服务
brew services restart gclm/tap/galaxy-router

# 卸载
brew uninstall gclm/tap/galaxy-router
```

## Docker

```bash
# 拉取镜像
docker pull ghcr.io/gclm/galaxy-router:latest

# 运行
docker run -d \
  --name galaxy-router \
  -p 8080:8080 \
  -v $(pwd)/data:/app/data \
  -v $(pwd)/config.toml:/app/config.toml \
  ghcr.io/galaxy-router:latest
```

使用 Docker Compose（推荐）：

```yaml
services:
  galaxy-router:
    image: ghcr.io/gclm/galaxy-router:latest
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data
      - ./config.toml:/app/config.toml
    restart: unless-stopped
```

## 从源码构建

**前置依赖：** Rust 1.80+、Node.js 22+、pnpm

```bash
# 克隆仓库
git clone https://github.com/gclm/galaxy-router.git
cd galaxy-router

# 构建前端 + 后端
make build

# 运行
./target/debug/galaxy-router --config config.toml
```

**Release 构建：**

```bash
make release
./target/release/galaxy-router --config config.toml
```

**交叉编译（需要对应 toolchain）：**

```bash
# macOS ARM64
make release-darwin-arm64

# Linux AMD64（需要 cross）
make release-linux-amd64
```

## 从 GitHub Release 下载

前往 [Releases 页面](https://github.com/gclm/galaxy-router/releases) 下载对应平台的预编译二进制。

```bash
# macOS ARM64 示例
curl -LO https://github.com/gclm/galaxy-router/releases/latest/download/galaxy-router-darwin-arm64.zip
unzip galaxy-router-darwin-arm64.zip
chmod +x galaxy-router
./galaxy-router --config config.toml
```

## 配置文件

Galaxy Router 使用 TOML 格式的配置文件。首次运行时会自动生成默认配置。

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "data/galaxy.db"

[logging]
level = "info"
format = "compact"
file = true
file_path = "logs/galaxy.log"

[auth]
jwt_secret = ""  # 留空则首次启动自动生成
token_expiry_hours = 24

[pricing]
cache_path = "data/pricing_cache.json"
refresh_interval_hours = 24
providers = ["openai", "anthropic", "deepseek", "google", "zhipuai", "minimax", "xai", "moonshot", "xiaomi", "stepfun"]
```

**CLI 参数可覆盖配置文件：**

```bash
# 指定配置文件
galaxy-router --config /path/to/config.toml

# 覆盖端口
galaxy-router --port 9090

# 覆盖监听地址
galaxy-router --host 0.0.0.0

# 覆盖日志级别
galaxy-router --log-level debug
```

## 首次初始化

1. 启动服务后访问 `http://127.0.0.1:8080`
2. 首次访问会自动跳转到初始化页面，设置管理员用户名和密码
3. 登录后即可开始使用
