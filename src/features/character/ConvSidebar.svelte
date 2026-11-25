<script lang="ts">
	import { _ } from "svelte-i18n";
	import { chatImport } from "./chatImport.svelte";
	import { ArrowDownUp, ChevronDown, Folder, FolderOpen, FolderPlus, PanelLeft, Pencil, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { ContextMenu, IconButton, Input, type MenuItem } from "$shared/ui";
	import { CHAT_FILES, pickFiles } from "$shared/ipc/dialog";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import ConvCard from "./ConvCard.svelte";
	import { CONV_SORT_KEYS, CONV_SORT_LABEL, compareConversations, isConvSortBy, type ConvSortBy } from "./convSort";
	import { call, commands } from "$shared/ipc/client";
	import { library, type Conversation } from "./library.svelte";

	let { charId, convId, onopen, oncreate, onclose }: { charId: string; convId: string; onopen: (id: string) => void; oncreate: () => void, onclose: () => void } = $props();

	$effect(() => {
		void library.loadFolders(charId);
	});

	let query = $state("");
	const SORT_KEY = "sc.convs.sort";
	const SORT_DESC_KEY = "sc.convs.sortDesc";
	function loadConvSort(): { sortBy: ConvSortBy; desc: boolean } {
		try {
			const raw = localStorage.getItem(SORT_KEY);
			const rawDesc = localStorage.getItem(SORT_DESC_KEY);
			if (isConvSortBy(raw)) {
				return { sortBy: raw, desc: rawDesc !== "0" };
			}
			// 이전 버전 정렬 키 호환 마이그레이션
			switch (raw) {
				case "oldest":
					return { sortBy: "recent", desc: false };
				case "name-desc":
					return { sortBy: "name", desc: true };
				case "most":
					return { sortBy: "messages", desc: true };
				case "least":
					return { sortBy: "messages", desc: false };
				case "newest":
				default:
					return { sortBy: "recent", desc: rawDesc !== "0" };
			}
		} catch {
			return { sortBy: "recent", desc: true };
		}
	}
	const sort0 = loadConvSort();
	let sortBy = $state<ConvSortBy>(sort0.sortBy);
	let desc = $state<boolean>(sort0.desc);
	$effect(() => {
		try {
			localStorage.setItem(SORT_KEY, sortBy);
			localStorage.setItem(SORT_DESC_KEY, desc ? "1" : "0");
		} catch {
			/* 무시 */
		}
	});
	// "mes:글"이면 제목이 아니라 메시지 내용으로 (백엔드에서 찾음, 입력이 멈춘 뒤)
	const mesQuery = $derived(/^mes:/i.test(query.trim()) ? query.trim().slice(4).trim() : null);
	let mesHits = $state<Map<string, number> | null>(null);
	$effect(() => {
		const t = mesQuery;
		const cid = charId;
		if (!t) return void (mesHits = null);
		const timer = setTimeout(async () => {
			const r = await call(commands.findConversationsByMessage(cid, t), { source: "chat", silent: true }).catch(() => []);
			if (mesQuery === t) mesHits = new Map(r);
		}, 250);
		return () => clearTimeout(timer);
	});
	const convs = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const match = (c: Conversation) => (mesQuery !== null ? !mesQuery || !!mesHits?.has(c.id) : !q || c.title.toLowerCase().includes(q));
		return library.conversations
			.filter((c) => c.characterId === charId && c.mode !== "phone" && match(c))
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || compareConversations(a, b, sortBy, desc));
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
			{ label: $_("settings.chrome.rename"), icon: Pencil, onselect: () => startFolderEdit(id, name) },
			{
				label: $_("settings.panels.deleteFolderKeepChats"),
				icon: Trash2,
				danger: true,
				separator: true,
				onselect: async () => {
					if (await confirmAction({ title: $_("settings.panels.deleteFolderTitle"), message: $_("settings.panels.deleteFolderMsg", { values: { name } }), confirmLabel: $_("common.delete"), danger: true }))
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
		placeholder={$_("settings.panels.folderName")}
		aria-label={$_("settings.panels.folderName")}
		autofocus
		onkeydown={(e) => {
			if (e.key === "Enter") commitFolder();
			if (e.key === "Escape") editingFolder = null;
		}}
		onblur={commitFolder}
	/>
{/snippet}

<aside class="convs sc-conv-list" aria-label={$_("settings.panels.convListAria")}>
	<div class="convs-head">
		<IconButton label={$_("common.close")} size="sm" variant="soft" onclick={onclose}><PanelLeft /></IconButton>
		<span class="spacer"></span>
		<IconButton label={$_("sidebar.newChat")} size="sm" variant="soft" onclick={oncreate}><Plus /></IconButton>
		<IconButton label={$_("settings.panels.importChatSt")} size="sm" onclick={importChat}><Upload /></IconButton>
		<IconButton label={$_("settings.panels.newFolder")} size="sm" onclick={() => startFolderEdit("new")}><FolderPlus /></IconButton>
	</div>
	<div class="convs-tools">
		<Input bind:value={query} placeholder={$_("settings.panels.searchChatsPlaceholder")} aria-label={$_("sidebar.searchChats")}>
			{#snippet leading()}<Search />{/snippet}
		</Input>
		<IconButton label={$_("settings.panels.sortStatus", { values: { by: CONV_SORT_LABEL[sortBy], dir: desc ? $_("settings.chrome.desc") : $_("settings.chrome.asc") } })} size="md" active={!!sortMenu} aria-haspopup="menu" onclick={openSortMenu}><ArrowDownUp /></IconButton>
	</div>
	<ContextMenu
		at={sortMenu}
		label={$_("settings.panels.sortAria")}
		items={[
			...CONV_SORT_KEYS.map((k) => ({ id: k, label: CONV_SORT_LABEL[k], checked: sortBy === k, onselect: () => (sortBy = k) })),
			{ id: "desc", label: $_("settings.chrome.desc"), separator: true, checked: desc, onselect: () => (desc = true) },
			{ id: "asc", label: $_("settings.chrome.asc"), checked: !desc, onselect: () => (desc = false) },
		]}
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
						<p class="empty-folder">{$_("settings.panels.emptyFolder")}</p>
					{/each}
				</div>
			{/if}
		{/each}
		<div class="loose" class:drop={dropTarget === "loose"} {...dnd(null, "loose")}>
			{#each convs.filter((c) => !c.folderId || !folders.some((f) => f.id === c.folderId)) as c (c.id)}
				<ConvCard conv={c} compact selected={c.id === convId} onopen={() => onopen(c.id)} />
			{/each}
		</div>
		{#if !convs.length}<p class="none">{query ? $_("settings.panels.noSearchResults") : $_("settings.panels.noConvs")}</p>{/if}
	</div>
	<ContextMenu at={folderMenuAt} items={folderMenuItems} label={$_("settings.panels.folderMenuAria")} onclose={() => (folderMenuAt = null)} />
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
