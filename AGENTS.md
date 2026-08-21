## Project

Dockrs 是一个中文优先、面向个人用户的开源 Docker 管理面板。

目标：

- UI 简洁、美观，默认浅色，支持深色模式
- 部署简单，尽量保持单二进制 / 单容器
- 支持本机 Docker 和远程 Agent
- Docker 是容器、镜像、网络等资源的唯一事实来源

## Repository

```text
dockrs/
├── crates/
│   ├── server/   # HTTP API、WebSocket、认证、SQLite、静态前端
│   ├── agent/    # 远程节点 Agent，主动连接 Server
│   └── docker/   # Docker 操作封装，Server 和 Agent 共用
└── web/          # React SPA
```

不要增加新的 Rust crate，除非确实存在明确且长期的独立边界。

## Tech Stack

### Backend

- Rust
- Axum
- Tokio
- Bollard
- SQLx + SQLite
- tower-sessions
- Argon2
- Utoipa
- tracing

### Frontend

- React + TypeScript
- Vite
- Tailwind CSS
- shadcn/ui
- React Router
- TanStack Query
- TanStack Table
- TanStack Virtual
- React Hook Form + Zod
- Zustand
- Monaco Editor
- xterm.js
- i18next

## Architecture

普通操作使用 REST：

```text
Browser -> Server -> Docker
Browser -> Server -> Agent -> Docker
```

实时功能使用 WebSocket：

- 容器日志
- 容器 Exec / Terminal
- Stats
- Agent 通信

Agent 必须主动连接 Server，不要求用户暴露 Docker TCP API。

## Crate Responsibilities

### `crates/docker`

只负责 Docker Engine 相关能力：

- containers
- images
- networks
- volumes
- logs
- stats
- exec

优先通过 Bollard 实现。

不要包含：

- HTTP handler
- 数据库
- 登录认证
- 前端相关逻辑

### `crates/server`

负责：

- Axum API
- WebSocket
- 登录和 Session
- SQLite
- Environment / Agent 管理
- Compose 项目
- OpenAPI
- 前端静态资源

Docker 资源不要镜像保存到 SQLite。

### `crates/agent`

负责：

- 主动连接 Dockrs Server
- 心跳和身份认证
- 接收 Docker 操作请求
- 调用 `crates/docker`
- 转发 logs / stats / exec 等流式数据

Agent 尽量保持无状态。

## Frontend Structure

优先按功能组织：

```text
web/src/
├── api/
├── components/
├── features/
│   ├── auth/
│   ├── containers/
│   ├── images/
│   ├── networks/
│   ├── volumes/
│   ├── compose/
│   ├── environments/
│   └── settings/
├── hooks/
├── lib/
├── stores/
└── locales/
```

服务端状态使用 TanStack Query。

Zustand 只保存 UI / 客户端状态，不要复制服务端数据。

## API Rules

- CRUD 和普通操作使用 REST
- logs / stats / exec / agent 使用 WebSocket
- API 前缀统一为 `/api`
- 返回明确的 HTTP status code
- 错误响应保持统一结构
- Rust API schema 通过 OpenAPI 暴露，前端类型尽量自动生成

## Docker Rules

- Docker Engine 是资源事实来源
- 不在数据库维护 containers/images/networks 的副本
- 创建、编辑容器时明确区分 `update` 和 `recreate`
- 不直接暴露未加密 Docker daemon
- 删除容器、镜像、Volume 等危险操作必须明确确认

## Compose

- 原始 `compose.yaml` 是 canonical data
- 使用 Monaco Editor 编辑 YAML
- 不自行实现完整 Compose 引擎
- 优先调用 Docker Compose 能力完成 config/up/down/pull/restart
- UI 解析仅用于展示和辅助编辑

## UI Guidelines

- 中文优先
- 默认浅色，深色作为可选主题
- 高信息密度，但避免拥挤
- 优先复用 shadcn/ui 基础组件
- 不直接套默认 shadcn 样式，保持 Dockrs 自己的视觉语言
- 危险操作使用明确的危险态和二次确认
- 表格、日志、终端等高频界面优先考虑性能

## Code Style

Rust：

```bash
# 修改后端 Rust 代码后，在仓库根目录执行
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features

# 提交前或涉及行为变更时执行
cargo test --workspace
```

Frontend：

```bash
# 修改前端代码后，在 web/ 目录执行
pnpm check   # = biome check .（Biome 及其配置在 web/）

# 提交前或涉及构建配置、类型与打包行为变更时执行
pnpm build
```

原则：

- 保持实现简单
- 避免过早抽象
- 不为未来假设增加复杂度
- 优先清晰的类型和错误处理
- 不使用 `unwrap()` 处理可预期错误
- 新功能应优先复用现有模块

## Before Finishing

修改完成前至少确认：

- 修改后端 Rust 代码后，`cargo fmt --all -- --check` 和 `cargo clippy --workspace --all-targets --all-features` 通过
- 修改前端代码后，在 `web/` 执行的 `pnpm check` 通过
- Rust 可以编译
- 前端可以构建
- 没有破坏现有 API
- 没有把 Docker 状态重复写入数据库
- 实时流没有无限制积累内存
- 危险操作有明确保护
<!-- TRELLIS:START -->

# Trellis Instructions

These instructions are for AI assistants working in this project.

This project is managed by Trellis. The working knowledge you need lives under `.trellis/`:

- `.trellis/workflow.md` — development phases, when to create tasks, skill routing
- `.trellis/spec/` — package- and layer-scoped coding guidelines (read before writing code in a given layer)
- `.trellis/workspace/` — per-developer journals and session traces
- `.trellis/tasks/` — active and archived tasks (PRDs, research, jsonl context)

If a Trellis command is available on your platform (e.g. `/trellis:finish-work`, `/trellis:continue`), prefer it over manual steps. Not every platform exposes every command.

If you're using Codex or another agent-capable tool, additional project-scoped helpers may live in:

- `.agents/skills/` — reusable Trellis skills
- `.codex/agents/` — optional custom subagents

Managed by Trellis. Edits outside this block are preserved; edits inside may be overwritten by a future `trellis update`.

<!-- TRELLIS:END -->
