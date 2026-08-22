import { CircleAlert, LoaderCircle, RefreshCw, Server } from "lucide-react";
import { type ReactNode, useState } from "react";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { cn } from "@/lib/utils";
import { EnvironmentCard } from "./environment-card";
import { useEnvironments } from "./use-environments";

type Filter = "all" | "online" | "local" | "agent";

function KpiCard(props: { label: string; value: ReactNode; hint?: ReactNode }) {
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

function PageState(props: {
	icon: ReactNode;
	message: string;
	action?: ReactNode;
}) {
	return (
		<div className="flex min-h-56 flex-col items-center justify-center gap-3 rounded-xl border border-border border-dashed text-center">
			<div className="text-muted-foreground">{props.icon}</div>
			<p className="text-muted-foreground text-sm">{props.message}</p>
			{props.action}
		</div>
	);
}

export function EnvironmentsPage() {
	const query = useEnvironments();
	const environments = query.data ?? [];
	const [filter, setFilter] = useState<Filter>("all");

	const onlineCount = environments.filter(
		(environment) => environment.status === "online",
	).length;
	const runningCount = environments.reduce(
		(sum, environment) => sum + environment.containers.running,
		0,
	);
	const totalCount = environments.reduce(
		(sum, environment) => sum + environment.containers.total,
		0,
	);
	const stoppedCount = environments.reduce(
		(sum, environment) => sum + environment.containers.stopped,
		0,
	);
	const runningRate =
		totalCount > 0 ? Math.round((runningCount / totalCount) * 1000) / 10 : 0;
	const memEnvs = environments.filter(
		(environment) => environment.memory !== null,
	);
	const avgMem =
		memEnvs.length > 0
			? Math.round(
					(memEnvs.reduce(
						(sum, environment) => sum + (environment.memory?.percent ?? 0),
						0,
					) /
						memEnvs.length) *
						100,
				) / 100
			: 0;

	const tabs: { key: Filter; label: string; count: number }[] = [
		{ key: "all", label: "全部环境", count: environments.length },
		{ key: "online", label: "在线", count: onlineCount },
		{
			key: "local",
			label: "本地 Socket",
			count: environments.filter((environment) => environment.kind === "local")
				.length,
		},
		{
			key: "agent",
			label: "远程 Agent",
			count: environments.filter((environment) => environment.kind === "agent")
				.length,
		},
	];
	const filtered = environments.filter((environment) => {
		if (filter === "online") return environment.status === "online";
		if (filter === "local") return environment.kind === "local";
		if (filter === "agent") return environment.kind === "agent";
		return true;
	});

	return (
		<div className="mx-auto w-full max-w-7xl space-y-6 px-6 py-8 lg:px-8">
			<header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
				<div>
					<h1 className="font-semibold text-xl">环境</h1>
				</div>
				<Button
					variant="outline"
					className="self-start sm:self-auto"
					onClick={() => void query.refetch()}
					disabled={query.isFetching}
				>
					<RefreshCw className={cn(query.isFetching && "animate-spin")} />
					刷新
				</Button>
			</header>

			{query.isPending ? (
				<PageState
					icon={
						<LoaderCircle className="size-5 animate-spin" aria-hidden="true" />
					}
					message="正在读取 Docker 环境"
				/>
			) : query.isError ? (
				<PageState
					icon={<CircleAlert className="size-5" aria-hidden="true" />}
					message={
						query.error instanceof Error
							? query.error.message
							: "环境列表加载失败，请稍后重试"
					}
					action={
						<Button
							variant="outline"
							onClick={() => void query.refetch()}
							disabled={query.isFetching}
						>
							<RefreshCw className={cn(query.isFetching && "animate-spin")} />
							重试
						</Button>
					}
				/>
			) : environments.length === 0 ? (
				<PageState
					icon={<Server className="size-5" aria-hidden="true" />}
					message="暂无 Docker 环境"
				/>
			) : (
				<>
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
							hint={`已停止 ${stoppedCount}`}
						/>
						<KpiCard
							label="平均内存"
							value={`${avgMem}%`}
							hint={`基于 ${memEnvs.length} 个环境`}
						/>
					</div>

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

					<div className="grid grid-cols-1 gap-5 md:grid-cols-2 xl:grid-cols-3">
						{filtered.map((environment) => (
							<EnvironmentCard key={environment.id} env={environment} />
						))}
					</div>
				</>
			)}
		</div>
	);
}
