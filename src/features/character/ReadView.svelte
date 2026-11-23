<script lang="ts">
	// 읽기 화면 (5-E-6-2, ADR-031): 읽기 모드로 둔 대화. 메시지는 읽기 전용, 입력칸·생성·수정 없음.
	// 있는 것: 목차·북마크·분기·상태 패널, 글 골라 북마크·카드, 메시지의 복사·여기서 분기·만든 음성 듣기, 채팅 모드로 돌아가기
	import { tick } from "svelte";
	import { fade, fly } from "svelte/transition";
	import { Bookmark, GitBranch, ListTree, Pen, Scroll, Variable } from "@lucide/svelte";
	import SidePanel from "$app/SidePanel.svelte";
	import { layout } from "$app/layout.svelte";
	import { panel } from "$app/panel.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { regexVersion } from "$shared/stores/regexVersion.svelte";
	import { fileSrc, imageSrc } from "$shared/ipc/file";
	import { EmptyState, IconButton, Spinner } from "$shared/ui";
	import { currentStatusBlocks } from "$features/status/current";
	import { statusView } from "$features/status/statusView.svelte";
	import StatusFloat from "$features/status/StatusFloat.svelte";
	import StatusTop from "$features/status/StatusTop.svelte";
	import { bookmarks, bookmarksOf, type ChatBookmark } from "./bookmarks";
	import { cgVariants, pickVariants } from "./cgVariants";
	import ChatPanel from "./ChatPanel.svelte";
	import ConvSidebar from "./ConvSidebar.svelte";
	import { findTextRange } from "./highlight";
	import { library } from "./library.svelte";
	import MessageItem from "./MessageItem.svelte";
	import SelectionMenu, { type CardInput } from "./SelectionMenu.svelte";
	import { ChatSession, metaOf, type Message } from "./session.svelte";
	import TextCardDialog from "./TextCardDialog.svelte";

	let { charId, convId, onopen }: { charId: string; convId: string; onopen: (id: string) => void } = $props();

	const conv = $derived(library.conversation(convId));
	const character = $derived(library.character(charId));

	// ---- 메시지: 불러오기만 (보내기·생성은 이 화면에 없음 — 백엔드도 거절) ----
	let session = $state<ChatSession | null>(null);
	$effect(() => {
		const s = new ChatSession(convId);
		session = s;
		shownFrom = -1;
		void s.load().then(async () => {
			shownFrom = Math.max(0, s.messages.length - PAGE);
			await tick();
			if (scroller) scroller.scrollTop = scroller.scrollHeight;
		});
	});
	// 긴 대화: 최근 PAGE개만 그리고, 위로 올리면 더 (대화 화면과 같음, 1-D-5)
	const PAGE = 80;
	let shownFrom = $state(-1);
	const start = $derived.by(() => {
		const len = session?.messages.length ?? 0;
		return shownFrom < 0 ? Math.max(0, len - PAGE) : Math.min(shownFrom, len);
	});
	const visible = $derived(session ? session.messages.slice(start) : []);
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
	// 화면용 글 (표시용 정규식·이름)
	$effect(() => {
		const s = session;
		if (!s) return;
		void s.messages.map((m) => m.content);
		const key = `${conv?.personaId}:${conv?.promptPresetId}:${character?.defaultPromptPresetId}:${appSetting.value?.selectedPreset}:${regexVersion.n}`;
		void s.refreshDisplay(key);
	});

	// ---- 오른쪽 패널: 읽기에 쓰는 탭만 ----
	$effect(() =>
		panel.register([
			{ id: "toc", label: "목차", icon: ListTree },
			{ id: "bookmarks", label: "북마크", icon: Bookmark },
			{ id: "branches", label: "분기", icon: GitBranch },
			...(plugins.on("status") ? [{ id: "status", label: "상태", icon: Variable }] : []),
		]),
	);

	// ---- 대화 목록 (대화 화면과 같은 기억 자리) ----
	const LIST_KEY = "sc.chat.list";
	let showList = $state(
		(() => {
			try {
				return localStorage.getItem(LIST_KEY) !== "0";
			} catch {
				return true;
			}
		})(),
	);
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

	// ---- 상태창 ----
	const statusOn = $derived(!!session && plugins.on("status"));
	const statusBlocks = $derived(statusOn && session ? currentStatusBlocks(conv, session.messages, session.display, plugins.on("html")) : []);
	$effect(() => {
		const id = character?.id;
		if (id && plugins.on("status")) {
			void library.loadStatusTemplate(id);
			void library.loadVarDefs(id);
		}
	});

	// ---- 그림·프로필 ----
	const showAvatars = $derived(!!(conv?.meta as { showAvatars?: boolean } | null)?.showAvatars);
	function ownerOf(m: Message) {
		return m.role === "assistant"
			? library.character(m.characterId ?? charId)
			: m.role === "user"
				? library.persona((m.meta as { sender?: { id?: string } } | null)?.sender?.id ?? library.convPersonaId(conv))
				: undefined;
	}
	function avatarOf(m: Message): string | null | undefined {
		if (!showAvatars) return undefined;
		const path = ownerOf(m)?.avatar;
		return path ? fileSrc(path) : null;
	}
	const imageVariants = $derived(cgVariants(library.imagesInUse(charId).map((i) => ({ key: i.key, src: imageSrc(i) ?? null })), !!appSetting.value?.cgGroupVariants));
	const chapters = $derived.by(() => {
		const out: Record<string, number> = {};
		let n = 0;
		for (const m of session?.messages ?? []) if (metaOf(m).chapter) out[m.id] = ++n;
		return out;
	});

	// ---- 스크롤·이동 ----
	let scroller: HTMLDivElement | undefined = $state();
	function onscroll() {
		if (scroller && scroller.scrollTop < 300 && start > 0) void showEarlier();
	}
	async function jumpTo(id: string, mark?: ChatBookmark) {
		const at = session?.messages.findIndex((m) => m.id === id) ?? -1;
		if (at >= 0 && at < start) {
			shownFrom = Math.max(0, at - 5);
			await tick();
		}
		const block = document.getElementById(`msg-${id}`);
		const hit = block && mark ? findTextRange(block, mark.text) : null;
		const target = hit ? (hit.startContainer.parentElement ?? block) : block;
		target?.scrollIntoView({ behavior: "smooth", block: hit ? "center" : "start" });
	}
	function jumpFromPanel(id: string, mark?: ChatBookmark) {
		if (layout.narrow && panel.open) panel.toggle();
		return jumpTo(id, mark);
	}
	/** 다른 대화의 그 메시지로 (분기 탭) */
	function openAt(id: string, messageId?: string) {
		if (id === convId) return messageId && jumpTo(messageId);
		onopen(id);
		if (messageId) setTimeout(() => jumpTo(messageId), 400);
	}

	// ---- 북마크·카드 ----
	const marksByMsg = $derived.by(() => {
		const map = new Map<string, ChatBookmark[]>();
		for (const b of bookmarksOf(conv)) map.set(b.messageId, [...(map.get(b.messageId) ?? []), b]);
		return map;
	});
	async function removeMark(b: ChatBookmark) {
		if (conv) await bookmarks.remove(conv, b.id);
	}
	let card = $state<CardInput | null>(null);
	function cardFromMark(b: ChatBookmark) {
		const msg = session?.messages.find((x) => x.id === b.messageId);
		card = { text: b.text, speaker: msg?.name ?? (msg?.role === "assistant" ? character?.name : undefined), time: msg?.createdAt };
	}

	// ---- 여기서 분기해 채팅하기: 새 대화(채팅 모드)로 ----
	async function branch(m: Message) {
		const c = await library.branch(convId, m.id).catch(() => null);
		if (c) onopen(c.id);
	}
	const noop = () => {};
