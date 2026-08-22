import {
	Box,
	CircleAlert,
	Container,
	HardDrive,
	Layers,
	type LucideIcon,
	Network,
	RefreshCw,
} from "lucide-react";
import { Link, useParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
	formatBytes,
	ResourcePageState,
} from "@/features/resources/resource-components";
import { cn } from "@/lib/utils";
import { useEnvironmentOverview } from "./use-environments";

function Meter({
	label,
	value,
	percent,
}: {
	label: string;
	value: string;
	percent: number;
}) {
	return (
		<div>
			<div className="mb-1 flex justify-between text-xs">
				<span className="text-muted-foreground">{label}</span>
				<span className="font-mono text-foreground/80 tabular-nums">
					{value}
				</span>
			</div>
			<div className="h-1.5 overflow-hidden rounded-full bg-muted">
				<div
					className={cn(
						"h-full rounded-full",
						percent > 80 ? "bg-amber-500" : "bg-primary",
					)}
					style={{ width: `${Math.min(Math.max(percent, 0), 100)}%` }}
				/>
			</div>
		</div>
	);
}

function ResourceCard(props: {
	label: string;
	description: string;
	count: number | null;
	error: string | null;
	href: string;
	icon: LucideIcon;
	meta?: string;
}) {
	const Icon = props.icon;
	return (
		<Link
			to={props.href}
			className="group rounded-xl bg-card p-5 ring-1 ring-foreground/10 transition-shadow hover:shadow-md focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring"
		>
			<div className="flex items-start justify-between gap-3">
				<div className="flex size-9 items-center justify-center rounded-lg bg-primary/10 text-primary">
					<Icon className="size-4" aria-hidden="true" />
				</div>
				<span
					className="text-muted-foreground transition-transform group-hover:translate-x-0.5"
					aria-hidden="true"
				>
					→
				</span>
			</div>
			<div className="mt-4">
				<div className="font-medium text-sm">{props.label}</div>
				<div className="mt-1 text-muted-foreground text-xs">
					{props.description}
				</div>
				<div className="mt-4 flex items-baseline gap-2">
					<span className="font-semibold font-mono text-2xl tabular-nums">
						{props.count === null ? "--" : props.count}
					</span>
					<span className="text-muted-foreground text-xs">
						{props.meta ?? "总数"}
					</span>
				</div>
				{props.error && (
					<p className="mt-2 text-amber-600 text-xs dark:text-amber-400">
						{props.error}
					</p>
				)}
			</div>
		</Link>
	);
}

export function EnvironmentDashboard() {
	const { environmentId } = useParams();
	const query = useEnvironmentOverview(environmentId);

	if (query.isPending)
		return <ResourcePageState message="正在读取环境仪表盘" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error ? query.error.message : "仪表盘加载失败"
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

	const { environment, resources } = query.data;
	const offline = environment.status === "offline";
	const memory = environment.memory;
	const prefix = `/environments/${environmentId}`;
	return (
		<div className="space-y-6">
			<div className="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
				<div>
					<h2 className="font-semibold text-lg">环境概览</h2>
				</div>
				<Button
					variant="outline"
					onClick={() => void query.refetch()}
					disabled={query.isFetching}
				>
					<RefreshCw className={cn(query.isFetching && "animate-spin")} />
					刷新数据
				</Button>
			</div>

			{offline && (
				<div className="flex items-start gap-3 rounded-xl border border-rose-200/70 bg-rose-50/70 p-4 text-rose-700 dark:border-rose-900/60 dark:bg-rose-950/30 dark:text-rose-300">
					<CircleAlert className="mt-0.5 size-4 shrink-0" aria-hidden="true" />
					<div className="min-w-0">
						<p className="font-medium text-sm">当前环境不可用</p>
						<p className="mt-1 text-xs">
							{environment.offlineReason ??
								environment.metricsError ??
								"Docker Engine 暂时无法连接"}
						</p>
						<Link
							className="mt-2 inline-block font-medium text-xs underline underline-offset-4"
							to="/"
						>
							返回环境列表
						</Link>
					</div>
				</div>
			)}

			<div className="grid gap-4 lg:grid-cols-[1.2fr_0.8fr]">
				<Card>
					<CardHeader className="border-b">
						<CardTitle>运行指标</CardTitle>
					</CardHeader>
					<CardContent className="space-y-4 pt-5">
						{environment.cpuPercent === null ? (
							<p className="text-muted-foreground text-sm">CPU 指标采集中</p>
						) : (
							<Meter
								label="容器 CPU"
								value={`${environment.cpuPercent.toFixed(1)}%`}
								percent={environment.cpuPercent}
							/>
						)}
						{memory === null ? (
							<p className="text-muted-foreground text-sm">内存指标采集中</p>
						) : (
							<Meter
								label="容器内存"
								value={`${formatBytes(memory.usedBytes)} / ${formatBytes(memory.totalBytes)}`}
								percent={memory.percent}
							/>
						)}
						{environment.metricsError && (
							<p className="text-amber-600 text-xs dark:text-amber-400">
								{environment.metricsError}
							</p>
						)}
					</CardContent>
				</Card>
				<Card>
					<CardHeader className="border-b">
						<CardTitle>容器状态</CardTitle>
					</CardHeader>
					<CardContent className="grid grid-cols-2 gap-4 pt-5 sm:grid-cols-4 lg:grid-cols-2">
						{[
							["总数", environment.containers.total, "text-foreground"],
							["运行中", environment.containers.running, "text-emerald-600"],
							["暂停", environment.containers.paused, "text-amber-600"],
							[
								"已停止",
								environment.containers.stopped,
								"text-muted-foreground",
							],
						].map(([label, value, color]) => (
							<div key={label}>
								<div className="text-muted-foreground text-xs">{label}</div>
								<div
									className={cn(
										"mt-1 font-semibold font-mono text-2xl tabular-nums",
										color,
									)}
								>
									{value}
								</div>
							</div>
						))}
					</CardContent>
				</Card>
			</div>

			<div>
				<div className="mb-3 flex items-baseline justify-between gap-3">
					<h2 className="font-semibold text-lg">Docker 资源</h2>
					<span className="text-muted-foreground text-xs">
						进入资源列表执行操作
					</span>
				</div>
				<div className="grid gap-4 sm:grid-cols-2 xl:grid-cols-5">
					<ResourceCard
						label="堆栈"
						description="Swarm 服务编排"
						count={resources.stacks.total}
						error={resources.stacks.error}
						meta="个 Stack"
						href={`${prefix}/stacks`}
						icon={Layers}
					/>
					<ResourceCard
						label="容器"
						description="运行状态与生命周期"
						count={resources.containers.total}
						error={resources.containers.error}
						meta="个容器"
						href={`${prefix}/containers`}
						icon={Box}
					/>
					<ResourceCard
						label="镜像"
						description="本地镜像缓存"
						count={resources.images.total}
						error={resources.images.error}
						meta="个镜像"
						href={`${prefix}/images`}
						icon={Container}
					/>
					<ResourceCard
						label="存储卷"
						description="持久化数据卷"
						count={resources.volumes.total}
						error={resources.volumes.error}
						meta="个 Volume"
						href={`${prefix}/volumes`}
						icon={HardDrive}
					/>
					<ResourceCard
						label="网络"
						description="容器网络与连接"
						count={resources.networks.total}
						error={resources.networks.error}
						meta="个 Network"
						href={`${prefix}/networks`}
						icon={Network}
					/>
				</div>
			</div>
		</div>
	);
}
