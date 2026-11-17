<script lang="ts">
	import { TRASH_HINT, trashedToast } from "$shared/trash";
	import { Camera, ImagePlus, Move, X as XIcon } from "@lucide/svelte";
	import { ChevronRight, Download, Ellipsis, Gamepad2, MessageSquarePlus, Pencil, Plus, Search, Star, StarOff, Trash2, Upload, UserRound, Users } from "@lucide/svelte";
	import type { GameProject_Serialize as GameProject } from "$ipc/bindings";
	import { Avatar, Badge, Button, ContextMenu, ItemCard, ListToolbar, SelectBar, SplitButton, Dialog, EmptyState, IconButton, Input, Segmented, Spinner, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, PACK_FILES, ST_FILES, pickFiles, pickSavePath } from "$shared/ipc/dialog";
	import { fileSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import ChatView from "./ChatView.svelte";
	import CharacterEditor from "./CharacterEditor.svelte";
	import { library, type Character } from "./library.svelte";
	import { charActions } from "./actions";
	import { fillNames, toBlocks } from "./format";
	import RichText from "./RichText.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { drops, extOf, storeFile } from "$app/drop.svelte";

	// 캐릭터 목록 → 캐릭터 상세 → 대화 상세 (대화 목록·폴더는 대화 상세의 왼쪽 사이드바)
	let { charId, convId, edit = false }: { charId?: string; convId?: string; edit?: boolean } = $props();

	$effect(() => {
		if (!library.loaded) void library.load();
	});

	function open(cid: string, conv?: string) {
		nav.go(conv ? { page: "character", charId: cid, convId: conv } : { page: "character", charId: cid });
	}

	// ---- 목록: 찾기 · 거르기 · 정렬 · 여러 개 삭제 (이미지·게임·로어북 목록과 같은 도구) ----
	let query = $state("");
	let filter = $state("all");
	let sort = $state("recent");
	let selecting = $state(false);
	let picked = $state<Set<string>>(new Set());
	$effect(() => {
		if (!selecting) picked = new Set();
	});
	const lastActive = (id: string) => library.convsOf(id)[0]?.updatedAt ?? 0;
	const allTags = $derived([...new Set(library.characters.flatMap((c) => c.tags))].sort((a, b) => a.localeCompare(b, "ko")));
	const FILTERS = $derived([
		{ value: "all", label: `전부 ${library.characters.length}` },
		{ value: "fav", label: "즐겨찾기" },
		{ value: "chat", label: "대화 있음" },
		{ value: "nochat", label: "대화 없음" },
		...allTags.map((t) => ({ value: `tag:${t}`, label: `#${t}` })),
	]);
	const SORTS = [
		{ value: "recent", label: "최근 대화 순" },
		{ value: "name", label: "이름 순" },
		{ value: "new", label: "최근 만든 순" },
	];
	const list = $derived.by(() => {
		const q = query.trim().toLowerCase();
		const pass = (c: Character) =>
			filter === "all" ||
			(filter === "fav" && c.favorite) ||
			(filter === "chat" && library.convsOf(c.id).length > 0) ||
			(filter === "nochat" && !library.convsOf(c.id).length) ||
			(filter.startsWith("tag:") && c.tags.includes(filter.slice(4)));
		const by =
			sort === "name"
				? (a: Character, b: Character) => a.name.localeCompare(b.name, "ko")
				: sort === "new"
					? (a: Character, b: Character) => b.createdAt - a.createdAt
					: (a: Character, b: Character) => Number(b.favorite) - Number(a.favorite) || lastActive(b.id) - lastActive(a.id) || a.name.localeCompare(b.name, "ko");
		return library.characters.filter((c) => pass(c) && (!q || c.name.toLowerCase().includes(q) || c.tags.some((t) => t.toLowerCase().includes(q)))).sort(by);
	});
	function pickChar(id: string) {
		const next = new Set(picked);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		picked = next;
	}
	async function removePicked() {
		const ids = [...picked];
		if (!ids.length) return;
		const convs = ids.reduce((n, id) => n + library.convsOf(id).length, 0);
		if (!(await confirmAction({ title: "캐릭터 삭제", message: `캐릭터 ${ids.length}개를 휴지통으로 옮김 (대화 ${convs}개는 캐릭터 없이 남음). 설정 > 데이터 > 휴지통에서 되살릴 수 있음`, confirmLabel: `${ids.length}개 삭제`, danger: true }))) return;
		for (const id of ids) await call(commands.deleteCharacter(id), { source: "character" });
		await library.load();
		selecting = false;
		toasts.success(`캐릭터 ${ids.length}개를 휴지통으로 옮김`);
	}
	/** 제작자 메모는 HTML이 섞여 올 때가 많음 → 태그를 빼고 글만 (스크립트 실행 없음) */
	function plain(html: string) {
		if (!/<[a-z][\s\S]*>/i.test(html)) return html;
		const withBreaks = html.replace(/<br\s*\/?>/gi, "\n").replace(/<\/(p|div|h\d|li)>/gi, "\n");
		const doc = new DOMParser().parseFromString(withBreaks, "text/html");
		return (doc.body.textContent ?? "").replace(/\n{3,}/g, "\n\n").trim();
	}
	// ---- 커버·아바타 바로 바꾸기 (편집 화면에 가지 않고) ----
	async function changeCover(id: string, remove = false) {
		let path: string | null = null;
		if (!remove) {
			[path] = await pickFiles([IMAGE_FILES], false);
			if (!path) return;
		}
		const saved = await call(commands.setCharacterCover(id, path), { source: "character" });
		const c = library.character(id);
		if (c) c.cover = saved.cover;
	}
	// 커버 위치 조절: 위아래로 끌어서 보일 부분을 고르고 "완료"로 저장
	let positioning = $state(false);
	let coverY = $state(50);
	let dragFrom: { y: number; start: number } | null = null;
	function startPosition(y: number) {
		coverY = y;
		positioning = true;
	}
	function onCoverDown(e: PointerEvent) {
		if (!positioning) return;
		// 완료·취소 버튼을 누른 것은 끌기가 아님 (포인터를 잡으면 버튼이 클릭을 못 받아 저장이 안 됐음)
		if ((e.target as Element).closest("button")) return;
		dragFrom = { y: e.clientY, start: coverY };
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
	}
	function onCoverMove(e: PointerEvent) {
		if (!positioning || !dragFrom) return;
		const h = (e.currentTarget as HTMLElement).clientHeight || 200;
		// 그림을 아래로 끌면 위쪽이 보인다 (값이 줄어듦)
		coverY = Math.min(100, Math.max(0, dragFrom.start - ((e.clientY - dragFrom.y) / h) * 100));
	}
	async function finishPosition(id: string) {
		positioning = false;
		dragFrom = null;
		await call(commands.setCharacterCoverPosition(id, Math.round(coverY)), { source: "character" });
		const c = library.character(id);
		if (c) c.coverY = Math.round(coverY);
	}

	async function changeAvatar(id: string) {
		const [path] = await pickFiles([IMAGE_FILES], false);
		if (!path) return;
		const saved = await call(commands.setCharacterAvatar(id, path), { source: "character" });
		const c = library.character(id);
		if (c) c.avatar = saved.avatar;
	}

	const firstLine = (s: string) => s.split("\n").find((l) => l.trim())?.trim() ?? "";

	// ---- 개요: 메시지와 같은 렌더링 (마크다운 줄·HTML·<cg>·{{char}}/{{user}}) ----
	// HTML 렌더링 플러그인이 꺼져 있으면 HTML 소개는 글만 뽑아 보인다 (스크립트 실행 없음)
	function overviewBlocks(ch: Character) {
		const raw = ch.creatorNotes?.trim() ? ch.creatorNotes : ch.description;
		const persona = library.persona(ch.defaultPersonaId) ?? library.persona("default") ?? library.personas[0];
		const text = fillNames(raw, { char: ch.name, user: persona?.name });
		return plugins.on("html") ? toBlocks(text, { rawHtml: true }) : toBlocks(plain(text));
	}

	async function importFiles() {
		await importPaths(await pickFiles([ST_FILES, PACK_FILES]));
	}
	/** 카드(PNG·JSON)·대화(.jsonl)·묶음(ZIP). 캐릭터 상세에서는 대화를 그 캐릭터에 붙임 */
	async function importPaths(paths: string[]) {
		for (const p of paths) {
			if (p.toLowerCase().endsWith(".zip")) {
				const r = await call(commands.importCharacterPack(p), { source: "character" }).catch(() => null);
				if (r) toasts.success(`캐릭터 ${r.characters.length}명 · 대화 ${r.conversationCount}개 가져옴`);
			} else await library.importSillyTavern(p, p.toLowerCase().endsWith(".jsonl") ? (charId ?? null) : null).catch(() => {});
		}
		if (paths.length) await library.load();
	}
	const DROP_EXT = ["png", "json", "jsonl", "zip"];
	$effect(() =>
		drops.register({
			label: charId ? `${library.character(charId)?.name ?? "캐릭터"}에 가져오기` : "캐릭터 가져오기",
			hint: charId ? "SillyTavern 채팅(.jsonl)은 이 캐릭터의 대화로, 카드(PNG·JSON)·묶음(ZIP)은 새 캐릭터로" : "카드(PNG·JSON), 캐릭터 묶음(ZIP), SillyTavern 채팅(.jsonl)",
			accept: (f) => DROP_EXT.includes(extOf(f)),
			ondrop: async (files) => importPaths(await Promise.all(files.map((f) => storeFile(f)))),
		}),
	);
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
	const overviewImages = $derived(charId ? Object.fromEntries((library.images[charId] ?? []).map((i) => [i.key, fileSrc(i.path) ?? ""])) : {});
	$effect(() => {
		if (charId && !edit && !convId && !library.images[charId]) void library.loadImages(charId);
	});
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

	let more = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const moreItems = $derived.by((): MenuItem[] => {
		const c = character;
		if (!c) return [];
		return [
			{ label: "편집", icon: Pencil, onselect: () => nav.go({ page: "character", charId: c.id, edit: true }) },
			{ label: c.favorite ? "즐겨찾기 해제" : "즐겨찾기", icon: c.favorite ? StarOff : Star, onselect: () => toggleFavorite(c) },
			{ label: "페르소나로 복사", icon: UserRound, separator: true, onselect: () => toPersona(c) },
			{ label: "게임에 넣기", icon: Gamepad2, onselect: () => openGamePick(c) },
			{ label: "내보내기…", icon: Download, separator: true, onselect: () => charActions.exportMenu(c) },
			{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => removeCharacter(c) },
		];
	});
	// ---- 복사: 원본은 그대로, 상대 쪽에 새로 만듦 ----
	async function toPersona(c: Character) {
		const p = await call(commands.copyCharacterToPersona(c.id), { source: "character" });
		library.personas = [...library.personas, p];
		toasts.show({ tone: "success", message: `페르소나 '${p.name}' 만듦`, action: { label: "열기", run: () => nav.go({ page: "persona", personaId: p.id }) } });
	}
	let gamePick = $state<{ c: Character; games: GameProject[] } | null>(null);
	let gamePickOpen = $state(false);
	async function openGamePick(c: Character) {
		const games = await call(commands.gameList(), { source: "character" }).catch(() => []);
		if (!games.length) return toasts.info("게임이 없음. 게임을 먼저 만들어야 함");
		gamePick = { c, games };
		gamePickOpen = true;
	}
	async function toGame(g: GameProject) {
		if (!gamePick) return;
		await call(commands.copyCharacterToGame(gamePick.c.id, g.id), { source: "character" });
		gamePickOpen = false;
		toasts.success(`'${g.title}' 게임에 '${gamePick.c.name}' 넣음 · 아바타는 게임 에셋으로 따로 넣어야 함`);
	}
	async function toggleFavorite(c: Character) {
		await call(commands.toggleFavoriteCharacter(c.id, !c.favorite), { source: "character" });
		c.favorite = !c.favorite;
	}
	// 목록 카드의 … 메뉴 (오른쪽 클릭도 같음)
	let cardMenu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	function openCardMenu(e: MouseEvent, c: Character) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const convs = library.convsOf(c.id);
		cardMenu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				...(convs.length ? [{ label: `이어서: ${convs[0].title}`, icon: MessageSquarePlus, onselect: () => open(c.id, convs[0].id) }] : []),
				{ label: "새 대화", icon: Plus, onselect: () => newConversation(c.id) },
				{ label: c.favorite ? "즐겨찾기 빼기" : "즐겨찾기", icon: c.favorite ? StarOff : Star, separator: true, onselect: () => toggleFavorite(c) },
				{ label: "편집", icon: Pencil, onselect: () => nav.go({ page: "character", charId: c.id, edit: true }) },
				{ label: "내보내기…", icon: Download, onselect: () => charActions.exportMenu(c) },
				{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => removeCharacter(c, false) },
			],
		};
	}
	async function removeCharacter(c: Character, leave = true) {
		const n = library.convsOf(c.id).length;
		if (!(await confirmAction({ title: "캐릭터 삭제", message: `'${c.name}'을(를) 휴지통으로 옮김${n ? ` (대화 ${n}개는 캐릭터 없이 남고, 되살리면 다시 이어짐)` : ""}. ${TRASH_HINT}`, confirmLabel: "삭제", danger: true }))) return;
		const item = await call(commands.deleteCharacter(c.id), { source: "character" });
		await library.load();
		trashedToast(item, () => library.load());
		if (leave) nav.replace({ page: "character" });
	}
