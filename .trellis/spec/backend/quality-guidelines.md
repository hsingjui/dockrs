# 质量指南（后端）

> 后端代码质量标准。

## 概览

- 语言遵循 `AGENTS.md` 代码风格：保持实现简单、避免过早抽象、不为未来假设增加复杂度、优先清晰的类型和错误处理、不使用 `unwrap()` 处理可预期错误、新功能优先复用现有模块。

## 修改后验证

修改 `crates/` 下的后端 Rust 代码后，在仓库根目录依次执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
```

提交前或涉及行为变更时，再执行完整测试：

```bash
cargo test --workspace
```

## 禁止模式

- `unwrap()` / `expect()` 处理可预期错误 → 用显式错误处理。
- 把 Docker 运行时状态（容器/镜像/网络/Volume）镜像写入数据库 → 违反「Docker 是唯一事实来源」。
- 为单一意图新增新的 Rust crate → 应复用现有 crate，除非存在明确长期独立边界。
- 实时流（logs/stats/exec）在服务端无限制累积内存 → 必须保证有界缓冲或即时转发。
- 危险操作（删除容器/镜像/Volume/Compose down 等）没有二次确认保护。

## 必用模式

- Compose 原始 `compose.yaml` 作为 canonical data 保存，不自行实现完整 Compose 引擎，优先调用 Docker Compose 能力。
- Docker 操作优先通过 Bollard 实现，且只放在 `crates/docker`。
- 实时功能（logs/exec/stats/agent 通信）走 WebSocket，普通 CRUD 走 REST（`/api` 前缀）。
- 创建、编辑容器时明确区分 `update` 与 `recreate`。

## 测试要求

- 用 `cargo test --workspace` 运行测试。
- 行为变更需补充/更新测试。
- 测试必须能验证逻辑是否被破坏，避免空洞测试。

## 代码评审清单

- 代码可编译（`cargo build` / `cargo clippy` 通过）。
- 未破坏现有 API。
- 未把 Docker 状态重复写入数据库。
- 实时流无无限内存累积。
- 危险操作有明确保护。
- 涉及受信任边界外的输入有校验与正确错误处理。