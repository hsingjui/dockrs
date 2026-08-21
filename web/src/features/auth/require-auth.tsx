import { LoaderCircle } from "lucide-react";
import { Navigate, useLocation } from "react-router-dom";
import { AppLayout } from "@/components/layout/app-layout";
import { useCurrentUser } from "@/features/auth/use-auth";

/**
 * 路由守卫：会话校验通过则渲染 AppLayout（含 Outlet），否则跳转登录页。
 * 首次校验期间展示骨架占位，避免未登录态子页面先发起请求再被拦截。
 */
export function RequireAuth() {
	const location = useLocation();
	const { data: user, isPending } = useCurrentUser();

	if (isPending) {
		return (
			<div className="flex h-dvh items-center justify-center">
				<LoaderCircle
					className="size-6 animate-spin text-muted-foreground"
					aria-hidden="true"
				/>
				<span className="sr-only">正在加载</span>
			</div>
		);
	}

	if (!user) {
		const from = `${location.pathname}${location.search}`;
		return <Navigate to="/login" replace state={{ from }} />;
	}

	return <AppLayout />;
}
