import { LoaderCircle, LogOut, Settings } from "lucide-react";
import { NavLink, useNavigate } from "react-router-dom";

import { Button } from "@/components/ui/button";
import { useCurrentUser, useLogout } from "@/features/auth/use-auth";

/** 侧边栏底部用户区：展示当前用户并支持账号设置与退出登录 */
export function UserMenu() {
	const navigate = useNavigate();
	const { data: user } = useCurrentUser();
	const logoutMutation = useLogout();

	if (!user) {
		return null;
	}

	const initial = user.username.slice(0, 1).toUpperCase() || "?";

	return (
		<div className="flex items-center justify-between rounded-lg p-2">
			<div className="flex min-w-0 items-center gap-2.5">
				<span className="flex size-8 shrink-0 items-center justify-center rounded-full bg-foreground font-medium text-background text-xs">
					{initial}
				</span>
				<div className="min-w-0">
					<div className="truncate font-semibold text-xs">{user.username}</div>
					<div className="truncate font-mono text-[11px] text-muted-foreground">
						本地模式
					</div>
				</div>
			</div>
			<div className="flex shrink-0 items-center gap-1.5 border-sidebar-border border-l pl-2">
				<Button
					asChild
					variant="ghost"
					size="icon-sm"
					className="text-muted-foreground"
				>
					<NavLink to="/settings" aria-label="账号设置" title="账号设置">
						<Settings aria-hidden="true" />
					</NavLink>
				</Button>
				<Button
					type="button"
					variant="ghost"
					size="icon-sm"
					className="text-muted-foreground"
					aria-label="退出登录"
					title="退出登录"
					disabled={logoutMutation.isPending}
					onClick={() =>
						logoutMutation.mutate(undefined, {
							onSuccess: () => navigate("/login", { replace: true }),
						})
					}
				>
					{logoutMutation.isPending ? (
						<LoaderCircle className="animate-spin" aria-hidden="true" />
					) : (
						<LogOut aria-hidden="true" />
					)}
				</Button>
			</div>
		</div>
	);
}