</script>

{#if !library.loaded}
	<div class="center"><Spinner /></div>
{:else if charId && convId}
	<ChatView {charId} {convId} onopen={(id) => open(charId, id)} />
{:else if charId && character && edit}
	{#key character.id}
		<CharacterEditor {character} />
	{/key}
{:else if charId && character}
	<div class="detail">
		<!-- svelte-ignore a11y_no_static_element_interactions (위치 조절은 끌기, 버튼으로 시작·끝) -->
		<div
			class="banner"
			class:has-cover={!!character.cover}
			class:positioning
			style:background-image={character.cover ? `url("${fileSrc(character.cover)}")` : undefined}
			style:background-position-y={character.cover ? `${positioning ? coverY : (character.coverY ?? 50)}%` : undefined}
			onpointerdown={onCoverDown}
			onpointermove={onCoverMove}
			onpointerup={() => (dragFrom = null)}
		>
			{#if positioning}
				<div class="cover-hint">위아래로 끌어서 보일 부분 고르기</div>
				<div class="cover-tools on">
					<Button size="sm" variant="secondary" onclick={() => (positioning = false)}>취소</Button>
					<Button size="sm" variant="primary" onclick={() => finishPosition(character.id)}>완료</Button>
				</div>
			{:else}
				<div class="cover-tools">
					<Button size="sm" variant="secondary" onclick={() => changeCover(character.id)}>{#snippet icon()}<ImagePlus />{/snippet}{character.cover ? "커버 바꾸기" : "커버 넣기"}</Button>
					{#if character.cover}
						<Button size="sm" variant="secondary" onclick={() => startPosition(character.coverY ?? 50)}>{#snippet icon()}<Move />{/snippet}위치 조절</Button>
						<Button size="sm" variant="secondary" onclick={() => changeCover(character.id, true)}>{#snippet icon()}<XIcon />{/snippet}커버 지우기</Button>
					{/if}
				</div>
			{/if}
		</div>
		<header class="profile">
			<button class="portrait" title="아바타 바꾸기" aria-label="아바타 바꾸기" onclick={() => changeAvatar(character.id)}>
				<Avatar name={character.name} src={fileSrc(character.avatar)} color={character.color} size={96} />
				<span class="cam" aria-hidden="true"><Camera /></span>
			</button>
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
							more = { x: r.right, y: r.bottom + 4, align: "end" };
						}}><Ellipsis /></IconButton
					>
				</div>
			</div>
		</header>
		<ContextMenu at={more} items={moreItems} label="캐릭터 메뉴" onclose={() => (more = null)} />
		<Dialog bind:open={gamePickOpen} title="게임에 넣기" description="복사본이 게임 캐릭터로 들어감. 이 캐릭터는 그대로 남음" size="sm">
			<ul class="games">
				{#each gamePick?.games ?? [] as g (g.id)}
					<li><button onclick={() => toGame(g)}><Gamepad2 /><span><b>{g.title}</b>{#if g.description}<small>{g.description}</small>{/if}</span></button></li>
				{/each}
			</ul>
		</Dialog>

		<div class="tabs">
			<Segmented label="캐릭터 탭" bind:value={tab} options={[{ value: "overview", label: "개요" }, { value: "recent", label: `최근 대화 ${recent.length}` }]} />
		</div>

		<div class="body">
			{#if tab === "overview"}
				{#if character.creatorNotes?.trim() || character.description.trim()}
					<div class="intro"><RichText blocks={overviewBlocks(character)} images={overviewImages} /></div>
				{:else}
					<p class="none">소개 없음 · <button class="link" onclick={() => nav.go({ page: "character", charId: character.id, edit: true })}>편집에서 쓰기</button></p>
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
			<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder="이름·태그" filters={FILTERS} sorts={SORTS} canSelect={!!library.characters.length}>
				<SplitButton label="새로 만들기" onclick={createCharacter} menuLabel="캐릭터 가져오기" items={[{ label: "파일에서 가져오기 (PNG·JSON·ZIP)", icon: Upload, onselect: importFiles }]} />
			</ListToolbar>
		</header>
		{#if selecting}<SelectBar count={picked.size} onall={() => (picked = new Set(list.map((c) => c.id)))} ondelete={removePicked} />{/if}
		{#if !library.characters.length}
			<EmptyState title="캐릭터 없음" description="SillyTavern 카드(PNG·JSON)나 캐릭터 묶음(zip)을 가져오거나 새로 만들기">
				{#snippet icon()}<Users />{/snippet}
			</EmptyState>
		{:else}
			<div class="grid">
				{#each list as ch (ch.id)}
					{@const convs = library.convsOf(ch.id)}
					<ItemCard
						title={ch.name}
						desc={firstLine(ch.description) || "소개 없음"}
						meta="대화 {convs.length}개{convs.length ? ` · ${relative(convs[0].updatedAt)}` : ''}"
						image={fileSrc(ch.cover) ?? fileSrc(ch.avatar)}
						imagePosition={ch.cover ? `center ${ch.coverY ?? 50}%` : undefined}
						{selecting}
						selected={picked.has(ch.id)}
						onclick={() => (selecting ? pickChar(ch.id) : open(ch.id))}
						oncontextmenu={(e) => openCardMenu(e, ch)}
					>
						{#snippet menu()}<IconButton label="{ch.name} 메뉴" size="sm" onclick={(e) => openCardMenu(e, ch)}><Ellipsis /></IconButton>{/snippet}
						{#snippet fallback()}<Avatar name={ch.name} color={ch.color} size={64} />{/snippet}
						{#snippet badges()}
							{#if ch.favorite}<Badge tone="warning"><Star /> 즐겨찾기</Badge>{/if}
							{#each ch.tags.slice(0, 3) as t (t)}<Badge>#{t}</Badge>{/each}
						{/snippet}
						{#snippet footer()}
							{#if convs.length && !selecting}<button class="resume" onclick={() => open(ch.id, convs[0].id)}>이어서: {convs[0].title}</button>{/if}
						{/snippet}
					</ItemCard>
				{:else}
					<p class="none">검색 결과 없음</p>
				{/each}
			</div>
		{/if}
	</div>
{/if}
{#if cardMenu}<ContextMenu at={cardMenu.at} items={cardMenu.items} label="캐릭터 메뉴" onclose={() => (cardMenu = null)} />{/if}

<style>
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.page {
		--selectbar-max: var(--page-wide);
		height: 100%;
		overflow: auto;
		padding: var(--space-6) var(--space-8);
	}
	.head {
		display: flex;
		flex-direction: row;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-4);
		/* 좁은 화면: 도구 줄이 제목 아래로 내려감 (제목이 "캐릭/터"처럼 잘리지 않게) */
		flex-wrap: wrap;
		max-width: var(--page-wide);
		margin: 0 auto var(--space-5);
	}
	.head h1 {
		white-space: nowrap;
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: var(--space-3);
		max-width: var(--page-wide);
		margin: 0 auto;
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
		position: relative;
		height: 200px;
		background:
			radial-gradient(ellipse at 70% 0%, color-mix(in oklab, var(--accent) 45%, transparent), transparent 55%),
			linear-gradient(180deg, #2f3d47 0%, #24343c 55%, #1a2328 100%);
	}
	.banner.has-cover {
		background-size: cover;
		background-position: center;
		background-repeat: no-repeat;
	}
	/* 커버 버튼은 마우스를 올리거나 키보드로 갔을 때 (터치 화면은 늘 보임) */
	.cover-tools {
		position: absolute;
		top: var(--space-3);
		right: var(--space-4);
		display: flex;
		gap: var(--space-2);
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.banner:hover .cover-tools,
	.cover-tools:focus-within,
	.cover-tools.on {
		opacity: 1;
	}
	.banner.positioning {
		cursor: ns-resize;
		touch-action: none;
		outline: 2px dashed var(--accent);
		outline-offset: -2px;
	}
	.cover-hint {
		position: absolute;
		left: 50%;
		bottom: var(--space-3);
		transform: translateX(-50%);
		padding: 4px 12px;
		border-radius: var(--radius-full);
		background: rgb(0 0 0 / 0.55);
		color: #fff;
		font-size: var(--text-xs);
		pointer-events: none;
	}
	@media (hover: none) {
		.cover-tools {
			opacity: 1;
		}
	}
	.portrait {
		position: relative;
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: var(--radius-lg);
	}
	.portrait .cam {
		position: absolute;
		inset: 4px;
		display: grid;
		place-items: center;
		border-radius: var(--radius-lg);
		background: rgb(0 0 0 / 0.4);
		color: #fff;
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.portrait:hover .cam,
	.portrait:focus-visible .cam {
		opacity: 1;
	}
	.profile {
		display: grid;
		grid-template-columns: auto minmax(0, 1fr) auto;
		gap: var(--space-4);
		align-items: end;
		max-width: var(--page-medium);
		margin: -48px auto 0;
		padding: 0 var(--space-6);
	}
	.portrait :global(.avatar) {
		border-radius: var(--radius-lg) !important;
		border: 4px solid var(--bg);
		box-shadow: var(--shadow-md);
		overflow: hidden;
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
		max-width: var(--page-medium);
		margin: var(--space-5) auto 0;
		padding: 0 var(--space-6) var(--space-3);
		border-bottom: 1px solid var(--border);
	}
	.body {
		max-width: var(--page-medium);
		margin: 0 auto;
		padding: 0 var(--space-6) var(--space-10);
	}
	.intro {
		margin-top: var(--space-5);
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
	.games {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.games button {
		display: flex;
		align-items: center;
		gap: var(--space-3);
		width: 100%;
		padding: var(--space-2) var(--space-3);
		border: none;
		border-radius: var(--radius-sm);
		background: none;
		color: var(--text);
		font: inherit;
		text-align: left;
		cursor: pointer;
	}
	.games button:hover {
		background: var(--surface-hover);
	}
	.games :global(svg) {
		width: 18px;
		height: 18px;
		color: var(--text-muted);
	}
	.games span {
		display: flex;
		flex-direction: column;
	}
	.games small {
		color: var(--text-muted);
	}
	.link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
	}
	.link:hover {
		text-decoration: underline;
	}
</style>
