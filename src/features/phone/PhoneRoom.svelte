<script lang="ts">
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { onMount, tick } from "svelte";
	import { BookOpen, ChevronLeft, ChevronsDownUp, ChevronsUpDown, Copy, ExternalLink, Eye, FileSearch, ImagePlus, Languages, Menu, MessageCircle, MessageCirclePlus, Pencil, Plus, RefreshCw, Send, Square, Trash2, UserRoundPen, Volume2, X } from "@lucide/svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import Ghostwrite from "$features/ghost/Ghostwrite.svelte";
	import { ghost } from "$features/ghost/ghost.svelte";
	import { drawMessage, generatedImagesOf, speak, speech, translateMessage, translationOf } from "$shared/stores/aiTools.svelte";
	import { Avatar, Button, ContextMenu, HtmlView, Textarea, type MenuItem } from "$shared/ui";
	import { call, commands } from "$shared/ipc/client";
	import { IMAGE_FILES, pickFiles } from "$shared/ipc/dialog";
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { theme } from "$app/theme.svelte";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { ChatSession, type Message } from "$features/character/session.svelte";
	import { toBlocks } from "$features/character/format";
	import RichText from "$features/character/RichText.svelte";
	import SelectionMenu, { type CardInput } from "$features/character/SelectionMenu.svelte";
	import TextCardDialog from "$features/character/TextCardDialog.svelte";
	import SentPromptDialog from "$features/character/SentPromptDialog.svelte";
	import type { ChatBookmark } from "$features/character/bookmarks";
	import { findTextRange } from "$features/character/highlight";
	import { bubbles, lookup, parsePeek, phone, type BubbleLine } from "./phone.svelte";
	import PhoneProfile, { type ProfileTarget } from "./PhoneProfile.svelte";
	import { splitSpeakers } from "./speakers";
	import TypingIndicator from "$features/character/TypingIndicator.svelte";
	import { isSlash, runSlash, slashQueue } from "$features/slash/commands";
	import QuickReplyBar from "$features/slash/QuickReplyBar.svelte";
	import { slashRuns } from "$features/slash/running.svelte";
	import { registerComposer } from "$shared/html/bridge";
	import { isStatusPin } from "$features/status/pin";
	import { panel } from "$app/panel.svelte";

	let { conv, onback, session: given }: { conv: Conversation; onback: () => void; session?: ChatSession } = $props();

	// 폰 화면이 만든 세션을 같이 씀 (오른쪽 패널의 목차·컨텍스트도 같은 메시지를 봄)
	// svelte-ignore state_referenced_locally
	const session = given ?? new ChatSession(conv.id);
	// 상대(연락처)와 나(폰 주인)
	const ch = $derived(lookup(phone.contactOf(conv)));
	const me = $derived(phone.owner);
	// 멀티 프로필 그림: 테마 설정에서 켰으면 답장의 `이름: 대사` 줄을 사람별 말풍선으로
	const multi = $derived(theme.chat.showMultiCharAvatars);
	/** 말한 사람의 그림: 대화 상대 → 같은 이름의 캐릭터 카드 → 이름 첫 글자 */
	function speakerLook(name: string | undefined) {
		if (!name || name === ch?.name) return { name: ch?.name ?? "?", avatar: ch?.avatar ?? null, avatarFit: ch?.avatarFit ?? null, color: ch?.color };
		const c = library.characters.find((x) => x.name === name);
		return { name, avatar: c?.avatar ?? null, avatarFit: c?.avatarFit ?? null, color: undefined };
	}
	/** 답장 한 개 → 말한 사람별 묶음 (단체 대화가 아니면 대화 상대 하나) */
	function speakers(text: string): { speaker?: string; text: string }[] {
		return multi ? splitSpeakers(text, ch?.name ?? "") : [{ text }];
	}
	const imgChar = $derived(ch?.kind === "character" ? ch.id : null);
	const images = $derived(Object.fromEntries((library.images[imgChar ?? ""] ?? []).map((i) => [i.key, imageSrc(i) ?? ""])));
	let draft = $state("");
	let scroller: HTMLDivElement | undefined = $state();
	/** 입력창 옆 + 버튼 아래 메뉴 열림 여부 */
	let plusOpen = $state(false);
	/** 사칭 모드: 켜면 내가 보낸 메시지가 상대방(assistant) 메시지로 전송됨 */
	let impersonating = $state(false);
	/** 첨부할 이미지 경로 목록 */
	let pendingImages = $state<string[]>([]);
	/** 접은 메시지 id */
	let folded = $state<Record<string, boolean>>({});
	/** 인라인 편집 중인 메시지 id와 초안 */
	let editingId = $state<string | null>(null);
	let editDraft = $state("");
	/** 소설 화면 UI 모드 (페이지 이동 없이 UI만 소설형으로 전환) */
	let novelView = $state(false);
	/** 이미지 카드 다이얼로그 */
	let card = $state<CardInput | null>(null);
	/** 보낸 컨텍스트 보기 다이얼로그 */
	let promptView = $state<{ msg: Message; data: Awaited<ReturnType<ChatSession["sentPrompt"]>> | null } | null>(null);
	/** 인스타그램 스타일 캐릭터 개인 페이지 (아바타 클릭 시 폰 내부에서 열림) */
	let profileTarget = $state<ProfileTarget | null>(null);
	function openProfilePage(speakerName?: string) {
		if (speakerName && speakerName !== ch?.name) {
			const c = library.characters.find((x) => x.name === speakerName);
			if (c) {
				profileTarget = {
					key: `character:${c.id}`,
					name: c.name,
					avatar: c.avatar,
					avatarFit: c.avatarFit ?? null,
					color: c.color ?? null,
					who: { kind: "character", id: c.id },
				};
				return;
			}
			profileTarget = {
				key: `speaker:${conv.id}:${speakerName}`,
				name: speakerName,
				avatar: null,
			};
			return;
		}
		if (!ch) return;
		profileTarget = {
			key: `${ch.kind}:${ch.id}`,
			name: ch.name,
			avatar: ch.avatar,
			avatarFit: ch.avatarFit,
			color: ch.color,
			who: { kind: ch.kind, id: ch.id },
		};
	}

	export function getDraft() {
		return draft;
	}
	export function setDraft(v: string) {
		draft = v;
	}
	export async function jumpTo(id: string, mark?: ChatBookmark) {
		await tick();
		const block = scroller?.querySelector<HTMLElement>(`#msg-${CSS.escape(id)}`) ?? document.getElementById(`msg-${id}`);
		const hit = block && mark ? findTextRange(block, mark.text) : null;
		const target = hit ? (hit.startContainer.parentElement ?? block) : block;
		target?.scrollIntoView({ behavior: "smooth", block: hit ? "center" : "start" });
	}
	export function openCard(mark: ChatBookmark) {
		const m = session.messages.find((x) => x.id === mark.messageId);
		card = { text: mark.text, speaker: m?.name ?? (m?.role === "assistant" ? ch?.name : me?.name), time: m?.createdAt };
	}

	onMount(async () => {
		void appSetting.load();
		if (imgChar && !library.images[imgChar]) void library.loadImages(imgChar);
		await session.load();
		phone.markRead(conv.id);
		await scrollDown();
	});

	async function scrollDown() {
		await tick();
		scroller?.scrollTo({ top: scroller.scrollHeight });
	}
	// 화면용 글 (표시용 정규식·이름)
	$effect(() => {
		void session.messages.map((m) => m.content);
		const key = `${regexVersion.n}`;
		if (!session.busy) void session.refreshDisplay(key);
	});

	// 답장이 들어오는 동안 아래로 따라감
	$effect(() => {
		void session.live?.text;
		void session.messages.length;
		void scrollDown();
	});

	async function done() {
		phone.markRead(conv.id);
		phone.setPreview(conv.id, session.messages);
	}
	async function pickImages() {
		const paths = await pickFiles([IMAGE_FILES], true);
		if (!paths.length) return;
		const imported = await call(commands.importGalleryImages(paths), { source: "phone" });
		const savedPaths = imported.map((i) => i.path).filter(Boolean);
		if (!savedPaths.length) return;
		pendingImages = [...pendingImages, ...savedPaths].slice(0, 6);
		plusOpen = false;
	}
	function removePendingImage(i: number) {
		pendingImages = pendingImages.filter((_, j) => j !== i);
	}
	async function send() {
		const text = draft.trim();
		const imgs = [...pendingImages];
		if ((!text && !imgs.length) || session.busy) return;
		draft = "";
		pendingImages = [];
		plusOpen = false;
		// 사칭 모드: 상대방(assistant) 역할로 메시지를 직접 추가
		if (impersonating) {
			impersonating = false;
			const saved = await call(
				commands.appendMessage({
					conversationId: conv.id,
					seq: (session.last?.seq ?? 0) + 1,
					role: "assistant",
					name: ch?.name ?? conv.title,
					content: text,
					characterId: ch?.kind === "character" ? ch.id : null,
					meta: {
						sender: { type: "assistant", name: ch?.name ?? conv.title, id: ch?.id ?? null },
						impersonated: true,
						...(imgs.length ? { attachments: imgs } : {}),
					},
				}),
				{ source: "phone" },
			);
			session.messages = [...session.messages, saved];
			await library.refresh(conv.id);
			await done();
			return;
		}
		// 슬래시 커맨드 (코어 플러그인 slash): 대화 화면과 같이 메시지로 보내지 않고 스크립트로 (5-E-20-6-13)
		if (plugins.on("slash") && !imgs.length && isSlash(text)) {
			await runSlash(text, session, { input: "", setInput: (v) => (draft = v) });
			await done();
			return;
		}
		await session.send(text, imgs, imgs.map((p) => fileSrc(p) ?? ""), ghost.takeUsed(session.convId), "phone");
		await done();
	}

	// ---- HTML 블록의 선택지 → 입력칸 / 보내기 / 슬래시(triggerSlash) — 대화 화면과 같은 규칙 ----
	const queueSlash = slashQueue();
	$effect(() =>
		registerComposer(({ action, text }) => {
			if (action === "slash") return queueSlash(text, session, () => ({ input: draft, setInput: (v) => (draft = v) }));
			draft = text;
			if (action === "send" && !session.busy) void send();
		}),
	);
	/** 스크립트가 이 대화에서 도는 중 (보내기 자리가 멈추기로) */
	const scriptRunning = $derived(slashRuns.isRunning(session.convId));
	/** 내 메시지 없이 답장만 더 받기 */
	async function nudge() {
		if (session.busy) return;
		await session.send("", [], [], false, "phone");
		await done();
	}
	/** 엿보기: 두 사람 몫을 AI가 이어서 씀 */
	async function peek() {
		if (session.busy) return;
		await session.peek();
		await done();
	}
	async function again() {
		await session.regenerate(false);
		await done();
	}
	function onkeydown(e: KeyboardEvent) {
		if (e.key !== "Enter" || e.shiftKey || e.isComposing) return;
		if (appSetting.sendOnEnter || e.ctrlKey || e.metaKey) {
			e.preventDefault();
			void send();
		}
	}

	// ---- 화면에 그릴 줄 ----
	interface Row {
		key: string;
		kind: "date" | "msg";
		date?: string;
		mine?: boolean;
		msg?: Message;
		lines?: BubbleLine[];
		time?: string;
		/** 같은 사람이 이어서 보내면 이름·아바타 생략 */
		cont?: boolean;
		/** 단체 대화에서 말한 사람 (없으면 대화 상대) */
		speaker?: string;
		pending?: boolean;
	}
	const day = (t: number) => new Date(t).toLocaleDateString("ko-KR", { year: "numeric", month: "long", day: "numeric", weekday: "long" });
	const hm = (t: number) => new Date(t).toLocaleTimeString("ko-KR", { hour: "numeric", minute: "2-digit" });

	const rows = $derived.by(() => {
		const out: Row[] = [];
		let lastDay = "";
		let lastSide: boolean | null = null;
		let lastSpeaker: string | undefined;
		const live = session.live;
		const push = (r: Omit<Row, "kind" | "key"> & { key: string; at: number }) => {
			const d = day(r.at);
			if (d !== lastDay) {
				out.push({ key: `d${r.at}`, kind: "date", date: d });
				lastDay = d;
				lastSide = null;
			}
			out.push({ ...r, kind: "msg", cont: lastSide === r.mine && lastSpeaker === r.speaker, time: hm(r.at) });
			lastSide = r.mine ?? null;
			lastSpeaker = r.speaker;
		};
		for (const m of session.messages) {
			const tr = plugins.on("translate") ? translationOf(m) : null;
			const content = live?.kind === "regenerate" && live.targetId === m.id ? live.text : tr && !originals[m.id] ? tr.text : (session.display[m.id] ?? m.content);
			// 이미지 생성 플러그인으로 그린 사진 + 사용자가 첨부한 사진은 말풍선 뒤에 사진으로
			const attachments = Array.isArray((m.meta as { attachments?: unknown } | null)?.attachments)
				? ((m.meta as { attachments: string[] }).attachments).map((p) => ({ kind: "photo" as const, text: p }))
				: [];
			const photos = [
				...attachments,
				...(plugins.on("imagegen") ? generatedImagesOf(m).map((g) => ({ kind: "photo" as const, text: g.path })) : []),
			];
			const pending = live?.targetId === m.id;
			if (m.role === "assistant" && multi) {
				const parts = speakers(content);
				parts.forEach((p, i) => {
					const lines = [...bubbles(p.text), ...(i === parts.length - 1 ? photos : [])];
					if (lines.length) push({ key: `${m.id}:${i}`, at: m.createdAt, mine: false, msg: m, lines, speaker: p.speaker, pending });
				});
				if (!parts.length && (photos.length || pending)) push({ key: m.id, at: m.createdAt, mine: false, msg: m, lines: photos, pending });
				continue;
			}
			const lines = [...bubbles(content), ...photos];
			if (!lines.length && !pending) continue;
			push({ key: m.id, at: m.createdAt, mine: m.role === "user", msg: m, lines, pending });
		}
		if (live?.kind === "peek") {
			const now = Date.now();
			const parts = parsePeek(live.text, me?.name ?? "", ch?.name ?? "");
			parts.forEach((p, i) => push({ key: `peek-${i}`, at: now, mine: p.mine, lines: bubbles(p.text), pending: true }));
			if (!parts.length) push({ key: "peek-wait", at: now, mine: false, lines: [], pending: true });
		}
		if (live?.kind === "send") {
			const now = Date.now();
			if (live.userText) push({ key: "live-user", at: now, mine: true, lines: bubbles(live.userText) });
			const parts = speakers(live.text).filter((p) => p.text);
			if (!parts.length) push({ key: "live-ai", at: now, mine: false, lines: [], pending: true });
			parts.forEach((p, i) => push({ key: `live-ai-${i}`, at: now, mine: false, lines: bubbles(p.text), speaker: p.speaker, pending: true }));
		}
		return out;
	});
	/** 상대가 아직 답하지 않은 내 마지막 메시지 → "1" */
	const unreadMine = $derived.by(() => {
		const last = rows.filter((r) => r.kind === "msg" && !r.pending).at(-1);
		return last?.mine ? last.key : null;
	});
	const lastAi = $derived([...session.messages].reverse().find((m) => m.role === "assistant"));

	// ---- 메뉴 ----
	let menu = $state<{ at: { x: number; y: number; align?: "start" | "end" }; items: MenuItem[] } | null>(null);
	/** 번역이 있는 메시지 중 원문을 보기로 한 것 */
	let originals = $state<Record<string, boolean>>({});

	function startEdit(m: Message) {
		editingId = m.id;
		editDraft = m.content;
	}
	function cancelEdit() {
		editingId = null;
		editDraft = "";
	}
	async function saveEdit(m: Message) {
		const next = editDraft;
		editingId = null;
		if (next !== m.content) {
			await session.edit(m, next);
			phone.setPreview(conv.id, session.messages);
		}
	}
	async function showPrompt(m: Message) {
		const data = await session.sentPrompt(m).catch(() => null);
		promptView = { msg: m, data: data ?? null };
	}

	function bubbleMenu(e: MouseEvent, r: Row) {
		if (!r.msg || session.busy || editingId) return;
		if (window.getSelection()?.toString().trim()) return;
		e.preventDefault();
		const m = r.msg;
		const isFolded = !!folded[m.id];
		menu = {
			at: { x: e.clientX, y: e.clientY },
			items: [
				{
					label: "복사",
					icon: Copy,
					onselect: () =>
						navigator.clipboard.writeText(m.content).then(
							() => toasts.success("복사함"),
							() => toasts.error("복사 못 함"),
						),
				},
				{ label: "편집", icon: Pencil, onselect: () => startEdit(m) },
				{ label: isFolded ? "펼치기" : "접기", icon: isFolded ? ChevronsUpDown : ChevronsDownUp, onselect: () => (folded[m.id] = !isFolded) },
				{ label: "보낸 메시지 컨텍스트 확인", icon: FileSearch, separator: true, onselect: () => void showPrompt(m) },
				...(m.id === session.last?.id && m.role === "assistant" ? [{ label: "답장 다시 받기", icon: RefreshCw, onselect: again }] : []),
				...(plugins.on("translate")
					? [
							translationOf(m)
								? { label: originals[m.id] ? "번역 보기" : "원문 보기", icon: Languages, onselect: () => (originals[m.id] = !originals[m.id]) }
								: { label: "번역", icon: Languages, onselect: async () => session.update(await translateMessage(m)) },
						]
					: []),
				...(plugins.on("tts") ? [{ label: speech.messageId === m.id ? "읽기 멈추기" : "소리 내어 읽기", icon: Volume2, onselect: () => speak(m, translationOf(m)?.text ?? m.content) }] : []),
				...(plugins.on("imagegen") && m.role === "assistant"
					? [{ label: "사진으로 그리기", icon: ImagePlus, onselect: async () => (toasts.info("그리는 중… (잠시 걸림)"), session.update(await drawMessage(m))) }]
					: []),
				{
					label: "삭제",
					icon: Trash2,
					danger: true,
					separator: true,
					onselect: async () => {
						if (await confirmAction({ title: "메시지 삭제", message: "이 메시지를 지움", confirmLabel: "삭제", danger: true })) {
							await session.remove(m);
							phone.setPreview(conv.id, session.messages);
						}
					},
				},
			],
		};
	}
	function roomMenu(e: MouseEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		menu = {
			at: { x: r.right, y: r.bottom + 4, align: "end" },
			items: [
				{ label: "답장 더 받기", icon: MessageCirclePlus, disabled: session.busy, onselect: nudge },
				{ label: "엿보기: 두 사람 대화를 AI가 이어 씀", icon: Eye, disabled: session.busy, onselect: peek },
				{ label: "마지막 답장 다시 받기", icon: RefreshCw, disabled: session.busy || !lastAi || session.last?.id !== lastAi.id, onselect: again },
				{ label: novelView ? "메신저 화면으로 보기" : "소설 화면으로 보기", icon: novelView ? MessageCircle : BookOpen, separator: true, onselect: () => (novelView = !novelView) },
				{ label: "대화 탭으로 옮기기", icon: ExternalLink, onselect: () => nav.go({ page: "character", charId: conv.characterId ?? undefined, convId: conv.id }) },
			],
		};
	}
