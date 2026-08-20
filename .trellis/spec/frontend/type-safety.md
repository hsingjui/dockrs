# 类型安全（前端）

> 本项目的类型安全约定。

## 概览

- TypeScript 严格模式，构建用 `tsc -b`（`web` 的 `pnpm build` 先做类型检查）。
- 运行时校验：**Zod**（与 React Hook Form 通过 `@hookform/resolvers` 集成）。

## 类型组织

- 前端类型尽量由 Rust OpenAPI schema（Utoipa）**自动生成**，避免手写与后端重复的类型定义（见 `AGENTS.md` API Rules「前端类型尽量自动生成」）。
- 功能内局部类型就近定义在 `features/<name>/` 下；跨功能共享类型放共享模块。

## 校验

- 表单/输入校验用 Zod schema + `zodResolver`（react-hook-form）。
- 对不可信输入（用户提交、外部数据）在边界做运行时校验，而不只依赖编译期类型。

## 常见模式

- `@/*` 路径别名导入类型。
- 使用 tooling 的严格选项与明确的类型收窄；复用后端 schema 生成的类型保持单一事实来源。

## 禁止模式

- `any`（可预期场景外）——用明确类型或泛型。
- 在不可信边界只靠 TS 假设类型安全而不做运行时校验。
- 手写一份与后端 OpenAPI schema 重复的 API 类型（应自动生成）。