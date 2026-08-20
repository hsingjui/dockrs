# 错误处理（后端）

> 本项目的错误处理约定。

## 概览

- 后端使用 **Axum**，错误通过返回明确的 HTTP status code 表达。
- 错误响应保持**统一结构**（见 `AGENTS.md` API Rules）。
- 不使用 `unwrap()` 处理可预期错误；优先清晰的类型与错误处理。

## 错误类型

- 使用 Rust 的 `Result` 与自定义错误类型传播错误。
- 错误类型应能区分：客户端错误（4xx）、服务端错误（5xx）、Docker 操作失败、认证失败等。

## 错误处理模式

- 在边界（handler / WebSocket 消息处理）处把内部错误映射为统一的 API 错误响应。
- 可预期错误（用户输入、资源不存在、权限不足）必须显式处理，禁止 `unwrap()` / `expect()`。

## API 错误响应

- 返回明确的 HTTP status code。
- 错误响应保持统一结构（统一字段格式），便于前端统一处理。

## 示例

```rust
// 边界（Axum handler）处把内部错误映射为统一响应，不向上泄漏内部细节
async fn delete_container(Path(id): Path<String>) -> Result<Json<()>, ApiError> {
    docker
        .remove_container(&id)
        .await
        .map_err(|e| ApiError::docker(StatusCode::BAD_REQUEST, &id, e))?;
    Ok(Json(()))
}
```

> `ApiError` 统一携带 `status`/`message`/`code` 字段供前端解析；真实 handler 在 `crates/server` 实现路由后落地。

## 常见错误

- 用 `unwrap()` 处理可预期错误导致 panic。
- 错误响应结构不统一，前端难以统一解析。
- 危险操作（删除容器/镜像/Volume）缺少二次确认保护（见 `AGENTS.md` Docker Rules）。