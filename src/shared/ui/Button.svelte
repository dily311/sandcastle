<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLButtonAttributes } from "svelte/elements";
	import Spinner from "./Spinner.svelte";

	type Variant = "primary" | "secondary" | "soft" | "ghost" | "danger";
	type Size = "sm" | "md" | "lg";

	interface Props extends HTMLButtonAttributes {
		variant?: Variant;
		size?: Size;
		loading?: boolean;
		block?: boolean;
		active?: boolean;
		/** 글자 앞 아이콘 */
		icon?: Snippet;
		children?: Snippet;
	}

	let {
		variant = "secondary",
		size = "md",
		loading = false,
		block = false,
		active = false,
		icon,
		children,
		disabled,
		type = "button",
		class: className = "",
		...rest
	}: Props = $props();
</script>

<button
	{type}
	class="btn {variant} {size} {className}"
	class:block
	class:active
	disabled={disabled || loading}
	aria-busy={loading || undefined}
	{...rest}
>
	{#if loading}
		<Spinner size={size === "lg" ? 18 : 14} />
	{:else if icon}
		<span class="icon">{@render icon()}</span>
	{/if}
	{#if children}<span class="label">{@render children()}</span>{/if}
</button>

<style>
	.btn {
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-2);
		height: var(--control-md);
		padding: 0 var(--space-4);
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		font-weight: 600;
		font-size: var(--text-md);
		line-height: 1;
		white-space: nowrap;
		cursor: pointer;
		user-select: none;
		transition:
			background var(--duration-fast) var(--ease),
			border-color var(--duration-fast) var(--ease),
			color var(--duration-fast) var(--ease),
			transform var(--duration-fast) var(--ease);
	}
	.btn:active:not(:disabled) {
		transform: translateY(1px);
	}
	.btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.block {
		display: flex;
		width: 100%;
	}

	/* 손가락: 보이는 크기는 그대로 두고 누르는 범위만 --touch-target(44px)까지 넓힌다 (1-C-3) */
	@media (pointer: coarse) {
		.btn::after {
			content: "";
			position: absolute;
			left: 50%;
			top: 50%;
			width: max(100%, var(--touch-target));
			height: max(100%, var(--touch-target));
			transform: translate(-50%, -50%);
		}
	}
	.sm {
		height: var(--control-sm);
		padding: 0 var(--space-3);
		font-size: var(--text-sm);
		border-radius: var(--radius-xs);
	}
	.lg {
		height: var(--control-lg);
		padding: 0 var(--space-5);
		font-size: var(--text-lg);
		border-radius: var(--radius-md);
	}

	.primary {
		background: var(--accent);
		color: var(--text-on-accent);
	}
	.primary:hover:not(:disabled),
	.primary.active {
		background: var(--accent-hover);
	}
	.secondary {
		background: var(--surface);
		border-color: var(--border-strong);
		color: var(--text);
		box-shadow: var(--shadow-sm);
	}
	.secondary:hover:not(:disabled),
	.secondary.active {
		background: var(--surface-hover);
	}
	.soft {
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.soft:hover:not(:disabled),
	.soft.active {
		background: var(--accent-soft-hover);
	}
	.ghost {
		background: transparent;
		color: var(--text-muted);
	}
	.ghost:hover:not(:disabled),
	.ghost.active {
		background: var(--surface-hover);
		color: var(--text);
	}
	.danger {
		background: var(--danger-soft);
		color: var(--danger);
	}
	.danger:hover:not(:disabled),
	.danger.active {
		background: var(--danger);
		color: white;
	}

	.icon {
		display: inline-flex;
		margin-left: calc(var(--space-1) * -1);
	}
	.icon :global(svg) {
		width: 16px;
		height: 16px;
	}
</style>
