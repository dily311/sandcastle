<script lang="ts">
	// 고정된 상태창들 (5-F-4-2, 5-F-5-2): 출처마다 하나씩 위아래로, 메시지와 같은 방식으로 그린다
	// (```html이면 격리된 iframe — triggerSlash로 변수를 바꿀 수 있음, 코드 블록 밖 HTML이면 거른 HTML)
	import RichText from "$features/character/RichText.svelte";
	import type { Block } from "$features/character/format";
	import { PIN_ATTR } from "./pin";

	let { blocks }: { blocks: Block[] } = $props();
</script>

<div class="status-window sc-status-window">
	{#if blocks.length}
		{#each blocks as b, i (i)}
			<div class="one sc-status-item"><RichText blocks={[b]} /></div>
		{/each}
	{:else}
		<p class="empty">상태창 HTML에 <code>{PIN_ATTR}</code>를 붙이면 가장 최근 것이 여기 모임</p>
	{/if}
</div>

<style>
	.status-window {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-width: 0;
	}
	.empty {
		margin: 0;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	code {
		font-size: inherit;
	}
</style>
