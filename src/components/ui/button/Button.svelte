<script lang="ts">
	import { cn } from "$/lib/utils/cn";
	import type { HTMLAnchorAttributes, HTMLButtonAttributes } from "svelte/elements";

	type VariantType =
		| "default"
		| "primary"
		| "secondary"
		| "outline"
		| "ghost"
		| "link"
		| "success"
		| "warning"
		| "danger"
		| "info";
	type SizeType = "default" | "xs" | "sm" | "lg" | "icon" | "icon-xs" | "icon-sm" | "icon-lg";
	type Props = (HTMLAnchorAttributes & HTMLButtonAttributes) & {
		href?: string;
		variant?: VariantType;
		size?: SizeType;
	};
	let {
		variant = "default",
		type = "button",
		href,
		class: className,
		size,
		disabled,
		children,
		...restProps
	}: Props = $props();
</script>

{#if href}
	<a {href} class={cn("btn", variant, size !== "default" && size, className)} {...restProps}>
		{@render children?.()}
	</a>
{:else}
	<button {type} {disabled} class={cn("btn", variant, size !== "default" && size, className)} {...restProps}>
		{@render children?.()}
	</button>
{/if}

<style>
	/* ==========================================
       1. 기본 버튼 레이아웃 및 공통 스타일
       ================================---------- */
	.btn {
		width: fit-content;
		height: fit-content;
		position: relative;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: 0.375rem; /* 6px로 살짝 좁힘 */
		font-family: inherit;
		font-size: 0.8125rem; /* 13px (기존 sm 크기) */
		font-weight: 500;
		line-height: 1.25rem;
		white-space: nowrap;
		text-decoration: none;
		user-select: none;
		cursor: pointer;
		border-radius: var(--radius-sm, 5px);
		border: 1px solid transparent;
		transition:
			background-color 0.15s ease,
			border-color 0.15s ease,
			color 0.15s ease,
			box-shadow 0.15s ease,
			opacity 0.15s ease;

		/* 기본 크기 (Default Size) = 기존 sm 크기 적용 */
		height: 2rem; /* 32px */
		padding: 0 0.75rem; /* 12px */
	}

	.btn:focus-visible {
		outline: none;
		box-shadow:
			0 0 0 2px var(--background-color),
			0 0 0 4px var(--accent-color, #3b82f6);
	}

	.btn:disabled,
	.btn[aria-disabled="true"] {
		opacity: 0.6;
		cursor: not-allowed;
		pointer-events: none;
	}

	/* 아이콘 스타일 */
	.icon {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.icon :global(svg) {
		width: 1em;
		height: 1em;
	}

	/* 로딩 스피너 스타일 */
	.spinner {
		width: 1em;
		height: 1em;
		border: 2px solid currentColor;
		border-right-color: transparent;
		border-radius: 50%;
		animation: spin 0.6s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		to {
			transform: rotate(360deg);
		}
	}

	.loading-text {
		opacity: 0.8;
	}

	/* ==========================================
       2. Variants (종류별 스타일)
       ================================---------- */
	.btn.default {
		background: var(--button-bg, #f3f4f6);
		color: var(--text-color);
		border-color: var(--border-color);
	}
	.btn.default:hover {
		background: var(--button-hover-bg);
	}

	.btn.primary {
		background: var(--accent-color);
		color: #fff;
		border-color: transparent;
	}
	.btn.primary:hover {
		background: var(--accent-color-hover, #1d4ed8);
	}

	.btn.secondary {
		background: var(--background-secondary);
		color: var(--secondary-text, #3730a3);
		border-color: transparent;
	}
	.btn.secondary:hover {
		background: var(--secondary-bg-hover, #c7d2fe);
	}

	.btn.outline {
		background: transparent;
		border-color: var(--border-strong, #d1d5db);
		color: var(--text-color);
	}
	.btn.outline:hover {
		background: var(--button-bg, #f3f4f6);
	}

	.btn.ghost {
		background: transparent;
		border-color: transparent;
		color: var(--text-color);
		padding: 0;
	}
	.btn.ghost:hover {
		background: transparent;
		border-color: transparent;
	}

	.btn.link {
		background: transparent;
		border-color: transparent;
		color: var(--accent-color);
		text-decoration: underline;
		text-underline-offset: 4px;
		padding: 0;
		height: auto;
	}
	.btn.link:hover {
		color: var(--accent-color-hover, #1d4ed8);
	}

	.btn.success {
		background: var(--color-green-bg, #ecfdf5);
		color: var(--color-green, #059669);
		border-color: var(--color-green-border, #a7f3d0);
	}
	.btn.success:hover {
		background: var(--color-green-hover, #d1fae5);
	}

	.btn.warning {
		background: var(--color-yellow-bg, #fffbeb);
		color: var(--color-yellow, #d97706);
		border-color: var(--color-yellow-border, #fde68a);
	}
	.btn.warning:hover {
		background: var(--color-yellow-hover, #fef3c7);
	}

	.btn.danger {
		background: var(--color-red-bg, #fef2f2);
		color: var(--color-red, #dc2626);
		border-color: var(--color-red-border, #fecaca);
	}
	.btn.danger:hover {
		background: var(--color-red-hover, #fee2e2);
	}

	.btn.info {
		background: var(--color-blue-bg, #eff6ff);
		color: var(--color-blue, #2563eb);
		border-color: var(--color-blue-border, #bfdbfe);
	}
	.btn.info:hover {
		background: var(--color-blue-hover, #dbeafe);
	}

	/* ==========================================
       3. 조정된 Sizes (축소된 크기 체계)
       ================================---------- */
	/* 초소형 크기 (24px) */
	.btn.xs {
		height: 1.5rem; /* 24px */
		padding: 0 0.5rem; /* 8px */
		font-size: 0.75rem; /* 12px */
		border-radius: calc(var(--radius-sm, 5px) - 1px);
	}

	/* 소형 크기 (28px) */
	.btn.sm {
		height: 1.75rem; /* 28px */
		padding: 0 0.625rem; /* 10px */
		font-size: 0.75rem; /* 12px */
	}

	/* 대형 크기 (36px) - 기존 default 크기 */
	.btn.lg {
		height: 2.25rem; /* 36px */
		padding: 0 1rem; /* 16px */
		font-size: 0.875rem; /* 14px */
	}

	/* 아이콘 전용 정사각형 크기 (Icon-only) */
	.btn.icon {
		width: 2rem; /* 32px (기본) */
		height: 2rem;
		padding: 0;
		background: none;
		border: none;
		color: var(--icon-color);
	}

	.btn.icon-xs {
		width: 1.5rem; /* 24px */
		height: 1.5rem;
		padding: 0;
		background: none;
		border: none;
		color: var(--icon-color);
	}

	.btn.icon-sm {
		width: 1.75rem; /* 28px */
		height: 1.75rem;
		padding: 0;
		background: none;
		border: none;
		color: var(--icon-color);
	}

	.btn.icon-lg {
		width: 2.25rem; /* 36px */
		height: 2.25rem;
		padding: 0;
		background: none;
		border: none;
		color: var(--icon-color);
	}
</style>
