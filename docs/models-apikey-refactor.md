# /v1/models 重构方案

> 参考 octopus 实现，将 `/v1/models` 改为只返回分组名，并为 API Key 增加模型选择功能

## 背景

### 当前问题

1. **`/v1/models` 返回内容混杂**：同时返回分组名和渠道模型，客户端无法区分
2. **绕过负载均衡**：客户端使用渠道模型名可直接命中渠道，失去分组的重试/负载均衡能力
3. **API Key 无模型限制**：所有 Key 都能访问所有模型，缺乏精细化控制

### 参考方案 (octopus)

- `/v1/models` 只返回分组名
- API Key 支持 `supported_models` 字段，逗号分隔
- 认证时过滤可用模型列表

---

## 重构内容

### 1. 后端：`/v1/models` 只返回分组名

**文件**: `src/api/handlers/proxy/models.rs`

```rust
use axum::{Json, extract::State, response::IntoResponse};
use sqlx::SqlitePool;

use crate::api::middleware::ApiKeyAuth;

/// 获取可用模型列表（仅分组名）
pub async fn list(auth: ApiKeyAuth, State(pool): State<SqlitePool>) -> impl IntoResponse {
    // 获取所有启用的分组名
    let groups = sqlx::query_scalar::<_, String>("SELECT name FROM groups WHERE enabled = 1")
        .fetch_all(&pool)
        .await
        .unwrap_or_default();

    // 获取 API Key 的支持模型列表
    let supported_models = get_supported_models(&pool, &auth.key_id).await;

    // 过滤：如果有 supported_models 限制，则只返回匹配的分组
    let models: Vec<String> = if let Some(supported) = supported_models {
        groups.into_iter().filter(|g| supported.contains(g)).collect()
    } else {
        groups
    };

    let data: Vec<serde_json::Value> = models
        .into_iter()
        .map(|name| {
            serde_json::json!({
                "id": name,
                "object": "model",
                "created": 0,
                "owned_by": "galaxy-router"
            })
        })
        .collect();

    Json(serde_json::json!({
        "object": "list",
        "data": data
    }))
    .into_response()
}

/// 获取 API Key 的支持模型列表
async fn get_supported_models(pool: &SqlitePool, key_id: &str) -> Option<Vec<String>> {
    let result = sqlx::query_scalar::<_, String>(
        "SELECT supported_models FROM api_keys WHERE id = ?"
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await
    .ok()??;

    if result.is_empty() {
        return None;
    }

    Some(
        result
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    )
}
```

---

### 2. 后端：API Key 增加 `supported_models` 字段

**文件**: `src/db/schema.sql`

```sql
ALTER TABLE api_keys ADD COLUMN supported_models TEXT DEFAULT '';
```

**文件**: `src/api/handlers/admin/api_keys.rs`

#### 2.1 更新 ApiKey 结构体

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ApiKey {
    pub id: String,
    pub name: String,
    pub api_key: String,
    pub enabled: bool,
    pub supported_models: Option<String>,  // 新增
    pub created_at: String,
    pub updated_at: String,
}
```

#### 2.2 更新 CreateApiKeyRequest

```rust
#[derive(Debug, Deserialize)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub supported_models: Option<String>,  // 新增，逗号分隔
}
```

#### 2.3 更新 UpdateApiKeyRequest

```rust
#[derive(Debug, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub name: Option<String>,
    pub enabled: Option<bool>,
    pub supported_models: Option<String>,  // 新增
}
```

#### 2.4 更新 SQL 查询

```rust
// list
"SELECT id, name, api_key, enabled, supported_models, created_at, updated_at FROM api_keys ORDER BY created_at DESC"

// get
"SELECT id, name, api_key, enabled, supported_models, created_at, updated_at FROM api_keys WHERE id = ?"

// create
"INSERT INTO api_keys (id, name, api_key, enabled, supported_models) VALUES (?, ?, ?, ?, ?)"
```

---

### 3. 后端：代理请求时验证模型访问权限

**文件**: `src/api/middleware.rs` (ApiKeyAuth 中间件)

```rust
/// 验证 API Key 并检查模型访问权限
pub async fn validate_model_access(
    pool: &SqlitePool,
    key_id: &str,
    model: &str,
) -> Result<(), ProxyError> {
    let supported = sqlx::query_scalar::<_, String>(
        "SELECT supported_models FROM api_keys WHERE id = ? AND enabled = 1"
    )
    .bind(key_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| ProxyError::DatabaseError(e.to_string()))?;

    // 如果 supported_models 为空，允许所有模型
    if let Some(models_str) = supported {
        if !models_str.is_empty() {
            let allowed: Vec<&str> = models_str
                .split(',')
                .map(|s| s.trim())
                .collect();
            if !allowed.contains(&model) {
                return Err(ProxyError::NoAvailableChannel(
                    format!("API Key 无权访问模型: {}", model)
                ));
            }
        }
    }

    Ok(())
}
```

**文件**: `src/proxy/mod.rs` (在 `handle_proxy_request` 中调用)

```rust
// 在 proxy_request 和 proxy_stream 开头添加
validate_model_access(&state.pool, api_key_id.unwrap_or(""), model).await?;
```

---

### 4. 前端：API Key 管理增加模型选择

**文件**: `frontend/src/api/types.ts`

```typescript
export interface ApiKey {
  id: string
  name: string
  api_key: string
  enabled: boolean
  supported_models: string | null  // 新增
  created_at: string
  updated_at: string
}

