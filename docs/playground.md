# 操练场（Playground）开发方案

## 目标

提供一个独立的测试界面，验证代理的**完整请求管线**：API Key 认证 → 分组路由 → 渠道选择 → 协议转换 → 上游请求，并展示每一步的决策信息。

与渠道测试的区别：

| 维度 | 渠道测试 | 操练场 |
|------|----------|--------|
| 认证 | 渠道自身 Key | 系统 API Key |
| 路由 | 直连上游 | 经分组路由 |
| 协议 | 只测渠道拥有的端点 | 客户端协议可不同于上游协议 |
| 输出 | 连通性 + 模型响应 | 路由追踪 + 协议转换 + 模型响应 |
| 场景 | "这个渠道能不能通" | "客户端发 Anthropic 请求，代理能否正确路由到 OpenAI 渠道" |

---

## 后端

### 1. 新增 `PlaygroundRequest`

文件：`src/api/handlers/admin/playground.rs`（新建）

```rust
#[derive(Debug, Deserialize)]
pub struct PlaygroundRequest {
    /// 客户端使用的协议
    pub client_protocol: String,       // openai_chat | openai_response | anthropic | openai_embedding | openai_images
    /// 请求模型名（自由输入，用于匹配分组）
    pub model: String,
    /// 系统中的 API Key（用哪个 key 发请求）
    pub api_key_id: String,
    /// 自定义 User-Agent
    pub user_agent: Option<String>,
    /// 自定义 prompt（可选，默认使用内置测试 prompt）
    pub prompt: Option<String>,
}
```

### 2. 新增 `PlaygroundResponse`

```rust
#[derive(Debug, Serialize)]
pub struct PlaygroundResponse {
    pub success: bool,
    pub message: String,
    pub latency_ms: u64,

    // 路由追踪
    pub trace: RouteTrace,
    // 模型输入输出
    pub input_prompt: String,
    pub output_content: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct RouteTrace {
    /// 匹配到的分组（名称 + ID）
    pub matched_group: Option<GroupTrace>,
    /// 选中的渠道
    pub selected_channel: ChannelTrace,
    /// 客户端协议
    pub client_protocol: String,
    /// 上游协议
    pub upstream_protocol: String,
    /// 是否发生了协议转换
    pub protocol_converted: bool,
    /// 重试次数
    pub retry_count: u32,
}

#[derive(Debug, Serialize)]
pub struct GroupTrace {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct ChannelTrace {
    pub id: String,
    pub name: String,
    pub endpoint_url: String,
}
```

### 3. Handler 实现

文件：`src/api/handlers/admin/playground.rs`

核心逻辑：

```
1. 根据 api_key_id 查询 API Key，校验启用状态
2. 根据 client_protocol 构建 OpenAI/Anthropic 格式请求体
3. 注入 User-Agent（如有）
4. 直接调用 proxy::proxy_request（复用现有代理管线）
5. 捕获路由追踪信息
```

**路由追踪的实现方案**：

现有 `proxy::proxy_request` 不返回路由决策信息。最小改动方案：

- 在 `ProxyState` 中新增一个 `tracing::Span` 或 thread-local 结构，用于在路由过程中记录 trace 信息
- **或更简单**：新增一个 `proxy_request_with_trace` 方法，返回 `(ProxySuccess, SelectionResult)` 元组，由 playground handler 拆包

推荐后者，改动最小：

```rust
// src/proxy/mod.rs 新增
pub async fn proxy_request_with_trace(
    state: &ProxyState,
    api_key_id: Option<&str>,
    headers: &HeaderMap,
    body: &serde_json::Value,
    client_endpoint: &EndpointType,
) -> Result<(ProxySuccess, SelectionResult), ProxyError>
```

与 `proxy_request` 逻辑相同，但额外返回 `SelectionResult`。playground handler 从中提取 channel/group/endpoint 信息。

### 4. 路由注册

`src/api/router.rs`：

```rust
.nest("/api/v1/admin/playground", playground_routes(playground_state))
```

### 5. 文件变更清单

| 文件 | 变更 |
|------|------|
| `src/api/handlers/admin/playground.rs` | 新建，playground handler |
| `src/api/handlers/admin/mod.rs` | 新增 `pub mod playground` |
| `src/api/router.rs` | 注册 playground 路由，注入 state |
| `src/proxy/mod.rs` | 新增 `proxy_request_with_trace` 方法 |

---

## 前端

### 1. 新增页面

文件：`frontend/src/pages/Playground.tsx`

#### 布局

