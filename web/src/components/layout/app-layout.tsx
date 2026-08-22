import {
	Box,
	Container,
	HardDrive,
	Home,
	Layers,
	LayoutDashboard,
	Network,
	Server,
} from "lucide-react";
import { NavLink, Outlet, useMatch } from "react-router-dom";
import { UserMenu } from "@/features/auth/user-menu";
import { useEnvironments } from "@/features/environments/use-environments";
import { cn } from "@/lib/utils";

interface NavItem {
	label: string;
	icon: React.ComponentType<{ className?: string }>;
	to: string;
	end?: boolean;
}

const itemClassName =
	"flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors";

function NavEntry({ item }: { item: NavItem }) {
	return (
		<NavLink
			to={item.to}
			end={item.end}
			className={({ isActive }) =>
				cn(
					itemClassName,
					isActive
						? "bg-primary/10 font-medium text-primary"
						: "text-sidebar-foreground/70 hover:bg-sidebar-accent hover:text-sidebar-foreground",
				)
			}
		>
			<item.icon className="size-4" />
			{item.label}
		</NavLink>
	);
}

export function AppLayout() {
	const environmentMatch = useMatch("/environments/:environmentId/*");
	const environmentId = environmentMatch?.params.environmentId;
	const environmentsQuery = useEnvironments();

	// 未进入环境时默认选中本地环境，菜单始终展示
	const environment = environmentId
		? environmentsQuery.data?.find((item) => item.id === environmentId)
		: (environmentsQuery.data?.find((item) => item.kind === "local") ??
			environmentsQuery.data?.[0]);
	const activeId = environment?.id;

	const envPath = (resource: string) =>
		`/environments/${encodeURIComponent(activeId as string)}/${resource}`;

	const envItems: NavItem[] = activeId
		? [
				{
					label: "仪表盘",
					icon: LayoutDashboard,
					to: `/environments/${encodeURIComponent(activeId)}`,
					end: true,
				},
				{ label: "容器", icon: Box, to: envPath("containers") },
				{ label: "镜像", icon: Layers, to: envPath("images") },
				{ label: "堆栈", icon: Layers, to: envPath("stacks") },
				{ label: "网络", icon: Network, to: envPath("networks") },
				{ label: "存储卷", icon: HardDrive, to: envPath("volumes") },
			]
		: [];

	return (
		<div className="flex h-dvh">
			<aside className="flex w-52 shrink-0 flex-col border-sidebar-border border-r bg-sidebar">
				{/* 品牌区 */}
				<div className="flex h-16 items-center border-sidebar-border/80 border-b px-5">
					<div className="flex items-center gap-2.5">
						<span className="flex size-8 items-center justify-center rounded-lg bg-primary text-primary-foreground">
							<Container className="size-4" />
						</span>
						<span className="font-semibold text-base tracking-tight">
							dockrs
						</span>
						<span className="rounded border border-border bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
							v0.1.0
						</span>
					</div>
				</div>

				{/* 导航 */}
				<nav className="flex-1 overflow-y-auto px-3 py-4">
					<ul className="flex flex-col gap-0.5">
						<li>
							<NavEntry
								item={{
									label: "首页",
									icon: Home,
									to: "/",
									end: true,
								}}
							/>
						</li>
					</ul>

					{environment && (
						<div className="mt-2">
							<div className="flex items-center gap-1.5 px-2.5 pb-1.5 text-muted-foreground">
								{environment.kind === "local" ? (
									<HardDrive className="size-3 shrink-0" aria-hidden="true" />
								) : (
									<Server className="size-3 shrink-0" aria-hidden="true" />
								)}
								<span className="min-w-0 flex-1 truncate text-xs">
									{environment.name}
								</span>
								<span
									className={
										environment.status === "online"
											? "size-1.5 shrink-0 rounded-full bg-emerald-500"
											: "size-1.5 shrink-0 rounded-full bg-rose-500"
									}
									aria-hidden="true"
								/>
							</div>
							<ul className="flex flex-col gap-0.5">
								{envItems.map((item) => (
									<li key={item.label}>
										<NavEntry item={item} />
									</li>
								))}
							</ul>
						</div>
					)}
				</nav>

				{/* 底部用户区 */}
				<div className="border-sidebar-border border-t p-3">
					<UserMenu />
				</div>
			</aside>

			<main className="min-w-0 flex-1 overflow-y-auto">
				<Outlet />
			</main>
		</div>
	);
}
