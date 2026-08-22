# 实施计划：环境仪表盘与 Docker 资源管理

## 1. 前置契约和任务衔接

- [x] 对齐 `08-21-environment-list-docker-metrics` 产出的环境 ID、环境详情、在线状态和 Docker 指标 DTO；不重复创建环境表或运行时资源表。
- [x] 固定 URL、资源命名、overview 响应、列表过滤参数、错误 status 和 OpenAPI schema。
- [x] 确认本地 Docker 和 Agent 请求都通过同一套资源服务接口，浏览器不传 Docker endpoint。
- [x] 在实现前检查 workspace 是否已加入共享 `crates/docker`；若未加入，按既有架构补齐，不新增资源专用 crate。

## 2. Docker 资源封装

- [ ] 在共享 Docker 层补齐容器、镜像、Volume、Network 的 list/inspect/操作方法和稳定的领域 DTO。
- [x] 实现 Swarm Stack 聚合：按 service 的 `com.docker.stack.namespace` 标签和 services/tasks 聚合已部署 Stack，计算服务数、期望副本、运行副本和任务异常。
- [x] 将资源操作错误分类为资源不存在、状态冲突、参数错误和 Engine 不可用，供 Server 统一映射。
- [x] 为 overview 提供资源计数所需的轻量查询；不为了仪表盘保存 Docker 资源快照到 SQLite。
- [x] 通过 Agent 复用同样的 Docker 操作请求和响应，设置请求超时与有界响应处理。

## 3. Server 路由、Handler 和 OpenAPI

- [x] 新增认证后的环境 overview handler。
- [x] 新增 stacks、containers、images、volumes、networks 的列表/详情 handler。
- [x] 新增容器生命周期、镜像拉取/删除、Volume 创建/删除、Network 创建/删除 handler。
- [x] 抽取或复用环境解析、登录校验和 Docker/Agent 调用，避免每个资源 handler 重复实现安全边界。
- [x] 所有 handler 使用统一 `ApiResponse`/`ApiError`，明确返回 401、404、409、422、502 和 500。
- [x] 将请求参数、响应 DTO 和新增路径加入 `utoipa` OpenAPI；不把 Bollard 内部类型直接暴露给前端。
- [x] 为资源名称、镜像引用、driver 和可选参数做边界校验；删除操作后端再次依赖 Docker 返回结果判断是否成功。

## 4. 前端完整实现：环境路由和仪表盘（必做）

- [x] 在 `App.tsx` 增加环境嵌套路由和 `EnvironmentLayout`。
- [x] 将环境卡片改为可访问的环境详情链接，处理更多按钮与卡片导航冲突。
- [x] 将环境详情、overview API 和 TanStack Query hook 接入仪表盘。
- [x] 实现环境头部、连接状态、Docker 信息、CPU/内存指标和容器状态摘要。
- [x] 实现五个资源入口卡片，分别链接到当前 `environmentId` 的资源列表。
- [x] 实现 overview 的加载、部分不可用、离线、404、请求失败和手动刷新状态。
- [x] 让 AppLayout 资源导航根据当前环境路由生成链接，并保持未选择环境时不误导用户。
- [x] 前端严格按 `design.md` 实现，不做后置套样式；复用现有 shadcn/ui、lucide-react 和语义 design token，完成中文文案、默认浅色/深色模式、响应式布局、键盘可访问性和高信息密度表格状态。

## 5. 五类资源页面

- [x] 为每类资源增加 API client、类型和 TanStack Query hooks，query key 必须包含环境 ID和筛选条件。
- [x] 实现 stacks 页面：Swarm Stack 聚合列表、状态/服务数/期望副本/运行副本/任务异常、空状态和服务/任务/容器跳转；非 manager 显示能力不可用，不显示未实现的 Stack YAML 或部署按钮。
- [x] 实现 containers 页面：状态筛选、搜索、容器表格和启动/停止/重启/暂停/恢复/删除操作。
- [x] 实现 images 页面：搜索、镜像表格、拉取镜像和删除镜像操作。
- [x] 实现 volumes 页面：Volume 表格、创建和删除操作，并展示挂载/使用冲突。
- [x] 实现 networks 页面：Network 表格、IPAM/连接容器摘要、创建和删除操作，并保护系统网络。
- [x] 为各页面实现统一加载、错误、空数据、刷新和 mutation 进行中状态；不复制服务端数据到 Zustand。
- [x] 所有危险操作使用明确危险态和二次确认，mutation 成功后失效资源列表和 overview query。
- [x] WebSocket 日志、Stats、Exec/Terminal 不在本任务重新实现，只保留不会触发空行为的后续入口或不展示入口。

## 6. 联调和边界场景

- [ ] 本地环境和在线 Agent 分别验证 overview、列表和变更请求。
- [ ] 验证离线 Agent、Docker socket 不可用、资源不存在、资源正在使用、系统网络删除和镜像被引用等错误。
- [ ] 验证切换环境、浏览器刷新、直接访问深链接时 query key 和 API 路径没有串环境。
- [ ] 验证空环境、没有 Swarm Stack、非 manager 节点、没有运行中容器和首次指标不可用状态。
- [ ] 检查资源刷新和 mutation 不会将资源状态写入 SQLite，也不会累积无界的流式数据或临时缓存。

## 7. 验证

后端修改后执行：

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
```

前端修改后执行：

```bash
cd web
pnpm check
pnpm build
```

重点检查：

- 环境卡片、仪表盘、五类资源页和侧边栏链接形成完整闭环。
- OpenAPI 与实际响应信封、错误结构和前端 API 类型一致。
- Docker Engine 仍是资源事实来源，SQLite 只保存环境/认证以及后续明确设计的 Stack 配置等持久化数据。
- 本地和 Agent 两条链路使用相同 DTO 与权限校验，浏览器无法绕过环境解析直接访问 Docker。
- 资源操作失败不会让列表页面崩溃或显示假成功状态。

本轮已执行并通过：`cargo fmt --all -- --check`、`cargo check --workspace`、`cargo clippy --workspace --all-targets --all-features`、`cargo test --workspace`、`cd web && pnpm check`、`cd web && pnpm build`。未连接真实远程节点执行 Docker 变更联调。
