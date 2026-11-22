<script lang="ts">
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { tick, untrack } from "svelte";
	import { Bookmark, ChevronRight, Gauge, Copy, CornerUpLeft, FastForward, GitBranch, ImagePlus, Images, IdCard, Languages, ListChecks, ListTree, PanelLeft, Plus, RefreshCw, Send, SlidersHorizontal, Square, Trash2, X, EllipsisVertical, BookOpen, Pen, Database, Scroll, SpellCheck, Pencil, UserRound, PinOff, Pin, FolderMinus, Download, Folder } from "@lucide/svelte";
	import { bookmarks, bookmarksOf, type ChatBookmark } from "./bookmarks";
	import { findTextRange } from "./highlight";
	import { childrenOf, pathOf } from "./branches";
	import SidePanel from "$app/SidePanel.svelte";
	import { layout } from "$app/layout.svelte";
	import { fade, fly } from "svelte/transition";
	import { drops, extOf, isImage, storeFile } from "$app/drop.svelte";
	import { Button, type MenuItem } from "$shared/ui";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { ContextMenu, Dialog, EmptyState, IconButton, Spinner, Textarea } from "$shared/ui";
	import Ghostwrite from "$features/ghost/Ghostwrite.svelte";
	import { ghost } from "$features/ghost/ghost.svelte";
	import { panel } from "$app/panel.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	appSetting.load();
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import type { PromptMessage_Serialize } from "$ipc/bindings";
	import ChatPanel from "./ChatPanel.svelte";
	import ConvSidebar from "./ConvSidebar.svelte";
	import TextCardDialog from "./TextCardDialog.svelte";
	import { registerComposer } from "$shared/html/bridge";
	import MessageItem from "./MessageItem.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { translateInput } from "$shared/stores/aiTools.svelte";
	import { toBlocks } from "./format";
	import RichText from "./RichText.svelte";
	import { chatImport } from "./chatImport.svelte";
	import { cgVariants, pickVariants } from "./cgVariants";
	import { viewed } from "./viewed.svelte";
	import { library } from "./library.svelte";
	import { ChatSession, metaOf, type Message } from "./session.svelte";
	import { call, commands } from "$/shared/ipc/client";
	import { convActions } from "./actions";
	import { nav } from "$/app/nav.svelte";

	let { charId, convId, onopen }: { charId: string; convId: string; onopen: (id: string) => void } = $props();

	const conv = $derived(library.conversation(convId));
	// "최근 본" 정렬용: 대화를 열 때마다 (이 기기에만)
	$effect(() => {
		const id = convId;
		untrack(() => viewed.mark(id));
	});
	const character = $derived(library.character(charId));

	// 대화가 바뀌면 새 세션
	let session = $state<ChatSession | null>(null);
	$effect(() => {
		const s = new ChatSession(convId);
		session = s;
		shownFrom = -1;
		void s.load().then(() => {
			// 불러온 뒤에는 시작을 고정 (새 메시지가 와도 위쪽이 사라지지 않게)
			shownFrom = Math.max(0, s.messages.length - PAGE);
			void scrollToEnd(true);
		});
	});

	// ---- 긴 대화: 최근 PAGE개만 그리고, 위로 올리면 더 그린다 (1-D-5) ----
	// 메시지 2000개를 한 번에 그리면 화면이 수십 초 멈췄다. 데이터(session.messages)는 전부 두고 그리는 것만 나눈다
	const PAGE = 80;
	/** 그리기 시작하는 메시지 번호. -1 = 아직 정하지 않음(최근 PAGE개) — 불러오자마자 전부 그리는 일이 없게 */
	let shownFrom = $state(-1);
	const start = $derived.by(() => {
		const len = session?.messages.length ?? 0;
		return shownFrom < 0 ? Math.max(0, len - PAGE) : Math.min(shownFrom, len);
	});
	const visible = $derived(session ? session.messages.slice(start) : []);
	/** 위쪽에 PAGE개 더 (읽던 자리는 그대로) */
	async function showEarlier(target = Math.max(0, start - PAGE)) {
		if (!scroller || start === 0) return;
		const before = scroller.scrollHeight - scroller.scrollTop;
		shownFrom = Math.max(0, target);
		await tick();
		if (scroller) scroller.scrollTop = scroller.scrollHeight - before;
	}
	$effect(() => {
		if (!library.images[charId]) void library.loadImages(charId);
	});

	// 화면용 글 (표시용 정규식·{{char}}/{{user}} 이름). 메시지가 바뀔 때마다
	$effect(() => {
		const s = session;
		if (!s) return;
		void s.messages.map((m) => m.content);
		// 정규식은 대화 → 캐릭터 기본 → 앱 설정 프리셋에 딸리기도 해서, 셋 중 무엇이 바뀌어도 다시 만든다
		const key = `${conv?.personaId}:${conv?.promptPresetId}:${character?.defaultPromptPresetId}:${appSetting.value?.selectedPreset}:${regexVersion.n}`;
		if (!s.busy) void s.refreshDisplay(key);
	});

	// 오른쪽 패널 탭은 타이틀바에 그려진다
	$effect(() =>
		panel.register([
			{ id: "settings", label: "대화 설정", icon: SlidersHorizontal },
			{ id: "context", label: "컨텍스트·프리셋", icon: Gauge },
			{ id: "toc", label: "목차", icon: ListTree },
			{ id: "bookmarks", label: "북마크", icon: Bookmark },
			...(plugins.on("summary") || plugins.on("memory") ? [{ id: "memory", label: "메모리", icon: Database }] : []),
			...(plugins.on("lint") ? [{ id: "lint", label: "linter", icon: SpellCheck }] : []),
			{ id: "branches", label: "분기", icon: GitBranch },
			{ id: "assets", label: "에셋", icon: Images },
		]),
	);

	const LIST_KEY = "sc.chat.list";
	let showList = $state((() => {
		try {
			return localStorage.getItem(LIST_KEY) !== "0";
		} catch {
			return true;
		}
	})());
	// 휴대폰: 목록은 겹쳐 여는 서랍, 기본 닫힘 (기억하지 않음)
	let mobileList = $state(false);
	const listShown = $derived(layout.narrow ? mobileList : showList);
	function toggleList() {
		if (layout.narrow) return void (mobileList = !mobileList);
		showList = !showList;
		try {
			localStorage.setItem(LIST_KEY, showList ? "1" : "0");
		} catch {
			/* 무시 */
		}
	}

	/** <cg>키</cg> → 이미지 주소 (켜진 것만이 아니라 전부: 예전 응답도 그대로 보이게) */
	/** 받는 중인 새 응답 (끝의 <image> 그림 프롬프트는 저장할 때 빠지므로 숨김) */
	const liveBlocks = $derived(session?.live?.kind === "send" ? toBlocks(session.live.text.replace(/<image>[\s\S]*?(<\/image>|$)/i, ""), { safeHtml: plugins.on("html") }) : []);
	/** 메시지 프로필 그림 (5-E-9): 대화 설정에서 켰을 때만. AI는 그 캐릭터, 내 글은 보낸 페르소나(없으면 대화의 페르소나) */
	const showAvatars = $derived(!!(conv?.meta as { showAvatars?: boolean } | null)?.showAvatars);
	function avatarOf(m: Message): string | null | undefined {
		if (!showAvatars) return undefined;
		const path =
			m.role === "assistant"
				? library.character(m.characterId ?? charId)?.avatar
				: m.role === "user"
					? library.persona((m.meta as { sender?: { id?: string } } | null)?.sender?.id ?? conv?.personaId)?.avatar
					: null;
		return path ? fileSrc(path) : null;
	}
	// 키 → 그림들. "같은 키 묶기"(5-E-25)를 켜면 smile-1·smile-2도 smile로 찾고, 메시지마다 하나를 고른다
	const imageVariants = $derived(cgVariants(library.imagesInUse(charId).map((i) => ({ key: i.key, src: imageSrc(i) ?? null })), !!appSetting.value?.cgGroupVariants));
	const images = $derived(pickVariants(imageVariants, "live"));

	// 챕터 번호
	const chapters = $derived.by(() => {
		const out: Record<string, number> = {};
		let n = 0;
		for (const m of session?.messages ?? []) if (metaOf(m).chapter) out[m.id] = ++n;
		return out;
	});

	// ---- 스크롤: 맨 아래 근처면 새 내용에 따라감 ----
	let scroller: HTMLDivElement | undefined = $state();
	let stick = true;
	function onscroll() {
		if (!scroller) return;
		stick = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight < 80;
		if (scroller.scrollTop < 300 && start > 0) void showEarlier();
	}
	async function scrollToEnd(force = false) {
		await tick();
		if (scroller && (force || stick)) scroller.scrollTop = scroller.scrollHeight;
	}
	$effect(() => {
		void session?.messages.length;
		void session?.live?.text;
		void scrollToEnd();
	});
	export async function jumpTo(id: string, mark?: ChatBookmark) {
		// 아직 그리지 않은 앞쪽 메시지면 거기까지 먼저 그린다
		const at = session?.messages.findIndex((m) => m.id === id) ?? -1;
		if (at >= 0 && at < start) {
			shownFrom = Math.max(0, at - 5);
			await tick();
		}
		const block = document.getElementById(`msg-${id}`);
		// 북마크면 그 글이 있는 자리로 (접혔거나 다른 스와이프라 못 찾으면 메시지 처음으로)
		const hit = block && mark ? findTextRange(block, mark.text) : null;
		const target = hit ? (hit.startContainer.parentElement ?? block) : block;
		target?.scrollIntoView({ behavior: "smooth", block: hit ? "center" : "start" });
	}
	/** 패널에서 이동: 폰에서는 패널이 화면을 덮으므로 닫고 그 자리를 바로 보여줌 */
	function jumpFromPanel(id: string, mark?: ChatBookmark) {
		if (layout.narrow && panel.open) panel.toggle();
		return jumpTo(id, mark);
	}
	/** 메시지별 북마크 (본문에 형광펜으로 칠함) */
	const marksByMsg = $derived.by(() => {
		const map = new Map<string, ChatBookmark[]>();
		for (const b of bookmarksOf(conv)) map.set(b.messageId, [...(map.get(b.messageId) ?? []), b]);
		return map;
	});

	// ---- 입력 ----
	let draft = $state("");
	/** 다음 메시지에 붙일 이미지 (보낼 때 디스크에 저장) */
	let pending = $state<{ file: File; url: string }[]>([]);
	const MAX_ATTACH = 6;
	function attach(files: File[]) {
		const imgs = files.filter(isImage).slice(0, Math.max(0, MAX_ATTACH - pending.length));
		if (imgs.length < files.filter(isImage).length) toasts.info(`이미지는 한 번에 ${MAX_ATTACH}장까지`);
		pending = [...pending, ...imgs.map((file) => ({ file, url: URL.createObjectURL(file) }))];
	}
	function unattach(i: number) {
		URL.revokeObjectURL(pending[i].url);
		pending = pending.filter((_, j) => j !== i);
	}
	function onpaste(e: ClipboardEvent) {
		const files = Array.from(e.clipboardData?.files ?? []).filter(isImage);
		if (!files.length) return;
		e.preventDefault();
		attach(files);
	}
	let sending = $state(false);
	let fileInput: HTMLInputElement | undefined = $state();
	async function send() {
		if (!session || session.busy || sending) return;
		// 빈 입력도 보낼 수 있다: 사용자 메시지 없이 AI가 이어서 답함 (답이 오류로 안 왔을 때 다시 시도)
		const text = draft;
		const items = pending;
		sending = true;
		let paths: string[];
		try {
			paths = await Promise.all(items.map((p) => storeFile(p.file, true)));
		} catch {
			sending = false;
			return; // 토스트는 call이 띄움. 입력과 첨부는 그대로 둠
		}
		draft = "";
		pending = [];
		sending = false;
		stick = true;
		await session.send(text, paths, items.map((p) => p.url), ghost.takeUsed(session.convId));
		for (const p of items) URL.revokeObjectURL(p.url);
	}

	let readerEl: HTMLElement | undefined = $state();
	// 파일을 끌어다 놓으면: 이미지는 첨부, .jsonl은 이 캐릭터의 대화로 가져오기
	$effect(() =>
		drops.register({
			priority: 1,
			// 읽는 영역에 놓은 것만 (오른쪽 패널은 에셋·프리셋이 받음)
			zone: () => readerEl,
			label: "이미지 첨부 · 대화 가져오기",
			hint: "이미지는 다음 메시지에 붙고, SillyTavern 채팅(.jsonl)은 이 캐릭터의 대화로 들어옴",
			accept: (f) => isImage(f) || extOf(f) === "jsonl",
			ondrop: async (files) => {
				attach(files.filter(isImage));
				await chatImport.run(files.filter((x) => extOf(x) === "jsonl"), charId, onopen);
			},
		}),
	);
	function onkeydown(e: KeyboardEvent) {
		if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
		// 설정: Enter로 보내기. 끄면 Ctrl+Enter
		if (appSetting.sendOnEnter || e.ctrlKey || e.metaKey) {
			e.preventDefault();
			void send();
		}
	}
	let plusMenu = $state<{ x: number; y: number } | null>(null);
	function openPlus(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		plusMenu = { x: r.left, y: r.top - 8 - 200 };
	}

	// ---- HTML 블록의 선택지 → 입력칸 / 보내기 ----
	$effect(() =>
		registerComposer(({ action, text }) => {
			if (action === "send" && session && !session.busy) {
				draft = text;
				void send();
			} else {
				draft = text;
			}
		}),
	);

	// ---- 글을 끌어서 고르면: 북마크 / 복사 ----
	let selMenu = $state<{ x: number; y: number; text: string; messageId: string } | null>(null);
	/** 본문이 아닌 곳: 머리줄(번호·이름·시각)·도구 줄·번역 안내·접기·CG 그림 이름 */
	const inMeta = (n: Element | null) => !!n?.closest(".who, .actions, .swipe, .tr-note, .fold, figcaption");
	/** 고른 글 중 한 메시지 본문에 든 부분. 끝까지 끌어 도구 줄·다음 칸까지 넘어가도 본문만 잘라 쓴다 */
	function pickedText(): { text: string; messageId: string; rect: DOMRect } | null {
		const sel = window.getSelection();
		if (!sel || sel.isCollapsed || !sel.rangeCount) return null;
		const el = (n: Node | null) => (n instanceof Element ? n : (n?.parentElement ?? null));
		if (el(sel.anchorNode)?.closest("textarea, input, [contenteditable='true']")) return null;
		const block = el(sel.anchorNode)?.closest<HTMLElement>("[id^=msg-]") ?? el(sel.focusNode)?.closest<HTMLElement>("[id^=msg-]");
		if (!block || !scroller?.contains(block)) return null;
		const range = sel.getRangeAt(0);
		let text = "";
		let lastLine: Element | null = null;
		const walker = document.createTreeWalker(block, NodeFilter.SHOW_TEXT);
		for (let n = walker.nextNode(); n; n = walker.nextNode()) {
			if (!range.intersectsNode(n) || inMeta(n.parentElement)) continue;
			const raw = n.textContent ?? "";
			const s = n === range.startContainer ? range.startOffset : 0;
			const e = n === range.endContainer ? range.endOffset : raw.length;
			const line = n.parentElement?.closest("p, h2, h3, h4, li, blockquote, pre") ?? null;
			if (lastLine && line !== lastLine) text += "\n";
			lastLine = line;
			text += raw.slice(s, e);
		}
		text = text.trim();
		return text ? { text, messageId: block.id.slice(4), rect: range.getBoundingClientRect() } : null;
	}
	/** 손가락: 꾹 눌러 고르는 동안 메뉴를 띄우면 포커스가 옮겨 가 고르기가 끊긴다 → 손을 떼고 고른 범위가 멈춘 뒤에 띄운다 */
	const coarse = typeof matchMedia !== "undefined" && matchMedia("(pointer: coarse)").matches;
	let touching = false;
	let selTimer: ReturnType<typeof setTimeout> | undefined;
	function menuAfterSettle() {
		clearTimeout(selTimer);
		selTimer = setTimeout(() => {
			if (touching || selecting || selMenu) return;
			const p = pickedText();
			if (p) selMenu = { x: p.rect.left + p.rect.width / 2, y: p.rect.bottom + 8, text: p.text, messageId: p.messageId };
		}, 450);
	}
	$effect(() => {
		if (!coarse) return;
		const down = () => {
			touching = true;
			clearTimeout(selTimer);
		};
		const up = () => {
			touching = false;
			menuAfterSettle();
		};
		const change = () => {
			if (!touching) menuAfterSettle();
		};
		document.addEventListener("touchstart", down, { passive: true });
		document.addEventListener("touchend", up, { passive: true });
		document.addEventListener("touchcancel", up, { passive: true });
		document.addEventListener("selectionchange", change);
		return () => {
			clearTimeout(selTimer);
			document.removeEventListener("touchstart", down);
			document.removeEventListener("touchend", up);
			document.removeEventListener("touchcancel", up);
			document.removeEventListener("selectionchange", change);
		};
	});
	/** 글을 끌어서 고른 뒤 오른쪽 클릭하면 고른 글 메뉴 (폰은 손을 뗀 뒤 저절로). 고치는 중(입력칸)이거나 고른 글이 없으면 띄우지 않는다 */
	function onSelectMenu(e: MouseEvent) {
		if (selecting) return;
		if ((e.target as Element | null)?.closest("textarea, input, [contenteditable='true']")) return;
		const p = pickedText();
		if (!p) return;
		e.preventDefault();
		if (coarse) return menuAfterSettle();
		selMenu = { x: e.clientX, y: e.clientY, text: p.text, messageId: p.messageId };
	}
	// 고른 글 → 이미지 카드
	let card = $state<{ text: string; speaker?: string; time?: number } | null>(null);
	function cardSelection() {
		const m = selMenu;
		if (!m) return;
		const msg = session?.messages.find((x) => x.id === m.messageId);
		card = { text: m.text, speaker: msg?.name ?? (msg?.role === "assistant" ? character?.name : undefined), time: msg?.createdAt };
	}
	/** 북마크 탭에서: 북마크한 글을 이미지 카드로 */
	function cardFromMark(b: ChatBookmark) {
		const msg = session?.messages.find((x) => x.id === b.messageId);
		card = { text: b.text, speaker: msg?.name ?? (msg?.role === "assistant" ? character?.name : undefined), time: msg?.createdAt };
	}
	async function bookmarkSelection() {
		const m = selMenu; // 메뉴가 닫히며 selMenu가 비워지므로 먼저 잡아 둠
		if (!m || !conv) return;
		const msg = session?.messages.find((x) => x.id === m.messageId);
		await bookmarks.add(conv, m.messageId, m.text, msg ? (metaOf(msg).swipeId ?? 0) : 0);
		window.getSelection()?.removeAllRanges();
	}

	// ---- 북마크 삭제 (본문의 형광펜을 눌러서) ----
	async function removeMark(b: ChatBookmark) {
		if (conv) await bookmarks.remove(conv, b.id);
	}

	// ---- 챕터 이름 바꾸기 (오른쪽 클릭 / 꾹 누르기) ----
	async function renameChapter(m: Message) {
		const cur = metaOf(m).chapter ?? "";
		const t = await promptText({ title: "챕터 이름 바꾸기", message: "비우면 챕터 표시를 지움", input: { value: cur, placeholder: "챕터 제목" }, confirmLabel: "바꾸기" });
		if (t === null || t === undefined) return;
		await session?.setChapter(m, t.trim() || null);
	}
	let pressTimer: ReturnType<typeof setTimeout> | undefined;
	function pressStart(e: PointerEvent, m: Message) {
		if (e.pointerType === "mouse") return;
		clearTimeout(pressTimer);
		pressTimer = setTimeout(() => void renameChapter(m), 550);
	}
	function pressEnd() {
		clearTimeout(pressTimer);
	}

	// ---- 분기: 부모 대화 ----
	const branchInfo = $derived((conv?.meta as { branch?: { parentId: string; messageId: string; seq: number } } | null)?.branch ?? null);
	const parentConv = $derived(branchInfo ? library.conversation(branchInfo.parentId) : undefined);
	function goParent() {
		if (!parentConv || !branchInfo) return;
		openAt(parentConv.id, branchInfo.messageId);
	}
	/** 다른 대화를 열고, 주면 그 메시지로 스크롤 (갈라진 지점) */
	function openAt(id: string, messageId?: string) {
		if (id === convId) return messageId && jumpTo(messageId);
		onopen(id);
		if (messageId) setTimeout(() => jumpTo(messageId), 400);
	}
	// 분기 경로: 맨 위 조상 › … › 지금. 갈래가 있으면 그 수도
	const path = $derived(conv ? pathOf(conv) : []);
	const kids = $derived(conv ? childrenOf(conv.id).length : 0);

	// ---- 여러 메시지 고르기 / 삭제 ----
	let selecting = $state(false);
	let picked = $state<Set<string>>(new Set());
	function startSelect() {
		picked = new Set();
		selecting = true;
	}
	function pick(m: Message, on: boolean) {
		const next = new Set(picked);
		if (on) next.add(m.id);
		else next.delete(m.id);
		picked = next;
	}
	function pickAll() {
		picked = picked.size === (session?.messages.length ?? 0) ? new Set() : new Set(session?.messages.map((m) => m.id));
	}
	async function removePicked() {
		if (!session || !picked.size) return;
		if (!(await confirmAction({ title: "메시지 삭제", message: `고른 메시지 ${picked.size}개를 지움. 되돌릴 수 없음`, confirmLabel: `${picked.size}개 삭제`, danger: true }))) return;
		const n = await session.removeMany([...picked]);
		toasts.success(`${n}개 지움`);
		selecting = false;
		picked = new Set();
	}
	function cancelPicked() {
		selecting = false;
		picked = new Set();
	}
	$effect(() => {
		void convId;
		selecting = false;
	});

	// ---- 분기 / 보낸 컨텍스트 ----
	async function branch(m: Message) {
		const c = await library.branch(convId, m.id).catch(() => null);
		if (c) onopen(c.id);
	}
	let prompt = $state<{ msg: Message; data: PromptMessage_Serialize | null } | null>(null);
	async function showPrompt(m: Message) {
		const data = await session?.sentPrompt(m).catch(() => null);
		prompt = { msg: m, data: data ?? null };
	}
	const roleName = { system: "시스템", user: "사용자", assistant: "AI" } as const;

	// ---- 읽기 모드 ----
	const readMode = $derived(!!conv?.readMode);
	function toggleReadMode() {
		if (conv) void library.setReadMode(conv.id, !conv.readMode);
	}
	
	// ---- 채팅 파일 관리 ----
	let menu = $state<{ x: number; y: number; align?: "start" | "end" } | null>(null);
	const menuItems = $derived.by((): MenuItem[] => {
		const c = conv;
		if (!c) return [];
		const parent = (c.meta as { branch?: { parentId: string } } | null)?.branch?.parentId;
		const folders = library.foldersOf(charId ?? "");
		return [
			...(charId ? [{ label: "캐릭터 페이지로", icon: UserRound, onselect: () => nav.go({ page: "character", charId: charId ?? undefined }) }] : []),
			{ label: "이름 바꾸기", icon: Pencil, separator: !!charId, onselect: startRename },
			{ label: c.pinned ? "고정 풀기" : "고정", icon: c.pinned ? PinOff : Pin, onselect: () => library.togglePin(convId) },
			{
				label: "복제 (포크)",
				icon: Copy,
				separator: true,
				onselect: async () => {
					const d = await library.duplicate(convId);
					if (d) nav.go({ page: "character", charId: d.characterId ?? undefined, convId: d.id });
				},
			},
			{
				label: "마지막 메시지에서 분기",
				icon: GitBranch,
				disabled: !c.messageCount,
				onselect: async () => {
					const [last] = await call(commands.getMessages(convId, 1, null), { source: "chat" });
					if (!last) return;
					const b = await library.branch(convId, last.id);
					nav.go({ page: "character", charId, convId: b.id });
					toasts.success("분기를 만듦");
				},
			},
			...(parent && library.conversation(parent)
				? [{ label: "갈라져 나온 대화로", icon: CornerUpLeft, onselect: () => nav.go({ page: "character", charId, convId: parent }) }]
				: []),
			...folders.map((f, i) => ({
				label: `폴더로 이동: ${f.name}`,
				icon: Folder,
				separator: i === 0,
				checked: c.folderId === f.id,
				onselect: () => library.move(c.id, f.id),
			})),
			...(c.folderId ? [{ label: "폴더에서 빼기", icon: FolderMinus, onselect: () => library.move(convId, null) }] : []),
			{ label: "내보내기…", icon: Download, separator: true, onselect: () => convActions.exportMenu(c) },
			{ label: "삭제", icon: Trash2, danger: true, onselect: () => convActions.remove(c) },

		]
	})

	function openMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = { x: r.right, y: r.bottom + 4, align: "end" };
	}

	async function startRename() {
		if (!conv) return;
		const cur = conv.title ?? "";
		const t = await promptText({ title: "대화방 이름 변경", input: { value: cur, placeholder: "대화방 이름" }, confirmLabel: "바꾸기" });
		if (t === null || t === undefined) return;
		if (draft.trim() && draft.trim() !== conv.title) await library.rename(conv.id, draft);
	}
