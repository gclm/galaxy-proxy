# 模块划分

## 目录结构

```
galaxy-proxy/
├── src/
│   ├── main.rs                     # 入口，CLI 参数解析，服务启动
│   ├── lib.rs                      # 库入口，模块导出
│   ├── config.rs                   # TOML 配置加载与解析
│   ├── error.rs                    # 统一错误类型
│   │
│   ├── api/                        # HTTP 层
│   │   ├── mod.rs
│   │   ├── router.rs               # axum Router 定义
│   │   ├── handlers/
│   │   │   ├── mod.rs
│   │   │   ├── proxy/              # 代理 API（/v1/*）
│   │   │   │   ├── mod.rs
│   │   │   │   ├── chat.rs         # /v1/chat/completions
│   │   │   │   ├── responses.rs    # /v1/responses
│   │   │   │   ├── messages.rs     # /v1/messages
│   │   │   │   ├── embeddings.rs   # /v1/embeddings
│   │   │   │   ├── images.rs       # /v1/images/generations
│   │   │   │   └── models.rs       # /v1/models
│   │   │   └── admin/              # 管理 API（/api/v1/admin/*）
│   │   │       ├── mod.rs
│   │   │       ├── auth.rs         # 认证 API
│   │   │       ├── channels.rs     # 渠道管理 API
│   │   │       ├── groups.rs       # 分组管理 API
│   │   │       ├── api_keys.rs     # API Key 管理 API
│   │   │       ├── stats.rs        # 统计查询 API
│   │   │       └── pricing.rs      # 定价管理 API
│   │   └── middleware/
│   │       ├── mod.rs
│   │       ├── auth.rs             # API Key 认证（代理 API）
│   │       ├── jwt.rs              # JWT 认证（管理 API）
│   │       ├── trace.rs            # 请求追踪
│   │       └── timeout.rs          # 超时控制
│   │
│   ├── protocol/                   # 协议转换层（参考 AxonHub）
│   │   ├── mod.rs
│   │   ├── model.rs                # 统一内部模型（LlmRequest, LlmResponse）
│   │   ├── interfaces.rs           # Inbound/Outbound trait 定义
│   │   │
│   │   ├── inbound/                # 入站转换器
│   │   │   ├── mod.rs
│   │   │   ├── openai_chat.rs      # OpenAI Chat → 统一格式
│   │   │   ├── openai_responses.rs # OpenAI Responses → 统一格式
│   │   │   └── anthropic.rs        # Anthropic Messages → 统一格式
│   │   │
│   │   └── outbound/               # 出站转换器
│   │       ├── mod.rs
│   │       ├── openai_chat.rs      # 统一格式 → OpenAI Chat
│   │       ├── openai_responses.rs # 统一格式 → OpenAI Responses
│   │       └── anthropic.rs        # 统一格式 → Anthropic Messages
│   │
│   ├── proxy/                      # 代理核心
│   │   ├── mod.rs
│   │   ├── dispatcher.rs           # 请求分发（协议识别 + 路由）
│   │   ├── forwarder.rs            # 请求转发（流式/非流式）
│   │   └── passthrough.rs          # 同格式直通优化
│   │
│   ├── scheduler/                  # 负载均衡与调度（参考 Sub2API）
│   │   ├── mod.rs
│   │   ├── scorer.rs               # 加权评分模型
│   │   ├── selector.rs             # Top-K + 加权随机选择
│   │   ├── sticky.rs               # 粘性会话管理
│   │   └── failover.rs             # 故障转移与拉黑
│   │
│   ├── channel/                    # 渠道管理
│   │   ├── mod.rs
│   │   ├── model.rs                # 渠道数据模型
│   │   ├── health.rs               # 健康探测
│   │   └── model_map.rs            # 模型映射
│   │
│   ├── stats/                      # 统计模块
│   │   ├── mod.rs
│   │   ├── recorder.rs             # 请求记录
│   │   ├── aggregator.rs           # 按天聚合
│   │   └── cost.rs                 # 成本计算（models.dev 数据）
│   │
│   ├── db/                         # 数据库层
│   │   ├── mod.rs
│   │   ├── schema.rs               # Schema 定义与迁移
│   │   ├── models.rs               # 数据库模型
│   │   └── usage_log.rs            # 用量日志 CRUD
│   │
│   └── stream/                     # 流式处理
│       ├── mod.rs
│       ├── sse.rs                  # SSE 解析器
│       └── event.rs                # 流事件类型
│
├── frontend/                       # Web 管理面板
│   ├── src/
│   │   ├── main.tsx
│   │   ├── App.tsx
│   │   ├── api/                    # API 客户端
│   │   ├── components/             # 通用组件
│   │   ├── pages/                  # 页面
│   │   └── types/                  # TypeScript 类型
│   ├── index.html
│   ├── package.json
│   ├── tsconfig.json
│   └── vite.config.ts
│
├── config.toml                     # 基础配置文件
├── Cargo.toml
└── docs/                           # 项目知识库
```

