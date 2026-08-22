# 实现环境仪表盘与 Docker 资源管理

## Goal

用户点击环境卡片后进入该环境的仪表盘，查看环境连接信息、Docker 运行指标和堆栈、容器、镜像、存储卷、网络的数量概览；点击资源卡片进入对应的环境级资源页面，使用 Docker Engine 或远程 Agent 的实时数据展示列表并执行该资源支持的基础操作。

本任务包含前端完整实现，不只是提供后端接口：环境路由、仪表盘、五类资源列表、操作反馈、加载/错误/空状态、响应式布局和深色模式都必须以 `design.md` 为实现依据，并遵循现有 Dockrs 整体视觉风格。页面不能以占位卡片、禁用导航或静态示例数据作为交付结果。

“堆栈”在本任务中指 Docker Swarm 的 Stack，不指 Compose 项目。Stack 相关接口只对 Swarm manager 环境可用；非 Swarm 环境、Swarm worker 或 Docker Engine 不可用时必须展示明确的能力不可用状态。所有资源都必须带环境作用域，不能把不同环境的 Docker 资源混在同一个列表中。

## Background

当前仓库仍是脚手架状态：

- `web/src/features/environments/environments-page.tsx` 的环境卡片使用占位环境数据，尚未导航到详情。
- `web/src/App.tsx` 只有环境列表和设置路由。
- `web/src/components/layout/app-layout.tsx` 中容器、镜像、堆栈、网络、存储卷导航仍是禁用项。
- Server 当前只有认证路由，还没有 Docker 资源 API；`crates/docker` 和 `crates/agent` 尚未加入 workspace。
- `08-21-environment-list-docker-metrics` 任务负责环境记录、连接状态和环境级 Docker 指标，本任务消费其环境标识和概要能力，不重复建立 Docker 资源数据库副本。

## Requirements

### 1. 环境导航与作用域

- 环境卡片整体可点击，并保留键盘访问能力，进入 `/environments/:environmentId`。
- 新增环境路由布局，统一显示当前环境名称、类型、在线状态、返回环境列表和切换环境入口。
- 资源路由统一使用当前环境 ID：
  - `/environments/:environmentId/stacks`
  - `/environments/:environmentId/containers`
  - `/environments/:environmentId/images`
  - `/environments/:environmentId/volumes`
  - `/environments/:environmentId/networks`
- 侧边栏资源导航根据当前 URL 生成环境作用域链接；没有当前环境时不生成指向错误环境的链接。
- 环境不存在、无权限、离线或 Docker 不可用时，页面分别展示明确的 404、认证错误或不可用状态，不渲染静态示例数据。

### 2. 环境仪表盘

仪表盘路径为 `/environments/:environmentId`，至少展示：

- 环境名称、local/agent 类型、endpoint、Docker 版本、连接状态和最近采集时间。
- 容器工作负载 CPU、内存和容器状态计数；指标暂不可用时展示“采集中”或原因。
- 五个资源入口卡片：堆栈、容器、镜像、存储卷、网络；每张卡片展示数量和该资源特有的摘要信息，并可点击进入列表。
- 容器卡片展示运行中/已停止等状态计数；其他卡片展示总数，具体字段由资源 API 返回。
- 仪表盘提供手动刷新，并沿用环境指标任务定义的有限频率刷新策略；不为每次页面渲染重复发起五个完整资源列表请求。
- 在线环境的单个资源统计失败不应阻塞其他统计；离线环境显示统一不可用提示和返回环境列表操作。

### 3. 资源列表与资源特有能力

所有列表页面都必须具备加载、请求失败、空列表和刷新状态，并通过 TanStack Query 管理服务端数据。

- **堆栈（Swarm Stack）**：展示 Stack 名称、运行状态、服务数、期望副本数、运行副本数和任务异常；首版从 Swarm services/tasks 的 `com.docker.stack.namespace` 标签发现已部署 Stack。Stack 详情展示服务和任务，并可跳转到对应容器。非 Swarm manager 环境展示能力不可用状态。Stack YAML 的创建、编辑、部署和文件持久化需要独立的存储/执行协议，若当前环境尚未具备，不显示无效的操作按钮。
- **容器**：展示名称、状态、镜像、端口、创建时间和所属堆栈；支持按状态筛选、名称/镜像搜索，以及启动、停止、重启、暂停、恢复和删除。删除必须二次确认。日志、Stats 和 Exec 使用 WebSocket，保留后续入口但不在本任务内重新实现流式协议。
- **镜像**：展示仓库/标签、镜像 ID、大小、创建时间和使用状态；支持按名称搜索、拉取镜像和删除镜像。删除必须二次确认，正在被容器使用的镜像由后端返回明确错误。
- **存储卷**：展示名称、driver、scope、mountpoint、标签和使用中的容器数量（能从 Docker 查询得到时）；支持创建和删除卷。删除必须二次确认，挂载中的卷由后端拒绝删除并返回统一错误。
- **网络**：展示名称、driver、scope、子网/网关和已连接容器数；支持按名称搜索、创建和删除网络。系统网络和仍有依赖的网络由后端保护，删除必须二次确认。

