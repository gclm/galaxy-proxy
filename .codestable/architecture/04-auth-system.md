# 认证与初始化系统

## 设计目标

- 单用户模式（个人/小团队使用）
- 首次启动时初始化管理员密码
- Web 管理面板需要登录
- API 代理端点不需要登录（客户端用 API Key 认证）

## 初始化流程

```
首次启动
    │
    ▼
检测 database.users 表是否为空
    │
    ├─ 空 → 进入初始化模式
    │       ├─ Web 端：重定向到 /admin/setup
    │       └─ CLI 端：交互式输入密码
    │
    └─ 非空 → 正常启动
```

## 初始化页面

**路由**: `/admin/setup`（仅首次可用）

**流程**:
1. 输入管理员用户名（默认 `admin`）
2. 输入密码（最少 8 位）
3. 确认密码
4. 提交 → 创建用户 → 重定向到登录页

## 登录流程

**路由**: `/admin/login`

**认证方式**: JWT Token

```
POST /admin/api/auth/login
{
  "username": "admin",
  "password": "xxx"
}

Response:
{
  "token": "eyJ...",
  "expires_in": 86400
}
```

**Token 存储**:
- 前端：localStorage
- 后端：JWT 签名验证（密钥存储在 TOML 配置文件）

## 权限模型

单用户模式，不需要 RBAC，只区分两种访问：

| 访问类型 | 认证方式 | 说明 |
|---------|---------|------|
| 管理面板 | JWT Token | Web 管理 API |
| 代理 API | API Key | 客户端调用（`/v1/*`） |

## 数据库 Schema

### users 表

```sql
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL,       -- argon2id 哈希
    created_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);
```

## JWT 密钥配置

JWT 密钥存储在 TOML 配置文件中：

```toml
[auth]
jwt_secret = "your-secret-key-here"  # 首次运行时自动生成并写入 config.toml
token_expiry_hours = 24
```

**首次运行行为**:
1. 检测 `auth.jwt_secret` 是否存在
2. 如果不存在，随机生成并写入 config.toml
3. 日志提示用户保管好密钥

## API 端点

| 端点 | 方法 | 说明 | 认证 |
|------|------|------|------|
| `/api/v1/admin/auth/setup` | POST | 初始化管理员 | 无（仅首次） |
| `/api/v1/admin/auth/login` | POST | 登录 | 无 |
| `/api/v1/admin/auth/logout` | POST | 登出 | JWT |
| `/api/v1/admin/auth/me` | GET | 当前用户信息 | JWT |
| `/api/v1/admin/auth/password` | PUT | 修改密码 | JWT |

## 中间件

```rust
// axum 中间件链
let admin_routes = Router::new()
    .nest("/api/v1/admin", admin_api_routes)
    .layer(axum::middleware::from_fn(auth_middleware));

// auth_middleware 逻辑：
// 1. 检查 Authorization: Bearer {token}
// 2. 验证 JWT 签名和过期时间
// 3. 失败返回 401
```

## 前端路由守卫

```tsx
// React Router 路由守卫
function ProtectedRoute({ children }) {
  const token = localStorage.getItem('token');

  if (!token) {
    return <Navigate to="/admin/login" />;
  }

  // 验证 token 有效性
  const { data: user } = useQuery('/api/v1/admin/auth/me');

  if (!user) {
    return <Navigate to="/admin/login" />;
  }

  return children;
}
```

## 安全考虑

| 考虑点 | 措施 |
|--------|------|
| 密码存储 | argon2id 哈希（Rust: `argon2` crate） |
| JWT 密钥 | 首次运行时随机生成，存储在 TOML 配置文件 |
| Token 过期 | 默认 24 小时，可配置 |
| 暴力破解 | 登录失败延迟（1s, 2s, 4s...），可选 IP 限流 |
| HTTPS | 生产环境建议反向代理 + TLS |

## 配置扩展

TOML 配置中增加认证相关配置：

```toml
[auth]
token_expiry_hours = 24        # JWT Token 过期时间
max_login_attempts = 5         # 最大登录失败次数
lockout_minutes = 15           # 锁定时间
```

## CLI 初始化命令（可选）

```bash
# 命令行初始化（不用 Web）
galaxy-router init --username admin --password xxx

# 重置密码
galaxy-router reset-password --username admin --password new-xxx
```