## 模块职责

| 模块 | 职责 | 依赖 |
|------|------|------|
| `api` | HTTP 路由、请求处理、中间件 | `protocol`, `proxy`, `scheduler` |
| `api::admin` | 管理 API（渠道/分组/统计/定价） | `channel`, `stats`, `db` |
| `api::auth` | 认证 API（初始化/登录/JWT） | `db` |
| `protocol` | 协议转换（Inbound/Outbound） | `stream` |
| `proxy` | 请求分发、转发、直通优化 | `protocol`, `scheduler`, `channel` |
| `scheduler` | 负载均衡、粘性会话、故障转移 | `channel` |
| `channel` | 渠道管理、健康探测、模型映射 | `db` |
| `stats` | 统计记录、聚合、成本计算 | `db` |
| `db` | 数据库 Schema、CRUD | — |
| `stream` | SSE 解析、流事件处理 | — |
| `config` | TOML 配置加载 | — |
| `frontend` | Web 管理面板（React） | — |

## 请求处理流程

```
客户端请求
    │
    ▼
api::handlers::*           # HTTP 处理器入口
    │
    ▼
proxy::dispatcher          # 识别协议类型，选择 Inbound
    │
    ▼
protocol::inbound::*       # 客户端格式 → 统一格式
    │
    ▼
scheduler::selector        # 选择渠道（加权评分 + 粘性会话）
    │
    ▼
channel::model_map         # 应用模型映射
    │
    ▼
proxy::forwarder           # 转发请求到上游
    │
    ▼
protocol::outbound::*      # 统一格式 → 上游格式（如果需要转换）
    │
    ▼
上游响应（流式/非流式）
    │
    ▼
protocol::outbound::*      # 上游响应 → 统一格式
    │
    ▼
protocol::inbound::*       # 统一格式 → 客户端格式
    │
    ▼
stats::recorder            # 记录统计
    │
    ▼
返回客户端
```

## 同格式直通优化

当入站和出站协议相同时（如 Anthropic→Anthropic），跳过协议转换：

```
客户端请求
    │
    ▼
proxy::passthrough         # 检测到同格式
    │
    ▼
直接转发（仅替换 URL + 认证头）
    │
    ▼
上游响应（流式/非流式）
    │
    ▼
解析 SSE 事件提取 usage 信息（不解析业务字段）
    │
    ▼
记录统计 + 返回客户端
```

**统计记录**: 即使直通模式，也需要解析 SSE 事件中的 `usage` 字段（如 `message_delta.usage`）来记录 Token 用量。

## 流式转换缓冲策略

跨协议流式转换时，事件粒度不同：

| 上游 | 下游 | 策略 |
|------|------|------|
| Anthropic (block 级) | OpenAI Chat (token 级) | 收到完整 block 后立即转为多个 token 事件 |
| OpenAI Chat (token 级) | Anthropic (block 级) | 缓冲 token，收到完整句子后转为 block |
| OpenAI Responses (part 级) | OpenAI Chat (token 级) | 收到 part 后立即转为 token 事件 |

**原则**: 尽量减少缓冲延迟，收到可转换单元后立即转发。

## 故障转移重试策略

| 场景 | 行为 |
|------|------|
| 非流式请求失败 | 重新走整个 Pipeline，选择下一个渠道 |
| 流式请求未开始返回 | 重新走整个 Pipeline，选择下一个渠道 |
| 流式请求已开始返回 | 向客户端返回错误事件，不重试（避免部分响应） |
| 同渠道重试 | 最多 N 次，间隔 500ms |
| 跨渠道切换 | 最多 M 次，排除已失败渠道 |
