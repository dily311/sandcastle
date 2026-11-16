<script lang="ts">
	import type { Snippet } from "svelte";
	import type { HTMLButtonAttributes } from "svelte/elements";

	interface Props extends HTMLButtonAttributes {
		/** 화면 낭독기와 툴팁에 쓰는 이름 (필수) */
		label: string;
		variant?: "ghost" | "soft" | "solid";
		size?: "sm" | "md" | "lg";
		active?: boolean;
		children: Snippet;
	}

	let {
		label,
		variant = "ghost",
		size = "md",
		active = false,
		children,
		type = "button",
		class: className = "",
		...rest
	}: Props = $props();
</script>

<button {type} class="icon-btn {variant} {size} {className}" class:active aria-label={label} title={label} {...rest}>
	{@render children()}
</button>

<style>
	.icon-btn {
		position: relative;
		display: inline-grid;
		place-items: center;
		flex: none;
		width: var(--control-md);
		height: var(--control-md);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
		transition:
			background var(--duration-fast) var(--ease),
			color var(--duration-fast) var(--ease);
	}
	/* 손가락: 보이는 크기는 그대로 두고 누르는 범위만 --touch-target(44px)까지 넓힌다 (1-C-3) */
	@media (pointer: coarse) {
		.icon-btn::after {
			content: "";
			position: absolute;
			left: 50%;
			top: 50%;
			width: max(100%, var(--touch-target));
			height: max(100%, var(--touch-target));
			transform: translate(-50%, -50%);
		}
	}
	.icon-btn :global(svg) {
		width: 18px;
		height: 18px;
	}
	.sm {
		width: var(--control-sm);
		height: var(--control-sm);
		border-radius: var(--radius-xs);
	}
	/* 손가락: 작은 버튼도 누르기 쉽게 */
	@media (pointer: coarse) {
		.sm {
			width: 34px;
			height: 34px;
		}
	}
	.sm :global(svg) {
		width: 15px;
		height: 15px;
	}
	.lg {
		width: var(--control-lg);
		height: var(--control-lg);
		border-radius: var(--radius-md);
	}
	.ghost:hover:not(:disabled),
	.ghost.active {
		background: var(--surface-hover);
		color: var(--text);
	}
	.soft {
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.soft:hover:not(:disabled) {
		background: var(--accent-soft-hover);
	}
	.solid {
		background: var(--accent);
		color: var(--text-on-accent);
	}
	.solid:hover:not(:disabled) {
		background: var(--accent-hover);
	}
	.icon-btn:disabled {
		opacity: 0.45;
		cursor: not-allowed;
	}
</style>
