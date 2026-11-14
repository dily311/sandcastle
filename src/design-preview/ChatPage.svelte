<script lang="ts">
	import { ArrowLeft, Folder, FolderOpen, FolderPlus, Inbox, MessageSquarePlus, Pencil, Pin, Search, Trash2 } from "@lucide/svelte";
	import { Avatar, Button, ContextMenu, Input, Select, type MenuItem } from "$shared/ui";
	import ChatDetail from "./ChatDetail.svelte";
	import ConvCard from "./ConvCard.svelte";
	import { characters, convActions, folderActions, relative, store } from "./mock.svelte";

	// 대화: 캐릭터 → 그 캐릭터의 대화 목록(폴더) → 대화 상세
	let charId: string | null = $state(null);
	let convId: string | null = $state(null);

	let query = $state("");
	let sort: "recent" | "name" | "count" = $state("recent");
	/** all | pinned | none(폴더 없음) | 폴더 id */
	let folder = $state("all");

	const mine = $derived(store.convs.filter((c) => c.charId === charId));
	const folders = $derived(store.folders.filter((f) => f.charId === charId));
	const list = $derived.by(() => {
		const q = query.trim().toLowerCase();
		return mine
			.filter((c) =>
				folder === "all" ? true : folder === "pinned" ? c.pinned : folder === "none" ? !c.folderId : c.folderId === folder,
			)
			.filter((c) => !q || c.title.toLowerCase().includes(q))
			.sort((a, b) => Number(b.pinned) - Number(a.pinned) || (sort === "recent" ? a.ago - b.ago : sort === "name" ? a.title.localeCompare(b.title) : b.count - a.count));
	});
	const character = $derived(characters.find((c) => c.id === charId));
	const stats = (id: string) => {
		const cs = store.convs.filter((c) => c.charId === id);
		return { n: cs.length, last: Math.min(...cs.map((c) => c.ago)) };
	};

	// ---- 폴더 만들기 / 이름 바꾸기 (같은 입력칸)
	let editingFolder: string | null = $state(null); // "new" 또는 폴더 id
	let folderName = $state("");
	function startFolderEdit(id: string, name = "") {
		editingFolder = id;
		folderName = name;
	}
	function commitFolder() {
		if (editingFolder === "new" && charId) {
			const id = folderActions.create(charId, folderName);
			if (id) folder = id;
		} else if (editingFolder) {
			folderActions.rename(editingFolder, folderName);
		}
		editingFolder = null;
	}

	// ---- 폴더 오른쪽 클릭
	let menu = $state<{ x: number; y: number } | null>(null);
	let menuItems: MenuItem[] = $state([]);
	function folderMenu(e: MouseEvent, id: string, name: string) {
		e.preventDefault();
		menuItems = [
			{ label: "이름 바꾸기", icon: Pencil, onselect: () => startFolderEdit(id, name) },
			{
				label: "폴더 삭제 (대화는 남김)",
				icon: Trash2,
				danger: true,
				separator: true,
				onselect: () => {
					folderActions.remove(id);
					if (folder === id) folder = "all";
				},
			},
		];
		menu = { x: e.clientX, y: e.clientY };
	}

	// ---- 카드를 폴더로 끌어다 놓기
	let dropTarget: string | null = $state(null);
	function drop(e: DragEvent, target: string | null) {
		e.preventDefault();
		const id = e.dataTransfer?.getData("text/conv-id");
		if (id) convActions.move(id, target);
		dropTarget = null;
	}
	const dnd = (target: string | null, key: string) => ({
		ondragover: (e: DragEvent) => {
			e.preventDefault();
			dropTarget = key;
		},
		ondragleave: () => (dropTarget = null),
		ondrop: (e: DragEvent) => drop(e, target),
	});
</script>

