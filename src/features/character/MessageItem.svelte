<script lang="ts">
	import { Bookmark, BookmarkX, ChevronLeft, ChevronRight, Copy, Eye, EyeOff, FileSearch, GitBranch, ImagePlus, Languages, Pencil, RefreshCw, Square, Trash2, Volume2 } from "@lucide/svelte";
	import { plugins } from "$shared/stores/plugins.svelte";
	import { drawMessage, generatedImagesOf, speak, speech, translateMessage, translationOf } from "$shared/stores/aiTools.svelte";
	import { Badge, Button, HtmlView, IconButton, Textarea } from "$shared/ui";
	import { absolute, relative } from "$shared/time";
	import { confirmAction, promptText } from "$shared/stores/confirm.svelte";
	import { toasts } from "$shared/stores/toast.svelte";
	import { appSetting } from "$shared/stores/appSetting.svelte";
	import { toBlocks } from "./format";
	import { metaOf, type ChatSession, type Message } from "./session.svelte";
	import { fileSrc } from "$shared/ipc/file";
	import { layout } from "$app/layout.svelte";

	interface Props {
		msg: Message;
		no: number;
		session: ChatSession;
		/** <cg>키</cg> → 이미지 주소 */
		images: Record<string, string>;
		isLast: boolean;
		onbranch: (m: Message) => void;
		onprompt: (m: Message) => void;
		/** 여러 개 고르기 모드 */
		selecting?: boolean;
		selected?: boolean;
		onselect?: (m: Message, on: boolean) => void;
	}
	let { msg, no, session, images, isLast, onbranch, onprompt, selecting = false, selected = false, onselect }: Props = $props();

	const meta = $derived(metaOf(msg));
	const isAi = $derived(msg.role === "assistant");
	const live = $derived(session.live?.kind === "regenerate" && session.live.targetId === msg.id ? session.live : null);
	// 번역이 있으면 번역문을 보여주고 "원문"으로 바꿔 볼 수 있다
	const translation = $derived(plugins.on("translate") ? translationOf(msg) : null);
	let showOriginal = $state(false);
	const text = $derived(live ? live.text : translation && !showOriginal ? translation.text : (session.display[msg.id] ?? msg.content));
	const drawn = $derived(plugins.on("imagegen") ? generatedImagesOf(msg) : []);
	let toolBusy = $state<"" | "translate" | "draw">("");
	async function translate() {
		if (translation) return void (showOriginal = !showOriginal);
		toolBusy = "translate";
		try {
			session.update(await translateMessage(msg));
			showOriginal = false;
		} finally {
			toolBusy = "";
		}
	}
	async function draw() {
		toolBusy = "draw";
		toasts.info("장면을 그리는 중… (잠시 걸림)");
		try {
			session.update(await drawMessage(msg));
		} finally {
			toolBusy = "";
		}
	}
	const blocks = $derived(toBlocks(text));
	const swipes = $derived(meta.swipes?.length ?? 0);
	const swipeId = $derived(meta.swipeId ?? 0);
	const model = $derived(meta.info?.model || undefined);

	let editing = $state(false);
	// 터치 화면: 메시지를 누르면 도구 줄을 보이고, 다른 곳을 누르면 숨김
	let tapped = $state(false);
	let el: HTMLDivElement | undefined = $state();
	function tap(e: MouseEvent) {
		if (selecting) return onselect?.(msg, !selected);
		if (!layout.touch || editing) return;
		if ((e.target as Element).closest("button, a, input, textarea")) return;
		if (window.getSelection()?.toString()) return;
		tapped = !tapped;
	}
	$effect(() => {
		if (!tapped) return;
		const off = (e: PointerEvent) => !el?.contains(e.target as Node) && (tapped = false);
		window.addEventListener("pointerdown", off);
		return () => window.removeEventListener("pointerdown", off);
	});
	let draft = $state("");
	function startEdit() {
		draft = msg.content;
		editing = true;
	}
	async function saveEdit() {
		await session.edit(msg, draft);
		editing = false;
	}
	async function copy() {
		try {
			await navigator.clipboard.writeText(msg.content);
			toasts.success("복사함");
		} catch {
			toasts.error("복사 못 함");
		}
	}
	async function chapter() {
		if (meta.chapter) return session.setChapter(msg, null);
		const t = await promptText({ title: "여기서 챕터 시작", message: `#${no} 메시지부터 새 챕터`, input: { value: "", placeholder: "챕터 제목" }, confirmLabel: "만들기" });
		if (t) await session.setChapter(msg, t);
	}
	async function remove() {
		if (await confirmAction({ title: "메시지 삭제", message: `#${no} 메시지를 지움. 되돌릴 수 없음`, confirmLabel: "삭제", danger: true })) await session.remove(msg);
	}
</script>

