# 模型信息数据格式

## 数据源

`models.dev API` → 按 `providers` 白名单过滤 → 扁平化输出

models.dev API 只提供 `id` + `cost`（$/1M tokens），
模型元数据（max_tokens、supports_* 等）来自 litellm 内置数据，
在 `sync_prices.py` 脚本中合并。galaxy-proxy 需自行从 models.dev
获取全部可用字段（未来 API 扩展后自动生效），同时支持手动补充。

## DB 表结构 `model_info`

```sql
CREATE TABLE model_info (
    id TEXT PRIMARY KEY,               -- 模型标识，如 'gpt-5.5', 'claude-sonnet-4-6'
    provider TEXT NOT NULL,             -- litellm provider, 如 'openai', 'anthropic'
    mode TEXT NOT NULL DEFAULT 'chat',  -- chat / embedding / image

    -- 定价（$/1M tokens，与上游一致）
    input_price REAL,                   -- 输入价格
    output_price REAL,                  -- 输出价格
    cache_read_price REAL,             -- 缓存读取价格
    cache_creation_price REAL,         -- 缓存写入价格

    -- 上下文窗口
    max_input_tokens INTEGER,
    max_output_tokens INTEGER,

    -- 能力标识（布尔值，NULL 表示未知）
    supports_function_calling BOOLEAN,
    supports_reasoning BOOLEAN,
    supports_vision BOOLEAN,
    supports_pdf_input BOOLEAN,
    supports_prompt_caching BOOLEAN,
    supports_system_messages BOOLEAN,
    supports_tool_choice BOOLEAN,

    -- 来源与时间
    source TEXT NOT NULL DEFAULT 'remote',  -- remote / manual
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## config.toml 配置

```toml
[pricing]
cache_path = "data/pricing_cache.json"
refresh_interval_hours = 24
# 只导入这些 provider 的模型
providers = [
    "openai",
    "anthropic",
    "deepseek",
    "google",
    "zhipuai",
    "minimax",
    "xai",
    "moonshot",
    "xiaomi",
    "stepfun",
]
```

## 缓存文件格式 (pricing_cache.json)

```json
{
  "gpt-5.5": {
    "provider": "openai",
    "mode": "chat",
    "input_price": 5.0,
    "output_price": 30.0,
    "cache_read_price": 0.5,
    "cache_creation_price": null,
    "max_input_tokens": 1050000,
    "max_output_tokens": 128000,
    "supports_function_calling": true,
    "supports_reasoning": true,
    "supports_vision": true,
    "supports_pdf_input": true,
    "supports_prompt_caching": true,
    "supports_system_messages": true,
    "supports_tool_choice": true
  },
  "claude-sonnet-4-6": {
    "provider": "anthropic",
    "mode": "chat",
    "input_price": 3.0,
    "output_price": 15.0,
    "cache_read_price": 0.3,
    "cache_creation_price": 3.75,
    "max_input_tokens": 1000000,
    "max_output_tokens": 64000,
    "supports_function_calling": true,
    "supports_reasoning": true,
    "supports_vision": true,
    "supports_pdf_input": true,
    "supports_prompt_caching": true,
    "supports_system_messages": true,
    "supports_tool_choice": true
  }
}
```

定价单位统一为 `USD / 1M tokens`（与 models.dev API 一致）。
缓存文件按模型名排序，方便 diff 和人工审查。