{#if charId && convId}
	<ChatDetail {convId} {charId} onback={() => (convId = null)} onopen={(id) => (convId = id)} />
{:else if charId && character}
	<div class="list-page">
		<nav class="folders" aria-label="폴더">
			<Button variant="ghost" size="sm" onclick={() => (charId = null)}>
				{#snippet icon()}<ArrowLeft />{/snippet}
				캐릭터
			</Button>
			<div class="who">
				<Avatar name={character.name} size={40} />
				<div><b>{character.name}</b><small class="muted">{character.intro}</small></div>
			</div>

			<ul>
				<li>
					<button class:on={folder === "all"} onclick={() => (folder = "all")}><Inbox />전체 대화<span>{mine.length}</span></button>
				</li>
				<li>
					<button class:on={folder === "pinned"} onclick={() => (folder = "pinned")}><Pin />고정됨<span>{mine.filter((c) => c.pinned).length}</span></button>
				</li>
				<li>
					<button class:on={folder === "none"} class:drop={dropTarget === "none"} onclick={() => (folder = "none")} {...dnd(null, "none")}>
						<Folder />폴더 없음<span>{mine.filter((c) => !c.folderId).length}</span>
					</button>
				</li>
			</ul>

			<p class="label">폴더</p>
			<ul>
				{#each folders as f (f.id)}
					<li>
						{#if editingFolder === f.id}
							{@render folderInput()}
						{:else}
							<button
								class:on={folder === f.id}
								class:drop={dropTarget === f.id}
								onclick={() => (folder = f.id)}
								oncontextmenu={(e) => folderMenu(e, f.id, f.name)}
								{...dnd(f.id, f.id)}
							>
								{#if folder === f.id}<FolderOpen />{:else}<Folder />{/if}{f.name}<span>{mine.filter((c) => c.folderId === f.id).length}</span>
							</button>
						{/if}
					</li>
				{/each}
				<li>
					{#if editingFolder === "new"}
						{@render folderInput()}
					{:else}
						<button class="add" onclick={() => startFolderEdit("new")}><FolderPlus />새 폴더</button>
					{/if}
				</li>
			</ul>
		</nav>

		<section class="main">
			<header class="tools">
				<Input bind:value={query} placeholder="대화 검색" aria-label="대화 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<Select aria-label="정렬" bind:value={sort} options={[{ value: "recent", label: "최근 순" }, { value: "name", label: "이름 순" }, { value: "count", label: "메시지 많은 순" }]} />
				<Button variant="primary">
					{#snippet icon()}<MessageSquarePlus />{/snippet}
					새 대화
				</Button>
			</header>
			<div class="cards">
				{#each list as c (c.id)}
					<ConvCard conv={c} onopen={() => (convId = c.id)} />
				{:else}
					<p class="none">{query ? "검색 결과 없음" : folder === "all" ? "아직 대화 없음" : "이 폴더는 비어 있음 · 대화 카드를 끌어다 놓기"}</p>
				{/each}
			</div>
		</section>
	</div>
	<ContextMenu at={menu} items={menuItems} label="폴더 메뉴" onclose={() => (menu = null)} />
{:else}
	<div class="page">
		<header class="head">
			<h1>대화</h1>
			<p class="muted">누구와 이야기할지 고르기</p>
		</header>
		<div class="chars">
			{#each characters as ch (ch.id)}
				{@const s = stats(ch.id)}
				<button class="char" onclick={() => (charId = ch.id)}>
					<Avatar name={ch.name} size={64} />
					<b>{ch.name}</b>
					<span class="muted">{ch.intro}</span>
					<span class="faint">대화 {s.n}개{#if s.n} · {relative(s.last)}{/if}</span>
				</button>
			{/each}
		</div>
	</div>
{/if}

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

<style>
	.page {
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		max-width: 1100px;
		margin: 0 auto var(--space-5);
	}
	.chars {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: var(--space-3);
		max-width: 1100px;
		margin: 0 auto;
	}
	.char {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		padding: var(--space-5) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		cursor: pointer;
		text-align: center;
		transition:
			border-color var(--duration-fast) var(--ease),
			transform var(--duration-fast) var(--ease);
	}
	.char:hover {
		border-color: var(--accent);
		transform: translateY(-1px);
	}
	.char b {
		margin-top: var(--space-2);
	}
	.char span {
		font-size: var(--text-sm);
	}

	/* ---- 목록 + 폴더 ---- */
	.list-page {
		display: grid;
		grid-template-columns: 240px minmax(0, 1fr);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.folders {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		padding: var(--space-3);
		border-right: 1px solid var(--border);
		background: var(--bg);
		overflow: auto;
	}
	.folders :global(.btn) {
		align-self: flex-start;
	}
	.who {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-2) var(--space-4);
	}
	.who div {
		display: flex;
		flex-direction: column;
		min-width: 0;
	}
	.who small {
		font-size: var(--text-xs);
	}
	.folders ul {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: 1px;
	}
	.label {
		margin: var(--space-4) var(--space-2) var(--space-1);
		font-size: var(--text-xs);
		font-weight: 600;
		color: var(--text-faint);
	}
	.folders li button {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		width: 100%;
		height: 32px;
		padding: 0 var(--space-2);
		border: 1px dashed transparent;
		border-radius: var(--radius-sm);
		background: transparent;
		color: var(--text-muted);
		text-align: left;
		cursor: pointer;
	}
	.folders li button :global(svg) {
		width: 16px;
		height: 16px;
		flex: none;
	}
	.folders li button span {
		margin-left: auto;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.folders li button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.folders li button.on {
		background: var(--surface);
		color: var(--text);
		font-weight: 600;
		box-shadow: var(--shadow-sm);
	}
	.folders li button.drop {
		border-color: var(--accent);
		background: var(--accent-soft);
		color: var(--accent-text);
	}
	.folders li button.add {
		color: var(--text-faint);
	}
	.folder-input {
		width: 100%;
		height: 32px;
		padding: 0 var(--space-2);
		border: 1px solid var(--accent);
		border-radius: var(--radius-sm);
		background: var(--surface);
		outline: none;
	}
	.main {
		display: flex;
		flex-direction: column;
		min-width: 0;
		overflow: auto;
		padding: var(--space-5) var(--space-6);
	}
	.tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) 160px auto;
		gap: var(--space-2);
		margin-bottom: var(--space-4);
	}
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
		gap: var(--space-3);
		align-content: start;
	}
	.none {
		grid-column: 1 / -1;
		padding: var(--space-10);
		text-align: center;
		color: var(--text-faint);
	}
</style>
