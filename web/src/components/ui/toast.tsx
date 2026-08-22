import { CircleAlert, CircleCheck, Info, X } from "lucide-react";
import { create } from "zustand";
import { cn } from "@/lib/utils";

type ToastVariant = "success" | "error" | "info";

type ToastItem = {
	id: number;
	message: string;
	variant: ToastVariant;
	leaving?: boolean;
};

const useToastStore = create<{
	toasts: ToastItem[];
	dismiss: (id: number) => void;
}>((set) => ({
	toasts: [],
	dismiss: (id) => {
		set((state) => ({
			toasts: state.toasts.map((t) =>
				t.id === id ? { ...t, leaving: true } : t,
			),
		}));
		setTimeout(
			() =>
				set((state) => ({ toasts: state.toasts.filter((t) => t.id !== id) })),
			150,
		);
	},
}));

let nextId = 0;

function push(variant: ToastVariant, message: string) {
	const id = ++nextId;
	useToastStore.setState((state) => ({
		toasts: [...state.toasts.slice(-4), { id, message, variant }],
	}));
	setTimeout(() => useToastStore.getState().dismiss(id), 4_000);
}

export const toast = {
	success: (message: string) => push("success", message),
	error: (message: string) => push("error", message),
	info: (message: string) => push("info", message),
};

const variantStyles: Record<ToastVariant, string> = {
	success: "bg-emerald-500/10 text-emerald-700 dark:text-emerald-400",
	error: "bg-destructive/10 text-destructive",
	info: "bg-primary/10 text-primary",
};

const variantIcons: Record<ToastVariant, typeof Info> = {
	success: CircleCheck,
	error: CircleAlert,
	info: Info,
};

export function Toaster() {
	const toasts = useToastStore((state) => state.toasts);
	const dismiss = useToastStore((state) => state.dismiss);
	return (
		<div
			aria-live="polite"
			className="pointer-events-none fixed top-4 left-1/2 z-50 flex w-full max-w-md -translate-x-1/2 flex-col items-center gap-2 px-4"
		>
			{toasts.map((t, i) => {
				const Icon = variantIcons[t.variant];
				const isTop = i === toasts.length - 1;
				return (
					<div
						key={t.id}
						role="status"
						className={cn(
							"pointer-events-auto flex w-fit max-w-full items-center gap-2 rounded-lg px-3 py-2 text-sm shadow-sm backdrop-blur duration-200 transition-[transform,opacity]",
							variantStyles[t.variant],
							i > 0 && "-mt-6",
							!isTop && "scale-95 opacity-90",
							t.leaving
								? "animate-out fade-out slide-out-to-top-2 zoom-out-95 fill-mode-forwards"
								: "animate-in fade-in slide-in-from-top-2 zoom-in-95",
						)}
					>
						<Icon className="size-4 shrink-0" aria-hidden="true" />
						<span className="truncate">{t.message}</span>
						<button
							type="button"
							onClick={() => dismiss(t.id)}
							aria-label="关闭"
							className="shrink-0 rounded-full p-0.5 opacity-60 transition-opacity hover:opacity-100"
						>
							<X className="size-3.5" />
						</button>
					</div>
				);
			})}
		</div>
	);
}
