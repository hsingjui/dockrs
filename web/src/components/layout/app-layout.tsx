import {
	Activity,
	Box,
	Container,
	FileCode2,
	HardDrive,
	Layers,
	LayoutGrid,
	Network,
	ScrollText,
	ShieldCheck,
	Terminal,
} from "lucide-react";
import { NavLink, Outlet } from "react-router-dom";
import { UserMenu } from "@/features/auth/user-menu";
import { cn } from "@/lib/utils";

interface NavItem {
	label: string;
	icon: React.ComponentType<{ className?: string }>;
	/** 已实现页面的路由；未实现页面为 null（禁用态） */
	to: string | null;
}

interface NavGroup {
	label: string;
	items: NavItem[];
}

const navGroups: NavGroup[] = [
	{
		label: "总览",
		items: [
			{ label: "环境列表", icon: LayoutGrid, to: "/" },
			{ label: "全局监控", icon: Activity, to: null },
		],
	},
	{
		label: "资源管理",
		items: [
			{ label: "容器", icon: Box, to: null },
			{ label: "镜像", icon: Layers, to: null },
			{ label: "Compose 编排", icon: FileCode2, to: null },
			{ label: "网络", icon: Network, to: null },
			{ label: "存储卷", icon: HardDrive, to: null },
		],
	},
	{
		label: "运维与工具",
		items: [
			{ label: "Web 终端", icon: Terminal, to: null },
			{ label: "实时日志流", icon: ScrollText, to: null },
			{ label: "Agent 节点配对", icon: ShieldCheck, to: null },
		],
	},
];

const itemClassName =
	"flex items-center gap-2.5 rounded-lg px-2.5 py-2 text-sm transition-colors";

function NavEntry({ item }: { item: NavItem }) {
	if (!item.to) {
		return (
			<span
				className={cn(
					itemClassName,
					"cursor-not-allowed text-muted-foreground/50",
				)}
				aria-disabled="true"
				title="即将上线"
			>
				<item.icon className="size-4" />
				{item.label}
			</span>
		);
	}
	return (
		<NavLink
			to={item.to}
			end={item.to === "/"}
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
	return (
		<div className="flex h-dvh">
			<aside className="flex w-60 shrink-0 flex-col border-sidebar-border border-r bg-sidebar">
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
				<nav className="flex-1 space-y-6 overflow-y-auto px-3 py-4">
					{navGroups.map((group) => (
						<div key={group.label}>
							<div className="px-2.5 pb-1.5 font-semibold text-[11px] text-muted-foreground/70 uppercase tracking-wider">
								{group.label}
							</div>
							<ul className="flex flex-col gap-0.5">
								{group.items.map((item) => (
									<li key={item.label}>
										<NavEntry item={item} />
									</li>
								))}
							</ul>
						</div>
					))}
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
