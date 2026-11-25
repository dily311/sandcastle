<script lang="ts">
	import { _ } from "svelte-i18n";
	import { Copy, Download, FolderInput, FolderOutput, MessageSquare, Pencil, Pin, PinOff, Trash2, Upload } from "@lucide/svelte";
	import { ContextMenu, IconButton, type MenuItem } from "$shared/ui";
	import { absolute, relative } from "$shared/time";
	import { library, type Conversation } from "./library.svelte";
	import { convActions } from "./actions";

	interface Props {
		conv: Conversation;
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
		if (!editing) return;
		editing = false;
		void library.rename(conv.id, draft);
	}
	const imported = $derived(!!(conv.meta && typeof conv.meta === "object" && "sillytavern" in conv.meta));

	// 오른쪽 클릭: 단축 버튼과 같은 동작 + 폴더로 이동
	let menu = $state<{ x: number; y: number } | null>(null);
	const items = $derived.by((): MenuItem[] => {
		const folders = library.foldersOf(conv.characterId).filter((f) => f.id !== conv.folderId);
		return [
			{ label: $_("convCard.rename"), icon: Pencil, onselect: startEdit },
			{ label: conv.pinned ? $_("convCard.unpin") : $_("convCard.pinTop"), icon: conv.pinned ? PinOff : Pin, onselect: () => library.togglePin(conv.id) },
			...folders.map((f, i) => ({ label: $_("convCard.moveToFolder", { values: { name: f.name } }), icon: FolderInput, separator: i === 0, onselect: () => library.move(conv.id, f.id) })),
			...(conv.folderId ? [{ label: $_("convCard.removeFolder"), icon: FolderOutput, separator: folders.length === 0, onselect: () => library.move(conv.id, null) }] : []),
			{ label: $_("convCard.duplicate"), icon: Copy, separator: true, onselect: () => library.duplicate(conv.id) },
			{ label: $_("convCard.exportMenu"), icon: Download, onselect: () => convActions.exportMenu(conv) },
			{ label: $_("common.delete"), icon: Trash2, danger: true, onselect: () => convActions.remove(conv) },
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
				aria-label={$_("convCard.nameAria")}
				autofocus
				onkeydown={(e) => {
					if (e.key === "Enter") commit();
					if (e.key === "Escape") editing = false;
				}}
				onblur={commit}
			/>
		{:else}
			<button class="title" onclick={onopen} title={conv.title}>{conv.title || $_("convCard.untitled")}</button>
		{/if}
		<div class="quick">
			<IconButton label={$_("convCard.rename")} size="sm" onclick={startEdit}><Pencil /></IconButton>
			<IconButton label={conv.pinned ? $_("convCard.unpin") : $_("convCard.pinTop")} size="sm" active={conv.pinned} onclick={() => library.togglePin(conv.id)}>
				<Pin class={conv.pinned ? "pinned" : ""} />
			</IconButton>
		</div>
	</div>
	<div class="bottom">
		<span class="meta">
			<time title={absolute(conv.updatedAt)}>{relative(conv.updatedAt)}</time>
			<span class="count"><MessageSquare />{conv.messageCount}</span>
			{#if imported}<span class="count" title={$_("convCard.importedStTitle")}><Upload /></span>{/if}
		</span>
		<div class="tools">
			<IconButton label={$_("convCard.duplicate")} size="sm" onclick={() => library.duplicate(conv.id)}><Copy /></IconButton>
			<IconButton label={$_("convCard.exportTooltip")} size="sm" onclick={() => convActions.exportMenu(conv)}><Download /></IconButton>
			<IconButton label={$_("common.delete")} size="sm" onclick={() => convActions.remove(conv)}><Trash2 /></IconButton>
		</div>
	</div>
</article>
<ContextMenu at={menu} {items} label={$_("convCard.menuAria")} onclose={() => (menu = null)} />

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
