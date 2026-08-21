# 目录结构（后端）

> 后端代码在本项目的组织方式。

## 概览

后端是 Rust workspace，按 **crate 边界** 划分职责。约定来源为根目录 `AGENTS.md`，本文件是其后端部分的可执行摘要。

当前仓库状态：`Cargo.toml` 的 `workspace.members` 已注册 `crates/server` 和 `crates/docker`（`crates/agent` 尚未创建）。

## 目录布局

```text
crates/
├── server/   # HTTP API、WebSocket、认证、SQLite、静态前端
├── agent/    # 远程节点 Agent，主动连接 Server（尚未创建）
└── docker/   # Docker 操作封装，Server 和 Agent 共用
```

## Crate 职责（commons 边界）

- **`crates/docker`**：只负责 Docker Engine 能力（containers/images/networks/volumes/logs/stats/exec），优先通过 Bollard 实现。**禁止包含** HTTP handler、数据库、登录认证、前端逻辑。
- **`crates/server`**：Axum API、WebSocket、登录与 Session、SQLite、Environment/Agent 管理、Compose 项目、OpenAPI、前端静态资源。Docker 资源**不要**镜像保存到 SQLite。
- **`crates/agent`**：主动连接 Server、心跳与身份认证、接收 Docker 操作请求、调用 `crates/docker`、转发 logs/stats/exec 流式数据。尽量无状态。**当前尚未创建，环境 API 保留受 token 保护的注册/心跳/快照接入口。**

## 模块组织

- 新功能先判断属于哪个 crate 的职责，再放入对应 crate。
- 不要仅为单一意图新增 crate；除非存在明确且长期的独立边界，一律复用现有 crate（见 `AGENTS.md`）。
- Server crate 内部按功能模块组织，普通操作走 REST，实时功能走 WebSocket（logs/exec/stats/agent）。

## 命名约定

- Rust 代码遵循 `cargo fmt` 与 cargo clippy 默认约定（snake_case 函数/变量、PascalCase 类型）。
- API 前缀统一为 `/api`。

## 示例

当前尚无成熟模块可供参考，以根目录 `AGENTS.md`「Crate Responsibilities」为准。