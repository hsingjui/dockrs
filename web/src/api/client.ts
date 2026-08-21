/** 后端统一信封响应，见 crates/server/src/error.rs 的 ApiResponse */
interface ApiEnvelope<T> {
	code: number;
	data: T;
	msg: string;
}

/** API 调用错误：message 取自信封 msg，status 保留原始 HTTP 状态码 */
export class ApiError extends Error {
	readonly status: number;

	constructor(status: number, message: string) {
		super(message);
		this.name = "ApiError";
		this.status = status;
	}
}

/**
 * 统一 API 请求入口：
 * - 同源携带 cookie（session）
 * - 解析 `{ code, data, msg }` 信封，失败抛 ApiError
 */
export async function apiFetch<T>(
	input: string,
	init?: RequestInit,
): Promise<T> {
	const res = await fetch(input, {
		...init,
		credentials: "include",
		headers: { "Content-Type": "application/json", ...init?.headers },
	});

	// 错误响应同样是信封，优先展示后端 msg
	const body = (await res.json().catch(() => null)) as ApiEnvelope<T> | null;

	if (!res.ok) {
		throw new ApiError(res.status, body?.msg ?? "请求失败，请稍后重试");
	}
	if (!body) {
		throw new ApiError(res.status, "响应格式错误");
	}
	return body.data;
}
