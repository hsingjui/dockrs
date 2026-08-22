import { zodResolver } from "@hookform/resolvers/zod";
import {
	Check,
	CircleAlert,
	HardDrive,
	LoaderCircle,
	MoreHorizontal,
	Pencil,
	Server,
	Trash2,
} from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { NavLink } from "react-router-dom";
import { z } from "zod";

import {
	AlertDialog,
	AlertDialogAction,
	AlertDialogCancel,
	AlertDialogContent,
	AlertDialogDescription,
	AlertDialogFooter,
	AlertDialogHeader,
	AlertDialogTitle,
	AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import {
	DropdownMenu,
	DropdownMenuContent,
	DropdownMenuItem,
	DropdownMenuSeparator,
	DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { Tooltip } from "@/components/ui/tooltip";
import { formatBytes } from "@/features/resources/resource-components";
import { cn } from "@/lib/utils";
import {
	type EnvironmentSummary,
	useDeleteEnvironment,
	useRenameEnvironment,
} from "./use-environments";

const renameSchema = z.object({
	name: z
		.string()
		.trim()
		.min(1, "环境名称不能为空")
		.max(64, "环境名称不能超过 64 个字符"),
});

type RenameFormValues = z.infer<typeof renameSchema>;

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

function EnvironmentCardMenu({
	env,
	onEdit,
}: {
	env: EnvironmentSummary;
	onEdit: () => void;
}) {
	const deleteMutation = useDeleteEnvironment();
	const canEdit = env.kind === "local";
	const canDelete = env.kind !== "local";

	return (
		<AlertDialog>
			<DropdownMenu>
				<Tooltip content="环境操作">
					<DropdownMenuTrigger asChild>
						<Button
							className="pointer-events-auto"
							variant="ghost"
							size="icon-xs"
							aria-label={`${env.name} 操作菜单`}
						>
							<MoreHorizontal />
						</Button>
					</DropdownMenuTrigger>
				</Tooltip>
				<DropdownMenuContent align="end">
					<Tooltip content={canEdit ? "修改环境名称" : "远程环境暂不支持改名"}>
						<DropdownMenuItem
							onSelect={() => {
								if (canEdit) onEdit();
							}}
							disabled={!canEdit}
							className={!canEdit ? "pointer-events-auto" : undefined}
						>
							<Pencil />
							编辑名称
						</DropdownMenuItem>
					</Tooltip>
					<DropdownMenuSeparator />
					<Tooltip content={canDelete ? "删除此环境" : "本地环境不支持删除"}>
						<AlertDialogTrigger asChild>
							<DropdownMenuItem
								variant="destructive"
								disabled={!canDelete}
								className={!canDelete ? "pointer-events-auto" : undefined}
							>
								<Trash2 />
								删除
							</DropdownMenuItem>
						</AlertDialogTrigger>
					</Tooltip>
				</DropdownMenuContent>
			</DropdownMenu>
			<AlertDialogContent>
				<AlertDialogHeader>
					<AlertDialogTitle>删除环境</AlertDialogTitle>
					<AlertDialogDescription>
						确定要删除远程环境「{env.name}」吗？该操作会移除其连接记录，已启动的
						Docker 资源不会被改动，Agent 重新连接后将再次注册。
					</AlertDialogDescription>
				</AlertDialogHeader>
				<AlertDialogFooter>
					<AlertDialogCancel disabled={deleteMutation.isPending}>
						取消
					</AlertDialogCancel>
					<AlertDialogAction
						className="bg-destructive text-white hover:bg-destructive/90"
						disabled={deleteMutation.isPending}
						onClick={(event) => {
							event.preventDefault();
							deleteMutation.mutate(env.id);
						}}
					>
						{deleteMutation.isPending ? (
							<LoaderCircle className="animate-spin" />
						) : (
							<Trash2 />
						)}
						删除
					</AlertDialogAction>
				</AlertDialogFooter>
			</AlertDialogContent>
		</AlertDialog>
	);
}

export function EnvironmentCard({ env }: { env: EnvironmentSummary }) {
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
		<div className="relative flex flex-col justify-between overflow-hidden rounded-xl bg-card ring-1 ring-foreground/10 transition-shadow hover:shadow-md">
			<NavLink
				to={`/environments/${env.id}`}
				className="absolute inset-0 z-0 rounded-xl focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
				aria-label={`打开 ${env.name} 环境仪表盘`}
			/>
			<div className="relative z-10 pointer-events-none p-5">
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
									className="pointer-events-auto flex min-w-0 flex-1 items-center gap-1"
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
					<EnvironmentCardMenu env={env} onEdit={toggleEditing} />
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

			<div className="relative z-10 pointer-events-none flex items-center justify-between border-t px-5 py-3">
				<span className="font-mono text-muted-foreground text-xs">
					{env.dockerVersion
						? `Docker v${env.dockerVersion}`
						: "Docker 版本未知"}
				</span>
				<span className="font-medium text-primary text-xs">进入仪表盘 →</span>
			</div>
		</div>
	);
}
