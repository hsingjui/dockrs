import { zodResolver } from "@hookform/resolvers/zod";
import { LoaderCircle, Network, Plus, RefreshCw, Trash2 } from "lucide-react";
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
import {
	useCreateNetwork,
	useNetworks,
	useRemoveNetwork,
} from "./use-networks";

const networkSchema = z.object({
	name: z.string().trim().min(1, "请输入网络名称").max(128, "名称过长"),
	driver: z.string().trim().max(128, "driver 过长").optional(),
	internal: z.boolean(),
});

type NetworkValues = z.infer<typeof networkSchema>;

export function NetworksPage() {
	const { environmentId } = useParams();
	const [q, setQ] = useState("");
	const [showCreate, setShowCreate] = useState(false);
	const query = useNetworks(environmentId, q);
	const createMutation = useCreateNetwork();
	const removeMutation = useRemoveNetwork();
	const {
		register,
		handleSubmit,
		reset,
		formState: { errors },
	} = useForm<NetworkValues>({
		resolver: zodResolver(networkSchema),
		defaultValues: { driver: "bridge", internal: false },
	});

	if (query.isPending) return <ResourcePageState message="正在读取网络" />;
	if (query.isError || !query.data) {
		return (
			<ResourcePageState
				variant="error"
				message={
					query.error instanceof Error
						? query.error.message
						: "网络列表加载失败"
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

	const onCreate = (values: NetworkValues) => {
		if (!environmentId) return;
		createMutation.mutate(
			{
				environmentId,
				request: {
					name: values.name,
					driver: values.driver || undefined,
					internal: values.internal,
				},
			},
			{
				onSuccess: () => {
					reset({ driver: "bridge", internal: false });
					setShowCreate(false);
					toast.success("网络已创建");
				},
				onError: (error) => toast.error(error.message),
			},
		);
	};

	const onRemove = (id: string, name: string) => {
		if (
			!environmentId ||
			!window.confirm(`确定删除网络“${name}”吗？仍有连接时 Docker 会拒绝删除。`)
		)
			return;
		removeMutation.mutate(
			{ environmentId, networkId: id },
			{
				onSuccess: () => toast.success("网络已删除"),
				onError: (error) => toast.error(error.message),
			},
		);
	};

	return (
		<div className="space-y-5">
			<ResourceHeader
				title="网络"
				count={query.data.length}
				onRefresh={() => void query.refetch()}
				isRefreshing={query.isFetching}
				action={
					<Button
						variant={showCreate ? "secondary" : "default"}
						onClick={() => setShowCreate((value) => !value)}
					>
						<Plus />
						创建网络
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
						<Label htmlFor="network-name">名称</Label>
						<Input
							id="network-name"
							placeholder="例如 frontend"
							aria-invalid={!!errors.name}
							{...register("name")}
						/>
						{errors.name && (
							<p className="text-destructive text-xs">{errors.name.message}</p>
						)}
					</div>
					<div className="w-full space-y-2 sm:w-48">
						<Label htmlFor="network-driver">Driver</Label>
						<Input
							id="network-driver"
							placeholder="bridge"
							aria-invalid={!!errors.driver}
							{...register("driver")}
						/>
						{errors.driver && (
							<p className="text-destructive text-xs">
								{errors.driver.message}
							</p>
						)}
					</div>
					<label className="flex h-8 items-center gap-2 whitespace-nowrap text-sm">
						<input
							type="checkbox"
							className="size-4 accent-primary"
							{...register("internal")}
						/>
						内部网络
					</label>
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
					placeholder="搜索网络名称"
					label="搜索网络"
				/>
			</div>
			<DataTable minWidth="min-w-[820px]">
				<TableHead>
					<tr>
						<TableHeaderCell>网络</TableHeaderCell>
						<TableHeaderCell>Driver</TableHeaderCell>
						<TableHeaderCell>Scope</TableHeaderCell>
						<TableHeaderCell>子网 / 网关</TableHeaderCell>
						<TableHeaderCell>连接容器</TableHeaderCell>
						<TableHeaderCell className="text-right">操作</TableHeaderCell>
					</tr>
				</TableHead>
				<TableBody>
					{query.data.length === 0 ? (
						<EmptyTableState colSpan={6} message="没有匹配的网络" />
					) : (
						query.data.map((network) => (
							<TableRow key={network.id}>
								<TableCell>
									<div className="flex items-center gap-2 font-medium">
										<Network
											className="size-4 text-muted-foreground"
											aria-hidden="true"
										/>
										{network.name}
										{network.system && (
											<span className="rounded bg-muted px-1.5 py-0.5 text-muted-foreground text-[11px]">
												系统
											</span>
										)}
									</div>
								</TableCell>
								<TableCell className="font-mono text-xs">
									{network.driver ?? "未知"}
								</TableCell>
								<TableCell className="text-muted-foreground text-xs">
									{network.scope ?? "未知"}
								</TableCell>
								<TableCell className="font-mono text-muted-foreground text-xs">
									{network.subnet ?? "-"}
									{network.gateway ? ` / ${network.gateway}` : ""}
								</TableCell>
								<TableCell className="font-mono tabular-nums">
									{network.containerCount ?? "未知"}
								</TableCell>
								<TableCell>
									<Tooltip
										content={network.system ? "系统网络不可删除" : "删除网络"}
									>
										<Button
											variant="ghost"
											size="icon-sm"
											className="text-destructive hover:text-destructive"
											disabled={network.system || removeMutation.isPending}
											onClick={() => onRemove(network.id, network.name)}
											aria-label={`删除 ${network.name}`}
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
