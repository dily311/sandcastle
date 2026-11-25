<script lang="ts">
	import { chatImport } from "./chatImport.svelte";
	import { _ } from "svelte-i18n";
	import { jobs, SKIPPED } from "$shared/stores/jobs.svelte";
	import { lastAt } from "$features/character/library.svelte";
	import { TRASH_HINT, trashedToast } from "$shared/trash";
	import { Camera, Copy, ImagePlus, Move, Pin, PinOff, X as XIcon } from "@lucide/svelte";
	import { ChevronRight, Download, Ellipsis, Gamepad2, MessageSquarePlus, Pencil, Plus, Star, StarOff, Trash2, Upload, UserRound, Users } from "@lucide/svelte";
	import type { GameProject_Serialize as GameProject } from "$ipc/bindings";
	import { Avatar, Button, ContextMenu, ListToolbar, SelectBar, SplitButton, Dialog, EmptyState, IconButton, Segmented, Spinner, type MenuItem } from "$shared/ui";
	import { nav } from "$app/nav.svelte";
	import { call, commands } from "$shared/ipc/client";
	import type { CharacterStats } from "$ipc/bindings";
	import { bookmarks, bookmarksOf } from "./bookmarks";
	import { IMAGE_FILES, PACK_FILES, ST_FILES, pickFiles, pickSavePath } from "$shared/ipc/dialog";
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { absolute, relative } from "$shared/time";
	import ChatView from "./ChatView.svelte";
	import ReadView from "./ReadView.svelte";
	import CharacterEditor from "./CharacterEditor.svelte";
	import CharacterGallery from "./CharacterGallery.svelte";
	import { library, type Character, type Conversation } from "./library.svelte";
	import { charActions, convActions } from "./actions";
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
	const lastActive = (id: string) => (library.convsOf(id)[0] ? lastAt(library.convsOf(id)[0]) : 0);
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
		selecting = false;
		// 파일 작업 대기열로 하나씩 (5-E-28). 요약 알림은 대기열이 한 번
		await Promise.allSettled(ids.map((id) => deleteJob(id, library.character(id)?.name ?? id)));
		await library.load();
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
		// 채팅(.jsonl) 여럿은 진행 막대와 함께 한꺼번에 (한 개면 지금처럼)
		const chats = paths.filter((p) => p.toLowerCase().endsWith(".jsonl"));
		if (chats.length > 1) {
			await chatImport.run(chats, charId ?? null);
			paths = paths.filter((p) => !chats.includes(p));
		}
		// 파일마다 대기열에 (5-E-28) — 앞 파일을 가져오는 동안 더 골라도 거절하지 않고 차례로
		const name = (p: string) => p.split(/[\\/]/).pop() ?? p;
		await Promise.allSettled(
			paths.map((p) =>
				jobs.run(
					name(p),
					async () => {
						if (!p.toLowerCase().endsWith(".zip")) return void (await library.importSillyTavern(p, p.toLowerCase().endsWith(".jsonl") ? (charId ?? null) : null));
						const r = await call(commands.importCharacterPack(p), { source: "character" });
						toasts.success(`캐릭터 ${r.characters.length}명 · 대화 ${r.conversationCount}개 가져옴`);
					},
					{ group: "캐릭터 가져오기" },
				),
			),
		);
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
	const overviewImages = $derived(charId ? Object.fromEntries((library.images[charId] ?? []).map((i) => [i.key, imageSrc(i) ?? ""])) : {});
	$effect(() => {
		if (charId && !edit && !convId && !library.images[charId]) void library.loadImages(charId);
	});
	const recent = $derived(charId ? library.convsOf(charId) : []);
	let tab: "overview" | "recent" | "bookmarks" | "gallery" | "stats" = $state("overview");
	// 채팅 목록 찾기·거르기·정렬
	let convQuery = $state("");
	let convFilter = $state("all");
	let convSort = $state("recent");
	const convFolders = $derived(charId ? library.foldersOf(charId) : []);
	const CONV_FILTERS = $derived([
		{ value: "all", label: `전부 ${recent.length}` },
		{ value: "pinned", label: "고정됨" },
		{ value: "read", label: "읽기 모드" },
		{ value: "chat", label: "대화 모드" },
		...convFolders.map((f) => ({ value: `folder:${f.id}`, label: f.name })),
	]);
	const CONV_SORTS = [
		{ value: "recent", label: "최근 활동 순" },
		{ value: "created", label: "최근 만든 순" },
		{ value: "name", label: "제목 순" },
		{ value: "messages", label: "메시지 많은 순" },
	];
	const filteredRecent = $derived.by(() => {
		const q = convQuery.trim().toLowerCase();
		const pass = (c: (typeof recent)[number]) =>
			convFilter === "all" ||
			(convFilter === "pinned" && !!c.pinned) ||
			(convFilter === "read" && !!c.readMode) ||
			(convFilter === "chat" && !c.readMode) ||
			(convFilter.startsWith("folder:") && c.folderId === convFilter.slice(7));
		const by =
			convSort === "created"
				? (a: (typeof recent)[number], b: (typeof recent)[number]) => b.createdAt - a.createdAt
				: convSort === "name"
					? (a: (typeof recent)[number], b: (typeof recent)[number]) => a.title.localeCompare(b.title, "ko")
					: convSort === "messages"
						? (a: (typeof recent)[number], b: (typeof recent)[number]) => b.messageCount - a.messageCount
						: (a: (typeof recent)[number], b: (typeof recent)[number]) => lastAt(b) - lastAt(a);
		return recent.filter((c) => pass(c) && (!q || c.title.toLowerCase().includes(q))).sort(by);
	});
	// 북마크 탭: 이 캐릭터의 대화마다 남긴 북마크 (대화 meta, 새로 읽지 않음)
	const marks = $derived(recent.flatMap((c) => bookmarksOf(c).map((b) => ({ ...b, conv: c }))).sort((a, b) => b.createdAt - a.createdAt));
	let markQuery = $state("");
	let markFilter = $state("all");
	let markSort = $state("recent");
	const MARK_FILTERS = $derived.by(() => {
		const convsWithMarks = [...new Map(marks.map((m) => [m.conv.id, m.conv])).values()];
		return [
			{ value: "all", label: `전부 ${marks.length}` },
			...convsWithMarks.map((c) => ({ value: `conv:${c.id}`, label: c.title })),
		];
	});
	const MARK_SORTS = [
		{ value: "recent", label: "최근 북마크 순" },
		{ value: "oldest", label: "오래된 순" },
		{ value: "conv", label: "대화 제목 순" },
	];
	const filteredMarks = $derived.by(() => {
		const q = markQuery.trim().toLowerCase();
		const pass = (m: (typeof marks)[number]) =>
			markFilter === "all" || (markFilter.startsWith("conv:") && m.conv.id === markFilter.slice(5));
		const by =
			markSort === "oldest"
				? (a: (typeof marks)[number], b: (typeof marks)[number]) => a.createdAt - b.createdAt
				: markSort === "conv"
					? (a: (typeof marks)[number], b: (typeof marks)[number]) => a.conv.title.localeCompare(b.conv.title, "ko") || b.createdAt - a.createdAt
					: (a: (typeof marks)[number], b: (typeof marks)[number]) => b.createdAt - a.createdAt;
		return marks
			.filter((m) => pass(m) && (!q || m.text.toLowerCase().includes(q) || m.conv.title.toLowerCase().includes(q)))
			.sort(by);
	});
	$effect(() => {
		if (charId) {
			void library.loadFolders(charId);
		}
		convQuery = "";
		convFilter = "all";
		markQuery = "";
		markFilter = "all";
	});
	// 통계 탭: 누를 때만 읽는다 (5-E-12)
	let stats = $state<CharacterStats | null>(null);
	let statsFor = "";
	$effect(() => {
		const id = charId;
		if (tab !== "stats" || !id || statsFor === id) return;
		statsFor = id;
		stats = null;
		void call(commands.getCharacterStats(id, -new Date().getTimezoneOffset()), { source: "character" })
			.then((s) => {
				if (charId === id) stats = s;
			})
			.catch(() => (statsFor = ""));
	});
	const DAYS = ["일", "월", "화", "수", "목", "금", "토"];
	function duration(sec: number) {
		const h = Math.floor(sec / 3600);
		const m = Math.round((sec % 3600) / 60);
		return h ? `${h}시간 ${m}분` : `${m}분`;
	}
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
	function openConvMenu(e: MouseEvent, c: Conversation) {
		e.preventDefault();
		e.stopPropagation();
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		cardMenu = {
			at: e.type === "contextmenu" ? { x: e.clientX, y: e.clientY } : { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{
					label: "이름 바꾸기",
					icon: Pencil,
					onselect: async () => {
						const t = await promptText({ title: "대화 이름", input: { value: c.title, placeholder: "대화 이름" }, confirmLabel: "저장" });
						if (t?.trim()) await library.rename(c.id, t);
					},
				},
				{ label: c.pinned ? "고정 풀기" : "고정", icon: c.pinned ? PinOff : Pin, onselect: () => library.togglePin(c.id) },
				{
					label: "복제",
					icon: Copy,
					onselect: async () => {
						const d = await library.duplicate(c.id);
						if (d) open(c.characterId, d.id);
					},
				},
				{ label: "내보내기…", icon: Download, onselect: () => convActions.exportMenu(c) },
				{ label: "삭제", icon: Trash2, danger: true, separator: true, onselect: () => convActions.remove(c) },
			],
		};
	}
	async function copyMark(text: string) {
		await navigator.clipboard.writeText(text).then(
			() => toasts.success("북마크 글 복사함"),
			() => toasts.error("복사 못 함"),
		);
	}
	async function removeCharacter(c: Character, leave = true) {
		const n = library.convsOf(c.id).length;
		if (!(await confirmAction({ title: "캐릭터 삭제", message: `'${c.name}'을(를) 휴지통으로 옮김${n ? ` (대화 ${n}개는 캐릭터 없이 남고, 되살리면 다시 이어짐)` : ""}. ${TRASH_HINT}`, confirmLabel: "삭제", danger: true }))) return;
		if (leave) nav.replace({ page: "character" });
		// 하나씩 빠르게 지워도 앞 것이 끝날 때까지 줄을 서서 기다린다 (5-E-28 — 전에는 "데이터베이스가 사용 중"으로 거절됨)
		const item = await deleteJob(c.id, c.name).catch(() => null);
		await library.load();
		if (item && item !== SKIPPED) trashedToast(item, () => library.load());
	}
	/** 캐릭터 하나를 휴지통으로 — 파일 작업 대기열에서 */
	function deleteJob(id: string, name: string) {
		return jobs.add(name, () => call(commands.deleteCharacter(id), { source: "character" }), { group: "캐릭터 지우기" });
	}
