import { apiFetch } from "@/api/client";

/** 当前登录用户（对应后端 UserResponse） */
export interface User {
	id: number;
	username: string;
}

export interface LoginRequest {
	username: string;
	password: string;
}

export interface ChangePasswordRequest {
	current_password: string;
	new_password: string;
}

export function login(req: LoginRequest): Promise<User> {
	return apiFetch<User>("/api/auth/login", {
		method: "POST",
		body: JSON.stringify(req),
	});
}

export function changePassword(
	req: ChangePasswordRequest,
): Promise<Record<string, never>> {
	return apiFetch("/api/auth/change-password", {
		method: "POST",
		body: JSON.stringify(req),
	});
}

export function logout(): Promise<Record<string, never>> {
	return apiFetch("/api/auth/logout", { method: "POST" });
}

export function fetchMe(): Promise<User> {
	return apiFetch<User>("/api/auth/me");
}
