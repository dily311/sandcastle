<script lang="ts">
	import { Camera, ImagePlus, Move, X as XIcon } from "@lucide/svelte";
	import { ChevronRight, Download, Ellipsis, Gamepad2, MessageSquarePlus, Pencil, Plus, Search, Star, StarOff, Trash2, Upload, UserRound, Users } from "@lucide/svelte";
	import type { GameProject_Serialize as GameProject } from "$ipc/bindings";
	import { Avatar, Button, ContextMenu, SplitButton, Dialog, EmptyState, IconButton, Input, Segmented, Spinner, type MenuItem } from "$shared/ui";
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

	// ---- 목록 ----
	let query = $state("");
	const lastActive = (id: string) => library.convsOf(id)[0]?.updatedAt ?? 0;
	const list = $derived(
		library.characters
			.filter((c) => !query.trim() || c.name.toLowerCase().includes(query.trim().toLowerCase()) || c.tags.some((t) => t.toLowerCase().includes(query.trim().toLowerCase())))
			.sort((a, b) => Number(b.favorite) - Number(a.favorite) || lastActive(b.id) - lastActive(a.id) || a.name.localeCompare(b.name, "ko")),
	);
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

	let more = $state<{ x: number; y: number } | null>(null);
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
							more = { x: r.right - 220, y: r.bottom + 4 };
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
			<div class="tools">
				<Input bind:value={query} placeholder="이름·태그 검색" aria-label="캐릭터 검색">
					{#snippet leading()}<Search />{/snippet}
				</Input>
				<SplitButton label="새로 만들기" onclick={createCharacter} menuLabel="캐릭터 가져오기" items={[{ label: "파일에서 가져오기 (PNG·JSON·ZIP)", icon: Upload, onselect: importFiles }]} />
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
		max-width: 960px;
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
