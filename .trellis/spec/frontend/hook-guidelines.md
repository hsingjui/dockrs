# Hook 指南（前端）

> 本项目自定义 hook 约定。

## 概览

- 命名统一使用 `useXxx`。
- 服务端数据获取用 **TanStack Query**（`@tanstack/react-query`）；自定义客户端状态逻辑用自定义 hook。

## 自定义 Hook 模式

- `web/src/hooks/` 存放跨功能复用、无 UI 的状态逻辑。
- hook 返回最小、清晰的接口（数据 + 动作），调用方负责渲染。

## 数据获取

- 服务端状态使用 TanStack Query 管理（缓存、失效、重取）。
- 表单使用 **React Hook Form + Zod**（`zodResolver`）做校验，不手写内联校验逻辑。
- 高频实时数据（日志、终端、stats）走 WebSocket，而非轮询。

## 命名约定

- 通用 hook：`useXxx`（如 `useContainers`）。
- 表单 hook / 渲染逻辑 hook 遵循同样的 `use` 前缀。

## 常见错误

- 用自定义 hook 复制服务端数据到客户端状态 -> 应由 TanStack Query 管理。
- 在 hook 内做重 UI / 副作用 -> 保持 hook 单一职责。