import { apiFetch } from "@/api/client";

export interface ContainerPort {
	containerPort: number;
	hostPort: number | null;
	protocol: string;
}

export interface ContainerResource {
	id: string;
	name: string;
	names: string[];
	image: string | null;
	state: string;
	status: string | null;
	created: number | null;
	ports: ContainerPort[];
	stack: string | null;
}

export interface ContainerFilters {
	all: boolean;
	status: string;
	q: string;
}

export type ContainerAction =
	| "start"
	| "stop"
	| "restart"
	| "pause"
	| "unpause";

function basePath(environmentId: string) {
	return `/api/environments/${encodeURIComponent(environmentId)}/containers`;
}

export function fetchContainers(
	environmentId: string,
	filters: ContainerFilters,
): Promise<ContainerResource[]> {
	const params = new URLSearchParams({ all: String(filters.all) });
	if (filters.status) params.set("status", filters.status);
	if (filters.q) params.set("q", filters.q);
	return apiFetch<ContainerResource[]>(`${basePath(environmentId)}?${params}`);
}

export function containerAction(
	environmentId: string,
	containerId: string,
	action: ContainerAction,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(
		`${basePath(environmentId)}/${encodeURIComponent(containerId)}/${action}`,
		{
			method: "POST",
		},
	);
}

export function removeContainer(
	environmentId: string,
	containerId: string,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(
		`${basePath(environmentId)}/${encodeURIComponent(containerId)}`,
		{
			method: "DELETE",
		},
	);
}
