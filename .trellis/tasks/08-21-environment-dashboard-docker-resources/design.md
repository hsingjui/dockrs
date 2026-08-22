# 技术设计：环境仪表盘与 Docker 资源管理

## 1. 路由和页面布局

### 1.1 URL 结构

环境列表继续使用 `/`。在认证路由下增加一组带环境参数的嵌套路由：

```text
/environments/:environmentId                 EnvironmentDashboard
/environments/:environmentId/stacks           StacksPage
/environments/:environmentId/containers       ContainersPage
/environments/:environmentId/images           ImagesPage
/environments/:environmentId/volumes          VolumesPage
/environments/:environmentId/networks         NetworksPage
```

使用 React Router 参数作为当前环境的唯一来源，不在 Zustand 保存 `currentEnvironment`。环境作用域布局负责：

- 根据 `environmentId` 查询环境详情。
- 渲染返回环境列表、当前环境标题、类型和状态。
- 为 `Outlet` 提供当前环境的轻量上下文，避免每个页面重复解析同一参数。
- 处理环境不存在、离线和加载失败状态。

侧边栏读取当前路由参数生成资源链接。环境列表页面不显示指向某个具体环境的资源链接；仪表盘和资源页面都能直接通过 URL 刷新或复制访问。

环境卡片使用 `NavLink` 或语义化链接包住可点击区域，卡片上的更多操作按钮单独阻止冒泡，避免点击操作菜单误进入仪表盘。

### 1.2 仪表盘查询策略

仪表盘只调用一个聚合接口：

```text
GET /api/environments/:environment_id/overview
```

接口在服务端并发获取环境详情、Docker 指标和资源计数，但不返回五类资源的完整记录。这样页面首屏不会启动五个完整列表查询，资源列表在用户进入对应页面时再请求。

建议响应语义：

```text
EnvironmentOverview {
  environment: EnvironmentDetail
  metrics: EnvironmentMetrics
  resources: {
    stacks: StackSummary
    containers: ContainerSummary
    images: ResourceCount
    volumes: ResourceCount
    networks: ResourceCount
  }
}
```

`metrics` 和每个资源摘要允许带 `null`/`error` 状态。服务端使用有界并发和单项错误收集；一个资源计数失败时，其他卡片仍然返回，前端只在对应卡片显示不可用状态。

## 2. Server 和 Docker 层

### 2.1 共享 Docker 封装

复用环境指标任务建立的 Docker 连接和 Agent 路由能力，在允许的职责边界内补齐 `crates/docker` 的资源操作，不在 `crates/server` 里直接散落 Bollard 调用：

- `containers`：list、inspect、start、stop、restart、pause、unpause、remove。
- `images`：list、pull、remove。
- `volumes`：list、create、inspect/remove。
- `networks`：list、create、inspect/remove。
- `swarm stacks`：list Stack、inspect services/tasks；首版通过 Swarm API 聚合已部署 Stack，不引入 Stack YAML 解析器。

当前 workspace 只有 `crates/server` 时，先按项目既定边界补齐共享 Docker crate；不为每种资源增加独立 Rust crate。Server handler 只调用共享封装，Agent 收到同一资源请求后在节点本地调用该封装。

### 2.2 环境解析和错误

所有资源 handler 先解析 `environment_id`：

1. 找不到环境返回 404。
2. 本地环境使用本地 Docker client。
3. Agent 环境使用既有 Agent 通信链路，客户端不能传入或覆盖 endpoint。
4. Agent 离线或 Docker Engine 不可用返回统一的不可用错误，记录服务端详细日志但不把内部连接信息返回给浏览器。

建议错误映射：

- `401`：未登录。
- `404`：环境或资源不存在。
- `409`：资源当前状态不允许操作，例如卷正在使用、网络仍有连接或镜像被容器引用。
- `422`：创建/拉取参数无效。
- `502`：Agent/Docker Engine 不可达或执行失败。
- `500`：未预期的服务端错误。

### 2.3 Swarm Stack 发现

首版“堆栈”按 Docker Swarm services/tasks 聚合，不把 Stack 当成 Docker Engine 资源写入数据库：

- 先读取 Swarm 节点角色和集群状态；只有 manager 能执行 Stack 级查询，非 Swarm 或 worker 返回能力不可用状态。
- 使用 Swarm service 的 `com.docker.stack.namespace` 标签聚合 Stack。
- 使用 service 名称、service mode 和 task 状态计算服务数、期望副本、运行副本和异常任务数。
- Stack 列表只展示至少存在一个带 Stack namespace 标签的 service；没有 Stack 时展示空状态。
- Stack 详情展示该 Stack 下的 services、tasks，以及能映射到容器的 task/container ID；资源详情仍从 Docker Engine 实时读取。
- 不使用 `com.docker.compose.project` 或 `com.docker.compose.service` 作为 Stack 识别依据。

Stack YAML 的 canonical 存储、编辑、创建、部署和 `stack deploy/rm` 执行不在本任务强行实现。它们需要明确 Stack 文件存储位置、远程 Agent 如何取得文件以及 Docker Stack CLI/Engine API 的部署前提，后续单独设计。

