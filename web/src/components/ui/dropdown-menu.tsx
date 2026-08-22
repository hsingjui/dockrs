import { DropdownMenu as DropdownMenuPrimitive } from "radix-ui";
import type * as React from "react";

import { cn } from "@/lib/utils";

export const DropdownMenu = DropdownMenuPrimitive.Root;
export const DropdownMenuTrigger = DropdownMenuPrimitive.Trigger;
export const DropdownMenuGroup = DropdownMenuPrimitive.Group;
export const DropdownMenuSeparator = DropdownMenuPrimitive.Separator;

export const DropdownMenuContent = ({
	className,
	sideOffset = 6,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Content>) => (
	<DropdownMenuPrimitive.Portal>
		<DropdownMenuPrimitive.Content
			data-slot="dropdown-menu-content"
			sideOffset={sideOffset}
			className={cn(
				"z-50 min-w-[8rem] overflow-hidden rounded-lg border border-border/80 bg-popover p-1 text-popover-foreground shadow-md",
				"origin-[var(--radix-dropdown-menu-content-transform-origin)] data-[state=open]:animate-in data-[state=closed]:animate-out data-[state=closed]:fade-out-0 data-[state=open]:fade-in-0 data-[state=closed]:zoom-out-95 data-[state=open]:zoom-in-95",
				className,
			)}
			{...props}
		/>
	</DropdownMenuPrimitive.Portal>
);

export const DropdownMenuItem = ({
	className,
	variant = "default",
	inset = false,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Item> & {
	inset?: boolean;
	variant?: "default" | "destructive";
}) => (
	<DropdownMenuPrimitive.Item
		data-slot="dropdown-menu-item"
		data-variant={variant}
		className={cn(
			"relative flex cursor-default items-center gap-2 rounded-md px-2 py-1.5 text-sm outline-none select-none transition-colors",
			"focus:bg-muted focus:text-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50",
			inset && "pl-8",
			variant === "destructive" &&
				"text-destructive focus:bg-destructive/10 focus:text-destructive",
			"data-[variant=destructive]:text-destructive data-[variant=destructive]:focus:bg-destructive/10 data-[variant=destructive]:focus:text-destructive",
			"[&_svg]:pointer-events-none [&_svg]:text-muted-foreground [&_svg]:shrink-0 [&_svg:not([class*='size-'])]:size-4",
			variant === "destructive" && "[&_svg]:text-destructive/80",
			className,
		)}
		{...props}
	/>
);

export const DropdownMenuLabel = ({
	className,
	inset = false,
	...props
}: React.ComponentProps<typeof DropdownMenuPrimitive.Label> & {
	inset?: boolean;
}) => (
	<DropdownMenuPrimitive.Label
		data-slot="dropdown-menu-label"
		className={cn(
			"px-2 py-1.5 text-xs font-medium text-muted-foreground",
			inset && "pl-8",
			className,
		)}
		{...props}
	/>
);
