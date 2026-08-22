import { zodResolver } from "@hookform/resolvers/zod";
import { HardDrive, LoaderCircle, Plus, RefreshCw, Trash2 } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { useParams } from "react-router-dom";
import { z } from "zod";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { toast } from "@/components/ui/toast";
import { Tooltip } from "@/components/ui/tooltip";
import {
	DataTable,
	EmptyTableState,
	ResourceHeader,
	ResourcePageState,
	SearchField,
	TableBody,
	TableCell,
	TableHead,
	TableHeaderCell,
	TableRow,
} from "@/features/resources/resource-components";
import { useCreateVolume, useRemoveVolume, useVolumes } from "./use-volumes";

const volumeSchema = z.object({
	name: z.string().trim().min(1, "请输入存储卷名称").max(128, "名称过长"),
	driver: z.string().trim().max(128, "driver 过长").optional(),
});

type VolumeValues = z.infer<typeof volumeSchema>;

export function VolumesPage() {
	const { environmentId } = useParams();
	const [q, setQ] = useState("");
	const [showCreate, setShowCreate] = useState(false);
	const query = useVolumes(environmentId, q);
	const createMutation = useCreateVolume();
	const removeMutation = useRemoveVolume();
	const {
		register,
		handleSubmit,
		reset,
		formState: { errors },
	} = useForm<VolumeValues>({
		resolver: zodResolver(volumeSchema),
		defaultValues: { driver: "local" },
	});

	if (query.isPending) return <ResourcePageState message="正在读取存储卷" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error
						? query.error.message
						: "存储卷列表加载失败"
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

	const onCreate = (values: VolumeValues) => {
		if (!environmentId) return;
		createMutation.mutate(
			{
				environmentId,
				request: { name: values.name, driver: values.driver || undefined },
			},
			{
				onSuccess: () => {
					reset({ driver: "local" });
					setShowCreate(false);
					toast.success("存储卷已创建");
				},
				onError: (error) => toast.error(error.message),
			},
		);
	};

	const onRemove = (name: string) => {
		if (
			!environmentId ||
			!window.confirm(
				`确定删除存储卷“${name}”吗？挂载中的存储卷会被 Docker 拒绝。`,
			)
		)
			return;
		removeMutation.mutate(
			{ environmentId, name },
			{
				onSuccess: () => toast.success("存储卷已删除"),
				onError: (error) => toast.error(error.message),
			},
		);
	};

	return (
		<div className="space-y-5">
			<ResourceHeader
				title="存储卷"
				count={query.data.length}
				onRefresh={() => void query.refetch()}
				isRefreshing={query.isFetching}
				action={
					<Button
						variant={showCreate ? "secondary" : "default"}
						onClick={() => setShowCreate((value) => !value)}
					>
						<Plus />
						创建存储卷
					</Button>
				}
			/>
			{showCreate && (
				<form
					className="flex flex-col gap-3 rounded-xl border bg-card p-4 sm:flex-row sm:items-end"
					onSubmit={handleSubmit(onCreate)}
					noValidate
				>
					<div className="min-w-0 flex-1 space-y-2">
						<Label htmlFor="volume-name">名称</Label>
						<Input
							id="volume-name"
							placeholder="例如 app-data"
							aria-invalid={!!errors.name}
							{...register("name")}
						/>
						{errors.name && (
							<p className="text-destructive text-xs">{errors.name.message}</p>
						)}
					</div>
					<div className="w-full space-y-2 sm:w-48">
						<Label htmlFor="volume-driver">Driver</Label>
						<Input
							id="volume-driver"
							placeholder="local"
							aria-invalid={!!errors.driver}
							{...register("driver")}
						/>
						{errors.driver && (
							<p className="text-destructive text-xs">
								{errors.driver.message}
							</p>
						)}
					</div>
					<Button type="submit" disabled={createMutation.isPending}>
						{createMutation.isPending && (
							<LoaderCircle className="animate-spin" />
						)}
						创建
					</Button>
				</form>
			)}
			<div className="flex items-center rounded-xl border bg-card p-3">
				<SearchField
					value={q}
					onChange={setQ}
					placeholder="搜索存储卷"
					label="搜索存储卷"
				/>
			</div>
			<DataTable minWidth="min-w-[780px]">
				<TableHead>
					<tr>
						<TableHeaderCell>名称</TableHeaderCell>
						<TableHeaderCell>Driver</TableHeaderCell>
						<TableHeaderCell>Scope</TableHeaderCell>
						<TableHeaderCell>挂载点</TableHeaderCell>
						<TableHeaderCell>使用中</TableHeaderCell>
						<TableHeaderCell className="text-right">操作</TableHeaderCell>
					</tr>
				</TableHead>
				<TableBody>
					{query.data.length === 0 ? (
						<EmptyTableState colSpan={6} message="没有匹配的存储卷" />
					) : (
						query.data.map((volume) => (
							<TableRow key={volume.name}>
								<TableCell>
									<div className="flex items-center gap-2 font-medium">
										<HardDrive
											className="size-4 text-muted-foreground"
											aria-hidden="true"
										/>
										{volume.name}
									</div>
								</TableCell>
								<TableCell className="font-mono text-xs">
									{volume.driver}
								</TableCell>
								<TableCell className="text-muted-foreground text-xs">
									{volume.scope ?? "未知"}
								</TableCell>
								<TableCell className="max-w-[280px] truncate font-mono text-muted-foreground text-xs">
									{volume.mountpoint || "未知"}
								</TableCell>
								<TableCell className="font-mono tabular-nums">
									{volume.usageContainers ?? "未知"}
								</TableCell>
								<TableCell>
									<Tooltip content="删除存储卷">
										<Button
											variant="ghost"
											size="icon-sm"
											className="text-destructive hover:text-destructive"
											disabled={removeMutation.isPending}
											onClick={() => onRemove(volume.name)}
											aria-label={`删除 ${volume.name}`}
										>
											<Trash2 />
										</Button>
									</Tooltip>
								</TableCell>
							</TableRow>
						))
					)}
				</TableBody>
			</DataTable>
		</div>
	);
}
