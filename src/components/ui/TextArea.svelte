<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAttributes } from "svelte/elements";

	type VariantType = "horizontal" | "vertical";
	type SizeType = "default" | "xs" | "sm" | "lg";
	type Props = HTMLAttributes<HTMLTextAreaElement> & {
		ref?: HTMLElement;
		value: string;
		rows: number;
		variant?: VariantType;
		size?: SizeType;
	};
	let {
		ref = $bindable(),
		value = $bindable(),
		variant = "horizontal",
		rows,
		size,
		class: className,
		children,
		...restProps
	}: Props = $props();
</script>

<textarea bind:this={ref} bind:value class={cn("textarea", size, className)} {rows} {...restProps}></textarea>

<style>
	.textarea {
		width: 100%;
		font-size: 0.8rem;
		padding: 10px 12px;
		border-radius: var(--radius-md);
		background: var(--bg-input);
		color: var(--text-color);
		font-size: 13.5px;
		line-height: 1.6;
		outline: none;
		resize: vertical;
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
		min-height: 1.6rem;
		background: var(--background-primary);
	}
	.textarea:focus {
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}
</style>
