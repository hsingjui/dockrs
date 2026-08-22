import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	type CreateVolumeRequest,
	createVolume,
	fetchVolumes,
	removeVolume,
} from "@/api/volumes";
import { ENVIRONMENT_OVERVIEW_QUERY_KEY } from "@/features/environments/use-environments";

export function useVolumes(environmentId: string | undefined, q: string) {
	return useQuery({
		queryKey: ["volumes", environmentId, { q }],
		queryFn: () => fetchVolumes(environmentId as string, q),
		enabled: Boolean(environmentId),
		staleTime: 5_000,
		retry: 1,
	});
}

function invalidate(
	queryClient: ReturnType<typeof useQueryClient>,
	environmentId: string,
) {
	void queryClient.invalidateQueries({ queryKey: ["volumes", environmentId] });
	void queryClient.invalidateQueries({
		queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, environmentId],
	});
}

export function useCreateVolume() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: {
			environmentId: string;
			request: CreateVolumeRequest;
		}) => createVolume(values.environmentId, values.request),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}

export function useRemoveVolume() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: { environmentId: string; name: string }) =>
			removeVolume(values.environmentId, values.name),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}
