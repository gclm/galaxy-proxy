# Galaxy Proxy 项目知识库

AI 协议互转代理网关 — 需求、设计与决策记录。

## 审查修复记录 (2026-05-25)

| 问题 | 修复 |
|------|------|
| Embedding/Images 策略矛盾 | 明确仅支持 OpenAI 兼容上游，不支持的返回 400 |
| API Key 体系缺失 | 新增 `api_keys` 表（客户端侧） |
| 粘性会话存储 | 改用内存 HashMap（单机部署） |
| 并发控制方案 | 内存计数器 + tokio Semaphore |
| 渠道 Key 设计 | 简化为 channels 表的 JSON 字段 |
| 模型匹配优先级 | 定义精确 > 正则 > 通配符规则 |
| 项目愿景表述 | 修正为"不做多用户系统" |
| JWT 密钥设计 | 简化为 TOML 配置项 |
| 同格式直通统计 | 说明解析 SSE usage 字段 |
| 流式转换缓冲 | 定义跨协议缓冲策略 |
| 故障转移重试 | 定义流式/非流式重试行为 |
| 健康探测策略 | 定义端点、阈值、恢复条件 |
| 成本计算降级 | 定义 models.dev 不可达时的行为 |
| 配置热更新机制 | 说明写穿透更新内存缓存 |
| 数据库迁移 | 采用 sqlx migrate |
| 测试策略 | 采用 TDD 开发 |

## 目录结构

```
docs/
├── README.md                          # 本文件
├── requirements/
│   ├── 01-project-vision.md           # 项目愿景与目标
│   ├── 02-core-requirements.md        # 核心需求（从参考项目提取）
│   ├── 03-protocol-matrix.md          # 协议支持矩阵
│   └── 04-open-questions.md           # 待确认问题
├── architecture/
│   ├── 01-tech-stack.md               # 技术栈选型
│   ├── 02-module-design.md            # 模块划分
│   ├── 03-config-format.md            # 配置格式与数据库 Schema
│   └── 04-auth-system.md              # 认证与初始化系统
├── development/
│   └── 01-dev-plan.md                 # 开发方案与阶段规划
└── references/
    ├── axonhub-analysis.md            # ⭐ AxonHub 协议转换库（核心参考）
    ├── octopus-analysis.md            # Octopus 项目分析
    ├── sub2api-analysis.md            # Sub2API 项目分析
    └── ccg-gateway-analysis.md        # CCG Gateway 项目分析
```

## 阅读顺序

1. 先看 `requirements/01-project-vision.md` — 理解项目要解决什么问题
2. 再看 `requirements/02-core-requirements.md` — 了解具体功能需求
3. 看 `requirements/03-protocol-matrix.md` — 明确协议转换边界
4. 看 `requirements/04-open-questions.md` — 确认待决问题
5. 最后看 `architecture/` — 技术方案
