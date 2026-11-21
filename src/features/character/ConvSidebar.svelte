<script lang="ts">
	import { chatImport } from "./chatImport.svelte";
	import { lastAt } from "$features/character/library.svelte";
	import { ArrowDownUp, ChevronDown, Folder, FolderOpen, FolderPlus, PanelLeft, Pencil, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { ContextMenu, IconButton, Input, type MenuItem } from "$shared/ui";
	import { CHAT_FILES, pickFiles } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import ConvCard from "./ConvCard.svelte";
	import { library, type Conversation } from "./library.svelte";

	let { charId, convId, onopen, oncreate, onclose }: { charId: string; convId: string; onopen: (id: string) => void; oncreate: () => void, onclose: () => void } = $props();

	$effect(() => {
		void library.loadFolders(charId);
	});

	let query = $state("");
	type Sort = "newest" | "oldest" | "name" | "name-desc" | "most" | "least";
	const SORT_KEY = "sc.convs.sort";
	let sort: Sort = $state((() => {
		try {
			return (localStorage.getItem(SORT_KEY) as Sort) || "newest";
		} catch {
			return "newest";
		}
	})());
	$effect(() => {
		try {
			localStorage.setItem(SORT_KEY, sort);
		} catch {
			/* 무시 */
		}
	});
	const sorts: { id: Sort; label: string }[] = [
		{ id: "newest", label: "최신순" },
		{ id: "oldest", label: "오래된순" },
		{ id: "name", label: "이름순 (가→하)" },
		{ id: "name-desc", label: "이름 역순 (하→가)" },
		{ id: "most", label: "메시지 많은 순" },
		{ id: "least", label: "메시지 적은 순" },
	];
	const compare: Record<Sort, (a: Conversation, b: Conversation) => number> = {
		newest: (a, b) => lastAt(b) - lastAt(a),
		oldest: (a, b) => lastAt(a) - lastAt(b),
		name: (a, b) => a.title.localeCompare(b.title, "ko"),
		"name-desc": (a, b) => b.title.localeCompare(a.title, "ko"),
		most: (a, b) => b.messageCount - a.messageCount,
		least: (a, b) => a.messageCount - b.messageCount,
	};
	const convs = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return library.conversations
			.filter((c) => c.characterId === charId && c.mode !== "phone" && (!q || c.title.toLowerCase().includes(q)))
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || compare[sort](a, b));
	});
	let sortMenu = $state<{ x: number; y: number } | null>(null);
	function openSortMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		sortMenu = { x: r.left, y: r.bottom + 4 };
	}
	const folders = $derived(library.foldersOf(charId));
	let collapsed: Record<string, boolean> = $state({});

	// 폴더 만들기 / 이름 바꾸기 (같은 입력칸)
	let editingFolder: string | null = $state(null);
	let folderName = $state("");
	function startFolderEdit(id: string, name = "") {
		editingFolder = id;
		folderName = name;
	}
	function commitFolder() {
		const id = editingFolder;
		editingFolder = null;
		if (id === "new") void library.createFolder(charId, folderName);
		else if (id) void library.renameFolder(id, folderName);
	}
	let folderMenuAt = $state<{ x: number; y: number } | null>(null);
	let folderMenuItems: MenuItem[] = $state([]);
	function folderMenu(e: MouseEvent, id: string, name: string) {
		e.preventDefault();
		folderMenuItems = [
			{ label: "이름 바꾸기", icon: Pencil, onselect: () => startFolderEdit(id, name) },
			{
				label: "폴더 삭제 (대화는 남김)",
				icon: Trash2,
				danger: true,
				separator: true,
				onselect: async () => {
					if (await confirmAction({ title: "폴더 삭제", message: `'${name}' 폴더만 지움. 안의 대화는 폴더 밖으로 나옴`, confirmLabel: "삭제", danger: true }))
						await library.removeFolder(id);
				},
			},
		];
		folderMenuAt = { x: e.clientX, y: e.clientY };
	}
	// 카드를 폴더(또는 폴더 밖)로 끌어다 놓기
	let dropTarget: string | null = $state(null);
	const dnd = (target: string | null, key: string) => ({
		ondragover: (e: DragEvent) => {
			e.preventDefault();
			dropTarget = key;
		},
		ondragleave: () => (dropTarget = null),
		ondrop: (e: DragEvent) => {
			e.preventDefault();
			const id = e.dataTransfer?.getData("text/conv-id");
			const c = id ? library.conversation(id) : null;
			if (c && c.folderId !== target) void library.move(c.id, target);
			dropTarget = null;
		},
	});

	async function importChat() {
		await chatImport.run(await pickFiles([CHAT_FILES], true), charId, onopen);
	}
</script>