</script>

{#if profileTarget}
	<PhoneProfile target={profileTarget} convId={conv.id} onback={() => (profileTarget = null)} />
{:else}
<div class="room sc-phone-room">
	<div class="room-bar sc-phone-room-bar">
		<button class="plain" aria-label="뒤로" onclick={onback}><ChevronLeft /></button>
		<strong>{ch?.name ?? conv.title}</strong>
		<button class="plain" aria-label="대화방 메뉴" onclick={roomMenu}><Menu /></button>
	</div>

	<div class="msgs sc-phone-msgs" class:novel={novelView} bind:this={scroller}>
		{#if session.loading}
			<p class="hint">불러오는 중…</p>
		{:else if !rows.length}
			<p class="hint">{ch?.name ?? "상대"}에게 첫 메시지를 보내보세요.{#if phone.isCharacterPhone}<br />또는 눈 버튼으로 두 사람 대화를 엿보세요.{/if}</p>
		{/if}
		{#if novelView}
			{#each session.messages as m, idx (m.id)}
				{@const mine = m.role === "user"}
				{@const tr = plugins.on("translate") ? translationOf(m) : null}
				{@const rawText = session.live?.kind === "regenerate" && session.live.targetId === m.id ? session.live.text : tr && !originals[m.id] ? tr.text : (session.display[m.id] ?? m.content)}
				{@const who = mine ? { name: me?.name ?? "나", avatar: me?.avatar ?? null, avatarFit: me?.avatarFit ?? null, color: me?.color } : speakerLook(m.name ?? undefined)}
				{@const attachments = Array.isArray((m.meta as { attachments?: unknown } | null)?.attachments) ? (m.meta as { attachments: string[] }).attachments : []}
				{@const genPhotos = plugins.on("imagegen") ? generatedImagesOf(m) : []}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					id="msg-{m.id}"
					class="novel-block"
					class:novel-user={mine}
					oncontextmenu={(e) => bubbleMenu(e, { key: m.id, kind: "msg", mine, msg: m })}
				>
					<div class="who">
						{#if !mine}
							<button type="button" class="av-btn" title="{who.name} 개인 페이지 보기" onclick={() => openProfilePage(m.name ?? undefined)}>
								<Avatar name={who.name} src={fileSrc(who.avatar)} fit={who.avatarFit} color={who.color} size={24} />
							</button>
						{:else}
							<Avatar name={who.name} src={fileSrc(who.avatar)} fit={who.avatarFit} color={who.color} size={24} />
						{/if}
						<span class="novel-no">#{idx + 1}</span>
						<b>{m.name ?? who.name}</b>
						<span class="novel-time">{hm(m.createdAt)}</span>
					</div>
					{#if editingId === m.id}
						<div class="inline-edit">
							<Textarea
								bind:value={editDraft}
								autosize
								maxRows={12}
								aria-label="메시지 편집"
								onkeydown={(e) => {
									if (e.key === "Escape") cancelEdit();
									if ((e.ctrlKey || e.metaKey) && e.key === "Enter") void saveEdit(m);
								}}
							/>
							<div class="inline-edit-btns">
								<Button size="sm" variant="ghost" onclick={cancelEdit}>취소</Button>
								<Button size="sm" variant="primary" onclick={() => saveEdit(m)}>저장</Button>
							</div>
						</div>
					{:else if folded[m.id]}
						<button type="button" class="b folded-b" onclick={() => (folded[m.id] = false)}>접힌 메시지 · 눌러서 펼치기</button>
					{:else}
						{#if attachments.length}
							<div class="novel-photos">{#each attachments as p, k (p + k)}<img class="photo" src={fileSrc(p)} alt="첨부 사진" loading="lazy" />{/each}</div>
						{/if}
						<div class="novel-body">
							<RichText blocks={toBlocks(rawText, { safeHtml: plugins.on("html") })} {images} />
						</div>
						{#if genPhotos.length}
							<div class="novel-photos">{#each genPhotos as g (g.path)}<img class="photo" src={fileSrc(g.path)} alt="그린 사진" loading="lazy" />{/each}</div>
						{/if}
					{/if}
				</div>
			{/each}
			{#if session.live?.kind === "send"}
				{#if session.live.userText}
					<div class="novel-block novel-user">
						<div class="who"><b>{me?.name ?? "나"}</b><span class="novel-time">보내는 중…</span></div>
						<div class="novel-body"><p>{session.live.userText}</p></div>
					</div>
				{/if}
				<div class="novel-block">
					<div class="who"><b>{ch?.name ?? "AI"}</b><span class="novel-time">생성 중…</span></div>
					{#if session.live.text}
						<div class="novel-body"><RichText blocks={toBlocks(session.live.text, { safeHtml: plugins.on("html") })} {images} /></div>
					{:else}
						<TypingIndicator name={ch?.name ?? ""} />
					{/if}
				</div>
			{/if}
		{:else}
		{#each rows as r (r.key)}
			{#if r.kind === "date"}
				<div class="date">{r.date}</div>
			{:else if r.mine}
				<div id={r.msg ? `msg-${r.msg.id}` : undefined} class="me sc-bubble-group sc-bubble-me" class:cont={r.cont} role="group" oncontextmenu={(e) => bubbleMenu(e, r)}>
					{#if r.msg && editingId === r.msg.id}
						<div class="inline-edit bubble-edit">
							<Textarea
								bind:value={editDraft}
								autosize
								maxRows={10}
								aria-label="메시지 편집"
								onkeydown={(e) => {
									if (e.key === "Escape") cancelEdit();
									if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && r.msg) void saveEdit(r.msg);
								}}
							/>
							<div class="inline-edit-btns">
								<Button size="sm" variant="ghost" onclick={cancelEdit}>취소</Button>
								<Button size="sm" variant="primary" onclick={() => r.msg && saveEdit(r.msg)}>저장</Button>
							</div>
						</div>
					{:else if r.msg && folded[r.msg.id]}
						<div class="line">
							<span class="t">{r.pending ? "" : r.time}</span>
							<button type="button" class="b folded-b" onclick={() => r.msg && (folded[r.msg.id] = false)}>접힌 메시지 · 눌러서 펼치기</button>
						</div>
					{:else}
						{#each r.lines ?? [] as l, i (i)}
							<div class="line">
								{#if i === (r.lines?.length ?? 0) - 1}
									<span class="t">{#if unreadMine === r.key}<span class="read">1</span>{/if}{r.pending ? "" : r.time}</span>
								{/if}
								{#if l.kind === "photo"}
									<img class="photo" src={fileSrc(l.text)} alt="첨부 사진" loading="lazy" />
								{:else}
									<div class="b sc-bubble">{l.text}</div>
								{/if}
							</div>
						{/each}
					{/if}
				</div>
			{:else}
				<div id={r.msg ? `msg-${r.msg.id}` : undefined} class="them sc-bubble-group sc-bubble-them" class:cont={r.cont} role="group" oncontextmenu={(e) => bubbleMenu(e, r)}>
					{#if !r.cont}{@const who = speakerLook(r.speaker)}<button type="button" class="av-btn" title="{who.name} 개인 페이지 보기" onclick={() => openProfilePage(r.speaker)}><Avatar name={who.name} src={fileSrc(who.avatar)} fit={who.avatarFit} color={who.color} size={36} /></button>{:else}<span class="pad"></span>{/if}
					<div class="stack">
						{#if !r.cont}<span class="name sc-bubble-name">{speakerLook(r.speaker).name}</span>{/if}
						{#if r.msg && editingId === r.msg.id}
							<div class="inline-edit bubble-edit">
								<Textarea
									bind:value={editDraft}
									autosize
									maxRows={10}
									aria-label="메시지 편집"
									onkeydown={(e) => {
										if (e.key === "Escape") cancelEdit();
										if ((e.ctrlKey || e.metaKey) && e.key === "Enter" && r.msg) void saveEdit(r.msg);
									}}
								/>
								<div class="inline-edit-btns">
									<Button size="sm" variant="ghost" onclick={cancelEdit}>취소</Button>
									<Button size="sm" variant="primary" onclick={() => r.msg && saveEdit(r.msg)}>저장</Button>
								</div>
							</div>
						{:else if r.msg && folded[r.msg.id]}
							<div class="line">
								<button type="button" class="b folded-b" onclick={() => r.msg && (folded[r.msg.id] = false)}>접힌 메시지 · 눌러서 펼치기</button>
								{#if !r.pending}<span class="t">{r.time}</span>{/if}
							</div>
						{:else}
							{#each r.lines ?? [] as l, i (i)}
								<div class="line">
									{#if l.kind === "image" && images[l.text]}
										<img class="photo" src={images[l.text]} alt={l.text} loading="lazy" />
									{:else if l.kind === "photo"}
										<img class="photo" src={fileSrc(l.text)} alt="그린 사진" loading="lazy" />
									{:else if l.kind === "html" && plugins.on("status") && isStatusPin({ kind: "html", code: l.text })}
										<!-- 상태창은 접고 상태 탭에 가장 최근 것 하나만 (5-F-4-2) -->
										<button type="button" class="b pin-ref sc-status-pin-ref" onclick={() => panel.show("status")}>상태창</button>
									{:else if l.kind === "html"}
										<div class="html-card"><HtmlView code={l.text} /></div>
									{:else}
										<div class="b sc-bubble">{l.kind === "image" ? `(사진: ${l.text})` : l.text}</div>
									{/if}
									{#if i === (r.lines?.length ?? 0) - 1 && !r.pending}<span class="t">{r.time}</span>{/if}
								</div>
							{/each}
							{#if r.pending && !(r.lines?.length)}
								<div class="line"><div class="b typing"><TypingIndicator name={ch?.name ?? ""} /></div></div>
							{/if}
						{/if}
					</div>
				</div>
			{/if}
		{/each}
		{/if}
	</div>

	{#if plugins.on("slash")}
		<div class="qr sc-phone-quick-reply">
			<QuickReplyBar {session} conv={library.conversation(conv.id) ?? conv} getDraft={() => draft} setDraft={(v) => (draft = v)} />
		</div>
	{/if}
	{#if pendingImages.length}
		<div class="attach-strip">
			{#each pendingImages as p, i (p + i)}
				<span class="attach-item">
					<img src={fileSrc(p)} alt="첨부 이미지" />
					<button type="button" aria-label="첨부 제거" onclick={() => removePendingImage(i)}><X /></button>
				</span>
			{/each}
		</div>
	{/if}
	{#if impersonating}
		<div class="imp-banner">
			<span><UserRoundPen /> <b>{ch?.name ?? "상대"}</b> 사칭 모드</span>
			<button type="button" onclick={() => (impersonating = false)}>취소</button>
		</div>
	{/if}
	<form
		class="input sc-phone-input"
		onsubmit={(e) => {
			e.preventDefault();
			void send();
		}}
	>
		<button type="button" class="plus-btn" class:on={plusOpen} aria-label="추가 기능 메뉴" aria-expanded={plusOpen} onclick={() => (plusOpen = !plusOpen)}><Plus /></button>
		{#if plugins.on("ghostwrite")}
			<Ghostwrite convId={session.convId} getDraft={() => draft} setDraft={(v) => (draft = v)} disabled={session.busy} />
		{/if}
		<textarea bind:value={draft} rows="1" readonly={ghost.isRunning(session.convId)} placeholder={impersonating ? `${ch?.name ?? "상대"}(으)로 사칭 입력` : phone.isCharacterPhone ? `${me?.name ?? "나"}(으)로 보내기` : "메시지 입력"} aria-label="메시지" {onkeydown}></textarea>
		{#if session.busy || scriptRunning}
			<!-- 스크립트가 돌면 스크립트째 멈춤 (5-E-20-6-8), 아니면 응답만 -->
			<button type="button" class="send on" aria-label={scriptRunning ? "스크립트 멈추기" : "멈추기"} onclick={() => (scriptRunning ? slashRuns.stop(session.convId, session) : session.cancel())}><Square /></button>
		{:else}
			<button class="send" class:on={!!draft.trim() || pendingImages.length > 0} aria-label="보내기" disabled={!draft.trim() && !pendingImages.length}><Send /></button>
		{/if}
	</form>
	{#if plusOpen}
		<div class="plus-tray" role="menu" aria-label="추가 메뉴">
			<button type="button" class="tray-btn" onclick={pickImages}>
				<span class="tray-ic img"><ImagePlus /></span>
				<span>이미지 전송</span>
			</button>
			<button type="button" class="tray-btn" class:active={impersonating} onclick={() => ((impersonating = !impersonating), (plusOpen = false))}>
				<span class="tray-ic imp"><UserRoundPen /></span>
				<span>{impersonating ? "사칭 끄기" : "사칭"}</span>
			</button>
			<button type="button" class="tray-btn" disabled={session.busy} onclick={() => ((plusOpen = false), void nudge())}>
				<span class="tray-ic nudge"><MessageCirclePlus /></span>
				<span>답장 더 받기</span>
			</button>
			{#if phone.isCharacterPhone}
				<button type="button" class="tray-btn" disabled={session.busy} onclick={() => ((plusOpen = false), void peek())}>
					<span class="tray-ic peek-ic"><Eye /></span>
					<span>엿보기</span>
				</button>
			{/if}
		</div>
	{/if}
</div>
{/if}

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label="메시지 메뉴" onclose={() => (menu = null)} />
{/if}

<SelectionMenu {scroller} conv={library.conversation(conv.id) ?? conv} {session} characterName={ch?.name} disabled={!!editingId} oncard={(c) => (card = c)} />
<TextCardDialog {card} charId={conv.characterId ?? undefined} convId={conv.id} onclose={() => (card = null)} />

<SentPromptDialog
	prompt={promptView}
	messageIndex={promptView ? session.messages.indexOf(promptView.msg) : 0}
	onclose={() => (promptView = null)}
/>

<style>
	.room {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: color-mix(in oklab, var(--accent) 8%, var(--bg));
		color: var(--text);
	}
	.room-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 14px 10px;
		border-bottom: 1px solid color-mix(in oklab, var(--border) 60%, transparent);
	}
	.room-bar strong {
		flex: 1;
		font-size: 16px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.room-bar :global(svg.group) {
		width: 15px;
		height: 15px;
		margin-left: 6px;
		vertical-align: -2px;
		color: var(--text-faint);
	}
	.room-bar :global(svg) {
		width: 22px;
		height: 22px;
	}
	.plain {
		display: grid;
		place-items: center;
		border: none;
		background: none;
		color: inherit;
		padding: 0;
		cursor: pointer;
	}
	.msgs {
		flex: 1;
		display: flex;
		flex-direction: column;
		gap: 10px;
		padding: 6px 12px 12px;
		overflow: auto;
	}
	.hint {
		align-self: center;
		margin-top: 40%;
		color: var(--text-muted);
		font-size: 13px;
	}
	.date {
		align-self: center;
		padding: 3px 12px;
		border-radius: 12px;
		background: color-mix(in oklab, var(--text) 14%, transparent);
		color: var(--text-muted);
		font-size: 11px;
	}
	.them {
		display: flex;
		gap: 8px;
		align-items: flex-start;
	}
	.them.cont,
	.me.cont {
		margin-top: -6px;
	}
	.them :global(.avatar) {
		border-radius: 38% !important;
		flex-shrink: 0;
		overflow: hidden;
	}

	.pad {
		width: 36px;
		flex-shrink: 0;
	}
	.stack {
		display: flex;
		flex-direction: column;
		gap: 4px;
		max-width: 78%;
		min-width: 0;
		width: 100%;
	}
	.name {
		font-size: 12px;
		color: var(--text-muted);
	}
	.line {
		display: flex;
		align-items: flex-end;
		gap: 5px;
	}
	.me {
		align-self: flex-end;
		max-width: 78%;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
		gap: 4px;
	}
	.b {
		padding: 7px 10px;
		border-radius: 14px;
		background: var(--bubble-ai);
		color: var(--bubble-ai-text);
		border: 1px solid color-mix(in oklab, var(--border) 65%, transparent);
		font-size: 14px;
		line-height: 1.4;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.me .b {
		background: var(--bubble-user);
		color: var(--bubble-user-text);
		border-color: transparent;
	}
	.photo {
		display: block;
		max-width: 200px;
		border-radius: 14px;
	}
	.t {
		flex: none;
		font-size: 10px;
		color: var(--text-faint);
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.read {
		color: var(--accent);
		font-weight: 700;
	}
	.typing {
		padding: 11px 12px;
		--typing-color: var(--text-faint);
	}
	/* 접은 상태창 (5-F-4-2) */
	.pin-ref {
		border: 1px dashed var(--border-strong);
		background: var(--surface);
		color: var(--text-muted);
		font-size: 12px;
		cursor: pointer;
	}
	.qr {
		padding: 6px 12px 0;
		background: var(--surface);
		border-top: 1px solid var(--border);
	}
	.input {
		display: flex;
		align-items: flex-end;
		gap: 8px;
		padding: 8px 12px 26px;
		background: var(--surface);
	}
	.input textarea {
		flex: 1;
		min-height: 36px;
		max-height: 96px;
		padding: 8px 14px;
		border: 1px solid var(--border);
		border-radius: 18px;
		background: var(--bg-sunken);
		outline: none;
		color: var(--text);
		font: inherit;
		font-size: 14px;
		resize: none;
		field-sizing: content;
	}
	.input textarea:focus {
		border-color: var(--accent);
	}
	.send {
		display: grid;
		place-items: center;
		flex: none;
		width: 36px;
		height: 36px;
		border: none;
		border-radius: 50%;
		background: transparent;
		color: var(--text-muted);
		cursor: pointer;
	}
	.send :global(svg) {
		width: 20px;
		height: 20px;
	}
	.send.on {
		background: var(--accent);
		color: var(--text-on-accent);
	}
	.send.on :global(svg) {
		width: 17px;
		height: 17px;
	}
	.hint {
		text-align: center;
		line-height: 1.6;
	}
	.send:disabled {
		cursor: default;
	}
	.html-card {
		width: min(100%, 360px);
	}
	.folded-b {
		border: 1px dashed var(--border-strong);
		background: var(--surface-2) !important;
		color: var(--text-muted) !important;
		font-size: 12px;
		cursor: pointer;
	}
	.attach-strip {
		display: flex;
		gap: 8px;
		padding: 6px 12px;
		background: var(--surface-2);
		border-top: 1px solid var(--border);
		overflow-x: auto;
	}
	.attach-item {
		position: relative;
		width: 56px;
		height: 56px;
		flex: none;
		border-radius: 10px;
		overflow: hidden;
		border: 1px solid var(--border);
	}
	.attach-item img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}
	.attach-item button {
		position: absolute;
		top: 2px;
		right: 2px;
		display: grid;
		place-items: center;
		width: 18px;
		height: 18px;
		padding: 0;
		border: none;
		border-radius: 50%;
		background: rgb(0 0 0 / 0.65);
		color: #fff;
		cursor: pointer;
	}
	.attach-item button :global(svg) {
		width: 11px;
		height: 11px;
	}
	.imp-banner {
		display: flex;
		align-items: center;
		justify-content: space-between;
		padding: 6px 14px;
		background: var(--warning-soft);
		color: var(--warning);
		font-size: 12px;
	}
	.imp-banner span {
		display: inline-flex;
		align-items: center;
		gap: 6px;
	}
	.imp-banner :global(svg) {
		width: 14px;
		height: 14px;
	}
	.imp-banner button {
		border: none;
		background: none;
		color: inherit;
		font-weight: 600;
		cursor: pointer;
	}
	.plus-btn {
		display: grid;
		place-items: center;
		width: 32px;
		height: 32px;
		flex: none;
		border: none;
		border-radius: 50%;
		background: var(--bg-sunken);
		color: var(--text-muted);
		cursor: pointer;
		transition: transform 150ms ease, background var(--duration-fast) var(--ease);
	}
	.plus-btn.on {
		transform: rotate(45deg);
		background: var(--accent-soft);
		color: var(--accent);
	}
	.plus-btn :global(svg) {
		width: 18px;
		height: 18px;
	}
	.plus-tray {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 12px 8px;
		padding: 14px 16px 26px;
		background: var(--surface-2);
		border-top: 1px solid var(--border);
	}
	.tray-btn {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 6px;
		border: none;
		background: none;
		color: var(--text-muted);
		font: inherit;
		font-size: 11.5px;
		cursor: pointer;
	}
	.tray-btn:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.tray-btn.active .tray-ic {
		outline: 2px solid var(--accent);
		outline-offset: 2px;
	}
	.tray-ic {
		display: grid;
		place-items: center;
		width: 46px;
		height: 46px;
		border-radius: 16px;
		color: #fff;
	}
	.tray-ic :global(svg) {
		width: 22px;
		height: 22px;
	}
	.tray-ic.img {
		background: #34c759;
	}
	.tray-ic.imp {
		background: #ff9500;
	}
	.tray-ic.nudge {
		background: var(--accent);
	}
	.tray-ic.peek-ic {
		background: #5856d6;
	}
	.av-btn {
		padding: 0;
		border: none;
		background: none;
		cursor: pointer;
		border-radius: 50%;
		display: inline-flex;
		flex: none;
	}
	.inline-edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		width: 100%;
		margin-top: 4px;
	}
	.bubble-edit {
		min-width: min(260px, 75vw);
		max-width: 100%;
		padding: 8px;
		border-radius: 12px;
		background: var(--surface, #fff);
		color: var(--text, #191919);
		box-shadow: var(--shadow-sm);
	}
	.inline-edit-btns {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-1);
	}
	.msgs.novel {
		background: var(--bg);
		color: var(--text);
		padding: var(--space-4) var(--space-4) var(--space-6);
		gap: var(--space-3);
	}
	.novel-block {
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		background: var(--surface);
		border: 1px solid var(--border);
	}
	.novel-block.novel-user {
		border-left: 3px solid var(--accent);
		background: var(--accent-soft);
	}
	.novel-block .who {
		display: flex;
		align-items: center;
		gap: var(--space-2);
		margin-bottom: var(--space-1);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.novel-no,
	.novel-time {
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.novel-body {
		font-size: var(--text-sm);
		line-height: 1.7;
		color: var(--text);
	}
	.novel-photos {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: var(--space-1) 0;
	}
</style>
