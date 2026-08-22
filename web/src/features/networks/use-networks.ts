import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	type CreateNetworkRequest,
	createNetwork,
	fetchNetworks,
	removeNetwork,
} from "@/api/networks";
import { ENVIRONMENT_OVERVIEW_QUERY_KEY } from "@/features/environments/use-environments";

export function useNetworks(environmentId: string | undefined, q: string) {
	return useQuery({
		queryKey: ["networks", environmentId, { q }],
		queryFn: () => fetchNetworks(environmentId as string, q),
		enabled: Boolean(environmentId),
		staleTime: 5_000,
		retry: 1,
	});
}

function invalidate(
	queryClient: ReturnType<typeof useQueryClient>,
	environmentId: string,
) {
	void queryClient.invalidateQueries({ queryKey: ["networks", environmentId] });
	void queryClient.invalidateQueries({
		queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, environmentId],
	});
}

export function useCreateNetwork() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: {
			environmentId: string;
			request: CreateNetworkRequest;
		}) => createNetwork(values.environmentId, values.request),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}

export function useRemoveNetwork() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: { environmentId: string; networkId: string }) =>
			removeNetwork(values.environmentId, values.networkId),
		onSuccess: (_data, variables) =>
			invalidate(queryClient, variables.environmentId),
	});
}
