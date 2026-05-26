# Galaxy Proxy

AI 协议互转代理网关，支持 OpenAI Chat Completions、OpenAI Responses、Anthropic Messages 三种协议互转。

## 功能特性

- **协议互转**: OpenAI Chat ↔ OpenAI Responses ↔ Anthropic Messages
- **多端点渠道**: 一个渠道支持多种协议端点
- **负载均衡**: 自适应加权评分 + 粘性会话
- **统计系统**: 按 Key/模型/渠道/时间维度统计用量和成本
- **Web 管理**: 渠道、分组、API Key 管理

## 快速开始

### 从源码构建

```bash
# 克隆项目
git clone <repo-url>
cd galaxy-proxy

# 构建
make build

# 运行
make run
```

### Docker 运行

```bash
# 构建镜像
make docker

# 运行容器
make docker-run
```

## 配置

### 基础配置 (config.toml)

```toml
[server]
host = "127.0.0.1"
port = 8080

[database]
path = "data/galaxy.db"

[auth]
jwt_secret = ""  # 首次运行自动生成
```

### 渠道配置

通过 Web 管理面板或 API 配置渠道：

```json
POST /api/v1/admin/channels
{
  "name": "百炼 Coding Plan",
  "api_keys": ["sk-xxx"],
  "endpoints": [
    {"type": "openai_chat", "base_url": "https://coding.dashscope.aliyuncs.com/v1"},
    {"type": "anthropic", "base_url": "https://coding.dashscope.aliyuncs.com/apps/anthropic/v1"}
  ]
}
```

## API 端点

### 代理 API

| 端点 | 方法 | 说明 |
|------|------|------|
| `/v1/chat/completions` | POST | OpenAI Chat |
| `/v1/responses` | POST | OpenAI Responses |
| `/v1/messages` | POST | Anthropic Messages |
| `/v1/embeddings` | POST | OpenAI Embedding |
| `/v1/images/generations` | POST | OpenAI Images |
| `/v1/models` | GET | 模型列表 |

### 管理 API

| 端点 | 方法 | 说明 |
|------|------|------|
| `/api/v1/admin/auth/setup` | POST | 初始化管理员 |
| `/api/v1/admin/auth/login` | POST | 登录 |
| `/api/v1/admin/channels` | GET/POST | 渠道管理 |
| `/api/v1/admin/groups` | GET/POST | 分组管理 |
| `/api/v1/admin/api-keys` | GET/POST | API Key 管理 |
| `/api/v1/admin/stats/overview` | GET | 统计概览 |

## 客户端使用

```python
# OpenAI SDK
from openai import OpenAI

client = OpenAI(
    base_url="http://localhost:8080",
    api_key="gp-your-api-key"
)

response = client.chat.completions.create(
    model="gpt-4o",
    messages=[{"role": "user", "content": "Hello"}]
)
```

```python
# Anthropic SDK
from anthropic import Anthropic

client = Anthropic(
    base_url="http://localhost:8080",
    api_key="gp-your-api-key"
)

response = client.messages.create(
    model="claude-sonnet-4-20250514",
    max_tokens=1024,
    messages=[{"role": "user", "content": "Hello"}]
)
```

## 开发

```bash
# 运行测试
make test

# 代码检查
make check

# 监听自动构建
make watch
```

## 许可证

MIT
