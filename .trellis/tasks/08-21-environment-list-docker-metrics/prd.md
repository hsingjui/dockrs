# 实现环境列表与 Docker 指标采集

## Goal

统一管理本地 Docker 环境和远程 Agent 环境，展示 Docker 版本、容器数量、Docker CPU/内存指标及在线状态，支持修改本地环境名称。

## Background

当前环境主页使用 `placeholderEnvironments` 展示静态示例数据。后端已经具备 SQLite、迁移、Session 和统一 API 响应基础，但尚未接入环境表、Docker 查询或 Agent 环境列表。

## Requirements

- 新增 `environments` 数据表，本地环境和远程 Agent 都作为记录保存。
- 首次启动自动创建固定 ID 为 `local` 的本地环境；已有本地名称不能被默认值覆盖。
- 本地环境名称可以通过已认证 API 修改，并在重启后保留。
- 环境列表同时返回本地环境和数据库中的远程 Agent 环境。
- 本地环境在线状态由 Docker Engine 连接检测得到。
- 远程 Agent 保存最近心跳时间，在线状态由心跳是否在有效窗口内动态计算，不保存固定的 `online` 布尔值。
- Docker 版本和容器数量通过 Docker Engine API 获取。
- CPU 和内存使用量采用 Docker 容器工作负载指标，不采集宿主机指标。
- CPU 指标按所有运行中容器汇总，并归一化到 Docker Engine 总 CPU 容量的 0～100%。
- 内存使用量汇总运行中容器的使用量并扣除 cgroup cache；总内存使用 Docker Engine 的 `MemTotal` 作为列表展示基准。
- 指标只保存在运行时快照或短期内存缓存中，不写入 SQLite。
- Server 和 Agent 只依赖 Docker Socket，不引入 cAdvisor、Node Exporter 或其他额外服务。
- 前端移除环境占位数组，使用真实 API，并处理加载、错误、离线、指标不可用和首次采样状态。
- 远程环境在本阶段只展示；远程配对、注册凭据和完整 Agent 管理流程按现有 Agent 协议落地。

## Out of Scope

- 宿主机级 CPU、内存、磁盘和网络指标。
- 历史指标曲线、告警和 Prometheus 集成。
- 容器运行时数据写入数据库。
- 远程环境删除、批量编辑和复杂标签管理。
- cAdvisor、Node Exporter 等外部采集服务。

## Acceptance Criteria

- [ ] 新数据库启动后列表至少包含一条本地环境记录。
- [ ] 重启 Server 后本地环境名称保持不变。
- [ ] 已登录用户可以修改本地环境名称；空名称和超过 64 个字符的名称被拒绝。
- [ ] 未登录请求不能读取或修改环境列表。
- [ ] 列表中的 Docker 版本来自 Docker Engine，不再使用静态版本字符串。
- [ ] 列表中的总数、运行中、暂停和已停止容器数来自 Docker Engine。
- [ ] 运行中容器存在时，CPU 和内存指标来自 Docker stats 汇总；第一次 CPU 采样可以显示暂无数据。
- [ ] 没有运行中容器时，容器 CPU 和内存使用量显示为 0，Docker 总内存仍可展示。
- [ ] Docker 不可用时本地环境显示离线或指标不可用原因，不影响其他环境返回。
- [ ] 远程 Agent 的在线状态由最近心跳时间决定，超时后显示离线。
- [ ] 前端不再展示示例环境、示例 IP、示例 CPU 或示例内存数据。
- [ ] 指标采集不依赖额外服务，Server/Agent 仅使用 Docker Socket 和既有通信链路。
- [ ] Docker 运行时资源没有被复制写入 SQLite。
