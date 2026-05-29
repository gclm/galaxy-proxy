# Galaxy Router 开发规范

## 项目概述

AI 协议互转代理网关，支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种协议互转。

## 技术栈

- 语言: Rust 2024
- Web 框架: axum 0.8
- 数据库: SQLite (sqlx 0.9)
- 异步运行时: tokio 1.x

## API 设计规范

### 响应格式分层

**管理 API** (`/api/v1/admin/*`): 统一 JSON 格式

```json
// 成功
{
  "code": 0,
  "message": "success",
  "data": { ... }
}

// 成功（无数据）
{
  "code": 0,
  "message": "success",
  "data": null
}

// 错误
{
  "code": 400,
  "message": "错误描述"
}
```

**代理 API** (`/v1/*`): 保持原生协议格式

- `/v1/chat/completions` → OpenAI Chat 格式
- `/v1/responses` → OpenAI Responses 格式
- `/v1/messages` → Anthropic Messages 格式

代理 API 不使用统一响应格式，直接透传上游响应，确保客户端 SDK 兼容。

### ID 规范

- 所有实体 ID 使用 UUID v7
- ID 类型为 TEXT，存储格式: `xxxxxxxx-xxxx-7xxx-xxxx-xxxxxxxxxxxx`

### 错误处理

使用 `ApiError` 返回统一错误：

```rust
ApiError::bad_request("参数错误")      // 400
ApiError::unauthorized("未授权")       // 401
ApiError::forbidden("禁止访问")        // 403
ApiError::not_found("资源不存在")      // 404
ApiError::conflict("资源冲突")         // 409
ApiError::internal_error("服务器错误") // 500
```

### 认证架构

管理 API 和代理 API 使用不同的认证机制，在路由层强制执行：

| 层级 | 机制 | 保护范围 |
|------|------|----------|
| `require_admin_auth` 中间件 | JWT Bearer Token | `/api/v1/admin/*`（排除 init、login） |
| `ApiKeyAuth` 提取器 | API Key（Bearer 或 x-api-key） | `/v1/*` 代理端点 |
| 无认证 | — | `/api/v1/health`、`/api/v1/init`、`/api/v1/admin/auth/login` |

管理 API 认证中间件位于 `src/api/middleware/mod.rs` 的 `require_admin_auth`，通过 `extensions` 获取 JWT secret。所有管理路由（渠道、分组、API Key、统计、设置、备份等）统一受此中间件保护，无需在每个 handler 中重复校验。

JWT 过期时间从 `config.toml` 的 `auth.token_expiry_hours` 读取，默认 24 小时。

## 数据库规范

### 表结构

- 所有表 ID 为 TEXT 类型 (UUID v7)
- 时间字段使用 TIMESTAMP，默认 CURRENT_TIMESTAMP
- 外键关系通过 TEXT 类型 ID 关联

### 迁移

- 使用内置迁移系统 (version + SQL)
- 迁移版本号递增，不可回滚
- 新增迁移追加到 `get_migrations()` 函数

## 代码规范

### 模块职责

| 模块 | 职责 |
|------|------|
| `api/` | HTTP 路由、请求处理 |
| `api/handlers/admin/` | 管理 API 处理器 |
| `api/handlers/proxy/` | 代理 API 处理器（统一走 `handle_proxy_request`） |
| `api/middleware/` | JWT 中间件 + API Key 提取器 |
| `api/response.rs` | 统一响应格式 |
| `auth/` | 密码哈希、JWT |
| `config.rs` | TOML 配置加载 |
| `db/` | 数据库连接、迁移 |
| `proxy/` | 代理核心（缓存、模型索引、渠道选择、协议转换） |
| `stats/` | 统计聚合（用量/成本）+ 模型定价信息 |

### 命名规范

- 文件名: snake_case
- 结构体: PascalCase
- 函数/变量: snake_case
- 常量: SCREAMING_SNAKE_CASE
- 数据库表名: snake_case

### 测试

- 单元测试: `#[test]` 或 `#[tokio::test]`
- 集成测试: `tests/integration_test.rs`
- 测试数据库: `/tmp/galaxy_test_*` 目录

