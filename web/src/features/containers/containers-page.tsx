import {
	Pause,
	Play,
	RefreshCw,
	RotateCcw,
	Square,
	Trash2,
} from "lucide-react";
import { useState } from "react";
import { useParams } from "react-router-dom";
import type { ContainerAction, ContainerResource } from "@/api/containers";
import { Button } from "@/components/ui/button";
import { Tooltip } from "@/components/ui/tooltip";
import {
	DataTable,
	EmptyTableState,
	formatTimestamp,
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
import {
	useContainerAction,
	useContainers,
	useRemoveContainer,
} from "./use-containers";

const stateLabels: Record<string, string> = {
	running: "运行中",
	paused: "已暂停",
	exited: "已停止",
	created: "已创建",
	restarting: "重启中",
	dead: "异常",
	removing: "删除中",
	stopping: "停止中",
};

function stateClass(state: string) {
	if (state === "running")
		return "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400";
	if (state === "paused" || state === "restarting")
		return "bg-amber-500/10 text-amber-700 dark:text-amber-400";
	if (state === "dead") return "bg-destructive/10 text-destructive";
	return "bg-muted text-muted-foreground";
}

function ContainerActions({ container }: { container: ContainerResource }) {
	const actionMutation = useContainerAction();
	const removeMutation = useRemoveContainer();
	const { environmentId } = useParams();
	if (!environmentId) return null;

	const run = (action: ContainerAction) => {
		actionMutation.mutate({ environmentId, containerId: container.id, action });
	};
	const remove = () => {
		if (!window.confirm(`确定删除容器“${container.name}”吗？此操作不可撤销。`))
			return;
		removeMutation.mutate({ environmentId, containerId: container.id });
	};
	const pending = actionMutation.isPending || removeMutation.isPending;
	return (
		<div className="flex items-center justify-end gap-0.5">
			{container.state === "running" ? (
				<>
					<Tooltip content="停止">
						<Button
							variant="ghost"
							size="icon-sm"
							disabled={pending}
							onClick={() => run("stop")}
							aria-label="停止"
						>
							<Square />
						</Button>
					</Tooltip>
					<Tooltip content="暂停">
						<Button
							variant="ghost"
							size="icon-sm"
							disabled={pending}
							onClick={() => run("pause")}
							aria-label="暂停"
						>
							<Pause />
						</Button>
					</Tooltip>
				</>
			) : container.state === "paused" ? (
				<Tooltip content="恢复">
					<Button
						variant="ghost"
						size="icon-sm"
						disabled={pending}
						onClick={() => run("unpause")}
						aria-label="恢复"
					>
						<Play />
					</Button>
				</Tooltip>
			) : (
				<Tooltip content="启动">
					<Button
						variant="ghost"
						size="icon-sm"
						disabled={pending}
						onClick={() => run("start")}
						aria-label="启动"
					>
						<Play />
					</Button>
				</Tooltip>
			)}
			<Tooltip content="重启">
				<Button
					variant="ghost"
					size="icon-sm"
					disabled={pending}
					onClick={() => run("restart")}
					aria-label="重启"
				>
					<RotateCcw />
				</Button>
			</Tooltip>
			<Tooltip content="删除" variant="destructive">
				<Button
					variant="ghost"
					size="icon-sm"
					className="text-destructive hover:text-destructive"
					disabled={pending}
					onClick={remove}
					aria-label="删除"
				>
					<Trash2 />
				</Button>
			</Tooltip>
		</div>
	);
}

export function ContainersPage() {
	const { environmentId } = useParams();
	const [q, setQ] = useState("");
	const [status, setStatus] = useState("");
	const query = useContainers(environmentId, { all: true, status, q });

	if (query.isPending) return <ResourcePageState message="正在读取容器" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error
						? query.error.message
						: "容器列表加载失败"
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

	return (
		<div className="space-y-5">
			<ResourceHeader
				title="容器"
				count={query.data.length}
				onRefresh={() => void query.refetch()}
				isRefreshing={query.isFetching}
			/>
			<div className="flex flex-col gap-3 rounded-xl border bg-card p-3 sm:flex-row sm:items-center">
				<SearchField
					value={q}
					onChange={setQ}
					placeholder="搜索名称或镜像"
					label="搜索容器"
				/>
				<label className="flex items-center gap-2 text-muted-foreground text-sm">
					<span className="whitespace-nowrap">状态</span>
					<select
						className="h-8 rounded-lg border border-input bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-ring"
						value={status}
						onChange={(event) => setStatus(event.target.value)}
						aria-label="按状态筛选"
					>
						<option value="">全部</option>
						<option value="running">运行中</option>
						<option value="paused">已暂停</option>
						<option value="exited">已停止</option>
						<option value="dead">异常</option>
					</select>
				</label>
			</div>
			<DataTable>
				<TableHead>
					<tr>
						<TableHeaderCell>容器</TableHeaderCell>
						<TableHeaderCell>状态</TableHeaderCell>
						<TableHeaderCell>镜像</TableHeaderCell>
						<TableHeaderCell>端口</TableHeaderCell>
						<TableHeaderCell>创建时间</TableHeaderCell>
						<TableHeaderCell className="text-right">操作</TableHeaderCell>
					</tr>
				</TableHead>
				<TableBody>
					{query.data.length === 0 ? (
						<EmptyTableState colSpan={6} message="没有匹配的容器" />
					) : (
						query.data.map((container) => (
							<TableRow key={container.id}>
								<TableCell>
									<div className="max-w-[220px] truncate font-medium">
										{container.name}
									</div>
									<div className="mt-0.5 max-w-[220px] truncate font-mono text-muted-foreground text-xs">
										{container.id.slice(0, 12)}
									</div>
									{container.stack && (
										<div className="mt-1 text-muted-foreground text-xs">
											堆栈：{container.stack}
										</div>
									)}
								</TableCell>
								<TableCell>
									<span
										className={cn(
											"inline-flex rounded-full px-2 py-0.5 font-medium text-xs",
											stateClass(container.state),
										)}
									>
										{stateLabels[container.state] ?? container.state}
									</span>
									{container.status && (
										<div className="mt-1 max-w-[150px] truncate text-muted-foreground text-xs">
											{container.status}
										</div>
									)}
								</TableCell>
								<TableCell className="max-w-[220px] truncate font-mono text-xs">
									{container.image ?? "未知"}
								</TableCell>
								<TableCell className="max-w-[180px] text-muted-foreground text-xs">
									{container.ports.length > 0
										? container.ports
												.map(
													(port) =>
														`${port.hostPort ?? "-"}:${port.containerPort}/${port.protocol}`,
												)
												.join(", ")
										: "无"}
								</TableCell>
								<TableCell className="whitespace-nowrap text-muted-foreground text-xs">
									{formatTimestamp(container.created)}
								</TableCell>
								<TableCell>
									<ContainerActions container={container} />
								</TableCell>
							</TableRow>
						))
					)}
				</TableBody>
			</DataTable>
		</div>
	);
}
