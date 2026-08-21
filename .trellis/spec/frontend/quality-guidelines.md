# 质量指南（前端）

> 前端代码质量标准。

## 概览

- 中文优先的界面文案（用户可见文案用中文）。
- UI 遵循 `AGENTS.md` UI Guidelines：简洁美观、默认浅色支持深色、高信息密度但避免拥挤、危险操作用明确危险态 + 二次确认、表格/日志/终端等高频率界面优先性能。

## 修改后验证

修改 `web/` 下的前端代码后，在 `web/` 目录执行：

```bash
pnpm check  # = biome check .（配置见 web/biome.json）
```

提交前或涉及构建配置、类型与打包行为变更时，再执行构建：

```bash
pnpm build  # = tsc -b && vite build，二者必须通过
```

## 禁止模式

- 用 Zustand 保存服务端数据副本。
- `any`（可预期场景外）。
- 危险操作无二次确认。
- 不必要的巨型依赖或重复造轮子组件（优先复用 shadcn/ui）。

## 必用模式

- 复用 shadcn/ui 基础组件并保持 Dockrs 自身视觉。
- TanStack Query 管服务端、Zustand 管 UI、RHF + Zod 管表单。
- 后端类型走自动生成。

## 验证命令（提交前）

在 `web/` 目录下执行：

```bash
pnpm check  # = biome check .（配置见 web/biome.json）
pnpm build  # = tsc -b && vite build，二者必须通过
```

- 前端 lint 统一用 **Biome**（`web/biome.json`），不要引入 oxlint/ESLint 双工具。
- ⚠️ 注意：脚本名请不要叫 `lint`。pnpm ≥11 内置了 `lint` 命令（基于 ESLint），会劫持 `package.json` 的 `lint` 脚本导致命令被解释成 ESLint 而报錯。本项目用 `check`，若要 lint 用 `pnpm run <script>`。

> 已经踩过的坑（勿回退）：
> - `biome.json` 的 `files.includes` 只含 `src/**/*` + 配置文件，**不要**让 biome 扫 `dist/`（构建产物含压缩 JS/CSS，会产生几千条误报）。
> - Tailwind v4（`index.css` 的 `@custom-variant`/`@theme`/`@apply`）必须在 `biome.json` 配 `css.parser.tailwindDirectives: true`，否则 `--write` 会因 CSS 解析错误整体中止、不落盘任何修复（表现：报 “No issues found” 却又 exit 1、文件没变）。
> - `main.tsx` 的 `createRoot(document.getElementById('root')!)` 是 vite 标准入口、DOM 不变量，保留（biome 的 `noNonNullAssertion` 是 warning 非 error）。

> 根目录不维护 `package.json`，Biome 及其配置只放在 `web/`。

## 测试要求

- 行为变更需验证；构建必须通过（`tsc -b` + vite build）。
- 交互/视觉回归按需，保持最小必要集。

## 代码评审清单

- 前端可构建。
- 用户可见文案为中文。
- 未复制服务端数据到客户端全局状态。
- 危险操作有保护（危险态 + 二次确认）。
- 深色模式可用、界面对齐 Dockrs 视觉语言。