<script lang="ts">
	// 뜻으로 찾기: 엔터를 눌렀을 때만 질의를 임베딩한다 (작업 하나, 요금이 작게 듦). 타이핑마다 부르지 않음
	import { Search } from "@lucide/svelte";
	import type { SearchHit, SearchResult } from "$ipc/bindings";
	import { Badge, Button, Input } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";

	let { conversationId = null, onopen }: { conversationId?: string | null; onopen: (hit: SearchHit) => void } = $props();

	let query = $state("");
	let busy = $state(false);
	let result = $state<SearchResult | null>(null);

	async function search() {
		const q = query.trim();
		if (!q || busy) return;
		busy = true;
		try {
			result = await call(commands.searchMemory({ query: q, scope: { conversationId }, limit: 10 }), { source: "memory" });
		} catch {
			/* 토스트는 call이 띄움 */
		} finally {
			busy = false;
		}
	}
</script>

<form class="bar" onsubmit={(e) => (e.preventDefault(), search())}>
	<Input bind:value={query} placeholder={conversationId ? "이 대화에서 뜻으로 찾기" : "모든 대화에서 뜻으로 찾기"} aria-label="뜻으로 찾기">
		{#snippet leading()}<Search />{/snippet}
	</Input>
	<Button type="submit" loading={busy} disabled={!query.trim()}>찾기</Button>
</form>

{#if result}
	{#if result.otherModelChunks > 0}
		<p class="faint">다른 모델로 색인된 {result.otherModelChunks}개는 찾지 못함 — 다시 색인하거나 정리하세요</p>
	{/if}
	{#if result.truncated}<p class="faint">훑는 상한에 걸려 일부만 검색함</p>{/if}
	{#if result.hits.length === 0}
		<p class="faint">찾은 것 없음 (색인한 대화가 있어야 찾을 수 있음)</p>
	{:else}
		<ol class="hits">
			{#each result.hits as h (h.conversationId + h.firstMessageId)}
				<li>
					<button class="hit" onclick={() => onopen(h)}>
						<span class="meta">
							<b>{h.conversationTitle || "(제목 없음)"}</b>
							<span class="score">{(h.score ?? 0).toFixed(2)}</span>
							{#if h.stale}<Badge tone="warning">오래됨</Badge>{/if}
						</span>
						<span class="snip">{h.snippet}</span>
					</button>
				</li>
			{/each}
		</ol>
	{/if}
{/if}

<style>
	.bar {
		display: flex;
		gap: var(--space-2);
	}
	.bar > :global(:first-child) {
		flex: 1;
	}
	.faint {
		margin: var(--space-2) 0 0;
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.hits {
		margin: var(--space-3) 0 0;
		padding: 0;
		list-style: none;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.hit {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		width: 100%;
		padding: var(--space-2) var(--space-3);
		border: none;
		border-radius: var(--radius-md);
		background: var(--surface-2);
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.hit:hover {
		background: var(--surface);
	}
	.meta {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		font-size: var(--text-sm);
	}
	.score {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.snip {
		display: -webkit-box;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		-webkit-box-orient: vertical;
		overflow: hidden;
		color: var(--text-muted);
		font-size: var(--text-xs);
		white-space: pre-line;
	}
</style>