```
┌─────────────────────────────────────────────────┐
│  操练场                                          │
│  测试代理完整请求管线：认证 → 路由 → 转换 → 上游    │
├─────────────────────┬───────────────────────────┤
│  配置面板            │  结果面板                   │
│                     │                           │
│  客户端协议 [▼]      │  ┌─ 路由追踪 ──────────┐  │
│  模型名称 [____]     │  │ 分组: qwen3.6-plus   │  │
│  API Key   [▼]      │  │ 渠道: 阿里云 Coding  │  │
│  User-Agent[▼]      │  │ 客户端: OpenAI Chat  │  │
│  Prompt    [____]   │  │ 上游:   OpenAI Chat  │  │
│                     │  │ 转换:   否           │  │
│  [▶ 发送请求]        │  └────────────────────┘  │
│                     │                           │
│                     │  ┌─ 输入 ──────────────┐  │
│                     │  │ Hello! Please...    │  │
│                     │  └────────────────────┘  │
│                     │                           │
│                     │  ┌─ 输出 ──────────────┐  │
│                     │  │ Hello! How can I... │  │
│                     │  └────────────────────┘  │
│                     │                           │
│                     │  ✓ 成功  耗时: 2350ms     │
└─────────────────────┴───────────────────────────┘
```

#### 关键交互

- **客户端协议**：固定 5 选项（OpenAI Chat / OpenAI Responses / Anthropic / Embedding / Images）
- **模型名称**：自由输入文本框（带已注册模型联想）
- **API Key**：下拉选择系统内已启用的 API Key
- **User-Agent**：与渠道测试相同的预设列表
- **Prompt**：可选，默认使用内置测试 prompt
- 结果面板展示路由追踪 + 输入输出，终端风格渲染

### 2. API 层

文件：`frontend/src/api/types.ts` 新增类型：

```typescript
export interface PlaygroundRequest {
  client_protocol: string
  model: string
  api_key_id: string
  user_agent?: string
  prompt?: string
}

export interface PlaygroundResponse {
  success: boolean
  message: string
  latency_ms: number
  trace: RouteTrace
  input_prompt: string
  output_content: string | null
}

export interface RouteTrace {
  matched_group: { id: string; name: string } | null
  selected_channel: { id: string; name: string; endpoint_url: string }
  client_protocol: string
  upstream_protocol: string
  protocol_converted: boolean
  retry_count: number
}
```

文件：`frontend/src/api/admin.ts` 或复用 `channels.ts`，新增：

```typescript
playground: (data: PlaygroundRequest) =>
  apiClient.post<PlaygroundResponse>('/playground/test', data),
```

### 3. 路由和导航

`frontend/src/pages/index.ts` 新增导出。

路由注册（`App.tsx` 或路由配置处）：

```tsx
{ path: '/playground', element: <Playground /> }
```

`Sidebar.tsx` 导航栏新增（放在"请求日志"和"设置"之间）：

```tsx
{
  title: '操练场',
  href: '/playground',
  icon: FlaskConical,  // lucide-react
}
```

### 4. 文件变更清单

| 文件 | 变更 |
|------|------|
| `frontend/src/pages/Playground.tsx` | 新建，操练场页面 |
| `frontend/src/pages/index.ts` | 新增导出 |
| `frontend/src/api/types.ts` | 新增 Playground 类型 |
| `frontend/src/api/channels.ts` 或新建 `admin.ts` | 新增 playground API |
| `frontend/src/components/layout/Sidebar.tsx` | 新增导航项 |
| 路由配置文件 | 注册 `/playground` 路由 |

---

## 实施顺序

```
Phase 1: 后端核心
  ├── 1.1 proxy/mod.rs 新增 proxy_request_with_trace
  ├── 1.2 新建 playground.rs handler
  ├── 1.3 注册路由
  └── 1.4 编译验证 + curl 测试

Phase 2: 前端页面
  ├── 2.1 类型定义 + API 函数
  ├── 2.2 Playground.tsx 页面（左右布局）
  ├── 2.3 Sidebar 导航 + 路由注册
  └── 2.4 浏览器端到端验证

Phase 3: 优化
  ├── 3.1 模型名称联想（从已有分组/渠道模型中取）
  ├── 3.2 路由追踪结果高亮（转换/未转换用不同颜色）
  └── 3.3 请求历史（localStorage 存储）
```

---

## 风险点

1. **proxy_request_with_trace 改动**：需要拆分现有 `proxy_request` 的返回值，确保不破坏现有调用方。方案是保留原方法不动，新增一个 with_trace 变体。
2. **API Key 明文**：playground 需要 API Key 明文发请求。后端根据 `api_key_id` 从 DB 取出明文 key 使用，不在响应中返回。前端下拉只显示 key 名称。
3. **超时**：操练场请求走完整管线，可能较慢。前端设 60s 超时，后端复用 proxy 的 30s 上游超时。
