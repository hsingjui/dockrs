# 状态管理（前端）

> 本项目的状态管理约定。

## 概览

- 服务端数据：**TanStack Query**。
- 客户端/UI 状态：**Zustand**。
- 核心原则：**Zustand 只保存 UI / 客户端状态，不要复制服务端数据**（见 `AGENTS.md` Frontend）。

## 状态分类

- **服务端状态**（容器、镜像、网络、Volume、Environment 等）：由 TanStack Query 缓存，跟随后端事实来源。
- **客户端 / UI 状态**（主题、展开状态、分页/筛选临时态、编辑器状态等）：Zustand 或组件局部 state。
- **URL 状态**（路由参数）：用 React Router。

## 何时使用全局状态

- 多个不相关组件需要共享同一份 UI 状态时，才提升到 Zustand store（位于 `web/src/stores/`）。
- 能用组件局部 state 就不用全局。

## 服务端状态

- 用 Query 管理服务端数据与失效；不把 Query 结果复制进 Zustand。
- 变更后通过 Query 失效/重取刷新，保持与 Docker 事实来源同步。

## 常见错误

- 把服务端数据放入 Zustand -> 造成两份副本、数据漂移，违反「Server 状态用 TanStack Query」。
- 为单组件临时状态引入全局 store。