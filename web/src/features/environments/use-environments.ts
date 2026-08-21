import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	fetchEnvironments,
	type RenameEnvironmentRequest,
	renameEnvironment,
} from "@/api/environments";

export type { EnvironmentSummary } from "@/api/environments";

export const ENVIRONMENTS_QUERY_KEY = ["environments"] as const;

export function useEnvironments() {
	return useQuery({
		queryKey: ENVIRONMENTS_QUERY_KEY,
		queryFn: fetchEnvironments,
		refetchInterval: 15_000,
		staleTime: 10_000,
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
		onSuccess: () => {
			void queryClient.invalidateQueries({ queryKey: ENVIRONMENTS_QUERY_KEY });
		},
	});
}
