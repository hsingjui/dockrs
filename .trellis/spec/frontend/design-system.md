# 设计系统（前端）

> 色彩、排版、间距与页面范式约定。事实来源：`web/src/index.css`（design tokens）与 `features/environments/environments-page.tsx`（页面范式）、`components/layout/app-layout.tsx`（布局框架）。

## 色彩系统

### Design Tokens

颜色统一通过 Tailwind 语义 token 使用（`bg-background`、`text-muted-foreground` 等），**不写裸色值**，语义色（emerald/rose/amber）除外。

| Token | 浅色 | 深色 | 用途 |
|---|---|---|---|
| `background` / `foreground` | 极浅冷灰蓝 `oklch(0.984 0.003 247)` | 近黑 | 页面底色 / 主文字 |
| `card` / `card-foreground` | 纯白 | 深灰 | 卡片表面 |
| `primary` | `#1d63ed` | `#5b8def` | 品牌蓝：主按钮、激活导航、进度条、链接 |
| `muted` / `muted-foreground` | 浅灰蓝 / 中灰 | 深灰 / 浅灰 | 次要表面 / 辅助文字 |
| `secondary` / `accent` | 同 muted | 同 muted | 次级按钮、hover 表面 |
| `border` / `input` | 浅灰 | 白 10%/15% | 分隔线、输入框 |
| `destructive` | 红 | 亮红 | 危险操作 |
| `sidebar-*` | 白色系 | 深灰系 | 侧边栏专用 |
| `chart-1..5` | 灰阶 | 灰阶 | 图表 |

- 深色模式：`.dark` 类切换（`@custom-variant dark`），默认浅色。
- 分隔优先用 `ring-1 ring-foreground/10`（卡片）而非阴影；hover 时才上 `hover:shadow-md`。

### 状态语义色

| 状态 | 颜色 | 典型用法 |
|---|---|---|
| 在线 / 成功 | emerald（`bg-emerald-50 text-emerald-700`，数值 `text-emerald-600`） | StatusPill、运行计数 |
| 离线 / 错误 | rose（`bg-rose-50 text-rose-600`，图标 `text-rose-500`） | StatusPill、错误提示块 |
| 高负载 / 告警 | amber（`bg-amber-500`） | 资源条 >80% |

状态点：`size-1.5 rounded-full` 圆点 + 文字，`rounded-full border px-2 py-0.5 text-[11px]` 胶囊。

## 排版

- 字体：Geist Variable（`@fontsource-variable/geist`），`html` 上 `font-sans`。
- 数字/版本/endpoint：一律 `font-mono tabular-nums`。
- 文案：中文优先。

| 层级 | 类名 |
|---|---|
| 页面标题 | `font-semibold text-xl` |
| 页面描述 | `text-sm text-muted-foreground` |
| 正文 | `text-sm` |
| 辅助/元信息 | `text-xs text-muted-foreground` |
| 微型标签/版本胶囊 | `text-[10px]`~`text-[11px]`，`rounded bg-muted px-1.5 py-0.5` 或带边框 mono 胶囊 |
| KPI 数值 | `font-semibold text-2xl tabular-nums` |
| 侧边栏分组标题 | `text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/70` |

## 圆角与间距

- 基础 `--radius: 0.625rem`；页面卡片 `rounded-xl`、按钮/输入 `rounded-lg`、小标签 `rounded`、状态点/胶囊 `rounded-full`。
- 页面容器：`mx-auto w-full max-w-7xl px-6 py-8 lg:px-8`，段落节奏 `space-y-6`。
- 卡片内边距 `p-5`，KPI 卡用 `Card size="sm"`。
- 高信息密度但不拥挤：靠 `space-y` 节奏和 muted 层级区分，不靠缩小到不可读。

## 布局框架

- 左侧 `w-60` 侧边栏：品牌区 `h-16` → 分组导航 → 底部用户区；激活项 `bg-primary/10 text-primary font-medium`。
- 主区域 `min-w-0 flex-1 overflow-y-auto`。
- 图标统一 `lucide-react`，正文图标 `size-4`，图文间距 `gap-2`~`gap-2.5`。

## 页面范式（新页面照此结构）

1. **页头**：左侧标题 + 描述，右侧主操作 `<Button>`。
2. **KPI 看板**（可选）：`grid sm:grid-cols-2 lg:grid-cols-4 gap-3`，`Card size="sm"`：xs 标签 → 2xl 数值 → xs hint。
3. **筛选/工具条**：分段 tab 用 `flex gap-1.5 rounded-lg border bg-muted p-1`，激活项 `bg-card shadow-sm`，带计数。
4. **内容区**：卡片网格 `grid md:grid-cols-2 xl:grid-cols-3 gap-5` 或表格。
5. **卡片底部元信息栏**：`border-t px-5 py-3`。
6. **空位/引导**：虚线卡片 `border-dashed hover:border-primary/50 hover:bg-primary/[0.02]`。

## 交互约定

- Hover：卡片 `hover:shadow-md transition-shadow`；主按钮 `hover:bg-primary/80`；可点文本 `hover:text-primary/80`。
- 未实现功能：入口保留但禁用，`cursor-not-allowed text-muted-foreground/50` + `title="即将上线"`。
- 危险操作：`destructive` 变体 + 二次确认（见 `AGENTS.md`）。
- 资源监控条：`h-1.5 rounded-full bg-muted` 细条，正常 `bg-primary`、>80% `bg-amber-500`。

## 常见错误

- 写裸色值 / 自造灰阶 → 用语义 token；新状态色先入 token 再用。
- 卡片用 `border shadow` 堆叠 → 用 `ring-1 ring-foreground/10`，hover 才加 shadow。
- 数字不用 `tabular-nums` → 列表/监控数字跳动。
- 隐藏未实现入口 → 保留禁用态，降低用户困惑。
