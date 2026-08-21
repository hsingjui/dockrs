import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	type ChangePasswordRequest,
	changePassword,
	fetchMe,
	type LoginRequest,
	login,
	logout,
	type User,
} from "@/api/auth";

export const USER_QUERY_KEY = ["auth", "me"] as const;

/** 当前登录用户；401 不重试，由路由守卫处理跳转 */
export function useCurrentUser() {
	return useQuery({
		queryKey: USER_QUERY_KEY,
		queryFn: fetchMe,
		retry: false,
	});
}

/** 登录：成功后把用户写入缓存，调用方在 mutate 的 onSuccess 里导航 */
export function useLogin() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (req: LoginRequest) => login(req),
		onSuccess: (user: User) => {
			queryClient.setQueryData(USER_QUERY_KEY, user);
		},
	});
}

/** 修改密码：成功后由设置页清空表单并显示结果 */
export function useChangePassword() {
	return useMutation({
		mutationFn: (req: ChangePasswordRequest) => changePassword(req),
	});
}

/** 登出：成功后清空当前用户缓存，调用方负责跳转登录页 */
export function useLogout() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: () => logout(),
		onSuccess: () => {
			queryClient.setQueryData(USER_QUERY_KEY, null);
		},
	});
}
