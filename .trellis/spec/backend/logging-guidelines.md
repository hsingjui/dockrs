# 日志指南（后端）

> 本项目的日志约定。

## 概览

- 日志库：**tracing**（见 `AGENTS.md` Tech Stack）。
- 当前仓库尚未落地日志初始化代码，遵循下面约定并在 `crates/server` 接入 tracing（fmt + subscriber）。

## 日志等级

- `debug`：细粒度调试信息。实时流（logs/stats）的原始数据**不要**在服务端刷屏。
- `info`：常规事件（服务启动、Agent 连接/断开、认证结果、关键容器/Compose 操作）。
- `warn`：可恢复的异常情况。
- `error`：确实失败、影响功能的错误。

## 结构化日志

- 使用 tracing 的结构化字段（span / field），而非字符串拼接。
- 保持结构化，便于后续采集与分析。

## 要记录

- 关键操作：创建/删除容器、镜像/Volume 删除、Compose up/down/pull/restart。
- Agent 心跳与连接状态变化。
- 错误发生时的上下文（哪个环境、哪个操作）。

## 不要记录

- 密码、token、Session secret、API key 等敏感信息。
- PII（个人身份信息）。
- 实时容器的原始 stdout 日志原样回写——实时日志应通过 WebSocket 转发到前端，不回写服务端日志文件。

## 示例

```rust
// structured fields + span，而不是字符串拼装
#[tracing::instrument(skip_all, fields(env_id = %env.id, container_id = %id))]
async fn restart_container(env: &Environment, id: &str) -> Result<(), Error> {
    tracing::info!("restarting container");
    docker.restart(id).await?;
    tracing::warn!("restart took longer than expected", elapsed_ms = elapsed.as_millis());
    Ok(())
}
```

> 日志通过 `tracing` 的 fmt subscriber 接入 `crates/server`（当前尚未落地初始化代码）。