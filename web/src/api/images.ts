import { apiFetch } from "@/api/client";

export interface ImageResource {
	id: string;
	repository: string | null;
	tags: string[];
	digests: string[];
	created: number;
	size: number;
	containers: number;
	dangling: boolean;
}

function basePath(environmentId: string) {
	return `/api/environments/${encodeURIComponent(environmentId)}/images`;
}

export function fetchImages(
	environmentId: string,
	filters: { q: string; dangling?: boolean },
): Promise<ImageResource[]> {
	const params = new URLSearchParams();
	if (filters.q) params.set("q", filters.q);
	if (filters.dangling) params.set("dangling", "true");
	const query = params.toString();
	return apiFetch<ImageResource[]>(
		`${basePath(environmentId)}${query ? `?${query}` : ""}`,
	);
}

export function pullImage(
	environmentId: string,
	reference: string,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(`${basePath(environmentId)}/pull`, {
		method: "POST",
		body: JSON.stringify({ reference }),
	});
}

export function removeImage(
	environmentId: string,
	imageId: string,
): Promise<{ id: string | null }> {
	return apiFetch<{ id: string | null }>(
		`${basePath(environmentId)}/${encodeURIComponent(imageId)}`,
		{
			method: "DELETE",
		},
	);
}
