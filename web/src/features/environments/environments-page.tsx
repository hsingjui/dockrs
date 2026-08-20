import {
	CircleAlert,
	HardDrive,
	MoreHorizontal,
	Plus,
	Server,
} from "lucide-react";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { type EnvironmentSummary, useEnvironments } from "./use-environments";

type Filter = "all" | "online" | "local" | "agent";

function KpiCard(props: {
	label: string;
	value: React.ReactNode;
	hint?: React.ReactNode;
}) {
	return (
		<Card size="sm">
			<CardContent>
				<div className="text-muted-foreground text-xs">{props.label}</div>
				<div className="mt-1 font-semibold text-2xl tabular-nums">
					{props.value}
				</div>
				{props.hint && (
					<div className="mt-1 text-muted-foreground text-xs">{props.hint}</div>
				)}
			</CardContent>
		</Card>
	);
}

function StatusPill({ status }: { status: EnvironmentSummary["status"] }) {
	const online = status === "online";
	return (
		<span
			className={cn(
				"inline-flex shrink-0 items-center gap-1 rounded-full border px-2 py-0.5 font-medium text-[11px]",
				online
					? "border-emerald-200/60 bg-emerald-50 text-emerald-700"
					: "border-rose-200/60 bg-rose-50 text-rose-600",
			)}
		>
			<span
				className={cn(
					"size-1.5 rounded-full",
					online ? "bg-emerald-500" : "bg-rose-500",
				)}
				aria-hidden="true"
			/>
			{online ? "在线" : "离线"}
		</span>
	);
}

function Meter(props: { label: string; value: string; percent: number }) {
	return (
		<div>
			<div className="mb-1 flex justify-between text-[11px] text-muted-foreground">
				<span>{props.label}</span>
				<span className="font-medium font-mono text-foreground/80 tabular-nums">
					{props.value}
				</span>
			</div>
			<div className="h-1.5 w-full overflow-hidden rounded-full bg-muted">
				<div
					className={cn(
						"h-full rounded-full",
						props.percent > 80 ? "bg-amber-500" : "bg-primary",
					)}
					style={{ width: `${Math.min(props.percent, 100)}%` }}
				/>
			</div>
		</div>
	);
}

function EnvironmentCard({ env }: { env: EnvironmentSummary }) {
	const online = env.status === "online";
	const total = env.containers.running + env.containers.stopped;
	const memPercent = env.memory
		? Math.round((env.memory.usedGb / env.memory.totalGb) * 100)
		: null;

	return (
		<div className="flex flex-col justify-between overflow-hidden rounded-xl bg-card ring-1 ring-foreground/10 transition-shadow hover:shadow-md">
			<div className="p-5">
				{/* 头部 */}
				<div className="flex items-start justify-between gap-3">
					<div className="min-w-0">
						<div className="flex items-center gap-2">
							{env.kind === "local" ? (
								<HardDrive className="size-4 shrink-0 text-muted-foreground" />
							) : (
								<Server className="size-4 shrink-0 text-muted-foreground" />
							)}
							<h3 className="truncate font-semibold text-sm">{env.name}</h3>
							<StatusPill status={env.status} />
						</div>
						<div className="mt-0.5 truncate pl-6 font-mono text-muted-foreground text-xs">
							{env.endpoint}
						</div>
					</div>
					<Button
						variant="ghost"
						size="icon-xs"
						aria-label={`${env.name} 更多操作`}
					>
						<MoreHorizontal />
					</Button>
				</div>

				{/* 标签 */}
				<div className="mt-3 flex flex-wrap gap-1.5">
					{env.tags.map((tag) => (
						<span
							key={tag}
							className="rounded bg-muted px-1.5 py-0.5 font-medium text-[10px] text-muted-foreground"
						>
							{tag}
						</span>
					))}
				</div>

				{online ? (
					<>
						{/* 容器计数 */}
						<div className="mt-4 text-muted-foreground text-xs">
							容器{" "}
							<span className="font-medium font-mono text-foreground tabular-nums">
								{total}
							</span>
							<span className="mx-1.5 text-border">|</span>
							运行中{" "}
							<span className="font-medium font-mono text-emerald-600 tabular-nums">
								{env.containers.running}
							</span>
							<span className="mx-1.5 text-border">|</span>
							已停止{" "}
							<span className="font-medium font-mono tabular-nums">
								{env.containers.stopped}
							</span>
						</div>

						{/* 资源监控条 */}
						{env.cpuPercent !== null && env.memory && memPercent !== null && (
							<div className="mt-4 space-y-2.5">
								<Meter
									label="CPU 负载"
									value={`${env.cpuPercent}%`}
									percent={env.cpuPercent}
								/>
								<Meter
									label="内存占用"
									value={`${env.memory.usedGb} GB / ${env.memory.totalGb} GB (${memPercent}%)`}
									percent={memPercent}
								/>
							</div>
						)}
					</>
				) : (
					/* 离线错误态 */
					<div className="mt-4 flex items-start gap-2.5 rounded-lg border border-rose-200/60 bg-rose-50/60 px-3 py-3">
						<CircleAlert className="mt-0.5 size-4 shrink-0 text-rose-500" />
						<p className="text-rose-600 text-xs leading-relaxed">
							{env.offlineReason ?? "环境离线"}
						</p>
					</div>
				)}
			</div>

			{/* 底部栏 */}
			<div className="flex items-center justify-between border-t px-5 py-3">
				<span className="font-mono text-muted-foreground text-xs">
					{env.dockerVersion
						? `Docker v${env.dockerVersion}`
						: "Docker 版本未知"}
				</span>
				<button
					type="button"
					className="inline-flex cursor-pointer items-center gap-1 font-medium text-primary text-xs transition-colors hover:text-primary/80"
				>
					进入控制台
					<span aria-hidden="true">→</span>
				</button>
			</div>
		</div>
	);
}

