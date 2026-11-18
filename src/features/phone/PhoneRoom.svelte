<script lang="ts">
	import { onMount, tick } from "svelte";
	import { BookOpen, ChevronLeft, Copy, Eye, ImagePlus, Languages, Menu, MessageCirclePlus, RefreshCw, Send, Square, Trash2, Volume2 } from "@lucide/svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import Ghostwrite from "$features/ghost/Ghostwrite.svelte";
	import { ghost } from "$features/ghost/ghost.svelte";
	import { drawMessage, generatedImagesOf, speak, speech, translateMessage, translationOf } from "$shared/stores/aiTools.svelte";
	import { Avatar, ContextMenu, HtmlView, type MenuItem } from "$shared/ui";
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { nav } from "$app/nav.svelte";
	import { library, type Conversation } from "$features/character/library.svelte";
	import { ChatSession, type Message } from "$features/character/session.svelte";
	import { bubbles, lookup, parsePeek, phone, type BubbleLine } from "./phone.svelte";

	let { conv, onback, session: given }: { conv: Conversation; onback: () => void; session?: ChatSession } = $props();

	// 폰 화면이 만든 세션을 같이 씀 (오른쪽 패널의 목차·컨텍스트도 같은 메시지를 봄)
	// svelte-ignore state_referenced_locally
	const session = given ?? new ChatSession(conv.id);
	// 상대(연락처)와 나(폰 주인)
	const ch = $derived(lookup(phone.contactOf(conv)));
	const me = $derived(phone.owner);
	const imgChar = $derived(ch?.kind === "character" ? ch.id : null);
	const images = $derived(Object.fromEntries((library.images[imgChar ?? ""] ?? []).map((i) => [i.key, imageSrc(i) ?? ""])));
	let draft = $state("");
	let scroller: HTMLDivElement | undefined = $state();

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
		if (!session.busy) void session.refreshDisplay();
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
	async function send() {
		const text = draft.trim();
		if (!text || session.busy) return;
		draft = "";
		await session.send(text, [], [], ghost.takeUsed(session.convId));
		await done();
	}
	/** 내 메시지 없이 답장만 더 받기 */
	async function nudge() {
		if (session.busy) return;
		await session.send("");
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
		pending?: boolean;
	}
	const day = (t: number) => new Date(t).toLocaleDateString("ko-KR", { year: "numeric", month: "long", day: "numeric", weekday: "long" });
	const hm = (t: number) => new Date(t).toLocaleTimeString("ko-KR", { hour: "numeric", minute: "2-digit" });

	const rows = $derived.by(() => {
		const out: Row[] = [];
		let lastDay = "";
		let lastSide: boolean | null = null;
		const live = session.live;
		const push = (r: Omit<Row, "kind" | "key"> & { key: string; at: number }) => {
			const d = day(r.at);
			if (d !== lastDay) {
				out.push({ key: `d${r.at}`, kind: "date", date: d });
				lastDay = d;
				lastSide = null;
			}
			out.push({ ...r, kind: "msg", cont: lastSide === r.mine, time: hm(r.at) });
			lastSide = r.mine ?? null;
		};
		for (const m of session.messages) {
			const tr = plugins.on("translate") ? translationOf(m) : null;
			const content = live?.kind === "regenerate" && live.targetId === m.id ? live.text : tr && !originals[m.id] ? tr.text : (session.display[m.id] ?? m.content);
			// 이미지 생성 플러그인으로 그린 사진은 말풍선 뒤에 사진으로
			const photos = plugins.on("imagegen") ? generatedImagesOf(m).map((g) => ({ kind: "photo" as const, text: g.path })) : [];
			const lines = [...bubbles(content), ...photos];
			if (!lines.length && !(live?.targetId === m.id)) continue;
			push({ key: m.id, at: m.createdAt, mine: m.role === "user", msg: m, lines, pending: live?.targetId === m.id });
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
			push({ key: "live-ai", at: now, mine: false, lines: bubbles(live.text), pending: true });
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

	function bubbleMenu(e: MouseEvent, r: Row) {
		if (!r.msg || session.busy) return;
		e.preventDefault();
		const m = r.msg;
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
				{ label: "소설 화면으로 보기", icon: BookOpen, separator: true, onselect: () => nav.go({ page: "character", charId: conv.characterId ?? undefined, convId: conv.id }) },
			],
		};
	}
</script>

<div class="room">
	<div class="room-bar">
		<button class="plain" aria-label="뒤로" onclick={onback}><ChevronLeft /></button>
		<strong>{ch?.name ?? conv.title}</strong>
		<button class="plain" aria-label="대화방 메뉴" onclick={roomMenu}><Menu /></button>
	</div>

	<div class="msgs" bind:this={scroller}>
		{#if session.loading}
			<p class="hint">불러오는 중…</p>
		{:else if !rows.length}
			<p class="hint">{ch?.name ?? "상대"}에게 첫 메시지를 보내 봄{#if phone.isCharacterPhone}<br />또는 눈 버튼으로 두 사람 대화를 엿봄{/if}</p>
		{/if}
		{#each rows as r (r.key)}
			{#if r.kind === "date"}
				<div class="date">{r.date}</div>
			{:else if r.mine}
				<div class="me" class:cont={r.cont} role="group" oncontextmenu={(e) => bubbleMenu(e, r)}>
					{#each r.lines ?? [] as l, i (i)}
						<div class="line">
							{#if i === (r.lines?.length ?? 0) - 1}
								<span class="t">{#if unreadMine === r.key}<span class="read">1</span>{/if}{r.pending ? "" : r.time}</span>
							{/if}
							<div class="b">{l.text}</div>
						</div>
					{/each}
				</div>
			{:else}
				<div class="them" class:cont={r.cont} role="group" oncontextmenu={(e) => bubbleMenu(e, r)}>
					{#if !r.cont}<Avatar name={ch?.name ?? "?"} src={fileSrc(ch?.avatar)} color={ch?.color} size={36} />{:else}<span class="pad"></span>{/if}
					<div class="stack">
						{#if !r.cont}<span class="name">{ch?.name}</span>{/if}
						{#each r.lines ?? [] as l, i (i)}
							<div class="line">
								{#if l.kind === "image" && images[l.text]}
									<img class="photo" src={images[l.text]} alt={l.text} loading="lazy" />
								{:else if l.kind === "photo"}
									<img class="photo" src={fileSrc(l.text)} alt="그린 사진" loading="lazy" />
								{:else if l.kind === "html"}
									<div class="html-card"><HtmlView code={l.text} /></div>
								{:else}
									<div class="b">{l.kind === "image" ? `(사진: ${l.text})` : l.text}</div>
								{/if}
								{#if i === (r.lines?.length ?? 0) - 1 && !r.pending}<span class="t">{r.time}</span>{/if}
							</div>
						{/each}
						{#if r.pending && !(r.lines?.length)}
							<div class="line"><div class="b typing" aria-label="입력 중"><span></span><span></span><span></span></div></div>
						{/if}
					</div>
				</div>
			{/if}
		{/each}
	</div>

	<form
		class="input"
		onsubmit={(e) => {
			e.preventDefault();
			void send();
		}}
	>
		{#if phone.isCharacterPhone}
			<button type="button" class="peek" aria-label="엿보기 (AI가 두 사람 대화를 이어서 씀)" title="엿보기: 두 사람 대화를 AI가 이어서 씀" disabled={session.busy} onclick={peek}><Eye /></button>
		{/if}
		{#if plugins.on("ghostwrite")}
			<Ghostwrite convId={session.convId} getDraft={() => draft} setDraft={(v) => (draft = v)} disabled={session.busy} />
		{/if}
		<textarea bind:value={draft} rows="1" readonly={ghost.isRunning(session.convId)} placeholder={phone.isCharacterPhone ? `${me?.name ?? "나"}(으)로 보내기` : "메시지 입력"} aria-label="메시지" {onkeydown}></textarea>
		{#if session.busy}
			<button type="button" class="send on" aria-label="멈추기" onclick={() => session.cancel()}><Square /></button>
		{:else}
			<button class="send" class:on={!!draft.trim()} aria-label="보내기" disabled={!draft.trim()}><Send /></button>
		{/if}
	</form>
</div>

{#if menu}
	<ContextMenu at={menu.at} items={menu.items} label="메시지 메뉴" onclose={() => (menu = null)} />
{/if}

<style>
	.room {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: color-mix(in oklab, var(--accent) 12%, #eef1f4);
	}
	.room-bar {
		display: flex;
		align-items: center;
		gap: 10px;
		padding: 6px 14px 10px;
	}
	.room-bar strong {
		flex: 1;
		font-size: 16px;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
		color: #6b6b6b;
		font-size: 13px;
	}
	.date {
		align-self: center;
		padding: 3px 12px;
		border-radius: 12px;
		background: rgb(0 0 0 / 0.12);
		color: white;
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
	}
	.name {
		font-size: 12px;
		color: #333;
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
		background: white;
		color: #191919;
		font-size: 14px;
		line-height: 1.4;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.me .b {
		background: var(--accent);
		color: white;
	}
	.photo {
		display: block;
		max-width: 200px;
		border-radius: 14px;
	}
	.t {
		flex: none;
		font-size: 10px;
		color: #555;
		display: flex;
		flex-direction: column;
		align-items: flex-end;
	}
	.read {
		color: var(--accent);
		font-weight: 700;
	}
	.typing {
		display: flex;
		gap: 4px;
		padding: 11px 12px;
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #9a9a9a;
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
			opacity: 0.25;
		}
	}
	.input {
		display: flex;
		align-items: flex-end;
		gap: 8px;
		padding: 8px 12px 26px;
		background: white;
	}
	.input textarea {
		flex: 1;
		min-height: 36px;
		max-height: 96px;
		padding: 8px 14px;
		border: none;
		border-radius: 18px;
		background: #f1f1f1;
		outline: none;
		color: #191919;
		font: inherit;
		font-size: 14px;
		resize: none;
		field-sizing: content;
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
		color: #666;
		cursor: pointer;
	}
	.send :global(svg) {
		width: 20px;
		height: 20px;
	}
	.send.on {
		background: var(--accent);
		color: white;
	}
	.send.on :global(svg) {
		width: 17px;
		height: 17px;
	}
	.peek {
		display: grid;
		place-items: center;
		flex: none;
		width: 36px;
		height: 36px;
		border: none;
		border-radius: 50%;
		background: #f1f1f1;
		color: #555;
		cursor: pointer;
	}
	.peek:hover:not(:disabled) {
		background: color-mix(in oklab, var(--accent) 18%, #f1f1f1);
		color: var(--accent);
	}
	.peek :global(svg) {
		width: 18px;
		height: 18px;
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
</style>
