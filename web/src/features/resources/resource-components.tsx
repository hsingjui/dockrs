import {
	AlertCircle,
	LoaderCircle,
	RefreshCw,
	Search,
	ServerOff,
} from "lucide-react";
import type { ReactNode } from "react";
import { Button } from "@/components/ui/button";
import { Card } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Tooltip } from "@/components/ui/tooltip";
import { cn } from "@/lib/utils";

export function ResourcePageState(props: {
	message: string;
	action?: ReactNode;
	variant?: "default" | "error" | "offline";
}) {
	const Icon =
		props.variant === "error"
			? AlertCircle
			: props.variant === "offline"
				? ServerOff
				: LoaderCircle;
	return (
		<div className="flex min-h-56 flex-col items-center justify-center gap-3 rounded-xl border border-border border-dashed px-6 text-center">
			<Icon
				className={cn(
					"size-5 text-muted-foreground",
					props.variant === "error" && "text-destructive",
					props.variant === "offline" && "text-amber-600",
					!props.variant && "animate-spin",
				)}
				aria-hidden="true"
			/>
			<p className="text-muted-foreground text-sm">{props.message}</p>
			{props.action}
		</div>
	);
}

export function ResourceHeader(props: {
	title: string;
	count?: number;
	onRefresh: () => void;
	isRefreshing: boolean;
	action?: ReactNode;
}) {
	return (
		<header className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
			<div>
				<div className="flex items-center gap-2">
					<h1 className="font-semibold text-xl">{props.title}</h1>
					{props.count !== undefined && (
						<span className="rounded bg-muted px-1.5 py-0.5 font-mono text-muted-foreground text-xs tabular-nums">
							{props.count}
						</span>
					)}
				</div>
			</div>
			<div className="flex items-center gap-2">
				{props.action}
				<Tooltip content="刷新">
					<Button
						variant="outline"
						size="icon"
						onClick={props.onRefresh}
						disabled={props.isRefreshing}
						aria-label="刷新"
					>
						<RefreshCw className={cn(props.isRefreshing && "animate-spin")} />
					</Button>
				</Tooltip>
			</div>
		</header>
	);
}

export function SearchField(props: {
	value: string;
	onChange: (value: string) => void;
	placeholder: string;
	label: string;
}) {
	return (
		<div className="relative min-w-0 flex-1 sm:max-w-sm">
			<Search
				className="pointer-events-none absolute top-1/2 left-2.5 size-4 -translate-y-1/2 text-muted-foreground"
				aria-hidden="true"
			/>
			<Input
				value={props.value}
				onChange={(event) => props.onChange(event.target.value)}
				placeholder={props.placeholder}
				aria-label={props.label}
				className="pl-8"
			/>
		</div>
	);
}

export function EmptyTableState(props: { message: string; colSpan: number }) {
	return (
		<tr>
			<td
				colSpan={props.colSpan}
				className="h-40 text-center text-muted-foreground text-sm"
			>
				{props.message}
			</td>
		</tr>
	);
}

export function Notice(props: {
	children: ReactNode;
	variant?: "error" | "success" | "warning";
}) {
	return (
		<div
			role={props.variant === "error" ? "alert" : "status"}
			className={cn(
				"rounded-lg px-3 py-2 text-sm",
				props.variant === "success" &&
					"bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
				props.variant === "warning" &&
					"bg-amber-500/10 text-amber-700 dark:text-amber-400",
				(!props.variant || props.variant === "error") &&
					"bg-destructive/10 text-destructive",
			)}
		>
			{props.children}
		</div>
	);
}

export function DataTable(props: { children: ReactNode; minWidth?: string }) {
	return (
		<Card className="overflow-hidden">
			<div className="overflow-x-auto">
				<table
					className={cn(
						"w-full text-left text-sm",
						props.minWidth ?? "min-w-[760px]",
					)}
				>
					{props.children}
				</table>
			</div>
		</Card>
	);
}

export function TableHead(props: { children: ReactNode }) {
	return (
		<thead className="border-b bg-muted/40 text-muted-foreground text-xs">
			{props.children}
		</thead>
	);
}

export function TableBody(props: { children: ReactNode }) {
	return <tbody className="divide-y divide-border/70">{props.children}</tbody>;
}

export function TableRow(props: {
	children: ReactNode;
	className?: string;
	onClick?: () => void;
}) {
	return (
		<tr
			onClick={props.onClick}
			className={cn("transition-colors hover:bg-muted/30", props.className)}
		>
			{props.children}
		</tr>
	);
}

export function TableHeaderCell(props: {
	children?: ReactNode;
	className?: string;
}) {
	return (
		<th
			className={cn("whitespace-nowrap px-4 py-3 font-medium", props.className)}
		>
			{props.children}
		</th>
	);
}

export function TableCell(props: {
	children: ReactNode;
	className?: string;
	colSpan?: number;
}) {
	return (
		<td
			colSpan={props.colSpan}
			className={cn("px-4 py-3 align-middle", props.className)}
		>
			{props.children}
		</td>
	);
}

export function formatBytes(bytes: number) {
	if (bytes < 1024) return `${bytes} B`;
	const units = ["KB", "MB", "GB", "TB"];
	let value = bytes;
	let unit = units[0];
	for (const nextUnit of units) {
		if (value < 1024) break;
		value /= 1024;
		unit = nextUnit;
	}
	return `${value >= 10 ? value.toFixed(0) : value.toFixed(1)} ${unit}`;
}

export function formatTimestamp(timestamp: number | null) {
	if (!timestamp) return "未知";
	return new Intl.DateTimeFormat("zh-CN", {
		dateStyle: "medium",
		timeStyle: "short",
	}).format(new Date(timestamp * 1000));
}
