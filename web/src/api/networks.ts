import { apiFetch } from "@/api/client";

export interface NetworkResource {
	id: string;
	name: string;
	driver: string | null;
	scope: string | null;
	subnet: string | null;
	gateway: string | null;
	labels: Record<string, string>;
	containerCount: number | null;
	system: boolean;
}

export interface CreateNetworkRequest {
	name: string;
	driver?: string;
	internal?: boolean;
}

function basePath(environmentId: string) {
	return `/api/environments/${encodeURIComponent(environmentId)}/networks`;
}

export function fetchNetworks(
	environmentId: string,
	q: string,
): Promise<NetworkResource[]> {
	const query = q ? `?q=${encodeURIComponent(q)}` : "";
	return apiFetch<NetworkResource[]>(`${basePath(environmentId)}${query}`);
}

export function createNetwork(
	environmentId: string,
	request: CreateNetworkRequest,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(`${basePath(environmentId)}`, {
		method: "POST",
		body: JSON.stringify(request),
	});
}

export function removeNetwork(
	environmentId: string,
	networkId: string,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(
		`${basePath(environmentId)}/${encodeURIComponent(networkId)}`,
		{
			method: "DELETE",
		},
	);
}
