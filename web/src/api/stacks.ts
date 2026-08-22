import { apiFetch } from "@/api/client";

export interface StackService {
	id: string;
	name: string;
	desiredReplicas: number;
	runningReplicas: number;
}

export interface StackTask {
	id: string;
	name: string | null;
	state: string | null;
	error: string | null;
	containerId: string | null;
}

export interface StackResource {
	name: string;
	status: "running" | "pending" | "degraded" | string;
	serviceCount: number;
	desiredReplicas: number;
	runningReplicas: number;
	failedTasks: number;
	services: StackService[];
	tasks: StackTask[];
}

export interface StackListResponse {
	available: boolean;
	reason: string | null;
	items: StackResource[];
}

function basePath(environmentId: string) {
	return `/api/environments/${encodeURIComponent(environmentId)}/stacks`;
}

export function fetchStacks(
	environmentId: string,
	q: string,
): Promise<StackListResponse> {
	const query = q ? `?q=${encodeURIComponent(q)}` : "";
	return apiFetch<StackListResponse>(`${basePath(environmentId)}${query}`);
}

export function fetchStack(
	environmentId: string,
	name: string,
): Promise<StackResource> {
	return apiFetch<StackResource>(
		`${basePath(environmentId)}/${encodeURIComponent(name)}`,
	);
}
