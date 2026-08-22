import { Tooltip as RadixTooltip } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

type TooltipVariant = "default" | "destructive";

type TooltipProps = Omit<
	React.ComponentPropsWithoutRef<typeof RadixTooltip.Content>,
	"children"
> & {
	content: React.ReactNode;
	children: React.ReactNode;
	delayDuration?: number;
	variant?: TooltipVariant;
};

function TooltipProvider({
	delayDuration = 300,
	skipDelayDuration = 150,
	...props
}: React.ComponentProps<typeof RadixTooltip.Provider>) {
	return (
		<RadixTooltip.Provider
			delayDuration={delayDuration}
			skipDelayDuration={skipDelayDuration}
			{...props}
		/>
	);
}

function Tooltip({
	content,
	children,
	className,
	delayDuration,
	sideOffset = 6,
	variant = "default",
	...props
}: TooltipProps) {
	const isDestructive = variant === "destructive";

	return (
		<RadixTooltip.Root delayDuration={delayDuration}>
			<RadixTooltip.Trigger asChild>{children}</RadixTooltip.Trigger>
			<RadixTooltip.Portal>
				<RadixTooltip.Content
					className={cn(
						"z-50 max-w-64 select-none rounded-md border px-2.5 py-1.5 text-xs font-medium leading-5 shadow-lg outline-none",
						"animate-in fade-in-0 zoom-in-95 data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=closed]:zoom-out-95",
						"data-[side=bottom]:slide-in-from-top-1 data-[side=left]:slide-in-from-right-1 data-[side=right]:slide-in-from-left-1 data-[side=top]:slide-in-from-bottom-1",
						isDestructive
							? "border-destructive/20 bg-destructive text-destructive-foreground"
							: "border-primary/20 bg-primary text-primary-foreground",
						className,
					)}
					sideOffset={sideOffset}
					{...props}
				>
					{content}
					<RadixTooltip.Arrow
						className={isDestructive ? "fill-destructive" : "fill-primary"}
					/>
				</RadixTooltip.Content>
			</RadixTooltip.Portal>
		</RadixTooltip.Root>
	);
}

export { Tooltip, TooltipProvider };
