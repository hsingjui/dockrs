import { HardDrive, Server } from "lucide-react";
import { Link, Outlet, useParams } from "react-router-dom";
import { Button } from "@/components/ui/button";
import {
	formatTimestamp,
	ResourcePageState,
} from "@/features/resources/resource-components";
import type { EnvironmentSummary } from "./use-environments";
import { useEnvironment } from "./use-environments";

function StatusPill({ status }: { status: EnvironmentSummary["status"] }) {
	const online = status === "online";
	return (
		<span
			className={
				online
					? "inline-flex items-center gap-1.5 rounded-full border border-emerald-200/60 bg-emerald-50 px-2 py-0.5 font-medium text-emerald-700 text-xs dark:border-emerald-800/60 dark:bg-emerald-950/40 dark:text-emerald-400"
					: "inline-flex items-center gap-1.5 rounded-full border border-rose-200/60 bg-rose-50 px-2 py-0.5 font-medium text-rose-600 text-xs dark:border-rose-800/60 dark:bg-rose-950/40 dark:text-rose-400"
			}
		>
			<span
				className={
					online
						? "size-1.5 rounded-full bg-emerald-500"
						: "size-1.5 rounded-full bg-rose-500"
				}
			/>
			{online ? "在线" : "离线"}
		</span>
	);
}

export function EnvironmentLayout() {
	const { environmentId } = useParams();
	const query = useEnvironment(environmentId);

	if (!environmentId || query.isPending) {
		return <ResourcePageState message="正在读取环境信息" />;
	}

	if (query.isError || !query.data) {
		return (
			<div className="mx-auto flex min-h-dvh w-full max-w-3xl items-center px-6 py-8">
				<ResourcePageState
					variant="error"
					message={
						query.error instanceof Error
							? query.error.message
							: "环境不存在或暂时无法访问"
					}
					action={
						<Button asChild variant="outline">
							<Link to="/">返回环境列表</Link>
						</Button>
					}
				/>
			</div>
		);
	}

	const environment = query.data;
	return (
		<div className="mx-auto w-full max-w-7xl space-y-6 px-6 py-6 lg:px-8 lg:py-8">
			<header className="space-y-4 border-b pb-5">
				<div className="flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
					<div className="min-w-0">
						<div className="flex flex-wrap items-center gap-2.5">
							{environment.kind === "local" ? (
								<HardDrive
									className="size-4 text-muted-foreground"
									aria-hidden="true"
								/>
							) : (
								<Server
									className="size-4 text-muted-foreground"
									aria-hidden="true"
								/>
							)}
							<h1 className="truncate font-semibold text-xl">
								{environment.name}
							</h1>
							<StatusPill status={environment.status} />
							<span className="rounded bg-muted px-1.5 py-0.5 text-muted-foreground text-xs">
								{environment.kind === "local" ? "本地 Socket" : "远程 Agent"}
							</span>
						</div>
						<div className="mt-2 flex flex-wrap gap-x-4 gap-y-1 font-mono text-muted-foreground text-xs">
							<span className="max-w-full truncate">
								{environment.endpoint}
							</span>
							{environment.dockerVersion && (
								<span>Docker {environment.dockerVersion}</span>
							)}
							<span>
								采集于 {formatTimestamp(environment.metricsCollectedAt)}
							</span>
						</div>
					</div>
				</div>
			</header>
			<Outlet context={{ environment }} />
		</div>
	);
}
