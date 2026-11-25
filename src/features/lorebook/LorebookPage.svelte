<script lang="ts">
	import { jobs } from "$shared/stores/jobs.svelte";
	import { _ } from "svelte-i18n";
	import { onMount } from "svelte";
	// 로어북 화면: 책 목록 → 책 하나(이름·설명, 쓰는 곳, 항목 편집)
	import { BookOpen, Check, ChevronRight, Download, Ellipsis, Funnel, Pencil, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, EmptyState, ExportSplit, IconButton, Input, ListToolbar, SelectBar, SplitButton, Spinner, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { JSON_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import EntryItem from "./EntryItem.svelte";
	import { Sortable } from "$shared/dnd/sortable.svelte";
	import { BOOK_FORMATS, entryTitle, lorebook, type Book, type BookFormat } from "./lorebook.svelte";

	let { bookId }: { bookId?: string } = $props();

	// 화면을 열 때마다 새로 (캐릭터 카드를 가져오면 로어북이 새로 생기므로)
	onMount(() => {
		void lorebook.load();
		if (!library.loaded) void library.load();
	});

	const open = (id?: string) => nav.go(id ? { page: "lorebook", bookId: id } : { page: "lorebook" });

	// ---- 목록 ----
	// ---- 찾기 · 거르기 · 정렬 · 여러 개 삭제 (다른 목록 화면과 같은 도구) ----
	let query = $state("");
	let bookFilter = $state("all");
	let bookSort = $state("name");
	let selecting = $state(false);
	let picked = $state<Set<string>>(new Set());
	$effect(() => {
		if (!selecting) picked = new Set();
	});
	const used = (id: string) => {
		const u = lorebook.usage(id);
		return u.characters.length + u.personas.length + u.conversations.length > 0;
	};
	const BOOK_FILTERS = $derived([
		{ value: "all", label: $_("lorebook.filterAll", { values: { count: lorebook.books.length } }) },
		{ value: "used", label: $_("lorebook.filterUsed") },
		{ value: "unused", label: $_("lorebook.filterUnused") },
	]);
	const BOOK_SORTS = $derived([
		{ value: "name", label: $_("lorebook.sortName") },
		{ value: "recent", label: $_("lorebook.sortRecent") },
		{ value: "size", label: $_("lorebook.sortSize") },
	]);
	const shown = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const by =
			bookSort === "recent"
				? (a: (typeof lorebook.books)[number], b: (typeof lorebook.books)[number]) => b.updatedAt - a.updatedAt
				: bookSort === "size"
					? (a: (typeof lorebook.books)[number], b: (typeof lorebook.books)[number]) => (lorebook.counts[b.id]?.entries ?? 0) - (lorebook.counts[a.id]?.entries ?? 0)
					: (a: (typeof lorebook.books)[number], b: (typeof lorebook.books)[number]) => a.name.localeCompare(b.name, "ko");
		return lorebook.books
			.filter((b) => (bookFilter === "all" || (bookFilter === "used") === used(b.id)) && (!q || b.name.toLowerCase().includes(q) || (b.description ?? "").toLowerCase().includes(q)))
			.sort(by);
	});
	function pickBook(id: string) {
		const next = new Set(picked);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		picked = next;
	}
	async function removePicked() {
		const ids = [...picked];
		if (!ids.length) return;
		const n = ids.reduce((s, id) => s + (lorebook.counts[id]?.entries ?? 0), 0);
		if (!(await confirmAction({ title: $_("lorebook.deleteTitle"), message: $_("lorebook.deleteManyMsg", { values: { books: ids.length, entries: n } }), confirmLabel: $_("lorebook.deleteCountBtn", { values: { count: ids.length } }), danger: true }))) return;
		selecting = false;
		// 파일 작업 대기열로 하나씩 (5-E-28). 요약 알림은 대기열이 한 번
		await Promise.allSettled(
			ids.map((id) => jobs.run(lorebook.book(id)?.name ?? id, () => call(commands.deleteWorldinfoBook(id), { source: "lorebook" }), { group: $_("lorebook.deleteJobGroup") })),
		);
		await lorebook.load();
		await library.load();
	}
	async function create() {
		const name = await promptText({ title: $_("lorebook.newTitle"), message: $_("lorebook.newMsg"), input: { value: "", placeholder: $_("lorebook.newPlaceholder") }, confirmLabel: $_("lorebook.createBtn") });
		if (!name?.trim()) return;
		const b = await lorebook.create(name.trim());
		open(b.id);
	}
	async function importFiles() {
		for (const p of await pickFiles([JSON_FILES])) await lorebook.importPath(p).catch(() => null);
	}
	$effect(() =>
		drops.register({
			label: $_("lorebook.dropLabel"),
			hint: $_("lorebook.dropHint"),
			accept: (f) => extOf(f) === "json",
			ondrop: async (files) => {
				let last: Book | null = null;
				for (const f of files) last = (await lorebook.importPath(await storeFile(f)).catch(() => null)) ?? last;
				if (last && files.length === 1) open(last.id);
			},
		}),
	);

	let menu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	function bookMenu(e: MouseEvent, b: Book) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{ label: $_("lorebook.renameMenu"), icon: Pencil, onselect: () => rename(b) },
				{ label: $_("lorebook.exportMenu"), icon: Download, onselect: () => lorebook.exportMenu(b) },
				{
					label: $_("common.delete"),
					icon: Trash2,
					danger: true,
					separator: true,
					onselect: async () => {
						if ((await lorebook.remove(b)) && bookId === b.id) nav.replace({ page: "lorebook" });
					},
				},
			],
		};
	}
	async function rename(b: Book) {
		const name = await promptText({ title: $_("lorebook.renameMenu"), input: { value: b.name }, confirmLabel: $_("lorebook.renameConfirm") });
		if (name?.trim() && name.trim() !== b.name) await lorebook.update(b.id, { name: name.trim(), description: b.description });
	}

	// ---- 책 하나 ----
	const book = $derived(lorebook.book(bookId));
	let name = $state("");
	let description = $state("");
	$effect(() => {
		name = book?.name ?? "";
		description = book?.description ?? "";
	});
	function saveInfo() {
		if (!book || !name.trim()) return void (name = book?.name ?? "");
		if (name.trim() === book.name && (description.trim() || null) === (book.description || null)) return;
		void lorebook.update(book.id, { name: name.trim(), description: description.trim() || null });
	}
	$effect(() => {
		if (bookId) void lorebook.loadEntries(bookId);
	});
	const entries = $derived(bookId ? (lorebook.entries[bookId] ?? null) : null);

	let filter = $state<"all" | "on" | "off" | "constant">("all");
	/** 분류로 거르기 (5-F-1). null = 전부 */
	let categoryFilter = $state<string | null>(null);
	let entryQuery = $state("");
	const shownEntries = $derived(
		(entries ?? []).filter((e) => {
			if (filter === "on" && !e.enabled) return false;
			if (filter === "off" && e.enabled) return false;
			if (filter === "constant" && !e.constant) return false;
			if (categoryFilter !== null && e.category !== categoryFilter) return false;
			const q = entryQuery.trim().toLowerCase();
			return !q || entryTitle(e).toLowerCase().includes(q) || e.keys.some((k) => k.toLowerCase().includes(q)) || e.content.toLowerCase().includes(q);
		}),
	);
	const FILTERS = $derived([
		{ id: "all", label: $_("lorebook.entryFilterAll") },
		{ id: "on", label: $_("lorebook.entryFilterOn") },
		{ id: "off", label: $_("lorebook.entryFilterOff") },
		{ id: "constant", label: $_("lorebook.entryFilterConstant") },
	] as const);
	let filterMenu = $state<{ x: number; y: number } | null>(null);
	const usedCategories = $derived([...new Set((entries ?? []).map((e) => e.category).filter(Boolean))].sort((a, b) => a.localeCompare(b, "ko")));
	const filterItems = $derived<MenuItem[]>([
		...FILTERS.map((f) => ({ label: f.label, icon: filter === f.id && categoryFilter === null ? Check : undefined, onselect: () => ((filter = f.id), (categoryFilter = null)) })),
		...usedCategories.map((c, i) => ({ label: $_("lorebook.categoryFilter", { values: { cat: c } }), separator: i === 0, icon: categoryFilter === c ? Check : undefined, onselect: () => (categoryFilter = c) })),
	]);
	function openFilter(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		filterMenu = { x: r.left, y: r.bottom + 4 };
	}

	// 끌어서 순서 바꾸기 (검색·필터가 없을 때만: 보이는 순서 = 전체 순서)
	const sort = new Sortable((from, to) => bookId && lorebook.reorder(bookId, from, to));
	$effect(() => {
		sort.enabled = filter === "all" && categoryFilter === null && !entryQuery.trim() && openId === null;
	});

	let openId = $state<string | null>(null);
	function addEntry() {
		if (!bookId) return;
		filter = "all";
		categoryFilter = null;
		entryQuery = "";
		openId = lorebook.addDraft(bookId).id;
	}
