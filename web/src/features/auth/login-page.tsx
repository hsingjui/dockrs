import { zodResolver } from "@hookform/resolvers/zod";
import { Container, Eye, EyeOff, LoaderCircle } from "lucide-react";
import { useState } from "react";
import { useForm } from "react-hook-form";
import { Navigate, useLocation, useNavigate } from "react-router-dom";
import { z } from "zod";

import { Button } from "@/components/ui/button";
import {
	Card,
	CardContent,
	CardDescription,
	CardHeader,
	CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { useCurrentUser, useLogin } from "@/features/auth/use-auth";

const loginSchema = z.object({
	username: z.string().min(1, "请输入账号"),
	password: z.string().min(1, "请输入密码"),
});

type LoginFormValues = z.infer<typeof loginSchema>;

export function LoginPage() {
	const navigate = useNavigate();
	const location = useLocation();
	const { data: currentUser, isPending } = useCurrentUser();
	const [showPassword, setShowPassword] = useState(false);

	const {
		register,
		handleSubmit,
		formState: { errors },
	} = useForm<LoginFormValues>({
		resolver: zodResolver(loginSchema),
		defaultValues: { username: "", password: "" },
	});

	const mutation = useLogin();

	// 已登录用户访问登录页 → 回首页
	if (!isPending && currentUser) {
		return <Navigate to="/" replace />;
	}
	// 首次会话校验中，避免已登录态闪过表单
	if (isPending) {
		return null;
	}

	const onSubmit = (values: LoginFormValues) => {
		mutation.mutate(values, {
			onSuccess: () => {
				const from = (location.state as { from?: string } | null)?.from;
				navigate(from ?? "/", { replace: true });
			},
		});
	};

	return (
		<div className="relative flex min-h-dvh items-center justify-center overflow-hidden bg-background p-4">
			{/* 背景装饰：主色径向渐变 + 细网格纹理 */}
			<div
				aria-hidden="true"
				className="pointer-events-none absolute inset-0 bg-[radial-gradient(ellipse_60%_50%_at_50%_0%,var(--color-primary)_0%,transparent_70%)] opacity-[0.07] dark:opacity-[0.12]"
			/>
			<div
				aria-hidden="true"
				className="pointer-events-none absolute inset-0 [background-image:linear-gradient(to_right,var(--color-foreground)_1px,transparent_1px),linear-gradient(to_bottom,var(--color-foreground)_1px,transparent_1px)] [background-size:40px_40px] opacity-[0.03] [mask-image:radial-gradient(ellipse_70%_60%_at_50%_40%,black,transparent)]"
			/>
			<Card className="relative w-full max-w-sm shadow-lg [--card-spacing:--spacing(6)]">
				<CardHeader className="justify-items-center text-center">
					<div className="mb-2 flex size-11 items-center justify-center rounded-xl bg-primary/10 text-primary">
						<Container className="size-6" aria-hidden="true" />
					</div>
					<CardTitle className="text-xl">登录 Dockrs</CardTitle>
					<CardDescription>使用账号密码管理你的 Docker 环境</CardDescription>
				</CardHeader>
				<CardContent>
					<form
						className="flex flex-col gap-4"
						onSubmit={handleSubmit(onSubmit)}
						noValidate
					>
						{mutation.isError && (
							<p
								role="alert"
								className="rounded-lg bg-destructive/10 px-3 py-2 text-sm text-destructive"
							>
								{mutation.error?.message ?? "登录失败，请稍后重试"}
							</p>
						)}

						<div className="flex flex-col gap-2">
							<Label htmlFor="username">账号</Label>
							<Input
								id="username"
								autoComplete="username"
								autoFocus
								aria-invalid={!!errors.username}
								aria-describedby={
									errors.username ? "username-error" : undefined
								}
								{...register("username")}
							/>
							{errors.username && (
								<p id="username-error" className="text-sm text-destructive">
									{errors.username.message}
								</p>
							)}
						</div>

						<div className="flex flex-col gap-2">
							<Label htmlFor="password">密码</Label>
							<div className="relative">
								<Input
									id="password"
									type={showPassword ? "text" : "password"}
									autoComplete="current-password"
									className="pr-10"
									aria-invalid={!!errors.password}
									aria-describedby={
										errors.password ? "password-error" : undefined
									}
									{...register("password")}
								/>
								<Button
									type="button"
									variant="ghost"
									size="icon-sm"
									className="absolute top-1/2 right-1.5 -translate-y-1/2 text-muted-foreground"
									onClick={() => setShowPassword((v) => !v)}
									aria-label={showPassword ? "隐藏密码" : "显示密码"}
									aria-pressed={showPassword}
								>
									{showPassword ? (
										<EyeOff aria-hidden="true" />
									) : (
										<Eye aria-hidden="true" />
									)}
								</Button>
							</div>
							{errors.password && (
								<p id="password-error" className="text-sm text-destructive">
									{errors.password.message}
								</p>
							)}
						</div>

						<Button
							type="submit"
							size="lg"
							className="mt-1 h-11 w-full"
							disabled={mutation.isPending}
						>
							{mutation.isPending && (
								<LoaderCircle className="animate-spin" aria-hidden="true" />
							)}
							{mutation.isPending ? "登录中…" : "登 录"}
						</Button>
					</form>
				</CardContent>
			</Card>
			<p className="absolute bottom-4 text-xs text-muted-foreground">
				Dockrs · 简洁的 Docker 管理面板
			</p>
		</div>
	);
}
