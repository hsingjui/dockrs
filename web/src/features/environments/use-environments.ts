import { useQuery } from "@tanstack/react-query";

export interface EnvironmentSummary {
	id: string;
	name: string;
	kind: "local" | "agent";
	status: "online" | "offline";
	/** socket 路径或 agent 地址 */
	endpoint: string;
	/** 离线环境为 null（未知） */
	dockerVersion: string | null;
	tags: string[];
	containers: {
		running: number;
		stopped: number;
	};
	/** stats 尚未接入，暂为占位数据；离线为 null */
	cpuPercent: number | null;
	memory: { usedGb: number; totalGb: number } | null;
	offlineReason: string | null;
}

// TODO: 后端 /api/environments 就绪后替换为真实请求
const placeholderEnvironments: EnvironmentSummary[] = [
	{
		id: "local",
		name: "local-engine",
		kind: "local",
		status: "online",
		endpoint: "/var/run/docker.sock",
		dockerVersion: "27.3.1",
		tags: ["Local Socket", "主力开发机", "Linux x86_64"],
		containers: { running: 12, stopped: 2 },
		cpuPercent: 14.2,
		memory: { usedGb: 4.8, totalGb: 16.0 },
		offlineReason: null,
	},
	{
		id: "nas",
		name: "rpi5-homelab",
		kind: "agent",
		status: "online",
		endpoint: "192.168.31.200 (Tailscale)",
		dockerVersion: "26.1.3",
		tags: ["ARM64", "Homelab 内网", "NAS 辅助"],
		containers: { running: 4, stopped: 2 },
		cpuPercent: 6.1,
		memory: { usedGb: 2.1, totalGb: 8.0 },
		offlineReason: null,
	},
	{
		id: "vps",
		name: "hk-prod-vps01",
		kind: "agent",
		status: "online",
		endpoint: "43.154.210.88 · 延迟 24ms",
		dockerVersion: "27.1.1",
		tags: ["Rust Agent", "生产环境", "Ubuntu 24.04"],
		containers: { running: 8, stopped: 0 },
		cpuPercent: 42.8,
		memory: { usedGb: 1.7, totalGb: 2.0 },
		offlineReason: null,
	},
	{
		id: "staging",
		name: "aliyun-staging",
		kind: "agent",
		status: "offline",
		endpoint: "120.79.14.33",
		dockerVersion: null,
		tags: ["测试环境", "连接断开 3 小时前"],
		containers: { running: 0, stopped: 0 },
		cpuPercent: null,
		memory: null,
		offlineReason:
			"无法与该节点的 Agent 建立通信，请检查服务器网络或重启 dockrs-agent 服务",
	},
];

async function listEnvironments(): Promise<EnvironmentSummary[]> {
	return placeholderEnvironments;
}

export function useEnvironments() {
	return useQuery({
		queryKey: ["environments"],
		queryFn: listEnvironments,
	});
}
