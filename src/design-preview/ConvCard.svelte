<script lang="ts">
	import { Copy, Download, FolderInput, FolderOutput, MessageSquare, Pencil, Pin, PinOff, Trash2, Upload } from "@lucide/svelte";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { absolute, convActions, relative, store, type MockConv } from "./mock.svelte";

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

	// 오른쪽 클릭: 단축 버튼과 같은 동작 + 폴더로 이동
	let menu = $state<{ x: number; y: number } | null>(null);
	const items = $derived.by((): MenuItem[] => {
		const folders = store.folders.filter((f) => f.charId === conv.charId && f.id !== conv.folderId);
		return [
			{ label: "이름 바꾸기", icon: Pencil, onselect: startEdit },
			{ label: conv.pinned ? "고정 해제" : "맨 위에 고정", icon: conv.pinned ? PinOff : Pin, onselect: () => convActions.togglePin(conv.id) },
			...folders.map((f, i) => ({ label: `'${f.name}'(으)로 이동`, icon: FolderInput, separator: i === 0, onselect: () => convActions.move(conv.id, f.id) })),
			...(conv.folderId ? [{ label: "폴더에서 빼기", icon: FolderOutput, separator: folders.length === 0, onselect: () => convActions.move(conv.id, null) }] : []),
			{ label: "복제", icon: Copy, separator: true, onselect: () => convActions.duplicate(conv.id) },
			{ label: "내보내기", icon: Download, onselect: () => {} },
			{ label: "삭제", icon: Trash2, danger: true, onselect: () => convActions.remove(conv.id) },
		];
	});
</script>

<article
	class="card"
	class:selected
	class:compact
	draggable="true"
	ondragstart={(e) => e.dataTransfer?.setData("text/conv-id", conv.id)}
	oncontextmenu={(e) => {
		e.preventDefault();
		menu = { x: e.clientX, y: e.clientY };
	}}
>
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
			<IconButton label={conv.pinned ? "고정 해제" : "맨 위에 고정"} size="sm" active={conv.pinned} onclick={() => convActions.togglePin(conv.id)}>
				<Pin class={conv.pinned ? "pinned" : ""} />
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
<ContextMenu at={menu} {items} label="대화 메뉴" onclose={() => (menu = null)} />

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
	.quick :global(.pinned) {
		fill: var(--accent);
		color: var(--accent);
		transform: rotate(45deg);
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
	/* 고정된 핀은 항상 보이게 */
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
