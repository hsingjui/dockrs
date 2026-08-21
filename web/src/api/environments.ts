import { apiFetch } from "@/api/client";

export interface EnvironmentSummary {
	id: string;
	name: string;
	kind: "local" | "agent";
	status: "online" | "offline";
	/** Docker Socket 路径或 Agent 地址 */
	endpoint: string;
	dockerVersion: string | null;
	containers: {
		total: number;
		running: number;
		paused: number;
		stopped: number;
	};
	cpuPercent: number | null;
	memory: {
		usedBytes: number;
		totalBytes: number;
		percent: number;
	} | null;
	metricsCollectedAt: number | null;
	metricsError: string | null;
	offlineReason: string | null;
	lastSeenAt: string | null;
}

export interface RenameEnvironmentRequest {
	name: string;
}

export interface EnvironmentNameResponse {
	id: string;
	name: string;
}

export function fetchEnvironments(): Promise<EnvironmentSummary[]> {
	return apiFetch<EnvironmentSummary[]>("/api/environments");
}

export function renameEnvironment(
	id: string,
	request: RenameEnvironmentRequest,
): Promise<EnvironmentNameResponse> {
	return apiFetch<EnvironmentNameResponse>(`/api/environments/${id}`, {
		method: "PATCH",
		body: JSON.stringify(request),
	});
}
