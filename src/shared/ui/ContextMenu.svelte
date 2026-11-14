<script lang="ts" module>
	import type { Component } from "svelte";
	export interface MenuItem {
		label: string;
		icon?: Component;
		danger?: boolean;
		disabled?: boolean;
		/** 앞에 구분선 */
		separator?: boolean;
		onselect: () => void;
	}
</script>

<script lang="ts">
	/**
	 * 오른쪽 클릭 메뉴. 사용:
	 *   let menu = $state<{ x: number; y: number } | null>(null);
	 *   <div oncontextmenu={(e) => { e.preventDefault(); menu = { x: e.clientX, y: e.clientY }; }}>
	 *   <ContextMenu at={menu} items={...} onclose={() => (menu = null)} />
	 */
	interface Props {
		at: { x: number; y: number } | null;
		items: MenuItem[];
		label?: string;
		onclose: () => void;
	}
	let { at, items, label = "메뉴", onclose }: Props = $props();

	let el: HTMLDivElement | undefined = $state();
	let pos = $state({ x: 0, y: 0 });

	// 화면 밖으로 나가지 않게 위치 보정, 첫 항목에 포커스
	$effect(() => {
		if (!at || !el) return;
		const r = el.getBoundingClientRect();
		pos = {
			x: Math.min(at.x, window.innerWidth - r.width - 8),
			y: Math.min(at.y, window.innerHeight - r.height - 8),
		};
		el.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus();
	});

	function onkeydown(e: KeyboardEvent) {
		if (e.key === "Escape") return onclose();
		if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
		e.preventDefault();
		const buttons = [...(el?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
		const i = buttons.indexOf(document.activeElement as HTMLButtonElement);
		buttons[(i + (e.key === "ArrowDown" ? 1 : buttons.length - 1)) % buttons.length]?.focus();
	}
</script>

<svelte:window onmousedown={(e) => at && el && !el.contains(e.target as Node) && onclose()} onblur={() => at && onclose()} />

{#if at}
	<div bind:this={el} class="menu" role="menu" aria-label={label} tabindex="-1" style:left="{pos.x}px" style:top="{pos.y}px" {onkeydown}>
		{#each items as it (it.label)}
			{#if it.separator}<div class="sep" role="separator"></div>{/if}
			<button
				role="menuitem"
				class:danger={it.danger}
				disabled={it.disabled}
				onclick={() => {
					it.onselect();
					onclose();
				}}
			>
				{#if it.icon}<it.icon />{/if}{it.label}
			</button>
		{/each}
	</div>
{/if}

<style>
	.menu {
		position: fixed;
		z-index: 1000;
		min-width: 168px;
		padding: 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		box-shadow: var(--shadow-lg);
		outline: none;
		animation: in var(--duration-fast) var(--ease);
	}
	@keyframes in {
		from {
			opacity: 0;
			transform: scale(0.97);
		}
	}
	button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		font-size: var(--text-sm);
		text-align: left;
		cursor: pointer;
	}
	button :global(svg) {
		width: 15px;
		height: 15px;
		color: var(--text-muted);
	}
	button:hover:not(:disabled),
	button:focus-visible {
		background: var(--surface-hover);
		outline: none;
	}
	button:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.danger,
	.danger :global(svg) {
		color: var(--danger);
	}
	.sep {
		height: 1px;
		margin: 4px 2px;
		background: var(--border);
	}
</style>
