# Journal - hsingjui (Part 1)

> AI development session journal
> Started: 2026-08-20

---

## 00-bootstrap-guidelines（in_progress）

### 完成内容
- **前端 Biome 统一 lint 落地**（`#` biome）：
  - 删除项目根 `package.json`/`biome.json`/`oxlint`，Biome 及其配置只放 `web/`
  - `web/package.json` 脚本改名 `lint`→`check`（因 pnpm ≥11 内置 `lint` 命令会劫持 `scripts.lint`）
  - 改写/格式化前端源码，`pnpm check` + `pnpm build` 均 exit 0
- **填充 `.trellis/spec/`**（backend/frontend 指南 + 代码示例全部 Done）：
  - 后端 database/error-handling/logging 补代码示例
  - 清理两个 index.md 的英文“How to Fill”模板段，改为中文收尾

### 关键坑（已写入 frontend/quality-guidelines.md）
1. pnpm ≥11 内置 `lint` 命令劫持 `scripts.lint` → 脚本名别用 lint，用 check
2. biome 只扫 `src/**`，别扫 `dist/`（构建产物误报几千条）
3. Tailwind v4 需 `css.parser.tailwindDirectives: true`，否则 `--write` 整体中止不落盘