export function EnvironmentsPage() {
	const { data: environments = [] } = useEnvironments();
	const [filter, setFilter] = useState<Filter>("all");

	const onlineCount = environments.filter((e) => e.status === "online").length;
	const runningCount = environments.reduce(
		(sum, e) => sum + e.containers.running,
		0,
	);
	const totalCount = environments.reduce(
		(sum, e) => sum + e.containers.running + e.containers.stopped,
		0,
	);
	const runningRate =
		totalCount > 0 ? Math.round((runningCount / totalCount) * 1000) / 10 : 0;
	const memEnvs = environments.filter((e) => e.memory !== null);
	const avgMem =
		memEnvs.length > 0
			? Math.round(
					(memEnvs.reduce(
						(sum, e) =>
							sum + (e.memory?.usedGb ?? 0) / (e.memory?.totalGb ?? 1),
						0,
					) /
						memEnvs.length) *
						1000,
				) / 10
			: 0;

	const tabs: { key: Filter; label: string; count: number }[] = [
		{ key: "all", label: "全部环境", count: environments.length },
		{ key: "online", label: "在线", count: onlineCount },
		{
			key: "local",
			label: "本地 Socket",
			count: environments.filter((e) => e.kind === "local").length,
		},
		{
			key: "agent",
			label: "远程 Agent",
			count: environments.filter((e) => e.kind === "agent").length,
		},
	];
	const filtered = environments.filter((e) => {
		if (filter === "online") return e.status === "online";
		if (filter === "local") return e.kind === "local";
		if (filter === "agent") return e.kind === "agent";
		return true;
	});

	return (
		<div className="mx-auto w-full max-w-7xl space-y-6 px-6 py-8 lg:px-8">
			{/* 页头 */}
			<header className="flex items-end justify-between gap-4">
				<div>
					<h1 className="font-semibold text-xl">环境</h1>
					<p className="mt-1 text-muted-foreground text-sm">
						管理本机与远程 Agent 的 Docker 环境
					</p>
				</div>
				<Button>
					<Plus />
					添加环境
				</Button>
			</header>

			{/* KPI 看板 */}
			<div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-4">
				<KpiCard
					label="环境"
					value={environments.length}
					hint={
						<span className="flex items-center gap-2">
							<span className="text-emerald-600">{onlineCount} 在线</span>
							<span className="text-border">•</span>
							<span className="text-rose-500">
								{environments.length - onlineCount} 离线
							</span>
						</span>
					}
				/>
				<KpiCard
					label="容器"
					value={totalCount}
					hint={`运行率 ${runningRate}%`}
				/>
				<KpiCard
					label="运行中"
					value={runningCount}
					hint={`已停止 ${totalCount - runningCount}`}
				/>
				<KpiCard
					label="平均内存"
					value={`${avgMem}%`}
					hint={`基于 ${memEnvs.length} 个在线环境`}
				/>
			</div>

			{/* 筛选 */}
			<div className="flex items-center justify-between gap-4">
				<div className="flex items-center gap-1.5 rounded-lg border border-border/80 bg-muted p-1">
					{tabs.map((tab) => (
						<button
							key={tab.key}
							type="button"
							onClick={() => setFilter(tab.key)}
							className={cn(
								"cursor-pointer rounded-md px-3 py-1 font-medium text-xs transition-colors",
								filter === tab.key
									? "bg-card text-foreground shadow-sm"
									: "text-muted-foreground hover:text-foreground",
							)}
						>
							{tab.label} ({tab.count})
						</button>
					))}
				</div>
			</div>

			{/* 环境卡片网格 */}
			<div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
				{filtered.map((env) => (
					<EnvironmentCard key={env.id} env={env} />
				))}

				{/* 接入引导卡 */}
				<button
					type="button"
					className="flex min-h-48 cursor-pointer flex-col items-center justify-center gap-2 rounded-xl border border-border border-dashed px-6 py-10 text-center transition-colors hover:border-primary/50 hover:bg-primary/[0.02]"
				>
					<Plus className="size-5 text-muted-foreground" />
					<span className="font-medium text-sm">添加环境</span>
					<span className="text-muted-foreground text-xs">
						支持本地 Socket、TCP 或远程 Agent
					</span>
				</button>
			</div>
		</div>
	);
}