## 配置规范

### TOML 配置 (基础设施)

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "data/galaxy.db"

[auth]
jwt_secret = ""  # 首次运行自动生成
```

### 数据库配置 (运行时)

通过 `settings` 表存储，按 `category` 分类：

- `scheduler` - 调度器配置
- `sticky_session` - 粘性会话配置
- `stats` - 统计配置

### Homebrew 本地部署排查

通过 Homebrew 部署运行时，不要默认查看仓库内的 `data/galaxy.db` 或 `logs/galaxy.log`。先以 launchd 配置为准确认实际路径：

```bash
plutil -p ~/Library/LaunchAgents/homebrew.mxcl.galaxy-router.plist
```

当前 Homebrew 服务通常使用：

- 配置文件: `/opt/homebrew/etc/galaxy-router/config.toml`
- 工作目录: `/opt/homebrew/var/lib/galaxy-router`
- 数据库: `/opt/homebrew/var/lib/galaxy-router/galaxy.db`
- 日志: `/opt/homebrew/var/log/galaxy-router/output.log`、`/opt/homebrew/var/log/galaxy-router/error.log`

排查 cc / Claude Code 报错时，优先按这个顺序核对：

1. 用报错里的时间戳换算服务日志时间。`output.log` 使用 UTC 时间（例如 `2026-05-29T07:52:24Z` 对应北京时间 `2026-05-29 15:52:24`）。
2. 查询实际运行库的 `usage_logs`，确认 `requested_model`、`actual_model`、`channel_id`、`status_code`、`is_stream`、`error_message`、`response_content`。
3. 用 `channel_id` 回查 `channels` 表，确认命中的渠道、端点和该渠道的 key 数量。
4. 如果 cc 看见了上游错误，但 Web 请求日志没有失败记录，重点检查是否为流式请求。

当前实现里：非流式上游失败会同时写入 `error_message` 和 `response_content`；流式请求在建立 SSE 前失败会写入 `usage_logs`；上游以 HTTP 200 建立 SSE 后发送错误事件时，会把 `error_message` 写入日志并将 `status_code` 记为失败态（例如 502）。如果首个 SSE 事件就是错误，代理可以在尚未向客户端输出内容前触发 key / 渠道重试；如果已经向客户端输出过正常内容，后续流内错误只会记录并透传，不会无感切换 key。

## Git 规范

### Commit 格式

```
<type>: <description>

[optional body]
```

Type 类型:
- `feat`: 新功能
- `fix`: 修复
- `refactor`: 重构
- `test`: 测试
- `docs`: 文档
- `chore`: 构建/工具

### 分支策略

- `master`: 稳定版本
- `feature/*`: 功能分支
- `fix/*`: 修复分支

## 架构要点

- **代理统一入口**: 所有代理 handler（chat/responses/messages/embeddings/images）统一调用 `handle_proxy_request`，不各自实现转发逻辑
- **缓存共享**: `ProxyCache` 被 admin handler（channels、groups）和 proxy 层共享，管理操作后自动失效缓存
- **模型反向索引**: `ProxyCache.model_index` 维护 model→channel_id 映射，加速模型路由查找
- **API Key 轮询**: 使用 `AtomicU64` 计数器实现无锁 round-robin；一次请求选中渠道后，先按 round-robin 起点选择一个 key，遇到 401/402/429 或余额不足、无可用资源包、insufficient_quota 等 key / 账号资源错误时，会在同渠道内尝试下一个 key。只有该渠道所有 key 都失败，才排除整个 `channel_id` 转向其他渠道；流式请求一旦已向客户端输出正常内容，后续错误不会触发无感 key 切换。**端点和 API Key 支持单独禁用**：`ChannelInfo::enabled_api_keys()` 过滤禁用 Key 后再轮询，`find_endpoint()` 跳过禁用端点。
- **上游错误脱敏**: `sanitize_upstream_error` 截断并提取关键信息，避免泄露上游内部细节
- **统计聚合对齐整点**: aggregator 使用 wall-clock aligned sleep，而非固定间隔
