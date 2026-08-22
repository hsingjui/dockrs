import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	deleteEnvironment,
	fetchEnvironment,
	fetchEnvironmentOverview,
	fetchEnvironments,
	type RenameEnvironmentRequest,
	renameEnvironment,
} from "@/api/environments";

export type { EnvironmentSummary } from "@/api/environments";

export const ENVIRONMENTS_QUERY_KEY = ["environments"] as const;
export const ENVIRONMENT_QUERY_KEY = ["environment"] as const;
export const ENVIRONMENT_OVERVIEW_QUERY_KEY = ["environment-overview"] as const;

export function useEnvironments() {
	return useQuery({
		queryKey: ENVIRONMENTS_QUERY_KEY,
		queryFn: fetchEnvironments,
		refetchInterval: 15_000,
		staleTime: 10_000,
		retry: 1,
	});
}

export function useEnvironment(id: string | undefined) {
	return useQuery({
		queryKey: [...ENVIRONMENT_QUERY_KEY, id],
		queryFn: () => fetchEnvironment(id as string),
		enabled: Boolean(id),
		staleTime: 10_000,
		retry: 1,
	});
}

export function useEnvironmentOverview(id: string | undefined) {
	return useQuery({
		queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, id],
		queryFn: () => fetchEnvironmentOverview(id as string),
		enabled: Boolean(id),
		staleTime: 10_000,
		refetchInterval: 15_000,
		retry: 1,
	});
}

export function useRenameEnvironment() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: ({
			id,
			request,
		}: {
			id: string;
			request: RenameEnvironmentRequest;
		}) => renameEnvironment(id, request),
		onSuccess: (_updated, variables) => {
			void queryClient.invalidateQueries({ queryKey: ENVIRONMENTS_QUERY_KEY });
			void queryClient.invalidateQueries({
				queryKey: [...ENVIRONMENT_QUERY_KEY, variables.id],
			});
			void queryClient.invalidateQueries({
				queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, variables.id],
			});
		},
	});
}

export function useDeleteEnvironment() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (id: string) => deleteEnvironment(id),
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ENVIRONMENTS_QUERY_KEY });
		},
	});
}
