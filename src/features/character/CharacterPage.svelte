<script lang="ts">
	import { ChevronRight, Download, Ellipsis, MessageSquarePlus, Pencil, Plus, Search, Star, StarOff, Trash2, Upload, Users } from "@lucide/svelte";
	import { Avatar, Button, ContextMenu, EmptyState, IconButton, Input, Segmented, Spinner, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { PACK_FILES, ST_FILES, pickFiles, pickSavePath } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import ChatView from "./ChatView.svelte";
	import { library, type Character } from "./library.svelte";

	// 캐릭터 목록 → 캐릭터 상세 → 대화 상세 (대화 목록·폴더는 대화 상세의 왼쪽 사이드바)
	let { charId, convId }: { charId?: string; convId?: string } = $props();

	$effect(() => {
		if (!library.loaded) void library.load();
	});

	function open(cid: string, conv?: string) {
		nav.go(conv ? { page: "character", charId: cid, convId: conv } : { page: "character", charId: cid });
	}

	// ---- 목록 ----
	let query = $state("");
	const lastActive = (id: string) => library.convsOf(id)[0]?.updatedAt ?? 0;
	const list = $derived(
		library.characters
			.filter((c) => !query.trim() || c.name.toLowerCase().includes(query.trim().toLowerCase()) || c.tags.some((t) => t.toLowerCase().includes(query.trim().toLowerCase())))
			.sort((a, b) => Number(b.favorite) - Number(a.favorite) || lastActive(b.id) - lastActive(a.id) || a.name.localeCompare(b.name, "ko")),
	);
	const firstLine = (s: string) => s.split("\n").find((l) => l.trim())?.trim() ?? "";

	async function importFiles() {
		const paths = await pickFiles([ST_FILES, PACK_FILES]);
		for (const p of paths) {
			if (p.toLowerCase().endsWith(".zip")) {
				const r = await call(commands.importCharacterPack(p), { source: "character" }).catch(() => null);
				if (r) toasts.success(`캐릭터 ${r.characters.length}명 · 대화 ${r.conversationCount}개 가져옴`);
			} else await library.importSillyTavern(p).catch(() => {});
		}
		if (paths.length) await library.load();
	}
	async function createCharacter() {
		const name = await promptText({ title: "새 캐릭터", message: "자세한 설정은 나중에 편집에서", input: { value: "", placeholder: "이름" }, confirmLabel: "만들기" });
		if (!name) return;
		const c = await call(
			commands.createCharacter({ name, description: "", alternateDescriptions: [], alternateFirstMessage: [], worldInfoIds: [], tags: [], favorite: false }),
			{ source: "character" },
		);
		library.characters.push(c);
		open(c.id);
	}

	// ---- 상세 ----
	const character = $derived(library.character(charId));
	const recent = $derived(charId ? library.convsOf(charId) : []);
	let tab: "overview" | "recent" = $state("overview");
	let starting = $state(false);
	async function newConversation(cid: string) {
		starting = true;
		try {
			open(cid, (await library.createConversation(cid)).id);
		} finally {
			starting = false;
		}
	}

	let more = $state<{ x: number; y: number } | null>(null);
	const moreItems = $derived.by((): MenuItem[] => {
		const c = character;
		if (!c) return [];
		return [
			{ label: "편집 (준비 중)", icon: Pencil, disabled: true, onselect: () => {} },
			{ label: c.favorite ? "즐겨찾기 해제" : "즐겨찾기", icon: c.favorite ? StarOff : Star, onselect: () => toggleFavorite(c) },
			{ label: "캐릭터와 대화 모두 내보내기", icon: Download, onselect: () => exportCharacter(c) },
			{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => removeCharacter(c) },
		];
	});
	async function toggleFavorite(c: Character) {
		await call(commands.toggleFavoriteCharacter(c.id, !c.favorite), { source: "character" });
		c.favorite = !c.favorite;
	}
	async function exportCharacter(c: Character) {
		const path = await pickSavePath(`${c.name}.zip`, [PACK_FILES]);
		if (!path) return;
		await call(commands.exportCharacterPack([c.id], null, path), { source: "character" });
		toasts.success("내보냄");
	}
	async function removeCharacter(c: Character) {
		const n = library.convsOf(c.id).length;
		if (!(await confirmAction({ title: "캐릭터 삭제", message: `'${c.name}'와 대화 ${n}개를 지움. 되돌릴 수 없음`, confirmLabel: "삭제", danger: true }))) return;
		await call(commands.deleteCharacter(c.id), { source: "character" });
		await library.load();
		nav.replace({ page: "character" });
	}
</script>

{#if !library.loaded}
	<div class="center"><Spinner /></div>
{:else if charId && convId}
	<ChatView {charId} {convId} onopen={(id) => open(charId, id)} />
{:else if charId && character}
	<div class="detail">
		<div class="banner"></div>
		<header class="profile">
			<div class="portrait"><Avatar name={character.name} src={fileSrc(character.avatar)} color={character.color} size={96} /></div>
			<div class="ident">
				<h1>{character.name}</h1>
				<p class="faint">{["캐릭터", character.category, ...character.tags.slice(0, 4)].filter(Boolean).join(" · ")}</p>
			</div>
			<div class="cta">
				<Button variant="primary" loading={starting} onclick={() => (recent[0] ? open(character.id, recent[0].id) : newConversation(character.id))}>
					{#snippet icon()}<MessageSquarePlus />{/snippet}
					{recent.length ? "이어서 대화" : "대화 시작"}
				</Button>
				<div class="sub">
					{#if recent.length}<Button size="sm" variant="ghost" disabled={starting} onclick={() => newConversation(character.id)}>새 대화</Button>{/if}
					<IconButton
						label="더 보기 (편집·내보내기·삭제)"
						size="sm"
						onclick={(e) => {
							const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
							more = { x: r.right - 220, y: r.bottom + 4 };
						}}><Ellipsis /></IconButton
					>
				</div>
			</div>
		</header>
		<ContextMenu at={more} items={moreItems} label="캐릭터 메뉴" onclose={() => (more = null)} />

		<div class="tabs">
			<Segmented label="캐릭터 탭" bind:value={tab} options={[{ value: "overview", label: "개요" }, { value: "recent", label: `최근 대화 ${recent.length}` }]} />
		</div>

		<div class="body">
			{#if tab === "overview"}
				{#if character.description.trim()}
					<div class="intro">{character.description}</div>
				{:else}
					<p class="none">소개 없음</p>
				{/if}
			{:else}
				<ul class="recent">
					{#each recent as c (c.id)}
						<li>
							<button onclick={() => open(character.id, c.id)}>
								<span><b>{c.title}</b><small title={absolute(c.updatedAt)}>마지막 활동 · {relative(c.updatedAt)} · {c.messageCount}개</small></span>
								<ChevronRight />
							</button>
						</li>
					{:else}
						<li class="none">아직 대화 없음</li>
					{/each}
				</ul>
			{/if}
		</div>
	</div>
{:else if charId}
	<EmptyState title="캐릭터를 찾을 수 없음" description="지워졌을 수 있음">
		{#snippet action()}<Button onclick={() => nav.replace({ page: "character" })}>목록으로</Button>{/snippet}
	</EmptyState>
{:else}
	<div class="page">
		<header class="head">
			<h1>캐릭터</h1>
			<div class="tools">
				<Input bind:value={query} placeholder="이름·태그 검색" aria-label="캐릭터 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<Button onclick={importFiles}>
					{#snippet icon()}<Upload />{/snippet}
					가져오기
				</Button>
				<Button variant="primary" onclick={createCharacter}>
					{#snippet icon()}<Plus />{/snippet}
					새 캐릭터
				</Button>
			</div>
		</header>
		{#if !library.characters.length}
			<EmptyState title="캐릭터 없음" description="SillyTavern 카드(PNG·JSON)나 캐릭터 묶음(zip)을 가져오거나 새로 만들기">
				{#snippet icon()}<Users />{/snippet}
			</EmptyState>
		{:else}
			<div class="grid">
				{#each list as ch (ch.id)}
					{@const convs = library.convsOf(ch.id)}
					<article class="char">
						<button class="body-btn" onclick={() => open(ch.id)}>
							<Avatar name={ch.name} src={fileSrc(ch.avatar)} color={ch.color} size={64} />
							<b>{#if ch.favorite}<Star class="fav" />{/if}{ch.name}</b>
							<span class="muted desc">{firstLine(ch.description) || "소개 없음"}</span>
							<span class="faint">대화 {convs.length}개{#if convs.length} · {relative(convs[0].updatedAt)}{/if}</span>
						</button>
						{#if convs.length}
							<button class="resume" onclick={() => open(ch.id, convs[0].id)}>이어서: {convs[0].title}</button>
						{/if}
					</article>
				{:else}
					<p class="none">검색 결과 없음</p>
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.page {
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: column;
		gap: var(--space-4);
		max-width: 1100px;
		margin: 0 auto var(--space-5);
	}
	.tools {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto auto;
		gap: var(--space-2);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-3);
		max-width: 1100px;
		margin: 0 auto;
	}
	.char {
		display: flex;
		flex-direction: column;
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
		transition: border-color var(--duration-fast) var(--ease);
	}
	.char:hover {
		border-color: var(--accent);
	}
	.body-btn {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: var(--space-1);
		flex: 1;
		padding: var(--space-5) var(--space-4) var(--space-4);
		border: none;
		background: none;
		text-align: center;
		cursor: pointer;
	}
	.body-btn b {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		margin-top: var(--space-2);
	}
	.body-btn b :global(.fav) {
		width: 14px;
		height: 14px;
		color: var(--accent);
		fill: var(--accent);
	}
	.body-btn span {
		font-size: var(--text-sm);
	}
	.desc {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.resume {
		padding: var(--space-2) var(--space-3);
		border: none;
		border-top: 1px solid var(--border);
		background: var(--surface-2);
		color: var(--accent-text);
		font-size: var(--text-xs);
		font-weight: 600;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.resume:hover {
		background: var(--accent-soft);
	}
	.detail {
		height: 100%;
		overflow: auto;
	}
	.banner {
		height: 200px;
		background:
			radial-gradient(ellipse at 70% 0%, color-mix(in oklab, var(--accent) 45%, transparent), transparent 55%),
			linear-gradient(180deg, #2f3d47 0%, #24343c 55%, #1a2328 100%);
	}
	.profile {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: var(--space-4);
		align-items: end;
		max-width: 960px;
		margin: -48px auto 0;
		padding: 0 var(--space-6);
	}
	.portrait :global(.avatar) {
		border-radius: var(--radius-lg) !important;
		border: 4px solid var(--bg);
		box-shadow: var(--shadow-md);
	}
	.portrait :global(.avatar img) {
		border-radius: inherit;
	}
	.ident,
	.cta {
		margin-top: 56px;
	}
	.ident {
		padding-bottom: var(--space-1);
	}
	.ident p {
		font-size: var(--text-sm);
	}
	.cta {
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: var(--space-1);
		padding-bottom: var(--space-1);
	}
	.sub {
		display: flex;
		align-items: center;
	}
	.tabs {
		max-width: 960px;
		margin: var(--space-5) auto 0;
		padding: 0 var(--space-6) var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.body {
		max-width: 960px;
		margin: 0 auto;
		padding: 0 var(--space-6) var(--space-10);
	}
	.intro {
		margin-top: var(--space-5);
		white-space: pre-wrap;
		line-height: 1.8;
	}
	.recent {
		list-style: none;
		margin: var(--space-3) 0 0;
		padding: 0;
	}
	.recent button {
		display: flex;
		align-items: center;
		width: 100%;
		padding: var(--space-3) 0;
		border: none;
		border-bottom: 1px solid var(--border);
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.recent li:last-child button {
		border-bottom: none;
	}
	.recent button span {
		flex: 1;
		display: flex;
		flex-direction: column;
	}
	.recent small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.recent button :global(svg) {
		color: var(--text-faint);
	}
	.recent button:hover b {
		color: var(--accent-text);
	}
	.none {
		padding: var(--space-6);
		text-align: center;
		color: var(--text-faint);
	}
</style>
