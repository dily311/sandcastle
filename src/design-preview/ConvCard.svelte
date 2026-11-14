<script lang="ts">
	import { Copy, Download, Heart, MessageSquare, Pencil, Trash2, Upload } from "@lucide/svelte";
	import { IconButton } from "$shared/ui";
	import { absolute, convActions, relative, type MockConv } from "./mock.svelte";

	interface Props {
		conv: MockConv;
		selected?: boolean;
		/** 사이드바용 좁은 카드 (단축 버튼은 호버 시에만) */
		compact?: boolean;
		onopen: () => void;
	}
	let { conv, selected = false, compact = false, onopen }: Props = $props();

	let editing = $state(false);
	let draft = $state("");
	function startEdit() {
		draft = conv.title;
		editing = true;
	}
	function commit() {
		convActions.rename(conv.id, draft);
		editing = false;
	}
</script>

<article class="card" class:selected class:compact>
	<div class="top">
		{#if editing}
			<!-- svelte-ignore a11y_autofocus -->
			<input
				class="rename"
				bind:value={draft}
				aria-label="대화 이름"
				autofocus
				onkeydown={(e) => {
					if (e.key === "Enter") commit();
					if (e.key === "Escape") editing = false;
				}}
				onblur={commit}
			/>
		{:else}
			<button class="title" onclick={onopen} title={conv.title}>{conv.title}</button>
		{/if}
		<div class="quick">
			<IconButton label="이름 바꾸기" size="sm" onclick={startEdit}><Pencil /></IconButton>
			<IconButton label={conv.favorite ? "즐겨찾기 해제" : "즐겨찾기"} size="sm" active={conv.favorite} onclick={() => convActions.toggleFavorite(conv.id)}>
				<Heart class={conv.favorite ? "fav" : ""} />
			</IconButton>
		</div>
	</div>
	<div class="bottom">
		<span class="meta">
			<time title={absolute(conv.ago)}>{relative(conv.ago)}</time>
			<span class="count"><MessageSquare />{conv.count}</span>
			{#if conv.imported}<span class="count" title="SillyTavern에서 가져옴"><Upload /></span>{/if}
		</span>
		<div class="tools">
			<IconButton label="복제" size="sm" onclick={() => convActions.duplicate(conv.id)}><Copy /></IconButton>
			<IconButton label="내보내기" size="sm"><Download /></IconButton>
			<IconButton label="삭제" size="sm" onclick={() => convActions.remove(conv.id)}><Trash2 /></IconButton>
		</div>
	</div>
</article>

<style>
	.card {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
		background: var(--surface);
		transition:
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.card:hover {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-sm);
	}
	.card.selected {
		border-color: var(--accent);
		box-shadow: 0 0 0 1px var(--accent);
	}
	.top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-2) var(--space-2) var(--space-4);
	}
	.title {
		flex: 1;
		min-width: 0;
		padding: 0;
		border: none;
		background: none;
		text-align: left;
		font-weight: 700;
		font-size: var(--text-lg);
		color: var(--accent-text);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.title:hover {
		text-decoration: underline;
	}
	.rename {
		flex: 1;
		min-width: 0;
		height: 28px;
		padding: 0 var(--space-2);
		border: 1px solid var(--accent);
		border-radius: var(--radius-xs);
		background: var(--surface);
		font-weight: 700;
		outline: none;
	}
	.quick,
	.tools {
		display: flex;
		flex: none;
	}
	.quick :global(.fav) {
		fill: var(--danger);
		color: var(--danger);
	}
	.bottom {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		padding: 0 var(--space-2) var(--space-2) var(--space-4);
		border-top: 1px solid transparent;
	}
	.meta {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		font-size: var(--text-sm);
		color: var(--text-faint);
	}
	.meta time {
		cursor: help;
	}
	.count {
		display: inline-flex;
		align-items: center;
		gap: 4px;
	}
	.count :global(svg) {
		width: 13px;
		height: 13px;
	}

	/* 사이드바: 작게, 단축 버튼은 호버 때만 */
	.compact .top {
		padding: var(--space-2) var(--space-1) 0 var(--space-3);
	}
	.compact .title {
		font-size: var(--text-sm);
	}
	.compact .bottom {
		padding: 0 var(--space-1) var(--space-1) var(--space-3);
	}
	.compact .meta {
		font-size: var(--text-xs);
	}
	/* 즐겨찾기된 하트는 항상 보이게 */
	.compact .quick :global(.icon-btn:not(.active)),
	.compact .tools {
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.compact:hover .quick :global(.icon-btn),
	.compact:hover .tools,
	.compact:focus-within .quick :global(.icon-btn),
	.compact:focus-within .tools {
		opacity: 1;
	}
</style>
