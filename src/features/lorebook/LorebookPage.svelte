<script lang="ts">
	import { onMount } from "svelte";
	// 로어북 화면: 책 목록 → 책 하나(이름·설명, 쓰는 곳, 항목 편집)
	import { BookOpen, Check, Download, Ellipsis, Funnel, ListFilter, Pencil, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { Avatar, Badge, Button, ContextMenu, EmptyState, ExportSplit, IconButton, Input, ItemCard, ListToolbar, SelectBar, SplitButton, Spinner, type MenuItem } from "$shared/ui";
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
		{ value: "all", label: `전부 ${lorebook.books.length}` },
		{ value: "used", label: "쓰이는 것" },
		{ value: "unused", label: "안 쓰이는 것" },
	]);
	const BOOK_SORTS = [
		{ value: "name", label: "이름 순" },
		{ value: "recent", label: "최근 바뀐 순" },
		{ value: "size", label: "항목 많은 순" },
	];
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
		if (!(await confirmAction({ title: "로어북 삭제", message: `로어북 ${ids.length}개와 항목 ${n}개를 휴지통으로 옮김. 쓰던 캐릭터·페르소나·대화에서도 빠짐. 설정 > 데이터 > 휴지통에서 되살릴 수 있음`, confirmLabel: `${ids.length}개 삭제`, danger: true }))) return;
		for (const id of ids) await call(commands.deleteWorldinfoBook(id), { source: "lorebook" });
		await lorebook.load();
		await library.load();
		selecting = false;
		toasts.success(`로어북 ${ids.length}개를 휴지통으로 옮김`);
	}
	async function create() {
		const name = await promptText({ title: "새 로어북", message: "세계관·장소·인물 설정을 모아 두는 책", input: { value: "", placeholder: "예: 엘다리아 대륙" }, confirmLabel: "만들기" });
		if (!name?.trim()) return;
		const b = await lorebook.create(name.trim());
		open(b.id);
	}
	async function importFiles() {
		for (const p of await pickFiles([JSON_FILES])) await lorebook.importPath(p).catch(() => null);
	}
	$effect(() =>
		drops.register({
			label: "로어북 가져오기",
			hint: "SillyTavern 월드인포 JSON",
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
				{ label: "이름 바꾸기", icon: Pencil, onselect: () => rename(b) },
				{ label: "내보내기…", icon: Download, onselect: () => lorebook.exportMenu(b) },
				{
					label: "삭제",
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
		const name = await promptText({ title: "이름 바꾸기", input: { value: b.name }, confirmLabel: "바꾸기" });
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
	let entryQuery = $state("");
	const shownEntries = $derived(
		(entries ?? []).filter((e) => {
			if (filter === "on" && !e.enabled) return false;
			if (filter === "off" && e.enabled) return false;
			if (filter === "constant" && !e.constant) return false;
			const q = entryQuery.trim().toLowerCase();
			return !q || entryTitle(e).toLowerCase().includes(q) || e.keys.some((k) => k.toLowerCase().includes(q)) || e.content.toLowerCase().includes(q);
		}),
	);
	const FILTERS = [
		{ id: "all", label: "전체" },
		{ id: "on", label: "켜진 것만" },
		{ id: "off", label: "꺼진 것만" },
		{ id: "constant", label: "항상 넣는 것만" },
	] as const;
	let filterMenu = $state<{ x: number; y: number } | null>(null);
	const filterItems = $derived<MenuItem[]>(FILTERS.map((f) => ({ label: f.label, icon: filter === f.id ? Check : undefined, onselect: () => (filter = f.id) })));
	function openFilter(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		filterMenu = { x: r.left, y: r.bottom + 4 };
	}

	// 끌어서 순서 바꾸기 (검색·필터가 없을 때만: 보이는 순서 = 전체 순서)
	const sort = new Sortable((from, to) => bookId && lorebook.reorder(bookId, from, to));
	$effect(() => {
		sort.enabled = filter === "all" && !entryQuery.trim() && openId === null;
	});

	let openId = $state<string | null>(null);
	function addEntry() {
		if (!bookId) return;
		filter = "all";
		entryQuery = "";
		openId = lorebook.addDraft(bookId).id;
	}
</script>

{#if !lorebook.loaded}
	<div class="center"><Spinner /></div>
{:else if bookId && !book}
	<div class="center">
		<EmptyState title="로어북을 찾을 수 없음" description="지워졌을 수 있음">
			{#snippet action()}<Button onclick={() => open()}>목록으로</Button>{/snippet}
		</EmptyState>
	</div>
{:else if book}
	<div class="page">
		<header class="book-head">
			<div class="info">
				<input class="name" bind:value={name} onblur={saveInfo} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} aria-label="로어북 이름" />
			</div>
			<div class="acts">
				<ExportSplit formats={BOOK_FORMATS} storageKey="sc.export.lorebook" size="sm" variant="secondary" label="내보내기" onexport={(f) => lorebook.export(book, f as BookFormat)} />
				<IconButton label="로어북 메뉴" size="sm" onclick={(e) => bookMenu(e, book)}><Ellipsis /></IconButton>
			</div>
		</header>

		<div class="tools">
			<Input bind:value={entryQuery} placeholder="제목·키워드·내용 검색" aria-label="항목 검색" type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<span class="filter" class:on={filter !== "all"}>
				<IconButton label="필터: {FILTERS.find((f) => f.id === filter)?.label}" active={filter !== "all"} aria-haspopup="menu" onclick={openFilter}><Funnel /></IconButton>
				{#if filter !== "all"}<button class="filter-chip" title="필터 풀기" onclick={() => (filter = "all")}>{FILTERS.find((f) => f.id === filter)?.label} ×</button>{/if}
			</span>
			<ContextMenu at={filterMenu} label="항목 필터링" items={filterItems} onclose={() => (filterMenu = null)} />
			<Button variant="primary" onclick={addEntry}>{#snippet icon()}<Plus />{/snippet}항목 추가</Button>
		</div>
		<p class="count faint small">
			항목 {lorebook.counts[book.id]?.entries ?? 0}개 · 켜짐 {lorebook.counts[book.id]?.enabled ?? 0}개 · 우선순위 높은 것부터 (넣을 자리가 모자라면 높은 것부터 들어감){sort.enabled ? " · 끌어서 순서(우선순위) 바꾸기" : ""}
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
			<EmptyState title={entries.length ? "맞는 항목 없음" : "항목 없음"} description={entries.length ? "검색어나 필터를 변경하세요." : "'항목 추가'로 장소·인물·규칙을 하나씩 적음"} />
		{/if}
	</div>
{:else}
	<div class="page">
		<header class="head">
			<h1>로어북</h1>
			<ListToolbar bind:query bind:filter={bookFilter} bind:sort={bookSort} bind:selecting placeholder="이름·설명" filters={BOOK_FILTERS} sorts={BOOK_SORTS} canSelect={!!lorebook.books.length}>
				<SplitButton label="새로 만들기" onclick={create} menuLabel="로어북 가져오기" items={[{ label: "파일에서 가져오기 (JSON)", icon: Upload, onselect: importFiles }]} />
			</ListToolbar>
		</header>
		{#if selecting}<SelectBar count={picked.size} onall={() => (picked = new Set(shown.map((b) => b.id)))} ondelete={removePicked} />{/if}
		{#if lorebook.books.length}
			<div class="grid">
				{#each shown as b (b.id)}
					{@const u = lorebook.usage(b.id)}
					{@const n = lorebook.counts[b.id]}
					<ItemCard
						title={b.name}
						desc={b.description || "설명 없음"}
						meta="항목 {n?.entries ?? 0}개{n && n.enabled !== n.entries ? ` (켜짐 ${n.enabled})` : ''} · {relative(b.updatedAt)}"
						{selecting}
						selected={picked.has(b.id)}
						onclick={() => (selecting ? pickBook(b.id) : open(b.id))}
						oncontextmenu={(e) => bookMenu(e, b)}
					>
						{#snippet fallback()}<BookOpen />{/snippet}
						{#snippet menu()}<IconButton label="{b.name} 메뉴" size="sm" variant="ghost" onclick={(e) => bookMenu(e, b)}><Ellipsis /></IconButton>{/snippet}
						{#snippet badges()}
							{#each [...u.characters, ...u.personas].slice(0, 4) as w (w.id)}<Badge><Avatar name={w.name} src={fileSrc(w.avatar)} color={w.color || null} size={14} />{w.name}</Badge>{/each}
						{/snippet}
					</ItemCard>
				{:else}
					<p class="faint">맞는 로어북 없음</p>
				{/each}
			</div>
		{:else}
			<EmptyState title="로어북 없음" description="새로 만들거나 SillyTavern 월드인포(.json)를 가져옴. 파일을 끌어다 놓아도 됨">
				{#snippet icon()}<BookOpen />{/snippet}
			</EmptyState>
		{/if}
	</div>
{/if}

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label="로어북 메뉴" onclose={() => (menu = null)} />
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
		padding: var(--space-6) var(--space-6) var(--space-8);
		overflow: auto;
	}
	.head,
	.book-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-4);
		/* 좁은 화면: 도구 줄이 제목 아래로 내려감 */
		flex-wrap: wrap;
	}
	h1 {
		font-size: var(--text-xl);
		white-space: nowrap;
	}
	.acts {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: none;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-3);
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
</style>
