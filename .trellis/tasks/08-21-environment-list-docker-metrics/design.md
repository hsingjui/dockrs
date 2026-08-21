# 技术设计：环境列表与 Docker 指标采集

## 1. 数据模型

`environments` 表保存环境配置和连接状态元数据：

- `id`：文本主键；本地固定为 `local`，Agent 使用注册 ID。
- `name`：用户可见名称。
- `kind`：`local` 或 `agent`。
- `endpoint`：本地 Docker Socket 或远程 Agent 标识/地址。
- `agent_id`：远程 Agent 身份，可为空。
- `last_seen_at`：远程 Agent 最近一次心跳时间；本地为空。
- `created_at`、`updated_at`：配置时间。

不保存 `online` 布尔值和 Docker 容器、镜像、CPU、内存等运行时副本。数据库迁移后通过幂等初始化插入本地记录，不能覆盖用户已经修改的名称。

## 2. 列表数据流

`GET /api/environments` 在已认证的 REST handler 中完成：

1. 查询环境配置记录，优先返回本地环境。
2. 检查本地 Docker 连接，并采集本地 Docker 快照。
3. 根据 Agent 心跳窗口计算远程在线状态。
4. 只对在线 Agent 请求实时 Docker 快照；离线 Agent 直接返回离线原因。
5. 使用有界超时和并发采集，单个环境失败不阻塞其他环境。
6. 返回环境配置、连接状态和最近一次实时指标快照。

列表接口可以返回扁平字段兼容现有卡片，但建议内部按以下语义组织：

- `containers.total/running/paused/stopped`
- `cpuPercent: number | null`
- `memory.usedBytes/totalBytes/percent`
- `metricsCollectedAt`
- `metricsError`

前端负责把 bytes 格式化为 GB，避免后端使用浮点 GB 造成精度和单位问题。

## 3. Docker 指标采集

共享 Docker 封装负责：

- `version()`：Docker 版本。
- `info()`：CPU 核数、总内存和容器总数/状态计数。
- `list_containers(all = true)`：需要容器明细时使用。
- `stats(stream = false)`：对运行中容器采集一次快照。

### CPU

Docker stats 的 CPU 百分比需要使用当前采样和上一采样的差值。环境级 CPU 采用所有运行中容器原始 CPU 使用量之和，再除以 Docker Engine 的 CPU 核数，归一化到 0～100%。

采样基准保存在 Server/Agent 进程内存中。进程重启或容器首次出现时没有上一采样，CPU 返回 `null`，避免展示伪造的 0。

### 内存

每个运行中容器的使用量按 Docker CLI 语义扣除 cgroup cache：

- cgroup v1 优先读取 `total_inactive_file`。
- cgroup v2 优先读取 `inactive_file`。
- 兼容旧版本的 `cache` 字段。

环境使用量是所有运行中容器处理后的使用量之和；总内存使用 Docker `info` 的 `MemTotal`。该字段明确表示容器工作负载占 Docker Engine 总内存的情况，不代表完整宿主机内存使用量。

### 快照缓存

为了避免浏览器刷新次数直接放大 Docker stats 请求，Server 和 Agent 应维护短期内存快照，建议采样间隔 10～15 秒、列表接口返回最近快照。缓存必须有界，并在环境删除或 Agent 断开时清理。

## 4. Agent 状态

Agent 通过既有 Agent 通信链路主动连接 Server：

- 首次注册或配对时创建/更新 `environments` 记录。
- 心跳更新 `last_seen_at`。
- Server 根据固定超时窗口计算在线状态。
- 实时 Docker 指标由 Agent 在节点本地采集，再返回汇总快照。

Server 不直接访问远程 Docker Socket，Agent 不访问 Server SQLite。

## 5. 前端

`useEnvironments` 改为 TanStack Query 请求真实列表，按 10～15 秒刷新。环境卡片：

- 保留当前本地/Agent 图标和在线状态。
- 本地环境提供改名入口。
- 远程环境暂时只展示，不提供本地改名操作。
- 指标为 `null` 时显示暂无数据或采集中，不显示静态示例值。
- 动态延迟、最近心跳和系统信息不要拼成静态标签。

## 6. 部署与安全

Server/Agent 只需挂载目标 Docker Socket；不需要挂载宿主机 `/proc`、`/sys`，也不需要 `privileged`。Docker Socket 本身具有较高权限，仍沿用 Docker 管理面板的既有部署安全边界。

## 7. 关键取舍

- 选择 Docker 工作负载指标，放弃第一阶段宿主机指标，保持无额外服务和单容器部署目标。
- 指标不进入 SQLite，避免违反 Docker Engine 作为运行时事实来源的约定。
- 在线状态由心跳时间推导，避免数据库状态漂移。
- 首版优先返回汇总指标，容器级明细留给后续容器页面。