</script>

{#if !lorebook.loaded}
	<div class="center"><Spinner /></div>
{:else if bookId && !book}
	<div class="center">
		<EmptyState title={$_("lorebook.notFoundTitle")} description={$_("lorebook.notFoundDesc")}>
			{#snippet action()}<Button onclick={() => open()}>{$_("lorebook.toList")}</Button>{/snippet}
		</EmptyState>
	</div>
{:else if book}
	<div class="page">
		<header class="book-head">
			<div class="info">
				<input class="name" bind:value={name} onblur={saveInfo} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} aria-label={$_("lorebook.bookNameAria")} />
				<input class="desc-input" bind:value={description} onblur={saveInfo} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} placeholder={$_("lorebook.bookDescPlaceholder")} aria-label={$_("lorebook.bookDescAria")} />
			</div>
			<div class="acts">
				<ExportSplit formats={BOOK_FORMATS} storageKey="sc.export.lorebook" size="sm" variant="secondary" label={$_("lorebook.exportBtn")} onexport={(f) => lorebook.export(book, f as BookFormat)} />
				<IconButton label={$_("lorebook.bookMenuAria")} size="sm" onclick={(e) => bookMenu(e, book)}><Ellipsis /></IconButton>
			</div>
		</header>

		<div class="tools">
			<Input bind:value={entryQuery} placeholder={$_("lorebook.entrySearchPlaceholder")} aria-label={$_("lorebook.entrySearchAria")} type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<span class="filter" class:on={filter !== "all"}>
				<IconButton label={$_("lorebook.filterAria", { values: { label: FILTERS.find((f) => f.id === filter)?.label ?? "" } })} active={filter !== "all"} aria-haspopup="menu" onclick={openFilter}><Funnel /></IconButton>
				{#if filter !== "all"}<button class="filter-chip" title={$_("lorebook.clearFilterTitle")} onclick={() => (filter = "all")}>{FILTERS.find((f) => f.id === filter)?.label} ×</button>{/if}
				{#if categoryFilter !== null}<button class="filter-chip" title={$_("lorebook.clearCatFilterTitle")} onclick={() => (categoryFilter = null)}>{categoryFilter} ×</button>{/if}
			</span>
			<ContextMenu at={filterMenu} label={$_("lorebook.entryFilterMenuAria")} items={filterItems} onclose={() => (filterMenu = null)} />
			<Button variant="primary" onclick={addEntry}>{#snippet icon()}<Plus />{/snippet}{$_("lorebook.addEntryBtn")}</Button>
		</div>
		<p class="count faint small">
			{$_("lorebook.entryStats", { values: { entries: lorebook.counts[book.id]?.entries ?? 0, enabled: lorebook.counts[book.id]?.enabled ?? 0 } })}{sort.enabled ? $_("lorebook.entryDragHint") : ""}
		</p>

		{#if !entries}
			<div class="center"><Spinner /></div>
		{:else if shownEntries.length}
			<ul class="entries">
				{#each shownEntries as e, i (e.id)}
					<EntryItem
						drag={sort.item(i)}
						grip={sort.handle(i)}
						dragging={sort.from === i}
						dragOver={sort.over === i && sort.from !== i}
						entry={e}
						open={openId === e.id}
						ontoggle={() => (openId = openId === e.id ? null : e.id)}
						onsaved={(o, n) => openId === o && (openId = n)}
					/>
				{/each}
			</ul>
		{:else}
			<EmptyState title={entries.length ? $_("lorebook.noMatchEntryTitle") : $_("lorebook.noEntryTitle")} description={entries.length ? $_("lorebook.noMatchEntryDesc") : $_("lorebook.noEntryDesc")} />
		{/if}
	</div>
{:else}
	<div class="page">
		<header class="head">
			<div class="title-wrap">
				<h1>{$_("nav.lorebook")}</h1>
				<span class="head-count">{lorebook.books.length}{$_("common.unitItem")}</span>
			</div>
			<ListToolbar bind:query bind:filter={bookFilter} bind:sort={bookSort} bind:selecting placeholder={$_("lorebook.searchPlaceholder")} filters={BOOK_FILTERS} sorts={BOOK_SORTS} canSelect={!!lorebook.books.length}>
				<SplitButton label={$_("common.create")} onclick={create} menuLabel={$_("common.import")} items={[{ label: $_("lorebook.importJson"), icon: Upload, onselect: importFiles }]} />
			</ListToolbar>
		</header>
		{#if selecting}<SelectBar count={picked.size} onall={() => (picked = new Set(shown.map((b) => b.id)))} ondelete={removePicked} />{/if}
		{#if lorebook.books.length}
			<ul class="book-list">
				{#each shown as b (b.id)}
					{@const u = lorebook.usage(b.id)}
					{@const n = lorebook.counts[b.id]}
					{@const users = [...u.characters, ...u.personas]}
					{@const selected = picked.has(b.id)}
					<li>
						<div
							class="book-row"
							class:selected
							role="button"
							tabindex="0"
							aria-pressed={selecting ? selected : undefined}
							aria-label={b.name}
							onclick={() => (selecting ? pickBook(b.id) : open(b.id))}
							onkeydown={(e) => {
								if (e.target !== e.currentTarget) return;
								if (e.key === "Enter" || e.key === " ") {
									e.preventDefault();
									if (selecting) pickBook(b.id);
									else open(b.id);
								}
							}}
							oncontextmenu={(e) => bookMenu(e, b)}
						>
							{#if selecting}
								<span class="book-check" aria-hidden="true">{selected ? "✓" : ""}</span>
							{:else}
								<span class="book-icon" aria-hidden="true"><BookOpen /></span>
							{/if}
							<div class="book-main">
								<b class="book-title">{b.name}</b>
								{#if b.description}<p class="book-desc">{b.description}</p>{/if}
								{#if users.length}
									<div class="book-badges">
										{#each users.slice(0, 5) as w (w.id)}
											<Badge><Avatar name={w.name} src={fileSrc(w.avatar)} fit={w.avatarFit} color={w.color || null} size={14} />{w.name}</Badge>
										{/each}
										{#if users.length > 5}<Badge>+{users.length - 5}</Badge>{/if}
									</div>
								{/if}
							</div>
							<span class="book-meta">{$_("lorebook.bookMeta", { values: { entries: n?.entries ?? 0 } })}{n && n.enabled !== n.entries ? $_("lorebook.bookMetaEnabled", { values: { enabled: n.enabled } }) : ""} · {relative(b.updatedAt)}</span>
							{#if !selecting}
								<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
								<span class="book-actions" onclick={(e) => e.stopPropagation()}>
									<IconButton label={$_("lorebook.itemMenuAria", { values: { name: b.name } })} size="sm" variant="ghost" onclick={(e) => bookMenu(e, b)}><Ellipsis /></IconButton>
									<span class="book-chev" aria-hidden="true"><ChevronRight /></span>
								</span>
							{/if}
						</div>
					</li>
				{:else}
					<li class="faint empty-row">{$_("lorebook.noMatchBook")}</li>
				{/each}
			</ul>
		{:else}
			<EmptyState title={$_("lorebook.emptyTitle")} description={$_("lorebook.emptyDesc")}>
				{#snippet icon()}<BookOpen />{/snippet}
			</EmptyState>
		{/if}
	</div>
{/if}

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label={$_("lorebook.bookMenuAria")} onclose={() => (menu = null)} />
{/if}

<style>
	.filter {
		display: flex;
		align-items: center;
		gap: 4px;
	}
	.filter-chip {
		padding: 2px 8px;
		border: 1px solid var(--accent);
		border-radius: var(--radius-full);
		background: var(--accent-soft);
		color: var(--accent-text);
		font: inherit;
		font-size: var(--text-xs);
		cursor: pointer;
	}
	.center {
		display: grid;
		place-items: center;
		height: 100%;
		padding: var(--space-8);
	}
	.page {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: var(--page-wide);
		height: 100%;
		margin: 0 auto;
		padding: var(--space-6) var(--space-8) var(--space-8);
		overflow: auto;
	}
	.head,
	.book-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		/* 좁은 화면: 도구 줄이 제목 아래로 내려감 */
		flex-wrap: wrap;
	}
	.title-wrap {
		display: flex;
		align-items: baseline;
		gap: var(--space-2);
	}
	.head h1 {
		white-space: nowrap;
	}
	.head-count {
		color: var(--text-faint);
		font-size: var(--text-sm);
		font-variant-numeric: tabular-nums;
	}
	.desc-input {
		width: 100%;
		padding: 2px 6px;
		margin-left: -6px;
		border: 1px solid transparent;
		border-radius: var(--radius-xs);
		background: transparent;
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
	}
	.desc-input:hover {
		border-color: var(--border);
	}
	.desc-input:focus {
		outline: none;
		border-color: var(--accent);
		background: var(--surface);
		color: var(--text);
	}
	.acts {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: none;
	}
	.book-list {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: auto;
	}
	.book-list li + li {
		border-top: 1px solid var(--border);
	}
	.book-row {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		cursor: pointer;
		transition: background var(--duration-fast) var(--ease);
	}
	.book-row:hover {
		background: var(--surface-hover);
	}
	.book-row:focus-visible {
		outline: 2px solid var(--focus-ring);
		outline-offset: -2px;
	}
	.book-row.selected {
		background: var(--accent-soft);
	}
	.book-icon {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		flex: none;
		border-radius: var(--radius-md);
		background: var(--surface-2);
		color: var(--text-muted);
	}
	.book-icon :global(svg) {
		width: 18px;
		height: 18px;
	}
	.book-check {
		display: grid;
		place-items: center;
		width: 22px;
		height: 22px;
		flex: none;
		border: 2px solid var(--border-strong);
		border-radius: 6px;
		background: var(--surface);
		color: #fff;
		font-size: 13px;
		font-weight: 700;
	}
	.book-row.selected .book-check {
		border-color: var(--accent);
		background: var(--accent);
	}
	.book-main {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 4px;
	}
	.book-title-row {
		display: flex;
		align-items: baseline;
		justify-content: space-between;
		gap: var(--space-3);
		flex-wrap: wrap;
	}
	.book-title {
		font-size: var(--text-md);
		font-weight: 600;
		color: var(--text);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.book-meta {
		font-size: var(--text-xs);
		color: var(--text-faint);
		white-space: nowrap;
	}
	.book-desc {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.book-badges {
		display: flex;
		flex-wrap: wrap;
		gap: 4px;
		margin-top: 2px;
	}
	.book-actions {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		flex: none;
	}
	.book-chev {
		display: grid;
		place-items: center;
		color: var(--text-faint);
	}
	.book-chev :global(svg) {
		width: 16px;
		height: 16px;
	}
	.empty-row {
		padding: var(--space-6);
		text-align: center;
	}
	.info {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		flex: 1;
		min-width: 0;
	}
	.name {
		width: 100%;
		padding: 2px 6px;
		margin-left: -6px;
		border: 1px solid transparent;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text);
		font: inherit;
		font-size: var(--text-xl);
		font-weight: 700;
	}
	.name:hover {
		border-color: var(--border);
	}
	.name:focus {
		outline: none;
		border-color: var(--accent);
		background: var(--surface);
	}
	.tools {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-2);
	}
	.tools > :global(:first-child) {
		flex: 1;
		min-width: 200px;
	}
	.count {
		margin: calc(var(--space-2) * -1) 0 0;
	}
	.entries {
		margin: 0;
		padding: 0;
	}
	.small {
		font-size: var(--text-xs);
	}
	/* 휴대폰 폭: 좌우 여백을 홈과 같게 줄여 도구 줄(찾기·거르기·정렬·선택 + 새로 만들기)이 한 줄에 들어가게 */
	@media (max-width: 600px) {
		.page {
			padding: var(--space-4);
		}
	}
</style>
