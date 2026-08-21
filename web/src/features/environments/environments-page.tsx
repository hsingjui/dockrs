import { zodResolver } from "@hookform/resolvers/zod";
import {
	Check,
	CircleAlert,
	HardDrive,
	LoaderCircle,
	MoreHorizontal,
	Plus,
	RefreshCw,
	Server,
	X,
} from "lucide-react";
import { type ReactNode, useState } from "react";
import { useForm } from "react-hook-form";
import { z } from "zod";

import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { cn } from "@/lib/utils";
import {
	type EnvironmentSummary,
	useEnvironments,
	useRenameEnvironment,
} from "./use-environments";

type Filter = "all" | "online" | "local" | "agent";

const renameSchema = z.object({
	name: z
		.string()
		.trim()
		.min(1, "环境名称不能为空")
		.max(64, "环境名称不能超过 64 个字符"),
});

type RenameFormValues = z.infer<typeof renameSchema>;

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
					style={{ width: `${Math.min(Math.max(props.percent, 0), 100)}%` }}
				/>
			</div>
		</div>
	);
}

function MetricPlaceholder({ label }: { label: string }) {
	return (
		<div className="flex items-center justify-between text-[11px] text-muted-foreground">
			<span>{label}</span>
			<span className="font-medium">采集中</span>
		</div>
	);
}

function formatBytes(bytes: number): string {
	if (bytes === 0) return "0 B";
	const units = ["B", "KB", "MB", "GB", "TB"];
	const index = Math.min(
		Math.floor(Math.log(bytes) / Math.log(1024)),
		units.length - 1,
	);
	const value = bytes / 1024 ** index;
	return `${value >= 10 || index === 0 ? value.toFixed(0) : value.toFixed(1)} ${units[index]}`;
}

