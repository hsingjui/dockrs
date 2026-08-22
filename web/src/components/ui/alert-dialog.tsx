import { AlertDialog as AlertDialogPrimitive } from "radix-ui";
import type * as React from "react";

import { buttonVariants } from "@/components/ui/button";
import { cn } from "@/lib/utils";

export const AlertDialog = AlertDialogPrimitive.Root;
export const AlertDialogTrigger = AlertDialogPrimitive.Trigger;

export const AlertDialogContent = ({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Content>) => (
	<AlertDialogPrimitive.Portal>
		<AlertDialogPrimitive.Overlay className="fixed inset-0 z-50 bg-background/80 backdrop-blur-sm data-[state=open]:animate-in data-[state=open]:fade-in-0" />
		<AlertDialogPrimitive.Content
			data-slot="alert-dialog-content"
			className={cn(
				"fixed top-1/2 left-1/2 z-50 w-[calc(100%-2rem)] max-w-md -translate-x-1/2 -translate-y-1/2 gap-4 rounded-xl border border-border/80 bg-popover p-5 text-popover-foreground shadow-lg",
				"data-[state=open]:animate-in data-[state=open]:fade-in-0 data-[state=open]:zoom-in-95",
				className,
			)}
			{...props}
		/>
	</AlertDialogPrimitive.Portal>
);

export const AlertDialogHeader = ({
	className,
	...props
}: React.ComponentProps<"div">) => (
	<div className={cn("flex flex-col gap-1.5", className)} {...props} />
);

export const AlertDialogTitle = ({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Title>) => (
	<AlertDialogPrimitive.Title
		data-slot="alert-dialog-title"
		className={cn("font-semibold text-base", className)}
		{...props}
	/>
);

export const AlertDialogDescription = ({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Description>) => (
	<AlertDialogPrimitive.Description
		data-slot="alert-dialog-description"
		className={cn("text-sm text-muted-foreground", className)}
		{...props}
	/>
);

export const AlertDialogFooter = ({
	className,
	...props
}: React.ComponentProps<"div">) => (
	<div
		className={cn("flex flex-row justify-end gap-2", className)}
		{...props}
	/>
);

export const AlertDialogAction = ({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Action>) => (
	<AlertDialogPrimitive.Action
		className={cn(buttonVariants(), className)}
		{...props}
	/>
);

export const AlertDialogCancel = ({
	className,
	...props
}: React.ComponentProps<typeof AlertDialogPrimitive.Cancel>) => (
	<AlertDialogPrimitive.Cancel
		className={cn(buttonVariants({ variant: "outline" }), className)}
		{...props}
	/>
);
