<script lang="ts">
	// 주 동작 버튼 + 옆 화살표 메뉴 (예: "새로 만들기" ⌄ 가져오기). 캐릭터·게임·로어북·이미지 화면이 같이 씀
	import { ChevronDown, Plus } from "@lucide/svelte";
	import type { Component } from "svelte";
	import ContextMenu, { type MenuItem } from "./ContextMenu.svelte";

	interface Props {
		label: string;
		onclick: () => void;
		/** 화살표 메뉴 항목 */
		items: MenuItem[];
		icon?: Component;
		disabled?: boolean;
		/** 화살표 버튼의 화면 낭독기 이름 */
		menuLabel?: string;
		size?: "sm" | "md";
	}
	let { label, onclick, items, icon: Icon = Plus, disabled = false, menuLabel = "다른 방법", size = "md" }: Props = $props();

	let box: HTMLDivElement | undefined = $state();
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	function open() {
		const r = box?.getBoundingClientRect();
		if (r) menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}
</script>

<div class="split" class:sm={size === "sm"} bind:this={box}>
	<button type="button" class="main" {disabled} {onclick}><Icon />{label}</button>
	<button type="button" class="more" aria-label={menuLabel} title={menuLabel} aria-haspopup="menu" aria-expanded={!!menu} onclick={open}><ChevronDown /></button>
</div>
<ContextMenu at={menu} label={menuLabel} {items} onclose={() => (menu = null)} />

<style>
	.split {
		display: inline-flex;
		align-items: stretch;
		border-radius: var(--radius-sm);
		background: var(--accent);
		color: var(--text-on-accent);
		box-shadow: var(--shadow-sm);
		flex-shrink: 0;
	}
	button {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		height: 36px;
		border: none;
		background: transparent;
		color: inherit;
		font: inherit;
		font-size: var(--text-sm);
		font-weight: 600;
		cursor: pointer;
	}
	.sm button {
		height: 30px;
	}
	button:hover:not(:disabled) {
		background: var(--accent-hover);
	}
	button:disabled {
		opacity: 0.55;
		cursor: default;
	}
	.main {
		padding: 0 var(--space-3) 0 var(--space-3);
		border-radius: var(--radius-sm) 0 0 var(--radius-sm);
	}
	.more {
		padding: 0 8px;
		border-left: 1px solid color-mix(in oklab, var(--text-on-accent) 35%, transparent);
		border-radius: 0 var(--radius-sm) var(--radius-sm) 0;
	}
	.split :global(svg) {
		width: 16px;
		height: 16px;
	}
	.more :global(svg) {
		width: 18px;
		height: 18px;
	}
</style>
