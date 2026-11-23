<script lang="ts">
	// 고정된 상태창 하나 (5-F-4-2): 가장 최근 메시지의 상태창 블록을 메시지와 같은 방식으로 그린다
	// (```html이면 격리된 iframe — triggerSlash로 변수를 바꿀 수 있음, 코드 블록 밖 HTML이면 거른 HTML)
	import RichText from "$features/character/RichText.svelte";
	import type { Block } from "$features/character/format";
	import { PIN_ATTR } from "./pin";

	let { block }: { block: Block | null } = $props();
</script>

<div class="status-window sc-status-window">
	{#if block}
		<RichText blocks={[block]} />
	{:else}
		<p class="empty">상태창 HTML에 <code>{PIN_ATTR}</code>를 붙이면 가장 최근 것이 여기 모임</p>
	{/if}
</div>

<style>
	.status-window {
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
