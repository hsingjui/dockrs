import { zodResolver } from "@hookform/resolvers/zod";
import { KeyRound, LoaderCircle } from "lucide-react";
import { useForm } from "react-hook-form";
import { useNavigate } from "react-router-dom";
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
import { useChangePassword, useLogout } from "@/features/auth/use-auth";

const changePasswordSchema = z
	.object({
		currentPassword: z.string().min(1, "请输入当前密码"),
		newPassword: z.string().min(1, "请输入新密码"),
		confirmPassword: z.string().min(1, "请再次输入新密码"),
	})
	.refine((values) => values.newPassword === values.confirmPassword, {
		path: ["confirmPassword"],
		message: "两次输入的新密码不一致",
	});

type ChangePasswordFormValues = z.infer<typeof changePasswordSchema>;

export function SettingsPage() {
	const navigate = useNavigate();
	const mutation = useChangePassword();
	const logoutMutation = useLogout();
	const isSubmitting = mutation.isPending || logoutMutation.isPending;
	const {
		register,
		handleSubmit,
		reset,
		formState: { errors },
	} = useForm<ChangePasswordFormValues>({
		resolver: zodResolver(changePasswordSchema),
		defaultValues: {
			currentPassword: "",
			newPassword: "",
			confirmPassword: "",
		},
	});

	const onSubmit = (values: ChangePasswordFormValues) => {
		logoutMutation.reset();
		mutation.mutate(
			{
				current_password: values.currentPassword,
				new_password: values.newPassword,
			},
			{
				onSuccess: () => {
					reset();
					logoutMutation.mutate(undefined, {
						onSuccess: () => navigate("/login", { replace: true }),
					});
				},
			},
		);
	};

	return (
		<div className="mx-auto w-full max-w-5xl space-y-6 px-6 py-8 lg:px-8">
			<header>
				<h1 className="font-semibold text-xl">设置</h1>
				<p className="mt-1 text-muted-foreground text-sm">管理账号与安全设置</p>
			</header>

			<Card className="max-w-2xl">
				<CardHeader className="border-b">
					<div className="flex items-start gap-3">
						<div className="flex size-9 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary">
							<KeyRound className="size-4" aria-hidden="true" />
						</div>
						<div>
							<CardTitle>修改密码</CardTitle>
							<CardDescription className="mt-1">
								使用当前密码验证身份并设置新的登录密码
							</CardDescription>
						</div>
					</div>
				</CardHeader>
				<CardContent className="pt-4">
					<form
						className="flex max-w-md flex-col gap-4"
						onSubmit={handleSubmit(onSubmit)}
						noValidate
					>
						{mutation.isError && (
							<p
								role="alert"
								className="rounded-lg bg-destructive/10 px-3 py-2 text-sm text-destructive"
							>
								{mutation.error?.message ?? "修改密码失败，请稍后重试"}
							</p>
						)}

						{logoutMutation.isError && (
							<p
								role="alert"
								className="rounded-lg bg-amber-500/10 px-3 py-2 text-amber-700 text-sm dark:text-amber-400"
							>
								密码已修改，但自动退出登录失败，请点击侧栏的退出按钮
							</p>
						)}

						{mutation.isSuccess && !logoutMutation.isError && (
							<p
								role="status"
								className="rounded-lg bg-emerald-500/10 px-3 py-2 text-emerald-700 text-sm dark:text-emerald-400"
							>
								{logoutMutation.isPending
									? "密码已修改，正在退出登录…"
									: "密码已修改"}
							</p>
						)}

						<div className="flex flex-col gap-2">
							<Label htmlFor="current-password">当前密码</Label>
							<Input
								id="current-password"
								type="password"
								autoComplete="current-password"
								aria-invalid={!!errors.currentPassword}
								aria-describedby={
									errors.currentPassword ? "current-password-error" : undefined
								}
								{...register("currentPassword")}
							/>
							{errors.currentPassword && (
								<p
									id="current-password-error"
									className="text-destructive text-sm"
								>
									{errors.currentPassword.message}
								</p>
							)}
						</div>

						<div className="flex flex-col gap-2">
							<Label htmlFor="new-password">新密码</Label>
							<Input
								id="new-password"
								type="password"
								autoComplete="new-password"
								aria-invalid={!!errors.newPassword}
								aria-describedby={
									errors.newPassword ? "new-password-error" : undefined
								}
								{...register("newPassword")}
							/>
							{errors.newPassword && (
								<p id="new-password-error" className="text-destructive text-sm">
									{errors.newPassword.message}
								</p>
							)}
						</div>

						<div className="flex flex-col gap-2">
							<Label htmlFor="confirm-new-password">确认新密码</Label>
							<Input
								id="confirm-new-password"
								type="password"
								autoComplete="new-password"
								aria-invalid={!!errors.confirmPassword}
								aria-describedby={
									errors.confirmPassword
										? "confirm-new-password-error"
										: undefined
								}
								{...register("confirmPassword")}
							/>
							{errors.confirmPassword && (
								<p
									id="confirm-new-password-error"
									className="text-destructive text-sm"
								>
									{errors.confirmPassword.message}
								</p>
							)}
						</div>

						<Button
							type="submit"
							className="mt-1 w-fit"
							disabled={isSubmitting}
						>
							{isSubmitting && (
								<LoaderCircle className="animate-spin" aria-hidden="true" />
							)}
							{logoutMutation.isPending
								? "退出中…"
								: mutation.isPending
									? "保存中…"
									: "保存密码"}
						</Button>
					</form>
				</CardContent>
			</Card>
		</div>
	);
}