<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
<div bind:this={el} class="block" class:ai={isAi} class:user={!isAi} class:hidden={meta.hidden} class:selecting class:picked={selected} id="msg-{msg.id}" data-no={no} class:tapped onclick={tap}>
	<div class="who">
		{#if selecting}<input class="pick" type="checkbox" checked={selected} aria-label="#{no} 메시지 고르기" onclick={(e) => e.stopPropagation()} onchange={(e) => onselect?.(msg, e.currentTarget.checked)} />{/if}
		<span class="no">#{no}</span><b>{msg.name ?? (isAi ? "AI" : "나")}</b>
		{#if meta.hidden}<Badge><EyeOff /> AI에게 숨김</Badge>{/if}
		{#if live}
			<span class="stamp">만드는 중…</span>
		{:else}
			<span class="stamp">
				{#if appSetting.showTimestamps}<time datetime={new Date(msg.createdAt).toISOString()} title={absolute(msg.createdAt)}>{relative(msg.createdAt)}</time>{/if}
				{#if model}<span class="model">{appSetting.showTimestamps ? "· " : ""}{model}</span>{/if}
				{#if meta.info?.cancelled}<span class="model">· 멈춤</span>{/if}
			</span>
		{/if}
		{#if !editing && !live && !selecting}
			<div class="actions" role="toolbar" aria-label="메시지 {no} 동작">
				<IconButton label="여기서 분기 나누기" size="sm" onclick={() => onbranch(msg)}><GitBranch /></IconButton>
				<IconButton label="복사" size="sm" onclick={copy}><Copy /></IconButton>
				<IconButton label="수정" size="sm" onclick={startEdit}><Pencil /></IconButton>
				<IconButton label={meta.chapter ? "챕터 표시 지우기" : "여기서 챕터 시작"} size="sm" onclick={chapter}>
					{#if meta.chapter}<BookmarkX />{:else}<Bookmark />{/if}
				</IconButton>
				<IconButton label={meta.hidden ? "AI에게 다시 보이기" : "AI에게 숨기기"} size="sm" onclick={() => session.setHidden(msg, !meta.hidden)}>
					{#if meta.hidden}<Eye />{:else}<EyeOff />{/if}
				</IconButton>
				{#if isAi && isLast}
					<IconButton label="처음부터 다시 생성 (스와이프 버림)" size="sm" disabled={session.busy} onclick={() => session.regenerate(false)}><RefreshCw /></IconButton>
				{/if}
				{#if isAi}
					<IconButton label="보낸 컨텍스트 보기" size="sm" onclick={() => onprompt(msg)}><FileSearch /></IconButton>
				{/if}
				{#if plugins.on("translate")}
					<IconButton label={translation ? (showOriginal ? "번역 보기" : "원문 보기") : "번역"} size="sm" disabled={toolBusy === "translate"} onclick={translate}><Languages /></IconButton>
				{/if}
				{#if plugins.on("tts")}
					<IconButton label={speech.messageId === msg.id ? "읽기 멈추기" : "소리 내어 읽기"} size="sm" disabled={speech.loading && speech.messageId === msg.id} onclick={() => speak(msg, translation && !showOriginal ? translation.text : msg.content)}>
						{#if speech.messageId === msg.id}<Square />{:else}<Volume2 />{/if}
					</IconButton>
				{/if}
				{#if plugins.on("imagegen") && isAi}
					<IconButton label="이 장면 그리기" size="sm" disabled={toolBusy === "draw"} onclick={draw}><ImagePlus /></IconButton>
				{/if}
				<IconButton label="삭제" size="sm" onclick={remove}><Trash2 /></IconButton>
			</div>
		{/if}
	</div>

	{#if editing}
		<div class="edit">
			<Textarea bind:value={draft} autosize maxRows={20} aria-label="메시지 수정" />
			<div class="edit-btns">
				<Button size="sm" variant="ghost" onclick={() => (editing = false)}>취소</Button>
				<Button size="sm" variant="primary" onclick={saveEdit}>저장</Button>
			</div>
		</div>
	{:else}
		{#if meta.attachments?.length}
			<div class="attach">
				{#each meta.attachments as a (a)}<a href={fileSrc(a)} target="_blank" rel="noreferrer"><img src={fileSrc(a)} alt="첨부 이미지" loading="lazy" /></a>{/each}
			</div>
		{/if}
		{#if translation && !live}<p class="tr-note">{showOriginal ? "원문" : `${translation.lang} 번역`} · <button class="link" onclick={() => (showOriginal = !showOriginal)}>{showOriginal ? "번역 보기" : "원문 보기"}</button></p>{/if}
		{#each blocks as b, i (i)}
			{#if b.kind === "p"}
				<p>
					{#each b.parts as part, j (j)}{#if part.kind === "action"}<em>{part.text}</em>{:else if part.kind === "speech"}<q>{part.text}</q>{:else}{part.text}{/if}{/each}
				</p>
			{:else if b.kind === "html"}
				<HtmlView code={b.code} />
			{:else if images[b.key]}
				<figure class="cg"><img src={images[b.key]} alt={b.key} loading="lazy" /><figcaption>{b.key}</figcaption></figure>
			{/if}
		{:else}
			{#if live}<p class="typing" aria-label="응답 기다리는 중"><span></span><span></span><span></span></p>{/if}
		{/each}
	{/if}

	{#if drawn.length}
		<div class="drawn">
			{#each drawn as d (d.path)}
				<a href={fileSrc(d.path)} target="_blank" rel="noreferrer" title={d.prompt}><img src={fileSrc(d.path)} alt={d.prompt} loading="lazy" /></a>
			{/each}
		</div>
	{/if}
	{#if toolBusy === "draw"}<p class="tr-note">장면을 그리는 중…</p>{/if}

	<!-- 스와이프는 마지막 AI 메시지에서만: < 번호 > 한 세트 (끝에서 > 를 누르면 새 응답) -->
	{#if isAi && isLast && !editing && !live}
		<div class="swipe">
			<IconButton label="이전 응답" size="sm" disabled={swipeId === 0 || session.busy} onclick={() => session.swipe(msg, -1)}><ChevronLeft /></IconButton>
			<span>{swipeId + 1} / {Math.max(swipes, 1)}</span>
			<IconButton label={swipeId >= swipes - 1 ? "새 응답 만들기 (스와이프)" : "다음 응답"} size="sm" disabled={session.busy} onclick={() => session.swipe(msg, 1)}><ChevronRight /></IconButton>
		</div>
	{/if}
</div>

<style>
	.tr-note {
		margin: 0 0 4px;
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.tr-note .link {
		padding: 0;
		border: none;
		background: none;
		color: var(--accent-text);
		font: inherit;
		cursor: pointer;
	}
	.drawn {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin-top: var(--space-2);
	}
	.drawn img {
		display: block;
		max-width: min(100%, 360px);
		border-radius: var(--radius-md);
	}
	.attach {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-2);
		margin: var(--space-1) 0 var(--space-2);
	}
	.attach img {
		display: block;
		max-width: 220px;
		max-height: 220px;
		border-radius: var(--radius-md);
		object-fit: cover;
		box-shadow: 0 0 0 1px var(--border);
	}
	.block {
		position: relative;
		margin: 0 calc(var(--space-3) * -1);
		padding: var(--space-2) var(--space-3);
		border-radius: var(--radius-md);
		scroll-margin-top: var(--space-6);
	}
	.block:hover {
		background: var(--surface-2);
	}
	.block.selecting {
		cursor: pointer;
	}
	.block.picked {
		outline: 2px solid var(--accent);
		outline-offset: -2px;
	}
	.pick {
		width: 15px;
		height: 15px;
		accent-color: var(--accent);
	}
	.block p {
		white-space: pre-wrap;
		overflow-wrap: anywhere;
	}
	.block p + p {
		margin-top: 0.6em;
	}
	em {
		color: var(--text-muted);
	}
	q {
		quotes: none;
		color: var(--text);
		font-weight: 500;
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
	.who b {
		white-space: nowrap;
	}
	.no {
		color: var(--text-faint);
		font-variant-numeric: tabular-nums;
	}
	.stamp {
		white-space: nowrap;
		display: inline-flex;
		gap: 4px;
		color: var(--text-faint);
	}
	.stamp time {
		cursor: help;
	}
	/* 자리를 차지하지 않게 블록 오른쪽 위에 띄움 */
	.actions {
		position: absolute;
		top: 2px;
		right: var(--space-2);
		z-index: 2;
		display: flex;
		padding: 2px;
		border: 1px solid var(--border);
		border-radius: var(--radius-sm);
		background: var(--surface);
		box-shadow: var(--shadow-sm);
		opacity: 0;
		pointer-events: none;
		transition: opacity var(--duration-fast) var(--ease);
	}
	.block:hover .actions,
	.block:focus-within .actions,
	.block.tapped .actions {
		opacity: 1;
		pointer-events: auto;
	}
	.user {
		border-left: 3px solid var(--accent);
		border-radius: 0 var(--radius-md) var(--radius-md) 0;
		background: var(--accent-soft);
	}
	.user:hover {
		background: var(--accent-soft-hover);
	}
	.hidden {
		opacity: 0.6;
		outline: 1px dashed var(--border-strong);
	}
	.cg {
		margin: var(--space-4) auto 0;
		width: min(320px, 100%);
		border-radius: var(--radius-lg);
		overflow: hidden;
		border: 1px solid var(--border);
	}
	.cg img {
		display: block;
		width: 100%;
	}
	.cg figcaption {
		padding: 2px var(--space-3);
		font-size: var(--text-xs);
		color: var(--text-faint);
	}
	.swipe {
		display: flex;
		align-items: center;
		justify-content: flex-end;
		gap: 2px;
		margin-top: var(--space-2);
		font-size: var(--text-xs);
		color: var(--text-muted);
	}
	.edit {
		display: flex;
		flex-direction: column;
		gap: var(--space-2);
		margin-top: var(--space-1);
	}
	.edit-btns {
		display: flex;
		justify-content: flex-end;
		gap: var(--space-1);
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
</style>
