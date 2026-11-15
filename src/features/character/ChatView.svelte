<script lang="ts">
	import { tick } from "svelte";
	import { Bookmark, Copy, CornerUpLeft, FastForward, Images, ListChecks, ListTree, NotebookPen, PanelLeft, Plus, RefreshCw, Send, SlidersHorizontal, Square, Trash2, X } from "@lucide/svelte";
	import { bookmarks } from "./bookmarks";
	import SidePanel from "$app/SidePanel.svelte";
	import { drops, extOf, isImage, storeFile } from "$app/drop.svelte";
	import { Button } from "$shared/ui";
	import { confirmAction } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { ContextMenu, Dialog, EmptyState, IconButton, Spinner, Textarea } from "$shared/ui";
	import { panel } from "$app/panel.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	appSetting.load();
	import { fileSrc } from "$shared/ipc/file";
	import type { PromptMessage_Serialize } from "$ipc/bindings";
	import ChatPanel from "./ChatPanel.svelte";
	import ConvSidebar from "./ConvSidebar.svelte";
	import MessageItem from "./MessageItem.svelte";
	import { toBlocks } from "./format";
	import { library } from "./library.svelte";
	import { ChatSession, metaOf, type Message } from "./session.svelte";

	let { charId, convId, onopen }: { charId: string; convId: string; onopen: (id: string) => void } = $props();

	const conv = $derived(library.conversation(convId));
	const character = $derived(library.character(charId));

	// 대화가 바뀌면 새 세션
	let session = $state<ChatSession | null>(null);
	$effect(() => {
		const s = new ChatSession(convId);
		session = s;
		void s.load().then(() => scrollToEnd(true));
	});
	$effect(() => {
		if (!library.images[charId]) void library.loadImages(charId);
	});

	// 오른쪽 패널 탭은 타이틀바에 그려진다
	$effect(() =>
		panel.register([
			{ id: "settings", label: "대화 설정", icon: SlidersHorizontal },
			{ id: "notes", label: "작가 노트·메모", icon: NotebookPen },
			{ id: "toc", label: "목차", icon: ListTree },
			{ id: "bookmarks", label: "북마크", icon: Bookmark },
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
	function toggleList() {
		showList = !showList;
		try {
			localStorage.setItem(LIST_KEY, showList ? "1" : "0");
		} catch {
			/* 무시 */
		}
	}

	/** <cg>키</cg> → 이미지 주소 (켜진 것만이 아니라 전부: 예전 응답도 그대로 보이게) */
	const images = $derived(Object.fromEntries((library.images[charId] ?? []).map((i) => [i.key, fileSrc(i.path) ?? ""])));

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
	export function jumpTo(id: string) {
		document.getElementById(`msg-${id}`)?.scrollIntoView({ behavior: "smooth", block: "start" });
	}

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
	async function send() {
		if (!session || session.busy || sending) return;
		const text = draft;
		if (!text.trim() && !pending.length) return;
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
		await session.send(text, paths, items.map((p) => p.url));
		for (const p of items) URL.revokeObjectURL(p.url);
	}

	// 파일을 끌어다 놓으면: 이미지는 첨부, .jsonl은 이 캐릭터의 대화로 가져오기
	$effect(() =>
		drops.register({
			priority: 1,
			label: "이미지 첨부 · 대화 가져오기",
			hint: "이미지는 다음 메시지에 붙고, SillyTavern 채팅(.jsonl)은 이 캐릭터의 대화로 들어옴",
			accept: (f) => isImage(f) || extOf(f) === "jsonl",
			ondrop: async (files) => {
				attach(files.filter(isImage));
				for (const f of files.filter((x) => extOf(x) === "jsonl")) {
					const r = await library.importSillyTavern(await storeFile(f), charId).catch(() => null);
					if (r?.kind === "chat") onopen(r.conversation.id);
				}
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

	// ---- 글을 끌어서 고르면: 북마크 / 복사 ----
	let selMenu = $state<{ x: number; y: number; text: string; messageId: string } | null>(null);
	function onSelectEnd() {
		setTimeout(() => {
			const sel = window.getSelection();
			const text = sel?.toString().trim() ?? "";
			if (!sel || !text || selecting) return;
			const node = sel.anchorNode instanceof Element ? sel.anchorNode : sel.anchorNode?.parentElement;
			const block = node?.closest<HTMLElement>("[id^=msg-]");
			if (!block) return;
			const r = sel.getRangeAt(0).getBoundingClientRect();
			selMenu = { x: r.left, y: r.bottom + 6, text, messageId: block.id.slice(4) };
		}, 0);
	}
	async function bookmarkSelection() {
		const m = selMenu; // 메뉴가 닫히며 selMenu가 비워지므로 먼저 잡아 둠
		if (!m || !conv) return;
		await bookmarks.add(conv, m.messageId, m.text);
		window.getSelection()?.removeAllRanges();
	}

	// ---- 분기: 부모 대화 ----
	const branchInfo = $derived((conv?.meta as { branch?: { parentId: string; messageId: string; seq: number } } | null)?.branch ?? null);
	const parentConv = $derived(branchInfo ? library.conversation(branchInfo.parentId) : undefined);
	function goParent() {
		if (!parentConv || !branchInfo) return;
		onopen(parentConv.id);
		// 갈라진 메시지로 스크롤
		setTimeout(() => jumpTo(branchInfo.messageId), 400);
	}

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
</script>

<div class="chat" class:no-list={!showList} class:no-panel={!panel.open}>
	{#if showList}
		<ConvSidebar {charId} {convId} {onopen} oncreate={async () => onopen((await library.createConversation(charId)).id)} />
	{/if}

	<section class="reader">
		<header class="bar" class:listing={showList}>
			<IconButton label={showList ? "대화 목록 닫기" : "대화 목록 열기"} size="sm" active={showList} onclick={toggleList}><PanelLeft /></IconButton>
			{#if parentConv && branchInfo}
				<button class="branch" onclick={goParent} title="갈라져 나온 대화로 돌아가기">
					<CornerUpLeft />{parentConv.title}<span>#{branchInfo.seq}에서 갈라짐</span>
				</button>
			{/if}
		</header>
		{#if selecting}
			<div class="selbar" role="toolbar" aria-label="메시지 고르기">
				<b>{picked.size}개 고름</b>
				<Button size="sm" variant="ghost" onclick={pickAll}>{picked.size === (session?.messages.length ?? 0) && picked.size ? "모두 풀기" : "모두 고르기"}</Button>
				<span class="sp"></span>
				<Button size="sm" variant="danger" disabled={!picked.size} onclick={removePicked}>{#snippet icon()}<Trash2 />{/snippet}삭제</Button>
				<IconButton label="고르기 끝내기" size="sm" onclick={() => (selecting = false)}><X /></IconButton>
			</div>
		{/if}

		<!-- svelte-ignore a11y_no_static_element_interactions (글 고르기 끝 감지) -->
		<div class="scroll" bind:this={scroller} {onscroll} onmouseup={onSelectEnd}>
			{#if !session || session.loading}
				<div class="center"><Spinner /></div>
			{:else if !conv}
				<EmptyState title="대화를 찾을 수 없음" description="지워졌거나 아직 불러오지 못함" />
			{:else}
				<article class="text">
					{#if !session.messages.length && !session.live}
						<p class="empty">첫 메시지를 보내 보세요</p>
					{/if}
					{#each session.messages as m, i (m.id)}
						{@const ch = metaOf(m).chapter}
						{#if ch}<h2 class="chapter" id="chapter-{m.id}"><span>{chapters[m.id]}장</span>{ch}</h2>{/if}
						<MessageItem msg={m} no={i + 1} {session} {images} isLast={i === session.messages.length - 1} onbranch={branch} onprompt={showPrompt} {selecting} selected={picked.has(m.id)} onselect={pick} />
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
							<div class="who"><b>{character?.name ?? "AI"}</b><span class="stamp">만드는 중…</span></div>
							{#each toBlocks(session.live.text) as b, j (j)}
								{#if b.kind === "p"}
									<p>{#each b.parts as part, k (k)}{#if part.kind === "action"}<em>{part.text}</em>{:else}{part.text}{/if}{/each}</p>
								{/if}
							{:else}
								<p class="typing" aria-label="응답 기다리는 중"><span></span><span></span><span></span></p>
							{/each}
						</div>
					{/if}
				</article>
			{/if}
		</div>

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
			class="composer"
			onsubmit={(e) => {
				e.preventDefault();
				void send();
			}}
		>
			<IconButton label="추가 기능" onclick={openPlus} disabled={!session || session.busy}><Plus /></IconButton>
			<Textarea bind:value={draft} rows={1} autosize maxRows={8} placeholder={appSetting.sendOnEnter ? "다음 행동이나 대사 입력 (Shift+Enter 줄바꿈)" : "다음 행동이나 대사 입력 (Ctrl+Enter 보내기)"} aria-label="메시지" {onkeydown} {onpaste} />
			{#if session?.busy}
				<IconButton label="멈추기 (받은 데까지 저장)" variant="solid" onclick={() => session?.cancel()}><Square /></IconButton>
			{:else}
				<IconButton label="보내기" variant="solid" type="submit" disabled={(!draft.trim() && !pending.length) || sending}><Send /></IconButton>
			{/if}
		</form>
		<ContextMenu
			at={plusMenu}
			label="추가 기능"
			items={[
				{ label: "이어서 응답 받기", icon: FastForward, onselect: () => session?.send("") },
				{
					label: "마지막 응답 다시 만들기",
					icon: RefreshCw,
					disabled: session?.last?.role !== "assistant",
					onselect: () => session?.regenerate(true),
				},
				{ label: showList ? "대화 목록 닫기" : "대화 목록 열기", icon: PanelLeft, separator: true, onselect: toggleList },
				{ label: "메시지 여러 개 지우기", icon: ListChecks, disabled: !session?.messages.length, onselect: startSelect },
				...(parentConv ? [{ label: "갈라져 나온 대화로", icon: CornerUpLeft, onselect: goParent }] : []),
			]}
			onclose={() => (plusMenu = null)}
		/>
	</section>

	{#if panel.open && conv && session}
		<SidePanel label="대화 패널">
			<ChatPanel {conv} {session} {chapters} onjump={jumpTo} />
		</SidePanel>
	{/if}
	<ContextMenu
		at={selMenu}
		label="고른 글"
		items={[
			{ label: "북마크", icon: Bookmark, onselect: bookmarkSelection },
			{
				label: "복사",
				icon: Copy,
				onselect: () =>
					navigator.clipboard.writeText(selMenu?.text ?? "").then(
						() => toasts.success("복사함"),
						() => toasts.error("복사 못 함"),
					),
			},
		]}
		onclose={() => (selMenu = null)}
	/>
</div>

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
	}
	.bar {
		display: flex;
		padding: var(--space-2) var(--space-3) 0;
	}
	/* 대화 목록 버튼은 마우스를 올리거나 키보드로 갔을 때만 보임 */
	.bar :global(.icon-btn) {
		opacity: 0;
		transition: opacity var(--duration-fast) var(--ease);
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
	}
	.branch span {
		color: var(--text-faint);
	}
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
		max-width: 720px;
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
	.block p + p {
		margin-top: 0.6em;
	}
	.block em {
		color: var(--text-muted);
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
		align-items: flex-end;
		gap: var(--space-2);
		max-width: 760px;
		width: 100%;
		margin: 0 auto;
		padding: var(--space-3) var(--space-4) var(--space-4);
	}
	.composer :global(.textarea),
	.composer :global(textarea) {
		flex: 1;
	}
	.composer :global(textarea) {
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
	@media (max-width: 900px) {
		.chat,
		.chat.no-list,
		.chat.no-panel {
			grid-template-columns: minmax(0, 1fr);
		}
		.chat > :global(.convs),
		.chat > :global(.panel) {
			display: none;
		}
	}
</style>
