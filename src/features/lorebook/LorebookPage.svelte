<script lang="ts">
	// 로어북 화면: 책 목록 → 책 하나(이름·설명, 쓰는 곳, 항목 편집)
	import { BookOpen, ChevronLeft, Download, Ellipsis, Pencil, Plus, Search, Trash2, Upload } from "@lucide/svelte";
	import { Avatar, Button, ContextMenu, EmptyState, IconButton, Input, Segmented, Spinner, Textarea, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { drops, extOf, storeFile } from "$app/drop.svelte";
	import { JSON_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { promptText } from "$shared/stores/confirm.svelte";
	import { relative } from "$shared/time";
	import { library } from "$features/character/library.svelte";
	import EntryItem from "./EntryItem.svelte";
	import { entryTitle, lorebook, type Book } from "./lorebook.svelte";

	let { bookId }: { bookId?: string } = $props();

	$effect(() => {
		if (!lorebook.loaded) void lorebook.load();
		if (!library.loaded) void library.load();
	});

	const open = (id?: string) => nav.go(id ? { page: "lorebook", bookId: id } : { page: "lorebook" });

	// ---- 목록 ----
	let query = $state("");
	const shown = $derived(
		lorebook.books.filter((b) => {
			const q = query.trim().toLowerCase();
			return !q || b.name.toLowerCase().includes(q) || (b.description ?? "").toLowerCase().includes(q);
		}),
	);
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

	let menu = $state<{ at: { x: number; y: number }; items: MenuItem[] } | null>(null);
	function bookMenu(e: MouseEvent, b: Book) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.left, y: r.bottom + 4 },
			items: [
				{ label: "이름 바꾸기", icon: Pencil, onselect: () => rename(b) },
				{ label: "내보내기 (SillyTavern JSON)", icon: Download, onselect: () => lorebook.export(b) },
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
	const usage = $derived(bookId ? lorebook.usage(bookId) : null);

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
		<div class="crumb">
			<IconButton label="로어북 목록으로" size="sm" onclick={() => open()}><ChevronLeft /></IconButton>
			<span>로어북</span>
		</div>
		<header class="book-head">
			<div class="info">
				<input class="name" bind:value={name} onblur={saveInfo} onkeydown={(e) => e.key === "Enter" && (e.currentTarget as HTMLInputElement).blur()} aria-label="로어북 이름" />
				<Textarea bind:value={description} rows={1} autosize maxRows={4} placeholder="설명 (선택)" aria-label="로어북 설명" onblur={saveInfo} />
			</div>
			<div class="acts">
				<Button size="sm" variant="ghost" onclick={() => lorebook.export(book)}>{#snippet icon()}<Download />{/snippet}내보내기</Button>
				<IconButton label="로어북 메뉴" size="sm" onclick={(e) => bookMenu(e, book)}><Ellipsis /></IconButton>
			</div>
		</header>

		{#if usage}
			<section class="usage">
				<h3>쓰는 곳</h3>
				{#if usage.characters.length || usage.personas.length || usage.conversations.length}
					<div class="chips">
						{#each usage.characters as c (c.id)}
							<button class="chip" onclick={() => nav.go({ page: "character", charId: c.id, edit: true })} title="캐릭터 편집에서 빼거나 넣음">
								<Avatar name={c.name} src={fileSrc(c.avatar)} color={c.color} size={20} />{c.name}<small>캐릭터</small>
							</button>
						{/each}
						{#each usage.personas as p (p.id)}
							<button class="chip" onclick={() => nav.go({ page: "persona", personaId: p.id })}>
								<Avatar name={p.name} src={fileSrc(p.avatar)} color={p.color || null} size={20} />{p.name}<small>페르소나</small>
							</button>
						{/each}
						{#if usage.conversations.length}<span class="chip static">대화 {usage.conversations.length}개에 직접 붙음</span>{/if}
					</div>
				{:else}
					<p class="faint small">아직 아무 데도 안 씀. 캐릭터·페르소나 편집에서 넣거나, 대화 패널의 로어북 탭에서 대화에 바로 붙일 수 있음</p>
				{/if}
			</section>
		{/if}

		<div class="tools">
			<Input bind:value={entryQuery} placeholder="제목·키워드·내용 검색" aria-label="항목 검색" type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<Segmented
				label="항목 거르기"
				bind:value={filter}
				options={[
					{ value: "all", label: "전체" },
					{ value: "on", label: "켜짐" },
					{ value: "off", label: "꺼짐" },
					{ value: "constant", label: "상시" },
				]}
			/>
			<Button variant="primary" onclick={addEntry}>{#snippet icon()}<Plus />{/snippet}항목 추가</Button>
		</div>
		<p class="count faint small">
			항목 {lorebook.counts[book.id]?.entries ?? 0}개 · 켜짐 {lorebook.counts[book.id]?.enabled ?? 0}개 · 우선순위 높은 것부터 (넣을 자리가 모자라면 높은 것부터 들어감)
		</p>

		{#if !entries}
			<div class="center"><Spinner /></div>
		{:else if shownEntries.length}
			<ul class="entries">
				{#each shownEntries as e (e.id)}
					<EntryItem
						entry={e}
						open={openId === e.id}
						ontoggle={() => (openId = openId === e.id ? null : e.id)}
						onsaved={(o, n) => openId === o && (openId = n)}
					/>
				{/each}
			</ul>
		{:else}
			<EmptyState title={entries.length ? "맞는 항목 없음" : "항목 없음"} description={entries.length ? "검색어나 거르기를 바꿔 봄" : "'항목 추가'로 장소·인물·규칙을 하나씩 적음"} />
		{/if}
	</div>
{:else}
	<div class="page">
		<header class="head">
			<h1>로어북</h1>
			<div class="acts">
				<Button onclick={importFiles}>{#snippet icon()}<Upload />{/snippet}가져오기</Button>
				<Button variant="primary" onclick={create}>{#snippet icon()}<Plus />{/snippet}새 로어북</Button>
			</div>
		</header>
		<p class="lead">대화에 키워드가 나오면 그 설정을 AI에게 함께 보냄. 캐릭터·페르소나·대화에 붙여서 씀</p>
		{#if lorebook.books.length}
			<Input bind:value={query} placeholder="로어북 검색" aria-label="로어북 검색" type="search">
				{#snippet leading()}<Search />{/snippet}
			</Input>
			<div class="grid">
				{#each shown as b (b.id)}
					{@const u = lorebook.usage(b.id)}
					{@const n = lorebook.counts[b.id]}
					<div class="card" role="button" tabindex="0" onclick={() => open(b.id)} onkeydown={(e) => e.key === "Enter" && open(b.id)} oncontextmenu={(e) => bookMenu(e, b)}>
						<div class="card-top">
							<span class="ic"><BookOpen /></span>
							<b>{b.name}</b>
							<IconButton label="{b.name} 메뉴" size="sm" onclick={(e) => bookMenu(e, b)}><Ellipsis /></IconButton>
						</div>
						<p class="desc">{b.description || "설명 없음"}</p>
						<div class="meta">
							<span>항목 {n?.entries ?? 0}개{n && n.enabled !== n.entries ? ` (켜짐 ${n.enabled})` : ""}</span>
							<span>{relative(b.updatedAt)}</span>
						</div>
						{#if u.characters.length || u.personas.length}
							<div class="who">
								{#each [...u.characters, ...u.personas].slice(0, 6) as w (w.id)}
									<Avatar name={w.name} src={fileSrc(w.avatar)} color={w.color || null} size={22} />
								{/each}
								<small>{[...u.characters, ...u.personas].map((w) => w.name).join(", ")}</small>
							</div>
						{/if}
					</div>
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
		max-width: 920px;
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
	}
	h1 {
		font-size: var(--text-xl);
	}
	.lead {
		margin: calc(var(--space-3) * -1) 0 0;
		color: var(--text-muted);
		font-size: var(--text-sm);
	}
	.acts {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		flex: none;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(260px, 1fr));
		gap: var(--space-3);
	}
	.card {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		padding: var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		cursor: pointer;
	}
	.card:hover {
		border-color: var(--border-strong);
	}
	.card:focus-visible {
		outline: 2px solid var(--accent);
	}
	.card-top {
		display: flex;
		align-items: center;
		gap: var(--space-2);
	}
	.card-top b {
		flex: 1;
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ic {
		display: grid;
		place-items: center;
		width: 30px;
		height: 30px;
		flex: none;
		border-radius: var(--radius-md);
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.ic :global(svg) {
		width: 16px;
		height: 16px;
	}
	.desc {
		margin: 0;
		font-size: var(--text-sm);
		color: var(--text-muted);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.meta {
		display: flex;
		justify-content: space-between;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.who {
		display: flex;
		align-items: center;
		gap: 4px;
		min-width: 0;
	}
	.who small {
		margin-left: 4px;
		font-size: var(--text-xs);
		color: var(--text-muted);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.crumb {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		margin: calc(var(--space-2) * -1) 0 calc(var(--space-2) * -1) calc(var(--space-2) * -1);
		font-size: var(--text-sm);
		color: var(--text-muted);
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
	.usage h3 {
		margin: 0 0 var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-muted);
		letter-spacing: 0.04em;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
	}
	.chip {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		padding: 3px 10px 3px 4px;
		border: 1px solid var(--border);
		border-radius: var(--radius-full);
		background: var(--surface);
		color: var(--text);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.chip:hover {
		border-color: var(--border-strong);
	}
	.chip small {
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.chip.static {
		padding: 3px 10px;
		cursor: default;
		color: var(--text-muted);
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