</script>

<div class="chat" class:no-list={!listShown || layout.narrow} class:no-panel={!panel.open || layout.narrow}>
	{#if listShown && layout.narrow}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (목록 버튼으로도 닫힘) -->
		<div class="list-scrim" transition:fade={{ duration: 150 }} onclick={() => (mobileList = false)}></div>
		<div class="list-drawer" transition:fly={{ x: -320, duration: 200 }}>
			<ConvSidebar {charId} {convId} onopen={(id) => { mobileList = false; onopen(id); }} oncreate={async () => { mobileList = false; onopen((await library.createConversation(charId)).id); }} onclose={toggleList} />
		</div>
	{:else if listShown}
		<ConvSidebar {charId} {convId} {onopen} oncreate={async () => onopen((await library.createConversation(charId)).id)} onclose={toggleList} />
	{/if}

	<section class="reader" bind:this={readerEl}>
		<header class="bar" class:listing={listShown}>
			<div class="left">
				<IconButton label={listShown ? "대화 목록 닫기" : "대화 목록 열기"} size="sm" active={listShown} onclick={toggleList}><Scroll /></IconButton>
				<!-- <div class="branch-nav">
					{#if path.length > 1}
						<nav class="bpath" aria-label="분기 경로">
							<GitBranch />
							{#each path.slice(0, -1) as step, i (step.conv?.id ?? `gone-${i}`)}
								{#if step.conv}
									<button onclick={() => openAt(step.conv!.id, step.branchAt?.messageId)} title="'{step.conv.title}'로 (#{step.branchAt?.seq}에서 갈라짐)">
										{step.conv.title}<span>#{step.branchAt?.seq}</span>
									</button>
								{:else}
									<span class="gone" title="원래 대화는 지워졌음">지운 대화<span>#{step.branchAt?.seq}</span></span>
								{/if}
								<ChevronRight />
							{/each}
							<b>지금</b>
						</nav>
					{/if}
					{#if kids}
						<button class="branch" onclick={() => { panel.tab = "branches"; if (!panel.open) panel.toggle(); }} title="이 대화에서 갈라진 대화 보기">
							<GitBranch />갈래 {kids}개
						</button>
					{/if}
				</div> -->
			</div>
			<div>
				{#if readMode}
					<IconButton label={"채팅 모드"} size="sm" onclick={toggleReadMode}><Pen /></IconButton>
				{:else}
					<IconButton label={"읽기 모드"} size="sm" onclick={toggleReadMode}><BookOpen /></IconButton>
				{/if}
				<IconButton label={"세부 정보"} size="sm" onclick={openMenu}><EllipsisVertical /></IconButton>
				<ContextMenu
					at={menu}
					label="파일 관리"
					items={menuItems}
					onclose={() => (menu = null)}
				/>
			</div>
		</header>
		{#if selecting}
			<div class="selbar" role="toolbar" aria-label="메시지 선택">
				<b>{picked.size}개 선택</b>
				<Button size="sm" variant="ghost" onclick={pickAll}>{picked.size === (session?.messages.length ?? 0) && picked.size ? "모두 해제" : "모두 선택"}</Button>
				<span class="sp"></span>
				<Button size="sm" variant="danger" disabled={!picked.size} onclick={removePicked}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
				<IconButton label="선택 끝내기" size="sm" onclick={cancelPicked}><X /></IconButton>
			</div>
		{/if}

		<!-- svelte-ignore a11y_no_static_element_interactions (글 고르기 끝 감지) -->
		<div class="scroll" bind:this={scroller} {onscroll} oncontextmenu={onSelectMenu}>
			{#if !session || session.loading}
				<div class="center"><Spinner /></div>
			{:else if !conv}
				<EmptyState title="대화를 찾을 수 없음" description="지워졌거나 아직 불러오지 못함" />
			{:else}
				<article class="text">
					{#if !session.messages.length && !session.live}
						<p class="empty">첫 메시지를 보내 보세요</p>
					{/if}
					{#if start > 0}
						<button class="earlier" onclick={() => showEarlier()}>이전 메시지 {Math.min(PAGE, start)}개 더 보기 <small>(앞에 {start}개)</small></button>
					{/if}
					{#each visible as m, j (m.id)}
						{@const i = start + j}
						{@const ch = metaOf(m).chapter}
						{#if ch}
							<!-- svelte-ignore a11y_no_noninteractive_element_interactions (오른쪽 클릭·꾹 누르기로 이름 바꾸기, 키보드는 메시지 동작의 챕터 버튼) -->
							<h2 class="chapter" id="chapter-{m.id}" title="오른쪽 클릭하거나 꾹 눌러 이름 바꾸기" oncontextmenu={(e) => { e.preventDefault(); void renameChapter(m); }} onpointerdown={(e) => pressStart(e, m)} onpointerup={pressEnd} onpointerleave={pressEnd}><span>{chapters[m.id]}장</span>{ch}</h2>
						{/if}
						<MessageItem msg={m} no={i + 1} avatar={avatarOf(m)} {session} images={pickVariants(imageVariants, m.id)} isLast={i === session.messages.length - 1} onbranch={branch} onprompt={showPrompt} {selecting} selected={picked.has(m.id)} onselect={pick} marks={marksByMsg.get(m.id)} oncard={cardFromMark} onmarkremove={removeMark} />
					{/each}

					{#if session.live?.kind === "send"}
						{#if session.live.userText || session.live.userImages}
							<div class="block user pending">
								<div class="who"><span class="no">#{session.messages.length + 1}</span><b>{library.persona(conv.personaId)?.name ?? "나"}</b><span class="stamp">보내는 중…</span></div>
								{#if session.live.userImages}<div class="attach-strip">{#each session.live.userImages as u (u)}<img src={u} alt="첨부 이미지" />{/each}</div>{/if}
								{#if session.live.userText}<p>{session.live.userText}</p>{/if}
							</div>
						{/if}
						<div class="block ai">
							<div class="who"><b>{character?.name ?? "AI"}</b><span class="stamp">생성 중…</span></div>
							{#if liveBlocks.length}
								<RichText blocks={liveBlocks} {images} />
							{:else}
								<p class="typing" aria-label="응답 기다리는 중"><span></span><span></span><span></span></p>
							{/if}
						</div>
					{/if}
				</article>
			{/if}
		</div>

		{#if !readMode}
			<div class="composer">
				{#if pending.length}
					<div class="attach-strip pending-strip" aria-label="붙일 이미지">
						{#each pending as p, i (p.url)}
							<span class="thumb">
								<img src={p.url} alt={p.file.name} title={p.file.name} />
								<button type="button" aria-label="{p.file.name} 빼기" onclick={() => unattach(i)}><X /></button>
							</span>
						{/each}
					</div>
				{/if}
			
				<form
					class="write"
					onsubmit={(e) => {
						e.preventDefault();
						void send();
					}}
				>
					<IconButton label="추가 기능" onclick={openPlus} disabled={!session || session.busy}><Plus /></IconButton>
					{#if session && plugins.on("ghostwrite")}
						<Ghostwrite convId={session.convId} getDraft={() => draft} setDraft={(v) => (draft = v)} disabled={session.busy} />
					{/if}
					<Textarea bind:value={draft} rows={1} autosize maxRows={8} placeholder={!layout.desktop ? "메시지 입력" : appSetting.sendOnEnter ? "메시지 입력 (Shift+Enter 줄바꿈)" : "메시지 입력 (Ctrl+Enter 보내기)"} aria-label="메시지" readonly={!!session && ghost.isRunning(session.convId)} {onkeydown} {onpaste} />
					{#if session?.busy}
						<IconButton label="멈추기 (받은 데까지 저장)" variant="solid" onclick={() => session?.cancel()}><Square /></IconButton>
					{:else}
						<IconButton label="보내기" variant="solid" type="submit" disabled={sending}><Send /></IconButton>
					{/if}
				</form>
			</div>
		{/if}
		<input
			bind:this={fileInput}
			type="file"
			accept="image/png,image/jpeg,image/webp,image/gif"
			multiple
			hidden
			onchange={(e) => {
				const input = e.currentTarget;
				attach(Array.from(input.files ?? []));
				input.value = "";
			}}
		/>
		<ContextMenu
			at={plusMenu}
			label="추가 기능"
			items={[
				{ label: "이미지 첨부", icon: ImagePlus, disabled: pending.length >= MAX_ATTACH, onselect: () => fileInput?.click() },
				...(plugins.on("translate")
					? [{ label: "입력 번역", icon: Languages, disabled: !draft.trim(), onselect: async () => (draft = await translateInput(draft).catch(() => draft)) }]
					: []),
				{ label: "이어서 응답 받기", icon: FastForward, separator: true, onselect: () => session?.send("") },
				{
					label: "마지막 응답 재생성",
					icon: RefreshCw,
					disabled: session?.last?.role !== "assistant",
					onselect: () => session?.regenerate(true),
				},
				{ label: listShown ? "대화 목록 닫기" : "대화 목록 열기", icon: PanelLeft, separator: true, onselect: toggleList },
				{ label: "메시지 여러 개 지우기", icon: ListChecks, disabled: !session?.messages.length, onselect: startSelect },
				...(parentConv ? [{ label: "갈라져 나온 대화로", icon: CornerUpLeft, onselect: goParent }] : []),
			]}
			onclose={() => (plusMenu = null)}
		/>
	</section>

	{#if panel.open && conv && session}
		<SidePanel label="대화 패널">
			<ChatPanel {conv} {session} {chapters} onjump={jumpFromPanel} oncard={cardFromMark} onopen={openAt} />
		</SidePanel>
	{/if}
	<ContextMenu
		at={selMenu}
		label="고른 글"
		items={[
			{
				label: "복사",
				icon: Copy,
				onselect: () =>
					navigator.clipboard.writeText(selMenu?.text ?? "").then(
						() => toasts.success("복사함"),
						() => toasts.error("복사 못 함"),
					),
			},
			{ label: "북마크", icon: Bookmark, onselect: bookmarkSelection },
			{ label: "이미지 카드로 만들기", icon: IdCard, onselect: cardSelection },
		]}
		onclose={() => (selMenu = null)}
	/>
</div>

<TextCardDialog {card} onclose={() => (card = null)} />

<Dialog open={!!prompt} title="보낸 컨텍스트" description={prompt ? `#${(session?.messages.indexOf(prompt.msg) ?? 0) + 1} 응답을 만들 때 보낸 프롬프트` : ""} size="lg" onclose={() => (prompt = null)}>
	{#if prompt?.data}
		{@const total = prompt.data.messages.reduce((s, m) => s + m.token, 0)}
		<p class="ptotal">메시지 {prompt.data.messages.length}개 · 약 {total.toLocaleString()} 토큰</p>
		<ol class="plist">
			{#each prompt.data.messages as pm, i (i)}
				<li>
					<div class="prole"><b>{roleName[pm.role]}</b><span>{pm.token} 토큰</span></div>
					<pre>{pm.content}</pre>
				</li>
			{/each}
		</ol>
	{:else if prompt}
		<p class="ptotal">기록 없음 (가져온 메시지, 멈춘 응답, 이 기능 이전에 만든 응답)</p>
	{/if}
</Dialog>

<style>
	.earlier {
		display: block;
		margin: 0 auto var(--space-4);
		padding: var(--space-2) var(--space-4);
		border: 1px solid var(--border);
		border-radius: var(--radius-full, 999px);
		background: var(--surface);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-sm);
		cursor: pointer;
	}
	.earlier small {
		color: var(--text-faint);
	}
	.chat {
		display: grid;
		grid-template-columns: 280px minmax(0, 1fr) var(--panel-width);
		grid-template-rows: minmax(0, 1fr);
		height: 100%;
	}
	.chat.no-list {
		grid-template-columns: minmax(0, 1fr) var(--panel-width);
	}
	.chat.no-panel {
		grid-template-columns: 280px minmax(0, 1fr);
	}
	.chat.no-list.no-panel {
		grid-template-columns: minmax(0, 1fr);
	}
	.reader {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		background: var(--surface);
		position: relative;
	}
	.bar {
		position: absolute;
		z-index: var(--z-sticky);
		display: flex;
		justify-content: space-between;
		padding: var(--space-2) var(--space-3) 0;
		width: 100%;
	}
	/* 대화 목록 버튼은 마우스를 올리거나 키보드로 갔을 때만 보임 */
	.bar :global(.icon-btn) {
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
		background: var(--surface);
	}
	.bar:hover :global(.icon-btn),
	.bar :global(.icon-btn:focus-visible),
	.bar.listing :global(.icon-btn) {
		opacity: 1;
	}
	.bar {
		align-items: center;
		gap: var(--space-2);
	}
	.bar .left {
		display: flex;
	}

	/* .branch-nav {
		display: flex;
		align-items: center;
		background: var(--surface);
	}
	.bpath {
		display: flex;
		align-items: center;
		gap: 2px;
		min-width: 0;
		overflow: hidden;
		font-size: var(--text-xs);
		color: var(--text-muted);
		background: var(--surface);
	}
	.bpath > :global(svg) {
		flex: none;
		width: 13px;
		height: 13px;
		margin-right: 4px;
		color: var(--accent);
	}
	.bpath :global(svg:not(:first-child)) {
		flex: none;
		width: 12px;
		height: 12px;
		color: var(--text-faint);
	}
	.bpath button,
	.bpath .gone {
		display: inline-flex;
		align-items: baseline;
		gap: 4px;
		min-width: 0;
		max-width: 180px;
		padding: 3px 6px;
		border: none;
		border-radius: var(--radius-xs);
		background: transparent;
		color: inherit;
		font: inherit;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.bpath button:hover {
		background: var(--surface-hover);
		color: var(--text);
	}
	.bpath .gone {
		cursor: default;
		font-style: italic;
	}
	.bpath span span,
	.bpath button span {
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.bpath b {
		flex: none;
		padding: 3px 6px;
		color: var(--text);
	}
	.branch {
		display: inline-flex;
		align-items: center;
		gap: 6px;
		min-width: 0;
		height: 26px;
		padding: 0 var(--space-2);
		border: 1px solid var(--border);
		border-radius: var(--radius-full);
		background: var(--surface-2);
		color: var(--text-muted);
		font: inherit;
		font-size: var(--text-xs);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		cursor: pointer;
	}
	.branch:hover {
		border-color: var(--accent);
		color: var(--text);
	}
	.branch :global(svg) {
		width: 13px;
		height: 13px;
		flex-shrink: 0;
	} */
	.attach-strip {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-bottom: var(--space-2);
	}
	.attach-strip img {
		width: 72px;
		height: 72px;
		border-radius: var(--radius-md);
		object-fit: cover;
		box-shadow: 0 0 0 1px var(--border);
	}
	.pending-strip {
		margin: 0;
		padding: var(--space-2) var(--space-4) 0;
	}
	.thumb {
		position: relative;
	}
	.thumb button {
		position: absolute;
		top: -6px;
		right: -6px;
		display: grid;
		place-items: center;
		width: 20px;
		height: 20px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: var(--text);
		color: var(--bg);
		cursor: pointer;
	}
	.thumb button :global(svg) {
		width: 12px;
		height: 12px;
	}
	.list-scrim {
		position: fixed;
		inset: 0;
		z-index: var(--z-drawer);
		background: rgb(0 0 0 / 0.4);
	}
	.list-drawer {
		position: fixed;
		top: 0;
		bottom: 0;
		left: 0;
		z-index: calc(var(--z-drawer) + 1);
		display: flex;
		width: min(88vw, 340px);
		padding-top: var(--safe-top);
		padding-bottom: var(--safe-bottom);
		background: var(--bg);
		box-shadow: var(--shadow-lg);
	}
	.list-drawer > :global(.convs) {
		flex: 1;
		width: 100%;
	}
	.selbar {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin: var(--space-2) var(--space-3) 0;
		padding: var(--space-1) var(--space-2) var(--space-1) var(--space-3);
		border: 1px solid var(--accent);
		border-radius: var(--radius-md);
		background: var(--accent-soft);
		font-size: var(--text-sm);
	}
	.selbar .sp {
		flex: 1;
	}
	.scroll {
		flex: 1;
		overflow: auto;
	}
	.center {
		display: grid;
		place-items: center;
		height: 100%;
	}
	.text {
		max-width: var(--page-reading);
		margin: 0 auto;
		padding: var(--space-6) var(--space-6) var(--space-10);
		font-size: 15.5px;
		line-height: 1.85;
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
	}
	.empty {
		padding: var(--space-10) 0;
		text-align: center;
		color: var(--text-faint);
	}
	.chapter {
		display: flex;
		flex-direction: column;
		align-items: center;
		margin: var(--space-8) 0 var(--space-4);
		font-size: var(--text-xl);
		text-align: center;
		scroll-margin-top: var(--space-6);
	}
	.chapter:first-child {
		margin-top: 0;
	}
	.chapter span {
		font-size: var(--text-xs);
		color: var(--accent-text);
		letter-spacing: 0.2em;
		font-weight: 700;
	}
	/* 응답 만드는 중 (저장 전) */
	.block {
		margin: 0 calc(var(--space-3) * -1);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
	}
	.block p {
		white-space: pre-wrap;
	}
	.block.user {
		border-left: 3px solid var(--accent);
		border-radius: 0 var(--radius-md) var(--radius-md) 0;
		background: var(--accent-soft);
	}
	.pending {
		opacity: 0.75;
	}
	.who {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		min-height: 28px;
		font-size: var(--text-xs);
		color: var(--text-muted);
		line-height: 1;
	}
	.no,
	.stamp {
		color: var(--text-faint);
	}
	.typing {
		display: flex;
		gap: 4px;
		padding: var(--space-2) 0;
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-faint);
		animation: blink 1.2s infinite;
	}
	.typing span:nth-child(2) {
		animation-delay: 0.2s;
	}
	.typing span:nth-child(3) {
		animation-delay: 0.4s;
	}
	@keyframes blink {
		50% {
			opacity: 0.2;
		}
	}
	.composer {
		display: flex;
		flex-direction: column;
		max-width: var(--page-narrow);
		width: 100%;
		margin: 0 auto;
	}

	.write {
		display: flex;
		align-items: flex-end;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.write :global(.textarea),
	.write :global(textarea) {
		flex: 1;
	}
	.write :global(textarea) {
		resize: none;
		border-radius: var(--radius-lg);
	}
	.ptotal {
		font-size: var(--text-sm);
		color: var(--text-muted);
		margin-bottom: var(--space-3);
	}
	.plist {
		list-style: none;
		margin: 0;
		padding: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-3);
	}
	.prole {
		display: flex;
		justify-content: space-between;
		font-size: var(--text-xs);
		color: var(--text-muted);
		margin-bottom: 4px;
	}
	.plist pre {
		margin: 0;
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-sm);
		background: var(--bg-sunken);
		font-family: var(--font-mono);
		font-size: 12px;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		max-height: 320px;
		overflow: auto;
	}
</style>
