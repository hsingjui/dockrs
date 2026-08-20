# 数据库指南（后端）

> 本项目的数据库约定。

## 概览

- 数据库：**SQLite**，通过 **SQLx** 访问（见 `AGENTS.md` Tech Stack）。
- 仅 **`crates/server`** 使用数据库；`crates/docker` 与 `crates/agent` 不接触数据库。
- **Docker 资源（containers/images/networks/volumes）不要镜像保存到 SQLite** —— Docker Engine 是资源唯一事实来源。数据库只存 Server 自身的状态（用户、Session、Environment/Agent 配置、Compose 项目元数据等）。

## 查询模式

- 使用 SQLx 的类型安全查询（`query!` / `query_as!` 宏）优先于运行时字符串拼接。
- 事务用于多步写入操作，保证原子性。

## 迁移

- 迁移通过 SQLx 管理（`sqlx::migrate!` / migrations 目录）。当前仓库尚未落地，遵循 SQLx 标准迁移目录约定。

## 命名约定

- 表名 / 列名使用 snake_case。
- 遵循 SQLx 与 SQLite 默认约定。

## 示例

```rust
// 类型安全查询（优于字符串拼装）
#[derive(sqlx::FromRow)]
struct Environment {
    id: String,
    name: String,
}

// 关键：只查/写 server 自身状态，不拉容器/镜像等 Docker 运行时数据
let envs: Vec<Environment> = sqlx::query_as::<_, Environment>(
    "SELECT id, name FROM environments WHERE agent_id = ?",
)
.bind(agent_id)
.fetch_all(&pool)
.await?;
```

> 以上是约定写法；`crates/server` 目前只有 `main.rs` 骨架，真实的查询/建表在首次 DB 接入时落地。

## 常见错误

- 把 Docker 运行时状态（容器列表、镜像列表）写入数据库 —— 违反「Docker 是唯一事实来源」原则，会造成状态漂移。
- 在 `crates/docker` 或 `crates/agent` 引入数据库依赖 —— 职责越界。