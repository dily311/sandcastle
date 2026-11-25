<script lang="ts">
	// 좁은 화면·Android: 채팅방 위에 늘 보이는 상태창 (5-F-4-2, 사용자 요청 — ADR-029 ⑤ 추가). 접을 수 있음
	import { ChevronDown } from "@lucide/svelte";
	import type { Block } from "$features/character/format";
	import StatusWindow from "./StatusWindow.svelte";
	import { statusView } from "./statusView.svelte";

	let { blocks }: { blocks: Block[] } = $props();
</script>

<div class="top sc-status-top" class:collapsed={statusView.topCollapsed}>
	<button type="button" class="head" aria-expanded={!statusView.topCollapsed} onclick={() => statusView.set({ topCollapsed: !statusView.topCollapsed })}>
		<ChevronDown class={statusView.topCollapsed ? "rot" : ""} /><b>상태창</b>
	</button>
	{#if !statusView.topCollapsed}
		<div class="body"><StatusWindow {blocks} /></div>
	{/if}
</div>

<style>
	.top {
		padding-top: 36px;
		flex: none;
		border-bottom: 1px solid var(--border);
		background: var(--surface);
	}
	.head {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		width: 100%;
		padding: var(--space-1) var(--space-3);
		border: none;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.head :global(svg) {
		width: 14px;
		height: 14px;
		transition: transform var(--duration-fast) var(--ease);
	}
	.head :global(.rot) {
		transform: rotate(-90deg);
	}
	.body {
		max-height: 30vh;
		overflow: auto;
		padding: 0 var(--space-3) var(--space-2);
	}
</style>