function EnvironmentCard({ env }: { env: EnvironmentSummary }) {
	const online = env.status === "online";
	const [editing, setEditing] = useState(false);
	const [renameSuccess, setRenameSuccess] = useState(false);
	const renameMutation = useRenameEnvironment();
	const {
		register,
		handleSubmit,
		reset,
		formState: { errors },
	} = useForm<RenameFormValues>({
		resolver: zodResolver(renameSchema),
		defaultValues: { name: env.name },
	});

	const toggleEditing = () => {
		renameMutation.reset();
		setRenameSuccess(false);
		if (editing) {
			reset({ name: env.name });
		}
		setEditing((value) => !value);
	};

	const onRename = (values: RenameFormValues) => {
		renameMutation.mutate(
			{ id: env.id, request: { name: values.name } },
			{
				onSuccess: (updated) => {
					reset({ name: updated.name });
					setRenameSuccess(true);
					setEditing(false);
				},
			},
		);
	};

	return (
		<div className="flex flex-col justify-between overflow-hidden rounded-xl bg-card ring-1 ring-foreground/10 transition-shadow hover:shadow-md">
			<div className="p-5">
				<div className="flex items-start justify-between gap-3">
					<div className="min-w-0 flex-1">
						<div className="flex items-center gap-2">
							{env.kind === "local" ? (
								<HardDrive className="size-4 shrink-0 text-muted-foreground" />
							) : (
								<Server className="size-4 shrink-0 text-muted-foreground" />
							)}
							{editing && env.kind === "local" ? (
								<form
									className="flex min-w-0 flex-1 items-center gap-1"
									onSubmit={handleSubmit(onRename)}
									noValidate
								>
									<Input
										{...register("name")}
										aria-label="环境名称"
										aria-invalid={!!errors.name}
										className="h-7 min-w-0 px-2 text-sm"
										autoFocus
									/>
									<Button
										type="submit"
										variant="ghost"
										size="icon-xs"
										disabled={renameMutation.isPending}
										aria-label="保存环境名称"
									>
										{renameMutation.isPending ? (
											<LoaderCircle className="animate-spin" />
										) : (
											<Check />
										)}
									</Button>
								</form>
							) : (
								<h3 className="truncate font-semibold text-sm">{env.name}</h3>
							)}
							<StatusPill status={env.status} />
						</div>
						{editing && env.kind === "local" && (
							<div className="pl-6 text-destructive text-[11px]">
								{errors.name?.message ?? renameMutation.error?.message}
							</div>
						)}
						{renameSuccess && !editing && (
							<div className="pl-6 text-emerald-600 text-[11px]" role="status">
								环境名称已更新
							</div>
						)}
						<div className="mt-0.5 truncate pl-6 font-mono text-muted-foreground text-xs">
							{env.endpoint}
						</div>
					</div>
					<Button
						variant="ghost"
						size="icon-xs"
						disabled={env.kind !== "local" || renameMutation.isPending}
						onClick={toggleEditing}
						aria-label={editing ? "取消修改环境名称" : `${env.name} 修改名称`}
						title={
							env.kind === "local" ? "修改环境名称" : "远程环境暂不支持编辑"
						}
					>
						{editing ? <X /> : <MoreHorizontal />}
					</Button>
				</div>

				{online ? (
					<>
						<div className="mt-4 text-muted-foreground text-xs">
							容器{" "}
							<span className="font-medium font-mono text-foreground tabular-nums">
								{env.containers.total}
							</span>
							<span className="mx-1.5 text-border">|</span>
							运行中{" "}
							<span className="font-medium font-mono text-emerald-600 tabular-nums">
								{env.containers.running}
							</span>
							<span className="mx-1.5 text-border">|</span>
							暂停{" "}
							<span className="font-medium font-mono text-amber-600 tabular-nums">
								{env.containers.paused}
							</span>
							<span className="mx-1.5 text-border">|</span>
							已停止{" "}
							<span className="font-medium font-mono tabular-nums">
								{env.containers.stopped}
							</span>
						</div>

						<div className="mt-4 space-y-2.5">
							{env.cpuPercent === null ? (
								<MetricPlaceholder label="CPU 负载" />
							) : (
								<Meter
									label="CPU 负载"
									value={`${env.cpuPercent.toFixed(1)}%`}
									percent={env.cpuPercent}
								/>
							)}
							{env.memory === null ? (
								<MetricPlaceholder label="内存占用" />
							) : (
								<Meter
									label="内存占用"
									value={`${formatBytes(env.memory.usedBytes)} / ${formatBytes(env.memory.totalBytes)} (${env.memory.percent.toFixed(1)}%)`}
									percent={env.memory.percent}
								/>
							)}
						</div>
						{env.metricsError && (
							<p className="mt-3 text-amber-600 text-xs dark:text-amber-400">
								{env.metricsError}
							</p>
						)}
					</>
				) : (
					<div className="mt-4 flex items-start gap-2.5 rounded-lg border border-rose-200/60 bg-rose-50/60 px-3 py-3">
						<CircleAlert className="mt-0.5 size-4 shrink-0 text-rose-500" />
						<p className="text-rose-600 text-xs leading-relaxed">
							{env.offlineReason ?? "环境离线"}
						</p>
					</div>
				)}
			</div>

			<div className="flex items-center justify-between border-t px-5 py-3">
				<span className="font-mono text-muted-foreground text-xs">
					{env.dockerVersion
						? `Docker v${env.dockerVersion}`
						: "Docker 版本未知"}
				</span>
				<button
					type="button"
					className="inline-flex cursor-not-allowed items-center gap-1 font-medium text-muted-foreground text-xs"
					disabled
					title="资源控制台尚未接入"
				>
					进入控制台
					<span aria-hidden="true">→</span>
				</button>
			</div>
		</div>
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
			<header className="flex items-end justify-between gap-4">
				<div>
					<h1 className="font-semibold text-xl">环境</h1>
					<p className="mt-1 text-muted-foreground text-sm">
						管理本机与远程 Agent 的 Docker 环境
					</p>
				</div>
				<Button disabled title="远程环境配对尚未接入">
					<Plus />
					添加环境
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
						<Button variant="outline" onClick={() => void query.refetch()}>
							<RefreshCw />
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

						<button
							type="button"
							disabled
							className="flex min-h-48 cursor-not-allowed flex-col items-center justify-center gap-2 rounded-xl border border-border border-dashed px-6 py-10 text-center text-muted-foreground opacity-60"
							title="远程环境配对尚未接入"
						>
							<Plus className="size-5" />
							<span className="font-medium text-sm">添加环境</span>
							<span className="text-xs">远程 Agent 配对即将支持</span>
						</button>
					</div>
				</>
			)}
		</div>
	);
}