{#snippet folderInput()}
	<!-- svelte-ignore a11y_autofocus -->
	<input
		class="folder-input"
		bind:value={folderName}
		placeholder="폴더 이름"
		aria-label="폴더 이름"
		autofocus
		onkeydown={(e) => {
			if (e.key === "Enter") commitFolder();
			if (e.key === "Escape") editingFolder = null;
		}}
		onblur={commitFolder}
	/>
{/snippet}

<aside class="convs" aria-label="대화 목록">
	<div class="convs-head">
		<IconButton label="닫기" size="sm" variant="soft" onclick={onclose}><PanelLeft /></IconButton>
		<span class="spacer"></span>
		<IconButton label="새 대화" size="sm" variant="soft" onclick={oncreate}><Plus /></IconButton>
		<IconButton label="대화 가져오기 (SillyTavern .jsonl)" size="sm" onclick={importChat}><Upload /></IconButton>
		<IconButton label="새 폴더" size="sm" onclick={() => startFolderEdit("new")}><FolderPlus /></IconButton>
	</div>
	<div class="convs-tools">
		<Input bind:value={query} placeholder="대화 검색" aria-label="대화 검색">
			{#snippet leading()}<Search />{/snippet}
		</Input>
		<IconButton label="정렬: {sorts.find((s) => s.id === sort)?.label}" size="md" active={!!sortMenu} aria-haspopup="menu" onclick={openSortMenu}><ArrowDownUp /></IconButton>
	</div>
	<ContextMenu
		at={sortMenu}
		label="정렬"
		items={sorts.map((s, i) => ({ label: s.label, checked: sort === s.id, separator: i === 2 || i === 4, onselect: () => (sort = s.id) }))}
		onclose={() => (sortMenu = null)}
	/>
	<div class="convs-list">
		{#if editingFolder === "new"}{@render folderInput()}{/if}
		{#each folders as f (f.id)}
			{@const items = convs.filter((c) => c.folderId === f.id)}
			{#if editingFolder === f.id}
				{@render folderInput()}
			{:else}
				<button
					class="folder"
					class:drop={dropTarget === f.id}
					aria-expanded={!collapsed[f.id]}
					onclick={() => (collapsed[f.id] = !collapsed[f.id])}
					oncontextmenu={(e) => folderMenu(e, f.id, f.name)}
					{...dnd(f.id, f.id)}
				>
					<ChevronDown class={collapsed[f.id] ? "closed" : ""} />
					{#if collapsed[f.id]}<Folder />{:else}<FolderOpen />{/if}
					<span class="fname">{f.name}</span>
					<span class="fcount">{items.length}</span>
				</button>
			{/if}
			{#if !collapsed[f.id]}
				<div class="in-folder">
					{#each items as c (c.id)}
						<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
					{:else}
						<p class="empty-folder">비어 있음</p>
					{/each}
				</div>
			{/if}
		{/each}
		<div class="loose" class:drop={dropTarget === "loose"} {...dnd(null, "loose")}>
			{#each convs.filter((c) => !c.folderId || !folders.some((f) => f.id === c.folderId)) as c (c.id)}
				<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
			{/each}
		</div>
		{#if !convs.length}<p class="none">{query ? "검색 결과 없음" : "대화 없음"}</p>{/if}
	</div>
	<ContextMenu at={folderMenuAt} items={folderMenuItems} label="폴더 메뉴" onclose={() => (folderMenuAt = null)} />
</aside>

<style>
	.convs {
		display: flex;
		flex-direction: column;
		min-height: 0;
		border-right: 1px solid var(--border);
		background: var(--bg);
	}
	.convs-head {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		padding: var(--space-3) var(--space-3) var(--space-2);
	}
	.spacer {
		flex: 1;
	}
	.convs-tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: var(--space-1);
		padding: 0 var(--space-3) var(--space-2);
	}
	.convs-list {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		overflow: auto;
		padding: 0 var(--space-3) var(--space-3);
	}
	.folder {
		display: flex;
		align-items: center;
		gap: 6px;
		width: 100%;
		height: 30px;
		flex: none;
		padding: 0 var(--space-2);
		border: 1px dashed transparent;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		font-size: var(--text-sm);
		font-weight: 600;
		text-align: left;
		cursor: pointer;
	}
	.folder:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.folder :global(svg) {
		width: 15px;
		height: 15px;
		flex: none;
		transition: transform var(--duration-fast) var(--ease);
	}
	.folder :global(.closed) {
		transform: rotate(-90deg);
	}
	.fname {
		flex: 1;
		min-width: 0;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.fcount {
		font-size: var(--text-xs);
		font-weight: 500;
		color: var(--text-faint);
	}
	.folder.drop,
	.loose.drop {
		border-color: var(--accent);
		background: var(--accent-soft);
	}
	.in-folder {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding-left: var(--space-3);
		margin-left: 9px;
		border-left: 1px solid var(--border);
	}
	.empty-folder {
		padding: 2px var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.loose {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		min-height: 24px;
		border: 1px dashed transparent;
		border-radius: var(--radius-md);
	}
	.folder-input {
		width: 100%;
		height: 30px;
		padding: 0 var(--space-2);
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		background: var(--surface);
		outline: none;
		flex: none;
	}
	.none {
		padding: var(--space-6);
		text-align: center;
		color: var(--text-faint);
		font-size: var(--text-sm);
	}
</style>
