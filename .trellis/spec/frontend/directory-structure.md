# 目录结构（前端）

> 前端代码的组织方式。

## 概览

- 技术栈：React + TypeScript + Vite + Tailwind CSS v4 + shadcn/ui（见 `AGENTS.md`）。
- 当前仓库为脚手架状态：`web/src/` 下仅有 `App.tsx`、`main.tsx`、`index.css`、`components/ui/`、`lib/utils.ts`、`assets/`。下面的目标结构来自 `AGENTS.md`，新功能按此组织。

## 目录布局（目标）

```text
web/src/
├── api/                    # API 请求封装
├── components/             # 通用/shared 组件
├── features/               # 按功能组织（推荐）
│   ├── auth/ containers/ images/ networks/
│   ├── volumes/ compose/ environments/ settings/
├── hooks/                  # 跨功能复用 hooks
├── lib/                    # 工具函数（已存在 lib/utils.ts）
├── stores/                 # Zustand 全局状态
└── locales/                # i18next 国际化（语言包尚未接入）
```

## 模块组织

- 优先按**功能**组织，而不是按技术类型（容器页面的组件就放在 `features/containers/`）。
- 路径别名 `@/*` → `./src/*`（已在 `tsconfig` 与 `vite.config` 配置）。
- shadcn/ui 基础组件放在 `components/ui/`（如现有 `button.tsx`、`card.tsx`）。

## 命名约定

- 组件/文件：PascalCase（`Button.tsx`）、小驼峰变量。
- Hook：`useXxx`。API 层按资源命名。

## 示例

- 入口链路（真实存在）：`web/src/main.tsx` → `App.tsx` → `components/ui/card.tsx`。
- 工具函数：`web/src/lib/utils.ts` 的 `cn()`。