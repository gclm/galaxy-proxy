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
| `api/handlers/proxy/` | 代理 API 处理器 |
| `api/middleware/` | 认证中间件 |
| `api/response.rs` | 统一响应格式 |
| `auth/` | 密码哈希、JWT |
| `config.rs` | TOML 配置加载 |
| `db/` | 数据库连接、迁移 |

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

## 待实现功能

- [x] 分组管理 API (Phase 4)
- [x] 客户端 API Key 管理 (Phase 5)
- [x] 协议转换层 (Phase 6)
- [x] 代理转发 (Phase 7)
- [x] 负载均衡 (Phase 8)
- [x] 统计系统 (Phase 9)
- [x] 前端管理面板 (Phase 10)
- [x] 渠道模型获取 (2026-05-26)
