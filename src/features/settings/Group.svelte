<script lang="ts">
	import type { Snippet } from "svelte";

	interface Props {
		title?: string;
		/** 제목 오른쪽 */
		aside?: Snippet;
		/** 행이 아닌 내용(목록·로그)일 때 안쪽 여백을 줄임 */
		flush?: boolean;
		children: Snippet;
	}
	let { title, aside, flush = false, children }: Props = $props();
</script>

{#if title || aside}
	<div class="title">
		{#if title}<h3>{title}</h3>{/if}
		{#if aside}{@render aside()}{/if}
	</div>
{/if}
<div class="card" class:flush>
	{@render children()}
</div>

<style>
	.title {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-3);
		margin: var(--space-6) 0 var(--space-3);
		/* 좁은 화면: 오른쪽 도구가 제목 아래로 (제목이 두 줄로 잘리지 않게) */
		flex-wrap: wrap;
	}
	.title h3 {
		white-space: nowrap;
	}
	.card {
		border-radius: var(--radius-lg);
		background: var(--bg-sunken);
		padding: 0 var(--space-5);
	}
	.card.flush {
		padding: var(--space-2);
	}
</style>
