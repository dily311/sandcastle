<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

	type SizeType = "default" | "xs" | "sm" | "lg";
	type Props = (HTMLAnchorAttributes & HTMLButtonAttributes) & {
		size?: SizeType;
	};
	let { type = "button", class: className, size, disabled, children, ...restProps }: Props = $props();
</script>

<button {type} {disabled} class={cn("clickable-icon", size !== "default" && size, className)} {...restProps}>
	{@render children?.()}
</button>

<style>
	.clickable-icon {
		display: flex;
		align-items: center;
		justify-content: center;
		width: 2rem;
		height: 2rem;
		padding: 0.4rem;
		color: var(--icon-color);
		border-radius: var(--radius-sm);
		font-size: var(--font-size-sm);
		cursor: pointer;
		position: relative;
	}

	.clickable-icon:hover,
	.clickable-icon.active {
		background: var(--button-hover-bg);
	}

	.clickable-icon:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.clickable-icon.xs {
		width: 0.8rem;
		height: 0.8rem;
	}
	.clickable-icon.sm {
		width: 1.75rem;
		height: 1.75rem;
	}
	.clickable-icon.lg {
		width: 2.25rem;
		height: 2.25rem;
	}
</style>