### 4. 后端接口

所有接口都挂在 `/api` 下并要求登录。资源始终通过 environment ID 路由到本地 Docker Engine 或对应 Agent，不接受客户端传入的 Docker endpoint。

建议接口契约：

- `GET /api/environments/:environment_id/overview`：返回环境详情、指标和五类资源计数。
- `GET /api/environments/:environment_id/stacks`
- `GET /api/environments/:environment_id/containers`
- `GET /api/environments/:environment_id/images`
- `GET /api/environments/:environment_id/volumes`
- `GET /api/environments/:environment_id/networks`
- 资源详情按同一资源路径追加 `/:resource_id`，仅在页面确实需要详情时实现。
- 容器生命周期操作使用资源作用域下的 action 路由；镜像拉取、镜像/卷/网络创建与删除使用对应 REST 方法，具体 DTO 在设计阶段固定并加入 OpenAPI。
- 每个成功的变更接口返回可判断成功的统一信封；失败使用现有 `ApiError` 结构和明确的 HTTP status code。
- 所有 Docker 资源列表和操作均实时读取 Docker Engine；不在 SQLite 保存容器、镜像、卷、网络或运行指标的副本。

### 5. 前端数据和交互

- 新增按功能组织的 API client、类型和 TanStack Query hooks，query key 必须包含 `environmentId`。
- 资源变更成功后只失效受影响资源查询和当前环境 overview 查询，不把服务端数据复制到 Zustand。
- 表格使用项目已有的 TanStack Table 依赖或现有基础组件，遵循 Dockrs 设计 token、中文文案、深色模式和高信息密度但可扫描的布局。
- 破坏性操作使用危险态和二次确认；按钮显示进行中状态，避免重复提交。
- Docker/Agent 错误不得直接泄漏内部错误文本或 endpoint 凭据。

## Out of Scope

- Docker Swarm Stack 的完整部署流水线、Stack YAML 编辑器、文件上传/持久化和跨节点同步；本任务实现已部署 Stack 的发现、列表、详情和服务/任务摘要，部署协议另行设计。
- 容器日志、实时 Stats、Exec/Terminal 的 WebSocket 实现。
- 历史指标、告警、宿主机监控和 Prometheus 集成。
- 批量删除、批量生命周期操作和复杂 RBAC。
- Docker 资源写入 SQLite 作为缓存或副本。

## Acceptance Criteria

- [ ] 登录用户点击环境卡片可以进入正确的环境仪表盘，刷新或直接访问 URL 后环境 ID 仍正确。
- [ ] 仪表盘展示真实环境信息、指标和五类资源计数；不再显示静态示例值。
- [ ] 仪表盘五张资源卡片分别进入当前环境作用域的资源列表。
- [ ] 侧边栏资源导航在当前环境内可用，切换环境后不会沿用旧环境 ID。
- [ ] 五类列表均具备加载、错误、空数据、刷新和中文状态展示。
- [ ] 容器、镜像、卷、网络的列表字段来自 Docker Engine；一个环境的资源不会出现在另一个环境。
- [ ] 容器生命周期、镜像拉取/删除、卷和网络创建/删除的成功与错误状态可反馈，危险操作均有二次确认。
- [ ] Stack 列表能按 Swarm services/tasks 的 `com.docker.stack.namespace` 标签聚合已部署 Stack，并正确显示服务、期望副本、运行副本和任务异常；无 Stack 或非 manager 环境展示对应状态。
- [ ] 本地环境和在线 Agent 的接口走统一资源 DTO；离线 Agent 返回可理解的不可用状态，不阻塞其他环境。
- [ ] 未登录请求被拦截并返回统一 401；不存在的环境或资源返回明确 404。
- [ ] OpenAPI 包含新增 DTO、参数、响应和错误契约。
- [ ] Docker 资源和运行时指标没有被复制写入 SQLite，列表刷新不会造成无界内存积累。
- [ ] 前端路由、环境布局、仪表盘和五类资源页面全部可用，不以占位页面、禁用导航或静态示例数据代替真实实现。
- [ ] 前端页面严格以 `design.md` 为实现依据，符合 Dockrs 整体风格：复用现有 shadcn/ui 和 lucide-react，使用语义 design token，默认浅色并支持深色，中文文案、响应式布局、键盘可访问性和高信息密度表格状态完整。
- [ ] 列表筛选、资源操作、加载态、空态、错误态、离线态和危险操作确认在前端均有明确可见反馈。
- [ ] 前端通过 `pnpm check`，后端通过格式化、Clippy 并完成编译验证。
