import { useQuery } from "@tanstack/react-query";
import { fetchStacks } from "@/api/stacks";

export function useStacks(environmentId: string | undefined, q: string) {
	return useQuery({
		queryKey: ["stacks", environmentId, { q }],
		queryFn: () => fetchStacks(environmentId as string, q),
		enabled: Boolean(environmentId),
		staleTime: 5_000,
		retry: 1,
	});
}
