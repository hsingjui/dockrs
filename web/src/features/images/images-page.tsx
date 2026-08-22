import { zodResolver } from "@hookform/resolvers/zod";
import { Download, LoaderCircle, RefreshCw, Trash2 } from "lucide-react";
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
	formatBytes,
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
import { useImages, usePullImage, useRemoveImage } from "./use-images";

const pullSchema = z.object({
	reference: z
		.string()
		.trim()
		.min(1, "请输入镜像引用")
		.max(256, "镜像引用过长"),
});

type PullValues = z.infer<typeof pullSchema>;

export function ImagesPage() {
	const { environmentId } = useParams();
	const [q, setQ] = useState("");
	const [showPull, setShowPull] = useState(false);
	const query = useImages(environmentId, { q });
	const pullMutation = usePullImage();
	const removeMutation = useRemoveImage();
	const {
		register,
		handleSubmit,
		reset,
		formState: { errors },
	} = useForm<PullValues>({ resolver: zodResolver(pullSchema) });

	if (query.isPending) return <ResourcePageState message="正在读取镜像" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error
						? query.error.message
						: "镜像列表加载失败"
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

	const onPull = (values: PullValues) => {
		if (!environmentId) return;
		pullMutation.mutate(
			{ environmentId, reference: values.reference },
			{
				onSuccess: () => {
					reset();
					setShowPull(false);
					toast.success("镜像拉取完成");
				},
				onError: (error) => toast.error(error.message),
			},
		);
	};

	const onRemove = (id: string, label: string) => {
		if (
			!environmentId ||
			!window.confirm(`确定删除镜像“${label}”吗？正在使用的镜像无法删除。`)
		)
			return;
		removeMutation.mutate(
			{ environmentId, imageId: id },
			{
				onSuccess: () => toast.success("镜像已删除"),
				onError: (error) => toast.error(error.message),
			},
		);
	};

	return (
		<div className="space-y-5">
			<ResourceHeader
				title="镜像"
				count={query.data.length}
				onRefresh={() => void query.refetch()}
				isRefreshing={query.isFetching}
				action={
					<Button
						variant={showPull ? "secondary" : "default"}
						onClick={() => setShowPull((value) => !value)}
					>
						<Download />
						拉取镜像
					</Button>
				}
			/>
			{showPull && (
				<form
					className="flex flex-col gap-3 rounded-xl border bg-card p-4 sm:flex-row sm:items-end"
					onSubmit={handleSubmit(onPull)}
					noValidate
				>
					<div className="min-w-0 flex-1 space-y-2">
						<Label htmlFor="image-reference">镜像引用</Label>
						<Input
							id="image-reference"
							placeholder="例如 nginx:latest"
							aria-invalid={!!errors.reference}
							{...register("reference")}
						/>
						{errors.reference && (
							<p className="text-destructive text-xs">
								{errors.reference.message}
							</p>
						)}
					</div>
					<Button type="submit" disabled={pullMutation.isPending}>
						{pullMutation.isPending && (
							<LoaderCircle className="animate-spin" />
						)}
						开始拉取
					</Button>
				</form>
			)}
			<div className="flex items-center rounded-xl border bg-card p-3">
				<SearchField
					value={q}
					onChange={setQ}
					placeholder="搜索仓库、标签或 ID"
					label="搜索镜像"
				/>
			</div>
			<DataTable minWidth="min-w-[820px]">
				<TableHead>
					<tr>
						<TableHeaderCell>仓库 / 标签</TableHeaderCell>
						<TableHeaderCell>镜像 ID</TableHeaderCell>
						<TableHeaderCell>大小</TableHeaderCell>
						<TableHeaderCell>创建时间</TableHeaderCell>
						<TableHeaderCell>使用中</TableHeaderCell>
						<TableHeaderCell className="text-right">操作</TableHeaderCell>
					</tr>
				</TableHead>
				<TableBody>
					{query.data.length === 0 ? (
						<EmptyTableState colSpan={6} message="没有匹配的镜像" />
					) : (
						query.data.map((image) => {
							const label = image.tags[0] ?? "无标签镜像";
							return (
								<TableRow key={image.id}>
									<TableCell>
										<div className="max-w-[250px] truncate font-medium">
											{label}
										</div>
										<div className="mt-1 text-muted-foreground text-xs">
											{image.tags.length > 1
												? `另有 ${image.tags.length - 1} 个标签`
												: image.dangling
													? "悬空镜像"
													: ""}
										</div>
									</TableCell>
									<TableCell className="font-mono text-muted-foreground text-xs">
										{image.id.replace(/^sha256:/, "").slice(0, 12)}
									</TableCell>
									<TableCell className="font-mono tabular-nums">
										{formatBytes(Math.max(image.size, 0))}
									</TableCell>
									<TableCell className="whitespace-nowrap text-muted-foreground text-xs">
										{formatTimestamp(image.created)}
									</TableCell>
									<TableCell className="font-mono tabular-nums">
										{image.containers < 0 ? "未知" : image.containers}
									</TableCell>
									<TableCell>
										<Tooltip
											content={
												image.containers > 0 ? "镜像正在使用" : "删除镜像"
											}
										>
											<Button
												variant="ghost"
												size="icon-sm"
												className="text-destructive hover:text-destructive"
												disabled={
													removeMutation.isPending || image.containers > 0
												}
												onClick={() => onRemove(image.id, label)}
												aria-label={`删除 ${label}`}
											>
												<Trash2 />
											</Button>
										</Tooltip>
									</TableCell>
								</TableRow>
							);
						})
					)}
				</TableBody>
			</DataTable>
		</div>
	);
}
