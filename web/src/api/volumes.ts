import { apiFetch } from "@/api/client";

export interface VolumeResource {
	name: string;
	driver: string;
	scope: string | null;
	mountpoint: string;
	labels: Record<string, string>;
	usageContainers: number | null;
}

export interface CreateVolumeRequest {
	name: string;
	driver?: string;
	options?: Record<string, string>;
}

function basePath(environmentId: string) {
	return `/api/environments/${encodeURIComponent(environmentId)}/volumes`;
}

export function fetchVolumes(
	environmentId: string,
	q: string,
): Promise<VolumeResource[]> {
	const query = q ? `?q=${encodeURIComponent(q)}` : "";
	return apiFetch<VolumeResource[]>(`${basePath(environmentId)}${query}`);
}

export function createVolume(
	environmentId: string,
	request: CreateVolumeRequest,
): Promise<VolumeResource> {
	return apiFetch<VolumeResource>(basePath(environmentId), {
		method: "POST",
		body: JSON.stringify(request),
	});
}

export function removeVolume(
	environmentId: string,
	name: string,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(
		`${basePath(environmentId)}/${encodeURIComponent(name)}`,
		{
			method: "DELETE",
		},
	);
}
