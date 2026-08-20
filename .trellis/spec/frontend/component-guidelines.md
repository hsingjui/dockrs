# 组件指南（前端）

> 本项目的组件约定。

## 概览

- 使用 React 函数组件 + TypeScript。
- 基础组件复用 **shadcn/ui**（位于 `components/ui/`），但**不直接套默认 shadcn 样式**，保持 Dockrs 自己的视觉语言（见 `AGENTS.md` UI Guidelines）。
- 图标使用 `lucide-react`。

## 组件结构

- 函数组件，大型文件内按 hooks → 派生值 → handlers → JSX 组织。
- `App.tsx`（真实示例）展示了 shadcn Card/Button 的组合用法。

## Props 约定

- Props 用 TypeScript 显式声明类型，一个组件一个 `Props` 类型。
- 组件数据通过 props / hooks 提供，不在组件内部直接写全局副作用。

## 样式

- 使用 Tailwind CSS v4 工具类（`index.css` 通过 `@import "tailwindcss"` 引入）。
- 深色模式通过 `@custom-variant dark (&:is(.dark *))` 使用 `.dark` 类切换。
- 合并样式通过 `lib/utils.ts` 的 `cn()`（clsx + tailwind-merge）。

## 无障碍

- 复用 shadcn 组件自带的可访问性实现（基于 radix-ui）。
- 保留语义化标签与键盘可操作性，不在基础交互上去掉 JS focus 管理。

## 常见错误

- 直接套默认 shadcn 视觉 -> 应融入 Dockrs 视觉语言。
- 高信息密度但拥挤 -> 表格、日志、终端等高频界面优先性能与可读性。