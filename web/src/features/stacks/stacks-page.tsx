import {
	ChevronDown,
	ChevronRight,
	ExternalLink,
	Layers,
	RefreshCw,
} from "lucide-react";
import { Fragment, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import {
	DataTable,
	EmptyTableState,
	Notice,
	ResourceHeader,
	ResourcePageState,
	SearchField,
	TableBody,
	TableCell,
	TableHead,
	TableHeaderCell,
	TableRow,
} from "@/features/resources/resource-components";
import { cn } from "@/lib/utils";
import { useStacks } from "./use-stacks";

function statusClass(status: string) {
	if (status === "running")
		return "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400";
	if (status === "degraded") return "bg-destructive/10 text-destructive";
	return "bg-amber-500/10 text-amber-700 dark:text-amber-400";
}

function statusLabel(status: string) {
	if (status === "running") return "运行中";
	if (status === "degraded") return "异常";
	return "等待中";
}

export function StacksPage() {
	const { environmentId } = useParams();
	const [q, setQ] = useState("");
	const [expanded, setExpanded] = useState<string | null>(null);
	const query = useStacks(environmentId, q);

	if (query.isPending)
		return <ResourcePageState message="正在读取 Swarm Stack" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error
						? query.error.message
						: "Stack 列表加载失败"
				}
				action={
					<Button variant="outline" onClick={() => void query.refetch()}>
						<RefreshCw />
						重试
					</Button>
				}
			/>
		);
	}
	if (!query.data.available) {
		return (
			<ResourcePageState
				variant="offline"
				message={query.data.reason ?? "当前环境不支持 Swarm Stack"}
				action={
					<Button variant="outline" onClick={() => void query.refetch()}>
						<RefreshCw />
						重新检查
					</Button>
				}
			/>
		);
	}

	return (
		<div className="space-y-5">
			<ResourceHeader
				title="堆栈"
				count={query.data.items.length}
				onRefresh={() => void query.refetch()}
				isRefreshing={query.isFetching}
			/>
			<Notice variant="warning">
				Stack 仅展示 Docker Swarm 中已部署的服务，不提供 YAML 部署操作。
			</Notice>
			<div className="flex items-center rounded-xl border bg-card p-3">
				<SearchField
					value={q}
					onChange={setQ}
					placeholder="搜索 Stack 名称"
					label="搜索 Stack"
				/>
			</div>
			<DataTable minWidth="min-w-[900px]">
				<TableHead>
					<tr>
						<TableHeaderCell className="w-10" />
						<TableHeaderCell>Stack</TableHeaderCell>
						<TableHeaderCell>状态</TableHeaderCell>
						<TableHeaderCell>服务</TableHeaderCell>
						<TableHeaderCell>副本</TableHeaderCell>
						<TableHeaderCell>任务异常</TableHeaderCell>
					</tr>
				</TableHead>
				<TableBody>
					{query.data.items.length === 0 ? (
						<EmptyTableState colSpan={6} message="当前没有已部署的 Stack" />
					) : (
						query.data.items.map((stack) => {
							const isExpanded = expanded === stack.name;
							return (
								<Fragment key={stack.name}>
									<TableRow
										className="cursor-pointer"
										onClick={() => setExpanded(isExpanded ? null : stack.name)}
									>
										<TableCell>
											<button
												type="button"
												className="flex size-7 items-center justify-center rounded-md hover:bg-muted"
												onClick={(event) => {
													event.stopPropagation();
													setExpanded(isExpanded ? null : stack.name);
												}}
												aria-label={isExpanded ? "收起详情" : "展开详情"}
											>
												{isExpanded ? (
													<ChevronDown className="size-4" />
												) : (
													<ChevronRight className="size-4" />
												)}
											</button>
										</TableCell>
										<TableCell>
											<div className="flex items-center gap-2 font-medium">
												<Layers
													className="size-4 text-muted-foreground"
													aria-hidden="true"
												/>
												{stack.name}
											</div>
										</TableCell>
										<TableCell>
											<span
												className={cn(
													"inline-flex rounded-full px-2 py-0.5 font-medium text-xs",
													statusClass(stack.status),
												)}
											>
												{statusLabel(stack.status)}
											</span>
										</TableCell>
										<TableCell className="font-mono tabular-nums">
											{stack.serviceCount}
										</TableCell>
										<TableCell className="font-mono tabular-nums">
											{stack.runningReplicas} / {stack.desiredReplicas}
										</TableCell>
										<TableCell
											className={cn(
												"font-mono tabular-nums",
												stack.failedTasks > 0 && "text-destructive",
											)}
										>
											{stack.failedTasks}
										</TableCell>
									</TableRow>
									{isExpanded && (
										<tr key={`${stack.name}-details`}>
											<TableCell colSpan={6} className="bg-muted/20 px-6 py-4">
												<div className="grid gap-5 lg:grid-cols-2">
													<div>
														<h3 className="font-medium text-sm">服务</h3>
														<div className="mt-2 divide-y rounded-lg border bg-card">
															{stack.services.map((service) => (
																<div
																	key={service.id}
																	className="flex items-center justify-between gap-3 px-3 py-2 text-xs"
																>
																	<span className="truncate">
																		{service.name}
																	</span>
																	<span className="shrink-0 font-mono text-muted-foreground">
																		{service.runningReplicas} /{" "}
																		{service.desiredReplicas}
																	</span>
																</div>
															))}
														</div>
													</div>
													<div>
														<h3 className="font-medium text-sm">任务</h3>
														<div className="mt-2 divide-y rounded-lg border bg-card">
															{stack.tasks.length === 0 ? (
																<div className="px-3 py-4 text-muted-foreground text-xs">
																	暂无任务
																</div>
															) : (
																stack.tasks.map((task) => (
																	<div
																		key={task.id}
																		className="flex items-center justify-between gap-3 px-3 py-2 text-xs"
																	>
																		<span className="min-w-0 truncate">
																			{task.name ?? task.id.slice(0, 12)}
																		</span>
																		<span
																			className={cn(
																				"shrink-0 font-mono",
																				task.error && "text-destructive",
																			)}
																		>
																			{task.state ?? "未知"}
																		</span>
																		{task.containerId && environmentId && (
																			<Tooltip content="查看任务容器">
																				<Link
																					className="shrink-0 text-primary"
																					to={`/environments/${environmentId}/containers?q=${encodeURIComponent(task.containerId)}`}
																					onClick={(event) =>
																						event.stopPropagation()
																					}
																					aria-label="查看任务容器"
																				>
																					<ExternalLink className="size-3.5" />
																				</Link>
																			</Tooltip>
																		)}
																	</div>
																))
															)}
														</div>
													</div>
												</div>
											</TableCell>
										</tr>
									)}
								</Fragment>
							);
						})
					)}
				</TableBody>
			</DataTable>
		</div>
	);
}