</script>

{#if !library.loaded}
	<div class="center"><Spinner /></div>
{:else if charId && convId}
	<!-- 읽기 모드로 둔 대화는 읽기 화면 (5-E-6-2, ADR-031) -->
	{#if library.conversation(convId)?.readMode}
		<ReadView {charId} {convId} onopen={(id) => open(charId, id)} />
	{:else}
		<ChatView {charId} {convId} onopen={(id) => open(charId, id)} />
	{/if}
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
			class:no-cover={!character.cover}
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
				<div class="cover-tools" class:on={!character.cover}>
					<Button size="sm" variant="secondary" onclick={() => changeCover(character.id)}>{#snippet icon()}<ImagePlus />{/snippet}{character.cover ? "커버 바꾸기" : "커버 넣기"}</Button>
					{#if character.cover}
						<Button size="sm" variant="secondary" onclick={() => startPosition(character.coverY ?? 50)}>{#snippet icon()}<Move />{/snippet}위치 조절</Button>
						<Button size="sm" variant="secondary" onclick={() => changeCover(character.id, true)}>{#snippet icon()}<XIcon />{/snippet}커버 지우기</Button>
					{/if}
				</div>
			{/if}
		</div>
		<header class="profile" class:no-cover={!character.cover}>
			<button class="portrait" title="아바타 바꾸기" aria-label="아바타 바꾸기" onclick={() => changeAvatar(character.id)}>
				<Avatar name={character.name} src={fileSrc(character.avatar)} fit={character.avatarFit} color={character.color} size={96} />
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
			<Segmented
				label="캐릭터 탭"
				bind:value={tab}
				options={[
					{ value: "overview", label: "개요" },
					{ value: "recent", label: `채팅 목록 ${recent.length}` },
					{ value: "bookmarks", label: `북마크 ${marks.length}` },
					{ value: "gallery", label: "갤러리" },
					{ value: "stats", label: "통계" },
				]}
			/>
		</div>

		<div class="body">
			{#if tab === "overview"}
				{#if character.creatorNotes?.trim() || character.description.trim()}
					<div class="intro"><RichText blocks={overviewBlocks(character)} images={overviewImages} /></div>
				{:else}
					<p class="none">소개 없음 · <button class="link" onclick={() => nav.go({ page: "character", charId: character.id, edit: true })}>편집에서 쓰기</button></p>
				{/if}
			{:else if tab === "bookmarks"}
				{#if marks.length}
					<div class="sub-toolbar">
						<ListToolbar bind:query={markQuery} bind:filter={markFilter} bind:sort={markSort} placeholder="북마크·대화 제목" filters={MARK_FILTERS} sorts={MARK_SORTS} />
					</div>
				{/if}
				<ul class="recent marks">
					{#each filteredMarks as b (b.conv.id + b.id)}
						<li class="recent-row">
							<button class="row-main" onclick={() => open(character.id, b.conv.id)}>
								<span><b class="mark-text">{b.text}</b><small>{b.conv.title} · {relative(b.createdAt)}</small></span>
								<ChevronRight />
							</button>
							<div class="row-acts">
								<IconButton size="sm" label="복사" onclick={() => copyMark(b.text)}><Copy /></IconButton>
								<IconButton size="sm" variant="danger" label="북마크 지우기" onclick={() => bookmarks.remove(b.conv, b.id)}><Trash2 /></IconButton>
							</div>
						</li>
					{:else}
						<li class="none">{marks.length ? "검색 결과 없음" : '북마크 없음 · 대화에서 글을 끌어 고른 뒤 "북마크"'}</li>
					{/each}
				</ul>
			{:else if tab === "gallery"}
				<CharacterGallery charId={character.id} />
			{:else if tab === "stats"}
				{#if !stats}
					<p class="none"><Spinner /></p>
				{:else}
					{@const max = Math.max(1, ...stats.weekdays)}
					<dl class="stats">
						<div><dt>누적 대화 시간</dt><dd>{duration(stats.chatSeconds)}</dd></div>
						<div><dt>대화방</dt><dd>{stats.conversations}개</dd></div>
						<div><dt>메시지</dt><dd>{stats.messages.toLocaleString()}개</dd></div>
						{#if stats.firstAt}<div><dt>처음 대화</dt><dd title={absolute(stats.firstAt)}>{relative(stats.firstAt)}</dd></div>{/if}
					</dl>
					<div class="week" role="img" aria-label="요일별 메시지 수: {DAYS.map((d, i) => `${d} ${stats?.weekdays[i] ?? 0}`).join(', ')}">
						{#each stats.weekdays as n, i (i)}
							<div class="day"><span class="bar" style:height="{(n / max) * 100}%" title="{DAYS[i]}요일 {n}개"></span><small>{DAYS[i]}</small></div>
						{/each}
					</div>
					<p class="faint-note">같은 대화에서 30분 안에 이어진 메시지 사이를 대화 시간으로 셈</p>
				{/if}
			{:else}
				{#if recent.length}
					<div class="sub-toolbar">
						<ListToolbar bind:query={convQuery} bind:filter={convFilter} bind:sort={convSort} placeholder="대화 제목" filters={CONV_FILTERS} sorts={CONV_SORTS} />
					</div>
				{/if}
				<ul class="recent">
					{#each filteredRecent as c (c.id)}
						<li class="recent-row" oncontextmenu={(e) => openConvMenu(e, c)}>
							<button class="row-main" onclick={() => open(character.id, c.id)}>
								<span><b>{c.title}</b><small title={absolute(lastAt(c))}>마지막 활동 · {relative(lastAt(c))} · {c.messageCount}개</small></span>
								<ChevronRight />
							</button>
							<div class="row-acts">
								<IconButton size="sm" label="{c.title} 메뉴" onclick={(e) => openConvMenu(e, c)}><Ellipsis /></IconButton>
							</div>
						</li>
					{:else}
						<li class="none">{recent.length ? "검색 결과 없음" : "아직 대화 없음"}</li>
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
			<div class="title-wrap">
				<h1>{$_("nav.character")}</h1>
				<span class="head-count">{library.characters.length}{$_("common.unitPerson")}</span>
			</div>
			<ListToolbar bind:query bind:filter bind:sort bind:selecting placeholder="이름·태그" filters={FILTERS} sorts={SORTS} canSelect={!!library.characters.length}>
				<SplitButton label={$_("common.create")} onclick={createCharacter} menuLabel={$_("common.import")} items={[{ label: "파일에서 가져오기 (PNG·JSON·ZIP)", icon: Upload, onselect: importFiles }]} />
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
					{@const cardImg = fileSrc(ch.avatar) ?? fileSrc(ch.cover)}
					{@const imgPos = ch.avatar && ch.avatarFit ? `${ch.avatarFit.x ?? 50}% ${ch.avatarFit.y ?? 50}%` : !ch.avatar && ch.cover ? `center ${ch.coverY ?? 50}%` : undefined}
					{@const summary = firstLine(ch.description)}
					{@const selected = picked.has(ch.id)}
					<!-- 카드 전체가 버튼 역할. 안쪽에 메뉴 버튼이 있어 div[role=button] 사용 -->
					<div
						class="char-card"
						class:selected
						role="button"
						tabindex="0"
						aria-pressed={selecting ? selected : undefined}
						aria-label={ch.name}
						onclick={() => (selecting ? pickChar(ch.id) : open(ch.id))}
						onkeydown={(e) => {
							if (e.target !== e.currentTarget) return;
							if (e.key === "Enter" || e.key === " ") {
								e.preventDefault();
								if (selecting) pickChar(ch.id);
								else open(ch.id);
							}
						}}
						oncontextmenu={(e) => openCardMenu(e, ch)}
					>
						{#if cardImg}
							<img class="char-img" src={cardImg} alt="" loading="lazy" style:object-position={imgPos} />
						{:else}
							<div class="char-fallback" style:--card-bg={ch.color ?? `oklch(0.45 0.08 ${[...ch.name].reduce((h, c) => (h * 31 + c.charCodeAt(0)) % 360, 17)})`}>
								<span class="char-initial" aria-hidden="true">{(ch.name.trim()[0] ?? "?").toUpperCase()}</span>
							</div>
						{/if}
						<div class="char-top">
							{#if selecting}
								<span class="char-check" aria-hidden="true">{selected ? "✓" : ""}</span>
							{:else if ch.favorite}
								<span class="char-fav" title="즐겨찾기"><Star /></span>
							{:else}
								<span></span>
							{/if}
							{#if !selecting}
								<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
								<span class="char-menu" onclick={(e) => e.stopPropagation()}>
									<IconButton label="{ch.name} 메뉴" size="sm" onclick={(e) => openCardMenu(e, ch)}><Ellipsis /></IconButton>
								</span>
							{/if}
						</div>
						<div class="char-overlay">
							<b class="char-name" title={ch.name}>{ch.name}</b>
							{#if summary}<p class="char-desc">{summary}</p>{/if}
							<div class="char-meta">
								{#each ch.tags.slice(0, 3) as t (t)}<span class="char-tag">#{t}</span>{/each}
								{#if convs.length}<span class="char-count">대화 {convs.length}</span>{/if}
							</div>
						</div>
					</div>
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
	.grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(min(200px, 44%), 1fr));
		gap: var(--space-4);
		max-width: var(--page-wide);
		margin: 0 auto;
	}
	.char-card {
		position: relative;
		aspect-ratio: 3 / 4;
		border-radius: var(--radius-lg);
		border: 1px solid var(--border);
		background: var(--surface);
		overflow: hidden;
		cursor: pointer;
		-webkit-user-select: none;
		user-select: none;
		-webkit-touch-callout: none;
		transition:
			transform var(--duration-fast) var(--ease),
			border-color var(--duration-fast) var(--ease),
			box-shadow var(--duration-fast) var(--ease);
	}
	.char-card:hover {
		border-color: var(--border-strong);
		box-shadow: var(--shadow-md);
		transform: translateY(-2px);
	}
	.char-card:focus-visible {
		outline: 2px solid var(--focus-ring);
		outline-offset: 2px;
	}
	.char-card.selected {
		border-color: var(--accent);
		box-shadow: 0 0 0 2px var(--accent);
	}
	.char-img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		display: block;
		transition: transform 240ms var(--ease);
	}
	.char-card:hover .char-img {
		transform: scale(1.03);
	}
	.char-fallback {
		position: absolute;
		inset: 0;
		display: grid;
		place-items: center;
		background: linear-gradient(160deg, var(--card-bg), color-mix(in oklab, var(--card-bg) 55%, #000));
	}
	.char-initial {
		font-size: 3rem;
		font-weight: 700;
		color: rgb(255 255 255 / 0.85);
		text-shadow: 0 2px 10px rgb(0 0 0 / 0.25);
		transform: translateY(-16%);
	}
	.char-top {
		position: absolute;
		top: var(--space-2);
		left: var(--space-2);
		right: var(--space-2);
		display: flex;
		align-items: center;
		justify-content: space-between;
		z-index: 2;
		pointer-events: none;
	}
	.char-check {
		display: grid;
		place-items: center;
		width: 24px;
		height: 24px;
		border: 2px solid #fff;
		border-radius: 6px;
		background: rgb(0 0 0 / 0.45);
		color: #fff;
		font-size: 13px;
		font-weight: 700;
	}
	.char-card.selected .char-check {
		background: var(--accent);
	}
	.char-fav {
		display: grid;
		place-items: center;
		width: 26px;
		height: 26px;
		border-radius: var(--radius-full);
		background: rgb(0 0 0 / 0.45);
		color: #facc15;
		backdrop-filter: blur(4px);
	}
	.char-fav :global(svg) {
		width: 14px;
		height: 14px;
		fill: currentColor;
	}
	.char-menu {
		pointer-events: auto;
		border-radius: var(--radius-full);
		background: rgb(0 0 0 / 0.42);
		backdrop-filter: blur(4px);
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.char-menu :global(button) {
		color: #fff;
	}
	.char-card:hover .char-menu,
	.char-card:focus-within .char-menu {
		opacity: 1;
	}
	@media (hover: none) {
		.char-menu {
			opacity: 1;
		}
	}
	.char-overlay {
		position: absolute;
		inset: auto 0 0 0;
		padding: var(--space-10) var(--space-3) var(--space-3);
		background: linear-gradient(to top, rgb(0 0 0 / 0.88) 0%, rgb(0 0 0 / 0.56) 55%, transparent 100%);
		color: #fff;
		display: flex;
		flex-direction: column;
		gap: 4px;
		z-index: 1;
		pointer-events: none;
	}
	.char-name {
		font-size: var(--text-md);
		font-weight: 700;
		line-height: 1.25;
		color: #fff;
		text-shadow: 0 1px 3px rgb(0 0 0 / 0.5);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.char-desc {
		margin: 0;
		font-size: var(--text-xs);
		line-height: 1.4;
		color: rgb(255 255 255 / 0.88);
		text-shadow: 0 1px 2px rgb(0 0 0 / 0.45);
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
	}
	.char-meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 4px;
		margin-top: 2px;
		min-height: 20px;
	}
	.char-tag,
	.char-count {
		font-size: 11px;
		line-height: 1.2;
		padding: 2px 6px;
		border-radius: var(--radius-full);
		background: rgb(255 255 255 / 0.16);
		color: rgb(255 255 255 / 0.92);
		backdrop-filter: blur(4px);
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.detail {
		height: 100%;
		overflow: auto;
	}
	.banner {
		position: relative;
		height: 200px;
	}
	.banner.no-cover {
		height: auto;
		max-width: var(--page-medium);
		margin: 0 auto;
		padding: var(--space-4) var(--space-6) 0;
		display: flex;
		justify-content: flex-end;
	}
	.banner.no-cover .cover-tools {
		position: static;
	}
	.banner.has-cover {
		background-size: cover;
		background-position: center;
		background-repeat: no-repeat;
	}
	.sub-toolbar {
		display: flex;
		justify-content: flex-end;
		margin-top: var(--space-3);
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
	.profile.no-cover {
		margin-top: var(--space-2);
		align-items: center;
	}
	.profile.no-cover .ident,
	.profile.no-cover .cta {
		margin-top: 0;
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
		align-items: center;
		gap: var(--space-1);
		padding-bottom: var(--space-1);
	}
	.sub {
		display: flex;
		align-items: center;
	}
	.stats {
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(140px, 1fr));
		gap: var(--space-3);
		margin: var(--space-5) 0 0;
	}
	.stats div {
		padding: var(--space-3) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-md);
	}
	.stats dt {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.stats dd {
		margin: var(--space-1) 0 0;
		font-size: var(--text-lg);
		font-weight: 600;
	}
	.week {
		display: grid;
		grid-template-columns: repeat(7, 1fr);
		gap: var(--space-2);
		height: 140px;
		margin-top: var(--space-5);
	}
	.day {
		display: flex;
		flex-direction: column;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-1);
		min-height: 0;
	}
	.bar {
		width: 60%;
		min-height: 2px;
		border-radius: var(--radius-sm) var(--radius-sm) 0 0;
		background: var(--accent);
	}
	.day small,
	.faint-note {
		color: var(--text-muted);
		font-size: var(--text-xs);
	}
	.mark-text {
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		font-weight: 500;
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
		border: 1px solid var(--border);
		border-radius: var(--radius-lg);
		background: var(--surface);
		overflow: hidden;
	}
	.recent-row {
		display: flex;
		align-items: center;
		gap: var(--space-1);
		padding-right: var(--space-2);
		transition: background var(--duration-fast) var(--ease);
	}
	.recent-row:hover {
		background: var(--surface-hover);
	}
	.recent li + li {
		border-top: 1px solid var(--border);
	}
	.row-main {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--space-3);
		padding: var(--space-3) var(--space-4);
		border: none;
		background: none;
		text-align: left;
		cursor: pointer;
	}
	.row-main span {
		flex: 1;
		min-width: 0;
		display: flex;
		flex-direction: column;
		gap: 2px;
	}
	.recent small {
		color: var(--text-faint);
		font-size: var(--text-xs);
	}
	.row-main :global(svg) {
		width: 16px;
		height: 16px;
		flex: none;
		color: var(--text-faint);
	}
	.row-main:hover b {
		color: var(--accent-text);
	}
	.row-acts {
		display: flex;
		align-items: center;
		gap: 2px;
		flex: none;
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
	/* 휴대폰 폭: 좌우 여백을 홈과 같게 줄여 도구 줄(찾기·거르기·정렬·선택 + 새로 만들기)이 한 줄에 들어가게 */
	@media (max-width: 600px) {
		.page {
			padding: var(--space-4);
		}
	}
	/* 좁은 화면: 이름이 한 줄에 잘리지 않게 버튼 줄은 아래로 내리고, 긴 이름은 줄바꿈 */
	.ident h1 {
		overflow-wrap: anywhere;
	}
	@media (max-width: 900px) {
		.profile {
			grid-template-columns: auto minmax(0, 1fr);
			padding: 0 var(--space-4);
		}
		.cta {
			grid-column: 1 / -1;
			flex-wrap: wrap;
			margin-top: 0;
		}
	}
</style>
