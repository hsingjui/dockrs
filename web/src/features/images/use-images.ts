import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { fetchImages, pullImage, removeImage } from "@/api/images";
import { ENVIRONMENT_OVERVIEW_QUERY_KEY } from "@/features/environments/use-environments";

export interface ImageFilters {
	q: string;
	dangling?: boolean;
}

export function useImages(
	environmentId: string | undefined,
	filters: ImageFilters,
) {
	return useQuery({
		queryKey: ["images", environmentId, filters],
		queryFn: () => fetchImages(environmentId as string, filters),
		enabled: Boolean(environmentId),
		staleTime: 5_000,
		retry: 1,
	});
}

function invalidate(
	queryClient: ReturnType<typeof useQueryClient>,
	environmentId: string,
) {
	void queryClient.invalidateQueries({ queryKey: ["images", environmentId] });
	void queryClient.invalidateQueries({
		queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, environmentId],
	});
}

export function usePullImage() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: { environmentId: string; reference: string }) =>
			pullImage(values.environmentId, values.reference),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}

export function useRemoveImage() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: { environmentId: string; imageId: string }) =>
			removeImage(values.environmentId, values.imageId),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}