## 3. REST API 契约

路径中的 `environment_id` 是环境记录 ID，资源 ID 使用 Docker 返回的 ID 或名称；所有响应沿用现有 `{ code, data, msg }` 信封。

### 3.1 查询接口

```text
GET /api/environments/:environment_id/overview
GET /api/environments/:environment_id/stacks
GET /api/environments/:environment_id/stacks/:stack_name
GET /api/environments/:environment_id/containers
GET /api/environments/:environment_id/images
GET /api/environments/:environment_id/volumes
GET /api/environments/:environment_id/networks
```

首版列表过滤使用 Docker 可表达的查询参数，避免在前端复制全量数据后再维护第二份状态：

- containers：`all`、`status`、`q`。
- images：`q`、`dangling`。
- stacks、volumes、networks：`q`；Stack 查询只对 Swarm manager 执行。

如果 Docker 返回的列表规模验证后确实需要分页，再为单个接口增加 `limit/offset` 或 Docker 原生分页；首版不为五类资源先造一套复杂的通用分页协议。

### 3.2 变更接口

容器操作：

```text
POST   /api/environments/:environment_id/containers/:container_id/start
POST   /api/environments/:environment_id/containers/:container_id/stop
POST   /api/environments/:environment_id/containers/:container_id/restart
POST   /api/environments/:environment_id/containers/:container_id/pause
POST   /api/environments/:environment_id/containers/:container_id/unpause
DELETE /api/environments/:environment_id/containers/:container_id
```

镜像操作：

```text
POST   /api/environments/:environment_id/images/pull
DELETE /api/environments/:environment_id/images/:image_id
```

卷操作：

```text
POST   /api/environments/:environment_id/volumes
DELETE /api/environments/:environment_id/volumes/:volume_name
```

网络操作：

```text
POST   /api/environments/:environment_id/networks
DELETE /api/environments/:environment_id/networks/:network_id
```

删除和创建 DTO 只暴露当前 UI 需要的字段；不接受任意 Docker API JSON 透传。所有请求参数加入 serde/utoipa schema，创建名称、镜像引用、driver 和选项在 handler 边界校验。

镜像拉取是可能持续较久的 REST 操作。首版可以等待 Docker client 完成后返回结果；如果真实使用证明需要进度，再单独设计有界 WebSocket/任务协议，不在本任务临时把拉取日志塞进内存。

## 4. 前端实现

### 4.1 模块组织

按现有功能目录增加：

```text
web/src/api/environments.ts
web/src/api/stacks.ts
web/src/api/containers.ts
web/src/api/images.ts
web/src/api/volumes.ts
web/src/api/networks.ts
web/src/features/environments/environment-layout.tsx
web/src/features/environments/environment-dashboard.tsx
web/src/features/stacks/stacks-page.tsx
web/src/features/containers/containers-page.tsx
web/src/features/images/images-page.tsx
web/src/features/volumes/volumes-page.tsx
web/src/features/networks/networks-page.tsx
```

可以提取少量真正重复的页面状态组件，例如空状态、错误状态和资源页头；不要把五类资源强行塞进一个泛型页面，保留各自字段和操作差异。

### 4.2 Query 和 Mutation

每个 query key 都包含环境 ID，例如：

```text
["environment-overview", environmentId]
["containers", environmentId, filters]
["images", environmentId, filters]
```

mutation 成功后失效受影响的资源 query 和 `environment-overview`；不把列表数据写入 Zustand。资源页面的搜索/筛选是 URL 参数或组件局部状态，便于返回和刷新后保持上下文。

### 4.3 页面差异

- 仪表盘：信息摘要和入口卡片，不渲染完整资源表格。
- 堆栈：Swarm Stack 聚合表，状态、服务数、期望/运行副本和任务异常为主，并可查看 Stack 下的服务、任务和容器跳转；非 manager 显示能力不可用。
- 容器：密度最高的状态表，提供状态筛选和生命周期菜单；危险删除使用确认对话框。
- 镜像：仓库/标签和大小信息为主，页头提供拉取镜像表单，删除前显示引用状态。
- 卷：名称、driver、mountpoint 和使用情况为主，页头提供创建卷表单。
- 网络：driver、scope、IPAM 和连接容器数为主，页头提供创建网络表单，系统网络禁用删除。

所有页面遵循现有 `max-w-7xl` 容器、表格/工具栏间距、语义色和暗色 token。状态数字使用 `font-mono tabular-nums`，资源操作按钮显示加载态，错误文案中文化。

## 5. 数据一致性和安全

- Docker Engine 是资源事实来源；列表返回前不从 SQLite 读取资源副本。
- mutation 完成后通过 Query 失效重新读取 Docker 状态，不乐观写入本地缓存。
- 删除卷、网络、镜像和容器必须前端确认、后端再次依赖 Docker 状态校验；前端确认不是安全边界。
- Agent 请求必须复用已认证的环境通道，不允许浏览器选择任意 Docker socket。
- 日志记录 environment ID、resource ID、操作和错误类型，避免记录密码、cookie、Agent 凭据和完整 Stack YAML 内容。
