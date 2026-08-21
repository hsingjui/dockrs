# 实施计划：环境列表与 Docker 指标采集

## 1. 后端基础

- [ ] 增加 `environments` 数据库迁移。
- [ ] 增加本地环境幂等初始化，保留用户已修改的名称。
- [ ] 增加环境模型、列表查询和本地名称更新接口。
- [ ] 抽取可复用的 Session 登录检查。
- [ ] 将环境接口加入 OpenAPI。

## 2. Docker 封装

- [ ] 创建或补齐共享 `crates/docker`，封装 Docker Engine 连接、`version`、`info` 和容器统计。
- [ ] 实现容器 CPU 采样差值计算。
- [ ] 实现 cgroup v1/v2 内存 cache 兼容处理。
- [ ] 实现环境级指标汇总和有界内存快照。
- [ ] 为 Docker 不可用、首次采样和单容器采集失败定义可返回错误状态。

## 3. Agent 链路

- [ ] 确认 Agent 注册/配对消息能创建或更新环境记录。
- [ ] 心跳更新 `last_seen_at`。
- [ ] 增加 Agent Docker 快照请求与汇总响应。
- [ ] 增加连接超时和离线状态处理。

## 4. 前端

- [ ] 增加环境 API client 和 TanStack Query hook。
- [ ] 删除 `placeholderEnvironments`。
- [ ] 适配真实 Docker 版本、容器数、CPU、内存和指标缺失状态。
- [ ] 增加列表加载、请求失败和空列表状态。
- [ ] 实现本地环境改名及成功/失败反馈。
- [ ] 将未实现的远程添加入口保持禁用，避免点击无行为。

## 5. 验证

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

- 新数据库迁移和已有用户/Session 迁移兼容。
- 本地名称修改不会被启动初始化覆盖。
- Docker 运行时数据没有写入数据库。
- CPU 首次采样、无运行容器、Docker 离线和 Agent 心跳超时状态正确。
- 指标采集缓存有界，不会因列表刷新无限积累。
- API 未登录时返回统一 401 响应。