export interface CreateApiKeyRequest {
  name: string
  supported_models?: string  // 新增
}

export interface UpdateApiKeyRequest {
  name?: string
  enabled?: boolean
  supported_models?: string  // 新增
}
```

**文件**: `frontend/src/components/ApiKeyForm.tsx` (新增或修改)

```tsx
import { useState, useEffect } from 'react'
import { groupApi } from '@/api/groups'

interface ApiKeyFormProps {
  apiKey?: ApiKey
  onSubmit: (data: CreateApiKeyRequest | UpdateApiKeyRequest) => Promise<void>
  onCancel: () => void
}

export function ApiKeyForm({ apiKey, onSubmit, onCancel }: ApiKeyFormProps) {
  const [name, setName] = useState(apiKey?.name ?? '')
  const [supportedModels, setSupportedModels] = useState<string[]>(
    apiKey?.supported_models?.split(',').map(s => s.trim()).filter(Boolean) ?? []
  )
  const [availableGroups, setAvailableGroups] = useState<string[]>([])

  useEffect(() => {
    // 获取所有分组名作为可选模型
    groupApi.list().then(groups => {
      setAvailableGroups(groups.map(g => g.name))
    })
  }, [])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    await onSubmit({
      name,
      supported_models: supportedModels.length > 0 
        ? supportedModels.join(',') 
        : undefined
    })
  }

  const toggleModel = (model: string) => {
    setSupportedModels(prev => 
      prev.includes(model) 
        ? prev.filter(m => m !== model)
        : [...prev, model]
    )
  }

  return (
    <form onSubmit={handleSubmit}>
      {/* 名称输入 */}
      <input 
        value={name} 
        onChange={e => setName(e.target.value)}
        placeholder="API Key 名称"
        required
      />
      
      {/* 模型选择 */}
      <div>
        <label>可用模型（留空表示全部可用）</label>
        <div className="model-grid">
          {availableGroups.map(group => (
            <label key={group} className="model-checkbox">
              <input
                type="checkbox"
                checked={supportedModels.includes(group)}
                onChange={() => toggleModel(group)}
              />
              {group}
            </label>
          ))}
        </div>
      </div>

      <button type="submit">保存</button>
    </form>
  )
}
```

---

## 数据库迁移

```sql
-- migration: add_supported_models_to_api_keys.sql
ALTER TABLE api_keys ADD COLUMN supported_models TEXT DEFAULT '';
```

---

## 测试用例

### 1. /v1/models 返回验证

```bash
# 应该只返回分组名
curl -H "Authorization: Bearer gp-xxx" http://localhost:3000/v1/models

# 预期响应
{
  "object": "list",
  "data": [
    {"id": "gpt-4o", "object": "model", "created": 0, "owned_by": "galaxy-router"},
    {"id": "claude-3", "object": "model", "created": 0, "owned_by": "galaxy-router"}
  ]
}
```

### 2. API Key 模型过滤

```bash
# 创建只允许 gpt-4o 的 API Key
curl -X POST http://localhost:3000/api/v1/admin/api-keys \
  -H "Content-Type: application/json" \
  -d '{"name": "limited-key", "supported_models": "gpt-4o,gpt-4o-mini"}'

# 使用该 Key 调用 /v1/models，应该只返回 gpt-4o 和 gpt-4o-mini
curl -H "Authorization: Bearer gp-limited-key" http://localhost:3000/v1/models

# 使用该 Key 调用不允许的模型，应该返回 403
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer gp-limited-key" \
  -d '{"model": "claude-3", "messages": [...]}'
# 预期: {"error": {"message": "API Key 无权访问模型: claude-3"}}
```

---

## 实施步骤

| 步骤 | 内容 | 预计时间 |
|------|------|----------|
| 1 | 数据库迁移脚本 | 5 分钟 |
| 2 | 后端：修改 `/v1/models` 只返回分组 | 15 分钟 |
| 3 | 后端：API Key 结构体和 CRUD 更新 | 20 分钟 |
| 4 | 后端：代理请求模型访问验证 | 15 分钟 |
| 5 | 前端：API Key 表单增加模型选择 | 30 分钟 |
| 6 | 测试验证 | 20 分钟 |

**总计**: 约 1.5-2 小时

---

## 兼容性说明

- `supported_models` 为空表示允许所有模型（向后兼容）
- 现有 API Key 自动获得全部模型访问权限
- `/v1/models` 返回格式不变，只是内容从"分组+渠道模型"变为"仅分组"

---

## 参考

- octopus 实现: `internal/op/group.go:25` GroupListModel
- octopus API Key 模型: `internal/model/apikey.go`
- octopus 认证中间件: `internal/server/middleware/auth.go:80`