</script>

<div class="chat sc-chat sc-read" class:no-list={!listShown || layout.narrow} class:no-panel={!panel.open || layout.narrow}>
	{#if listShown && layout.narrow}
		<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions (목록 버튼으로도 닫힘) -->
		<div class="list-scrim" transition:fade={{ duration: 150 }} onclick={() => (mobileList = false)}></div>
		<div class="list-drawer" transition:fly={{ x: -320, duration: 200 }}>
			<ConvSidebar {charId} {convId} onopen={(id) => { mobileList = false; onopen(id); }} oncreate={async () => { mobileList = false; onopen((await library.createConversation(charId)).id); }} onclose={toggleList} />
		</div>
	{:else if listShown}
		<ConvSidebar {charId} {convId} {onopen} oncreate={async () => onopen((await library.createConversation(charId)).id)} onclose={toggleList} />
	{/if}

	<section class="reader">
		<header class="bar" class:listing={listShown}>
			<IconButton label={listShown ? "대화 목록 닫기" : "대화 목록 열기"} size="sm" active={listShown} onclick={toggleList}><Scroll /></IconButton>
			<IconButton label="채팅 모드" size="sm" onclick={() => conv && library.setReadMode(conv.id, false)}><Pen /></IconButton>
		</header>

		{#if statusOn && layout.narrow && statusView.aboveChat}
			<StatusTop blocks={statusBlocks} />
		{/if}
		{#if statusOn && !layout.narrow && statusView.floating}
			<StatusFloat blocks={statusBlocks} />
		{/if}
		<div class="scroll sc-chat-scroll" bind:this={scroller} {onscroll}>
			{#if !session || session.loading}
				<div class="center"><Spinner /></div>
			{:else if !conv}
				<EmptyState title="대화를 찾을 수 없음" description="지워졌거나 아직 불러오지 못함" />
			{:else}
				<article class="text sc-chat-log">
					{#if !session.messages.length}<p class="empty">메시지 없음</p>{/if}
					{#if start > 0}
						<button class="earlier" onclick={() => showEarlier()}>이전 메시지 {Math.min(PAGE, start)}개 더 보기 <small>(앞에 {start}개)</small></button>
					{/if}
					{#each visible as m, j (m.id)}
						{@const i = start + j}
						{@const ch = metaOf(m).chapter}
						{#if ch}<h2 class="chapter sc-chapter" id="chapter-{m.id}"><span>{chapters[m.id]}장</span>{ch}</h2>{/if}
						<MessageItem readonly msg={m} no={i + 1} avatar={avatarOf(m)} avatarFit={ownerOf(m)?.avatarFit} {session} images={pickVariants(imageVariants, m.id)} isLast={i === session.messages.length - 1} onbranch={branch} onprompt={noop} marks={marksByMsg.get(m.id)} oncard={cardFromMark} onmarkremove={removeMark} />
					{/each}
				</article>
			{/if}
		</div>
	</section>

	{#if panel.open && conv && session}
		<SidePanel label="읽기 패널">
			<ChatPanel {conv} {session} {chapters} onjump={jumpFromPanel} oncard={cardFromMark} onopen={openAt} draft={{ get: () => "", set: noop }} />
		</SidePanel>
	{/if}
	<SelectionMenu {scroller} {conv} {session} characterName={character?.name} oncard={(c) => (card = c)} />
</div>

<TextCardDialog {card} onclose={() => (card = null)} />

<style>
	/* 배치는 대화 화면(ChatView)과 같게 — 목록·본문·패널 */
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
		position: relative;
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		background: var(--surface);
	}
	.bar {
		position: absolute;
		z-index: var(--z-sticky);
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-2);
		width: 100%;
		padding: var(--space-2) var(--space-3) 0;
	}
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
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		max-width: var(--page-reading);
		margin: 0 auto;
		padding: var(--space-6) var(--space-6) var(--space-10);
		font-size: 15.5px;
		line-height: 1.85;
	}
	.empty {
		padding: var(--space-10) 0;
		text-align: center;
		color: var(--text-faint);
	}
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
		letter-spacing: 0.2em;
		font-weight: 700;
		color: var(--accent-text);
	}
</style>
