# 配置格式设计

## 配置分层

| 层级 | 存储方式 | 内容 | 更新方式 |
|------|---------|------|---------|
| 基础配置 | TOML 文件 | 服务器、数据库、日志 | 重启生效 |
| 业务配置 | SQLite 数据库 | 渠道、分组、定价、模型映射 | Web 面板实时生效 |

## TOML 基础配置

**文件路径**: `./config.toml`（可通过 `GALAXY_PROXY_CONFIG` 环境变量覆盖）

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "data/galaxy.db"

[logging]
level = "info"              # trace | debug | info | warn | error
format = "compact"          # compact | json
file = false                # 是否输出到文件
file_path = "logs/galaxy.log"

[auth]
jwt_secret = ""             # 首次运行自动生成
token_expiry_hours = 24
```

## 数据库 settings 表

运行时可调的配置存储在数据库中，通过 Web 面板管理。

```sql
CREATE TABLE settings (
    key TEXT PRIMARY KEY,
    category TEXT NOT NULL DEFAULT 'general',
    value TEXT NOT NULL,
    description TEXT,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**默认配置**（首次运行时插入）：

| key | category | 默认值 | 说明 |
|-----|----------|--------|------|
| `scheduler.top_k` | scheduler | `7` | Top-K 候选数量 |
| `scheduler.score_weights` | scheduler | `{"priority":1.0,...}` | 评分权重 |
| `sticky_session.enabled` | sticky_session | `true` | 是否启用粘性会话 |
| `sticky_session.ttl_seconds` | sticky_session | `3600` | 会话保持时间（秒） |
| `stats.log_detail_mode` | stats | `failures_only` | 日志模式 |
| `stats.cost.source` | stats | `models.dev` | 成本数据源 |
| `stats.cost.refresh_interval_hours` | stats | `24` | 刷新间隔（小时） |

**前端按 category 分组显示设置项。**

## 数据库 Schema

### channels 表

```sql
CREATE TABLE channels (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL UNIQUE,
    base_url TEXT NOT NULL,                      -- 基础 URL
    api_keys TEXT NOT NULL DEFAULT '[]',        -- JSON 数组，如 ["sk-xxx", "sk-yyy"]
    supported_types TEXT NOT NULL DEFAULT '[]', -- JSON 数组，支持的协议类型
    model_maps TEXT NOT NULL DEFAULT '{}',       -- JSON 对象，如 {"claude-*": "claude-sonnet-4-20250514"}
    rate_limit_rpm INTEGER,
    rate_limit_tpm INTEGER,
    failure_threshold INTEGER NOT NULL DEFAULT 3,
    blacklist_minutes INTEGER NOT NULL DEFAULT 10,
    concurrency INTEGER NOT NULL DEFAULT 10,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

**字段说明**:
- `api_keys`: 多 Key 时使用轮询选择
- `supported_types`: 支持的协议类型，如 `["openai_chat", "anthropic"]`
- `model_maps`: key 是源模型（支持通配符），value 是目标模型

**端点路径映射**（按协议类型自动拼接）:

| 协议类型 | 端点路径 |
|---------|---------|
| `openai_chat` | `{base_url}/v1/chat/completions` |
| `openai_response` | `{base_url}/v1/responses` |
| `anthropic` | `{base_url}/v1/messages` |
| `embedding` | `{base_url}/v1/embeddings` |
| `images` | `{base_url}/v1/images/generations` |

### api_keys 表（客户端侧 Key，用于访问 Proxy）

```sql
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,                     -- Key 名称（便于识别）
    api_key TEXT NOT NULL UNIQUE,           -- 客户端使用的 Key
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### groups 表

```sql
CREATE TABLE groups (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,       -- 对外暴露的模型名
    mode TEXT NOT NULL CHECK (mode IN ('round_robin', 'random', 'failover', 'weighted')),
    match_regex TEXT,                -- 可选：正则匹配
    retry_enabled BOOLEAN NOT NULL DEFAULT TRUE,
    max_retries INTEGER NOT NULL DEFAULT 3,
    first_token_timeout_secs INTEGER NOT NULL DEFAULT 30,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

### group_items 表

```sql
CREATE TABLE group_items (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    group_id INTEGER NOT NULL REFERENCES groups(id) ON DELETE CASCADE,
    channel_id INTEGER NOT NULL REFERENCES channels(id) ON DELETE CASCADE,
    model_name TEXT NOT NULL,        -- 上游实际模型名
    priority INTEGER NOT NULL DEFAULT 1,
    weight INTEGER NOT NULL DEFAULT 100,
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    UNIQUE(group_id, channel_id, model_name)
);
```

### model_pricing 表

```sql
CREATE TABLE model_pricing (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    model TEXT NOT NULL UNIQUE,
    input_per_million REAL NOT NULL,
    output_per_million REAL NOT NULL,
    cache_read_per_million REAL,
    cache_creation_per_million REAL,
    source TEXT NOT NULL DEFAULT 'manual',  -- 'models.dev' | 'manual'
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## API 端点设计

### 代理 API（客户端使用，无需 /api 前缀）

客户端只需配置 `http://ip:port`，SDK 自动拼接 `/v1/*` 路径。

| 端点 | 方法 | 说明 |
|------|------|------|
| `/v1/chat/completions` | POST | OpenAI Chat Completions |
| `/v1/responses` | POST | OpenAI Responses |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/embeddings` | POST | OpenAI Embedding |
| `/v1/images/generations` | POST | OpenAI Images |
| `/v1/models` | GET | 获取可用模型列表 |

### 管理 API（Web 面板使用）

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/admin/auth/setup` | POST | 初始化管理员 |
| `/api/v1/admin/auth/login` | POST | 登录 |
| `/api/v1/admin/auth/password` | PUT | 修改密码 |
| `/api/v1/admin/channels` | GET/POST | 渠道列表/创建 |
| `/api/v1/admin/channels/:id` | GET/PUT/DELETE | 渠道详情/更新/删除 |
| `/api/v1/admin/groups` | GET/POST | 分组列表/创建 |
| `/api/v1/admin/groups/:id` | GET/PUT/DELETE | 分组详情/更新/删除 |
| `/api/v1/admin/groups/:id/items` | POST | 添加分组项 |
| `/api/v1/admin/api-keys` | GET/POST | API Key 列表/创建 |
| `/api/v1/admin/api-keys/:id` | DELETE | 删除 API Key |
| `/api/v1/admin/stats/overview` | GET | 统计概览 |
| `/api/v1/admin/stats/daily` | GET | 按天统计 |
| `/api/v1/admin/stats/models` | GET | 按模型统计 |
| `/api/v1/admin/pricing` | GET/PUT | 定价管理 |

## Web 前端技术栈

| 技术 | 版本 | 说明 |
|------|------|------|
| React | 18+ | UI 框架 |
| TypeScript | 5+ | 类型安全 |
| Vite | 5+ | 构建工具 |
| Tailwind CSS | 3+ | 样式 |
| shadcn/ui | — | 组件库（参考 AxonHub） |
| TanStack Query | — | 数据获取 |
| React Router | — | 路由 |

## 前端目录结构

```
frontend/
├── src/
│   ├── main.tsx
│   ├── App.tsx
│   ├── api/                    # API 客户端
│   │   ├── channels.ts
│   │   ├── groups.ts
│   │   ├── stats.ts
│   │   └── pricing.ts
│   ├── components/             # 通用组件
│   │   ├── ui/                 # shadcn/ui 组件
│   │   └── layout/             # 布局组件
│   ├── pages/                  # 页面
│   │   ├── Dashboard.tsx       # 统计概览
│   │   ├── Channels.tsx        # 渠道管理
│   │   ├── Groups.tsx          # 分组管理
│   │   └── Settings.tsx        # 设置
│   └── types/                  # TypeScript 类型
│       └── index.ts
├── index.html
├── package.json
├── tsconfig.json
├── tailwind.config.js
└── vite.config.ts
```

## 嵌入式部署

前端构建产物嵌入到 Rust 二进制中：

```rust
// 使用 rust-embed 或 include_dir
#[derive(RustEmbed)]
#[folder = "frontend/dist"]
struct Frontend;

// axum 路由
let app = Router::new()
    .nest("/admin/api", admin_api_routes)
    .fallback(serve_frontend);
```

## 模型匹配优先级

请求的 `model` 字段匹配分组的规则：

1. **精确匹配优先**: `groups.name` == model → 直接使用该分组
2. **正则匹配**: 按 `groups.match_regex` 匹配，多个匹配时取第一个定义的
3. **通配符匹配**: `channel_model_maps.source_model` 支持 `*`（任意长度）和 `?`（单字符）
4. **匹配顺序**: 精确 > 正则 > 通配符

**示例**:
- 请求 `model = "claude-sonnet-4-20250514"`
- 匹配顺序：`groups.name = "claude-sonnet-4-20250514"` > `groups.match_regex = "^claude-.*"` > `channel_model_maps.source = "claude-*"`

## 渠道多 Key 选择策略

当一个渠道有多个 Key 时，使用**轮询（Round Robin）**选择，避免单个 Key 触发速率限制。

## 配置热更新机制

Web 面板写入 SQLite 后，内存缓存通过**写穿透**方式更新：

1. Web API 写入 SQLite
2. 写入成功后，更新内存缓存（Mutex/RwLock 保护的 HashMap）
3. 下次请求立即使用新配置

**注意**: TOML 基础配置需要重启才能生效。

## 健康探测策略（P2）

| 项目 | 说明 |
|------|------|
| 探测端点 | `/v1/models`（轻量级，不消耗 Token） |
| 探测间隔 | 默认 60 秒 |
| 判断标准 | HTTP 状态码 + 响应时间 |
| 失败阈值 | 连续 3 次失败标记为不健康 |
| 恢复条件 | 连续 2 次成功恢复为健康 |

## 成本计算降级策略

| 场景 | 行为 |
|------|------|
| 启动时 models.dev 不可达 | 使用本地 `model_pricing` 表，日志警告 |
| 运行时刷新失败 | 继续使用上次成功拉取的数据 |
| 本地覆盖优先级 | `source = 'manual'` 的记录优先于 `source = 'models.dev'` |
