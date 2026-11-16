<script lang="ts" module>
	import type { Component } from "svelte";
	export interface MenuItem {
		label: string;
		icon?: Component;
		danger?: boolean;
		disabled?: boolean;
		/** 앞에 구분선 */
		separator?: boolean;
		/** 선택 표시 (정렬 등 하나를 고르는 메뉴) */
		checked?: boolean;
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
		/** align "end": x가 메뉴의 오른쪽 끝 (버튼 오른쪽 끝에 맞춰 버튼 아래에 띄울 때). 메뉴 실제 폭으로 맞춘다 */
		at: { x: number; y: number; align?: "start" | "end" } | null;
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
		const left = at.align === "end" ? at.x - r.width : at.x;
		pos = {
			x: Math.max(8, Math.min(left, window.innerWidth - r.width - 8)),
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
				role={it.checked === undefined ? "menuitem" : "menuitemradio"}
				aria-checked={it.checked}
				class:danger={it.danger}
				disabled={it.disabled}
				onclick={() => {
					it.onselect();
					onclose();
				}}
			>
				{#if it.icon}<it.icon />{/if}{it.label}
				{#if it.checked}<span class="check" aria-hidden="true">✓</span>{/if}
			</button>
		{/each}
	</div>
{/if}

<style>
	.menu {
		position: fixed;
		z-index: var(--z-menu);
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
	.check {
		margin-left: auto;
		padding-left: var(--space-3);
		color: var(--accent-text);
		font-weight: 700;
	}
	.sep {
		height: 1px;
		margin: 4px 2px;
		background: var(--border);
	}
</style>
