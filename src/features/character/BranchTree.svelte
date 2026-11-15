<script lang="ts">
	// 오른쪽 패널 '분기' 탭: 이 대화가 속한 갈래 전체를 나무로
	import { GitBranch } from "@lucide/svelte";
	import { relative } from "$shared/time";
	import { familyOf, familySize, pathOf, type BranchNode } from "./branches";
	import type { Conversation } from "./library.svelte";

	let { conv, onopen }: { conv: Conversation; onopen: (id: string, jumpTo?: string) => void } = $props();

	const tree = $derived(familyOf(conv));
	const size = $derived(familySize(tree));
	/** 지금 대화까지 가는 길에 있는 대화 (강조) */
	const onPath = $derived(new Set(pathOf(conv).flatMap((s) => (s.conv ? [s.conv.id] : []))));
	const orphan = $derived(!pathOf(conv)[0].conv);
</script>

{#snippet node(n: BranchNode, depth: number)}
	<li style:--depth={depth}>
		<button class="row" class:current={n.conv.id === conv.id} class:path={onPath.has(n.conv.id)} aria-current={n.conv.id === conv.id ? "page" : undefined} onclick={() => onopen(n.conv.id)}>
			<span class="dot" aria-hidden="true"></span>
			<span class="t">
				<b>{n.conv.title}</b>
				<small>
					{#if n.at}#{n.at.seq}에서 갈라짐{" · "}{/if}메시지 {n.conv.messageCount}개 · {relative(n.conv.updatedAt)}
				</small>
			</span>
			{#if n.conv.id === conv.id}<span class="here">지금</span>{/if}
		</button>
		{#if n.children.length}
			<ul>
				{#each n.children as c (c.conv.id)}{@render node(c, depth + 1)}{/each}
			</ul>
		{/if}
	</li>
{/snippet}

<div class="panel-body">
	<section>
		<h4>분기 <span class="faint">대화 {size}개</span></h4>
		{#if orphan}<p class="note">갈라져 나온 원래 대화는 지워졌음</p>{/if}
		{#if size > 1}
			<ul class="tree">{@render node(tree, 0)}</ul>
		{:else}
			<div class="panel-none">
				<GitBranch />
				<p>아직 갈래 없음. 메시지 위 도구의 '여기서 분기 나누기'로 다른 전개를 따로 이어 갈 수 있음</p>
			</div>
		{/if}
	</section>
</div>

<style>
	.tree,
	.tree ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.tree ul {
		position: relative;
		margin-left: 11px;
		padding-left: 10px;
		border-left: 1px solid var(--border);
	}
	.row {
		display: flex;
		align-items: flex-start;
		gap: var(--space-2);
		width: 100%;
		padding: 6px var(--space-2);
		border: none;
		border-radius: var(--radius-sm);
		background: transparent;
		text-align: left;
		color: var(--text-muted);
		cursor: pointer;
	}
	.row:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.row.path {
		color: var(--text);
	}
	.row.current {
		background: var(--accent-soft);
	}
	.dot {
		flex: none;
		width: 8px;
		height: 8px;
		margin-top: 6px;
		border-radius: 50%;
		border: 2px solid var(--border-strong);
		background: var(--bg);
	}
	.path .dot {
		border-color: var(--accent);
	}
	.current .dot {
		background: var(--accent);
		border-color: var(--accent);
	}
	.t {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		font-size: var(--text-sm);
		line-height: 1.35;
	}
	.t b {
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.t small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.here {
		flex: none;
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--accent-text);
	}
	.note {
		margin: 0;
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.panel-none :global(svg) {
		width: 22px;
		height: 22px;
	}
	.panel-none p {
		margin: var(--space-2) 0 0;
	}
</style>
