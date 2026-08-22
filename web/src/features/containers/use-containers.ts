import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
	type ContainerAction,
	type ContainerFilters,
	containerAction,
	fetchContainers,
	removeContainer,
} from "@/api/containers";
import { toast } from "@/components/ui/toast";
import { ENVIRONMENT_OVERVIEW_QUERY_KEY } from "@/features/environments/use-environments";

export function useContainers(
	environmentId: string | undefined,
	filters: ContainerFilters,
) {
	return useQuery({
		queryKey: ["containers", environmentId, filters],
		queryFn: () => fetchContainers(environmentId as string, filters),
		enabled: Boolean(environmentId),
		staleTime: 5_000,
		retry: 1,
	});
}

function invalidate(
	queryClient: ReturnType<typeof useQueryClient>,
	environmentId: string,
) {
	void queryClient.invalidateQueries({
		queryKey: ["containers", environmentId],
	});
	void queryClient.invalidateQueries({
		queryKey: [...ENVIRONMENT_OVERVIEW_QUERY_KEY, environmentId],
	});
}

export function useContainerAction() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: {
			environmentId: string;
			containerId: string;
			action: ContainerAction;
		}) =>
			containerAction(values.environmentId, values.containerId, values.action),
		onSuccess: (_data, variables) => {
			toast.success("容器操作已执行");
			invalidate(queryClient, variables.environmentId);
		},
		onError: (error) => toast.error(`操作失败：${error.message}`),
	});
}

export function useRemoveContainer() {
	const queryClient = useQueryClient();
	return useMutation({
		mutationFn: (values: { environmentId: string; containerId: string }) =>
			removeContainer(values.environmentId, values.containerId),
		onSuccess: (_data, variables) => {
			toast.success("容器已删除");
			invalidate(queryClient, variables.environmentId);
		},
		onError: (error) => toast.error(`删除失败：${error.message}`),
	});
}
