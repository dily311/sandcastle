<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAttributes } from "svelte/elements";

	type Props = HTMLAttributes<HTMLDivElement> & {
		toggle?: (
			e: MouseEvent & {
				currentTarget: EventTarget & HTMLDivElement;
			},
			change: boolean,
		) => void;
		checked: boolean;
	};
	let { checked, class: className, toggle, ...restProps }: Props = $props();
</script>

<div
	class={cn("toggle-btn", checked && "on", className)}
	{...restProps}
	onclick={(e) => {
        const change = !checked;
		checked = change;
		toggle?.(e, change);
	}}
></div>

<style>
	.toggle-btn {
		width: 2.4rem;
		height: 1.4rem;
		border-radius: 0.8rem;
		background: var(--gray-5);
		position: relative;
		cursor: pointer;
		transition: all 0.3s cubic-bezier(0.18, 0.89, 0.35, 1.15);
	}
	.toggle-btn::after {
		content: "";
		width: 1rem;
		height: 1rem;
		border-radius: 100%;
		background: #fff;
		position: absolute;
		top: 50%;
		transform: translateY(-50%);
		left: 0.2rem;
		transition: all 0.3s cubic-bezier(0.18, 0.89, 0.35, 1.15);
	}
	.toggle-btn.on {
		background: var(--accent-color);
	}
	.toggle-btn.on::after {
		left: auto;
		right: 0.2rem;
	}
</style>
