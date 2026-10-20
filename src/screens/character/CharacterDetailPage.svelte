<script lang="ts">
	import { _ } from "svelte-i18n";
	import Avatar from "$/components/avatar.svelte";
	import Button from "$/components/ui/button/Button.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import { pop, push, router } from "svelte-spa-router";

	// props로 편집할 characterId를 받습니다. (null일 경우 새로 만들기)
	let { params }: { params?: { characterId?: string } } = $props();

	const characterId = $derived(params?.characterId ?? null);
	// 초기 캐릭터 데이터 탐색
	const existingChar = $derived(characterId ? characters.value.find((c) => c.id === characterId) : null);

	let form = $state({
		avatar: "🤖",
		name: "",
		description: "",
		firstMessage: "",
		tags: [] as string[],
	});

	$effect(() => {
		if (existingChar) {
			form.avatar = existingChar.avatar ?? "🤖";
			form.name = existingChar.name ?? "";
			form.description = existingChar.description ?? "";
			form.firstMessage = existingChar.firstMessage ?? "";
			form.tags = existingChar.tags ? [...existingChar.tags] : [];
		}
	});

	// UI 보조 상태
	let showEmojiPicker = $state(false);
	let tagInput = $state("");
	let isSaved = $state(false);

	// 태그 추가
	function addTag() {
		const trimmed = tagInput.trim();
		if (trimmed && !form.tags.includes(trimmed)) {
			form.tags = [...form.tags, trimmed];
			tagInput = "";
		}
	}

	// 태그 키다운 (Enter, Comma)
	function handleTagKeyDown(e: KeyboardEvent) {
		if (e.key === "Enter" || e.key === ",") {
			e.preventDefault();
			addTag();
		}
	}

	// 태그 삭제
	function removeTag(tagToRemove: string) {
		form.tags = form.tags.filter((t) => t !== tagToRemove);
	}

	// 저장 처리
	function handleSave() {
		if (!form.name.trim()) return;

		const payload: Partial<Character> = {
			name: form.name,
			avatar: form.avatar,
			description: form.description,
			firstMessage: form.firstMessage,
			tags: form.tags,
			updatedAt: Date.now(),
		};

		if (characterId && existingChar) {
			characters.patch(characterId, payload);
		} else {
			// const created = characters.create(payload);
			// 저장 후 해당 캐릭터 상세 페이지/목록으로 이동 또는 ID 갱신
			//   app.go({ name: "edit", characterId: created.id });
		}

		// 저장 완료 피드백
		isSaved = true;
		setTimeout(() => {
			isSaved = false;
		}, 2000);
	}

	// 채팅 바로 시작
	function handleStartChat() {
		if (!characterId) return;
	}
</script>

<div class="page narrow">
	<!-- 상단 헤더 -->
	<header class="page-head sticky">
		<div class="head-left">
			<Button variant="ghost" size="icon-lg" aria-label={$_("global.back")} onclick={() => pop()}>
				<Icon name={"ChevronLeft"} />
			</Button>
			<div>
				<div class="breadcrumb">작업공간 / 캐릭터 편집</div>
				<h1 class="page-title sm">
					{characterId ? form.name || "캐릭터 편집" : "신규 캐릭터 작성"}
				</h1>
			</div>
		</div>
		<div class="head-actions">
			{#if isSaved}
				<span class="saved-chip">
					<Icon name="check" size={13} /> 저장됨
				</span>
			{/if}
			{#if characterId}
				<button class="btn ghost sm" onclick={handleStartChat}>
					<Icon name="chat" size={14} /> 대화하기
				</button>
			{/if}
			<button class="btn primary sm" onclick={handleSave}> 저장 </button>
		</div>
	</header>

	<section class="editor-hero">
		<div class="emoji-picker-wrap">
			<button class="avatar xl" onclick={() => (showEmojiPicker = !showEmojiPicker)} title="이미지 변경">
				<Avatar avatar={form.avatar} />
			</button>
		</div>
	</section>

	<hr class="divider" />

	<!-- 주요 필드 설정 -->
	<div class="field">
		<label class="field-label" for="char-desc">상세 페르소나 / 프롬프트</label>
		<p class="field-desc">AI의 성격, 어조, 배경 설정, 제약 사항 등을 자세하게 기술합니다.</p>
		<textarea
			id="char-desc"
			class="textarea tall"
			placeholder="예시 : 당신은 밝고 건강한 AI 어시스턴트입니다. 어미에 '~다요'를 붙여 말합니다."
			bind:value={form.description}></textarea>
	</div>

	<div class="field">
		<label class="field-label" for="char-greeting">첫 인사말</label>
		<p class="field-desc">대화를 시작할 때 캐릭터가 처음 보낼 메시지입니다.</p>
		<textarea
			id="char-greeting"
			class="textarea"
			placeholder="예시: 안녕하세요! 오늘은 무슨 얘기를 할까요?"
			bind:value={form.firstMessage}
			rows="3"></textarea>
	</div>

	<!-- 실시간 프리뷰 영역 -->
	{#if form.firstMessage || form.name}
		<div class="preview-block">
			<div class="preview-label">미리보기</div>
			<div class="preview-bubble">
				<Avatar avatar={form.avatar} />
				<div>
					<div class="preview-name">{form.name || "캐릭터"}</div>
					<div class="preview-text">{form.firstMessage || "..."}</div>
				</div>
			</div>
		</div>
	{/if}

	<hr class="divider" />

	<!-- 태그 입력 -->
	<div class="field">
		<label class="field-label" for="tag-input-field">태그</label>
		<div class="tag-input-box">
			<div class="tag-row small">
				{#each form.tags as t (t)}
					<span class="tag">
						{t}
						<button class="tag-del" onclick={() => removeTag(t)}>×</button>
					</span>
				{/each}
			</div>
			<input
				id="tag-input-field"
				type="text"
				class="tag-input"
				placeholder="태그를 입력하고 Enter..."
				bind:value={tagInput}
				onkeydown={handleTagKeyDown}
				onblur={addTag}
			/>
		</div>
	</div>
</div>

<style>
	/* ============================================================
   Sidebar
   ============================================================ */

	.sidebar {
		width: var(--sidebar-w);
		flex: none;
		background: var(--bg-sidebar);
		border-right: 1px solid var(--border-color);
		display: flex;
		flex-direction: column;
		padding: 8px 6px 0;
		overflow: hidden;
		user-select: none;
	}
	.sidebar.collapsed {
		width: 44px;
		align-items: center;
		padding-top: 10px;
	}

	.sidebar-head {
		display: flex;
		align-items: center;
		gap: 4px;
		padding: 2px 4px 8px;
	}
	.workspace {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		padding: 4px 4px;
		border-radius: var(--radius-md);
	}
	.ws-badge {
		width: 22px;
		height: 22px;
		border-radius: 5px;
		background: var(--text-color);
		color: var(--background-primary);
		font-size: 10px;
		font-weight: 700;
		display: inline-flex;
		align-items: center;
		justify-content: center;
		letter-spacing: 0.03em;
		flex: none;
	}
	.ws-text {
		min-width: 0;
	}
	.ws-name {
		font-size: 13.5px;
		font-weight: 600;
		line-height: 1.25;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.ws-sub {
		font-size: 11.5px;
		color: var(--text-muted);
		line-height: 1.3;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	.sidebar-nav {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-bottom: 10px;
	}

	.nav-item {
		display: flex;
		align-items: center;
		gap: 8px;
		width: 100%;
		min-width: 0;
		padding: 5px 8px;
		border-radius: var(--radius-md);
		color: var(--text-soft);
		font-size: 13.5px;
		text-align: left;
		transition:
			background 100ms ease,
			color 100ms ease;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.nav-item:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}
	.nav-item.active {
		background: var(--bg-active);
		color: var(--text-color);
		font-weight: 500;
	}
	.nav-item svg {
		flex: none;
		opacity: 0.8;
	}
	.nav-emoji {
		width: 18px;
		text-align: center;
		flex: none;
		font-size: 13px;
	}

	.nav-item.row {
		padding: 0;
		gap: 0;
	}
	.nav-item.row .row-main {
		display: flex;
		align-items: center;
		gap: 8px;
		flex: 1;
		min-width: 0;
		padding: 5px 4px 5px 8px;
		color: inherit;
		font-size: 13.5px;
		text-align: left;
	}
	.row-action {
		width: 24px;
		height: 24px;
		margin-right: 4px;
		border-radius: 4px;
		color: var(--text-muted);
		display: none;
		align-items: center;
		justify-content: center;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.nav-item.row:hover .row-action {
		display: inline-flex;
	}
	.row-action:hover {
		background: var(--bg-active);
		color: var(--danger);
	}

	.sidebar-section {
		display: flex;
		flex-direction: column;
		gap: 1px;
		padding-bottom: 12px;
	}
	.sidebar-section.grow {
		flex: 1;
		overflow-y: auto;
		min-height: 0;
	}
	.section-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
		padding: 6px 8px 3px;
	}
	.section-empty {
		font-size: 12px;
		color: var(--text-muted);
		padding: 2px 8px 6px;
	}

	.sidebar-foot {
		display: flex;
		align-items: center;
		gap: 7px;
		border-top: 1px solid var(--border-color);
		padding: 9px 10px;
		font-size: 11.5px;
		color: var(--text-muted);
	}
	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: #4caf7d;
		flex: none;
	}

	/* ============================================================
   Page (search / edit / settings)
   ============================================================ */

	.page {
		flex: 1;
		overflow-y: auto;
		padding: 40px 56px 90px;
	}
	.page.narrow > * {
		max-width: 720px;
		margin-left: auto;
		margin-right: auto;
	}

	.page-head {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: 20px;
		margin-bottom: 26px;
	}
	.page-head.sticky {
		position: sticky;
		top: -40px;
		z-index: 5;
		background: var(--background-primary);
		padding: 12px 0;
		margin: -12px 0 20px;
		border-bottom: 1px solid var(--border-color);
		align-items: center;
	}
	.head-left {
		display: flex;
		align-items: center;
		gap: 10px;
		min-width: 0;
	}
	.head-actions {
		display: flex;
		align-items: center;
		gap: 6px;
		flex: none;
	}
	.breadcrumb {
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 4px;
	}
	.page-title {
		font-size: 30px;
		font-weight: 700;
		letter-spacing: -0.02em;
		line-height: 1.2;
	}
	.page-title.sm {
		font-size: 18px;
		font-weight: 600;
	}
	.page-sub {
		color: var(--text-soft);
		margin-top: 6px;
		font-size: 13.5px;
	}
	.saved-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: #3f9e6d;
		margin-right: 4px;
	}

	/* search */
	.searchbar {
		display: flex;
		align-items: center;
		gap: 9px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		padding: 8px 12px;
		background: var(--bg-input);
		color: var(--text-muted);
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}
	.searchbar:focus-within {
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}
	.searchbar-input {
		flex: 1;
		border: none;
		outline: none;
		background: transparent;
		font-size: 14.5px;
		color: var(--text-color);
	}

	.filter-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 16px;
		margin-top: 14px;
		flex-wrap: wrap;
	}
	.tag-row {
		display: flex;
		flex-wrap: wrap;
		gap: 6px;
		align-items: center;
	}
	.tag-row.small .tag {
		height: 20px;
		font-size: 11.5px;
	}
	.tag-count {
		color: var(--text-muted);
		font-size: 10.5px;
	}
	.filter-right {
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.select.mini {
		width: auto;
		height: 28px;
		padding: 0 8px;
		font-size: 12.5px;
	}
	.result-count {
		display: flex;
		align-items: center;
		gap: 10px;
		margin: 22px 0 12px;
		font-size: 12.5px;
		color: var(--text-muted);
	}

	.card-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(248px, 1fr));
		gap: 12px;
	}
	.char-card {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-lg);
		padding: 16px;
		background: var(--background-primary);
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 8px;
		transition:
			box-shadow 140ms ease,
			border-color 140ms ease,
			transform 140ms ease;
	}
	.char-card:hover {
		box-shadow: var(--shadow);
		border-color: transparent;
		transform: translateY(-1px);
	}
	.char-card-top {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
	}
	.avatar {
		width: 30px;
		height: 30px;
		font-size: 15px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}
	.avatar.lg {
		width: 40px;
		height: 40px;
		font-size: 20px;
	}
	.avatar.xl {
		width: 56px;
		height: 56px;
		font-size: 28px;
		cursor: pointer;
	}
	.user-avatar {
		background: var(--text-color);
		color: var(--background-primary);
		font-size: 12px;
		font-weight: 600;
		border-color: transparent;
	}
	.star-btn {
		color: var(--text-muted);
		padding: 4px;
		border-radius: 4px;
		display: inline-flex;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.star-btn:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}
	.star-btn.on {
		color: #e0a800;
	}
	.char-name {
		font-size: 15px;
		font-weight: 600;
	}
	.char-tagline {
		font-size: 13px;
		color: var(--text-soft);
		line-height: 1.5;
		display: -webkit-box;
		-webkit-line-clamp: 2;
		line-clamp: 2;
		-webkit-box-orient: vertical;
		overflow: hidden;
		min-height: 39px;
	}
	.char-card-actions {
		display: flex;
		gap: 4px;
		margin-top: 6px;
		padding-top: 10px;
		border-top: 1px solid var(--border-color);
	}

	.char-list {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-lg);
		overflow: hidden;
	}
	.char-row {
		display: flex;
		align-items: center;
		gap: 12px;
		padding: 10px 14px;
		cursor: pointer;
		transition: background 100ms ease;
	}
	.char-row + .char-row {
		border-top: 1px solid var(--border-color);
	}
	.char-row:hover {
		background: var(--bg-hover);
	}
	.char-row-main {
		flex: 1;
		min-width: 0;
	}
	.char-row-name {
		display: flex;
		align-items: center;
		gap: 8px;
		font-size: 14px;
		font-weight: 500;
	}
	.chip {
		font-size: 10.5px;
		color: var(--text-muted);
		border: 1px solid var(--border-color);
		border-radius: 4px;
		padding: 0 5px;
		line-height: 16px;
	}
	.char-row-sub {
		font-size: 12.5px;
		color: var(--text-muted);
	}
	.char-row-actions {
		display: flex;
		gap: 2px;
		flex: none;
	}

	/* ============================================================
   Character editor
   ============================================================ */

	.editor-hero {
		display: flex;
		gap: 18px;
		align-items: center;
		margin-top: 8px;
	}
	.emoji-picker-wrap {
		position: relative;
		flex: none;
	}
	.emoji-pop {
		position: absolute;
		z-index: 20;
		top: 62px;
		left: 0;
		width: 232px;
		display: grid;
		grid-template-columns: repeat(6, 1fr);
		gap: 2px;
		padding: 8px;
		background: var(--background-primary);
		border-radius: var(--radius-lg);
		box-shadow: var(--shadow-lg);
	}
	.emoji-pop button {
		height: 32px;
		border-radius: 5px;
		font-size: 17px;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.emoji-pop button:hover,
	.emoji-pop button.on {
		background: var(--bg-hover);
	}
	.editor-hero-fields {
		flex: 1;
		min-width: 0;
	}
	.title-input {
		width: 100%;
		border: none;
		outline: none;
		background: transparent;
		font-size: 28px;
		font-weight: 700;
		letter-spacing: -0.02em;
		padding: 2px 0;
	}
	.title-input::placeholder {
		color: var(--text-muted);
		opacity: 0.55;
	}
	.sub-input {
		width: 100%;
		border: none;
		outline: none;
		background: transparent;
		font-size: 13.5px;
		color: var(--text-soft);
		padding: 2px 0;
	}
	.sub-input::placeholder {
		color: var(--text-muted);
	}
	.textarea.tall {
		min-height: 170px;
	}
	.tag-input {
		border: none;
		outline: none;
		background: transparent;
		min-width: 160px;
		flex: 1;
		font-size: 13px;
		padding: 2px 0;
	}
	.preview-block {
		margin-bottom: 40px;
	}
	.preview-label {
		font-size: 11px;
		font-weight: 600;
		letter-spacing: 0.05em;
		text-transform: uppercase;
		color: var(--text-muted);
		margin-bottom: 10px;
	}
	.preview-bubble {
		display: flex;
		gap: 12px;
		padding: 14px 16px;
		border-radius: var(--radius-lg);
		background: var(--background-secondary);
		border: 1px solid var(--border-color);
	}
	.preview-name {
		font-size: 13px;
		font-weight: 600;
		margin-bottom: 2px;
	}
	.preview-text {
		font-size: 13.5px;
		color: var(--text-soft);
		white-space: pre-wrap;
	}

	/* ============================================================
   Chat
   ============================================================ */

	.chat {
		flex: 1;
		display: flex;
		flex-direction: column;
		min-height: 0;
	}
	.chat-head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 12px;
		padding: 10px 20px;
		border-bottom: 1px solid var(--border-color);
		background: var(--background-primary);
		flex: none;
	}
	.chat-title-wrap {
		min-width: 0;
	}
	.chat-title {
		font-size: 14.5px;
		font-weight: 600;
		padding: 1px 4px;
		margin-left: -4px;
		border-radius: 4px;
		text-align: left;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.chat-title:hover {
		background: var(--bg-hover);
	}
	.title-inline {
		font-size: 14.5px;
		font-weight: 600;
		border: 1px solid var(--accent-color);
		border-radius: 4px;
		padding: 0 4px;
		background: var(--bg-input);
		outline: none;
	}
	.chat-sub {
		font-size: 11.5px;
		color: var(--text-muted);
	}

	.chat-scroll {
		flex: 1;
		overflow-y: auto;
		padding: 30px 20px 20px;
	}
	.chat-inner {
		max-width: 720px;
		margin: 0 auto;
	}
	.chat-intro {
		text-align: center;
		padding: 12px 0 36px;
	}
	.chat-intro h2 {
		font-size: 20px;
		font-weight: 600;
		margin-top: 12px;
	}
	.chat-intro p {
		color: var(--text-muted);
		font-size: 13px;
		margin-top: 4px;
	}

	.msg {
		display: flex;
		gap: 12px;
		padding: 12px 8px;
		border-radius: var(--radius-lg);
	}
	.msg:hover {
		background: var(--background-secondary);
	}
	.msg-avatar {
		flex: none;
		padding-top: 2px;
	}
	.msg-body {
		min-width: 0;
		flex: 1;
	}
	.msg-meta {
		display: flex;
		align-items: baseline;
		gap: 8px;
		margin-bottom: 2px;
	}
	.msg-name {
		font-size: 13px;
		font-weight: 600;
	}
	.msg-time {
		font-size: 11px;
		color: var(--text-muted);
	}

	.md p {
		margin: 0 0 2px;
		line-height: 1.72;
		white-space: pre-wrap;
		word-break: break-word;
	}
	.md-space {
		height: 10px;
	}
	.md-li {
		display: flex;
		gap: 8px;
		line-height: 1.72;
		padding-left: 2px;
	}
	.md-marker {
		color: var(--text-muted);
		flex: none;
		min-width: 14px;
	}
	.md blockquote {
		margin: 4px 0;
		padding: 2px 0 2px 12px;
		border-left: 3px solid var(--border-strong);
		color: var(--text-soft);
		font-size: 13px;
	}
	.md code {
		font-family: var(--mono);
		font-size: 12.5px;
		background: var(--bg-code);
		border-radius: 3px;
		padding: 1px 5px;
		color: #c7254e;
	}
	[data-theme="dark"] .md code {
		color: #ff9ab0;
	}

	.typing {
		display: flex;
		gap: 4px;
		padding: 8px 0 4px;
	}
	.typing span {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-muted);
		animation: blink 1.1s infinite ease-in-out;
	}
	.typing span:nth-child(2) {
		animation-delay: 0.16s;
	}
	.typing span:nth-child(3) {
		animation-delay: 0.32s;
	}
	@keyframes blink {
		0%,
		80%,
		100% {
			opacity: 0.25;
			transform: translateY(0);
		}
		40% {
			opacity: 1;
			transform: translateY(-2px);
		}
	}

	.suggestions {
		display: flex;
		flex-wrap: wrap;
		gap: 8px;
		padding: 18px 8px 4px;
	}
	.suggestion {
		border: 1px solid var(--border-strong);
		border-radius: 999px;
		padding: 6px 13px;
		font-size: 12.5px;
		color: var(--text-soft);
		transition:
			background 120ms ease,
			color 120ms ease;
		background: transparent;
		cursor: pointer;
	}
	.suggestion:hover {
		background: var(--bg-hover);
		color: var(--text-color);
	}

	.composer-wrap {
		flex: none;
		padding: 8px 20px 18px;
		background: var(--background-primary);
	}
	.composer {
		max-width: 720px;
		margin: 0 auto;
		display: flex;
		align-items: flex-end;
		gap: 8px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-lg);
		background: var(--bg-input);
		padding: 8px 8px 8px 12px;
		box-shadow: var(--shadow-sm);
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}
	.composer:focus-within {
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}
	.composer-input {
		flex: 1;
		border: none;
		outline: none;
		resize: none;
		background: transparent;
		font-size: 14px;
		line-height: 1.6;
		max-height: 180px;
		padding: 3px 0;
	}
	.composer-hint {
		max-width: 720px;
		margin: 7px auto 0;
		font-size: 11.5px;
		color: var(--text-muted);
		text-align: center;
	}

	/* ============================================================
   Settings
   ============================================================ */

	.tabs {
		display: flex;
		gap: 18px;
		border-bottom: 1px solid var(--border-color);
		margin-bottom: 8px;
	}
	.tab {
		padding: 8px 2px;
		font-size: 13.5px;
		color: var(--text-muted);
		border-bottom: 2px solid transparent;
		margin-bottom: -1px;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.tab:hover {
		color: var(--text-color);
	}
	.tab.on {
		color: var(--text-color);
		font-weight: 500;
		border-bottom-color: var(--text-color);
	}

	.set-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 24px;
		padding: 16px 0;
		border-bottom: 1px solid var(--border-color);
	}
	.set-row-text {
		min-width: 0;
	}
	.set-row-title {
		font-size: 13.5px;
		font-weight: 500;
	}
	.set-row-desc {
		font-size: 12.5px;
		color: var(--text-muted);
		margin-top: 2px;
	}
	.set-row-control {
		flex: none;
		display: flex;
		align-items: center;
		gap: 8px;
	}
	.input.compact,
	.select.compact {
		width: 200px;
		height: 30px;
		padding: 0 9px;
	}
	.key-row {
		display: flex;
		gap: 8px;
	}
	.field {
		margin-top: 22px;
	}

	.stat-grid {
		display: grid;
		grid-template-columns: repeat(3, 1fr);
		gap: 10px;
		margin: 20px 0 8px;
	}
	.stat {
		border: 1px solid var(--border-color);
		border-radius: var(--radius-lg);
		padding: 14px 16px;
	}
	.stat-num {
		font-size: 22px;
		font-weight: 700;
		letter-spacing: -0.02em;
	}
	.stat-label {
		font-size: 12px;
		color: var(--text-muted);
	}
	.foot-note {
		font-size: 12.5px;
		color: var(--text-muted);
		line-height: 1.7;
		padding-bottom: 40px;
	}

	/* ============================================================
   Responsive
   ============================================================ */

	@media (max-width: 900px) {
		.page {
			padding: 28px 22px 80px;
		}
		.hide-sm {
			display: none;
		}
		.stat-grid {
			grid-template-columns: 1fr;
		}
	}

	@media (max-width: 700px) {
		.sidebar {
			width: 190px;
		}
		.page-title {
			font-size: 24px;
		}
		.set-row {
			flex-direction: column;
			align-items: flex-start;
			gap: 10px;
		}
	}
	/* ============================================================
   Editor Specific Additions
   ============================================================ */

	.field-label {
		display: block;
		font-size: 13.5px;
		font-weight: 600;
		color: var(--text-color);
		margin-bottom: 4px;
	}

	.field-desc {
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 8px;
		line-height: 1.4;
	}

	.field-head-row {
		display: flex;
		align-items: center;
		justify-content: space-between;
		margin-bottom: 8px;
	}

	.field-val {
		font-size: 12.5px;
		font-weight: 600;
		font-family: var(--mono, monospace);
		color: var(--text-soft);
	}

	.tag-input-box {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius-md);
		background: var(--bg-input);
	}

	.tag-del {
		background: none;
		border: none;
		color: var(--text-muted);
		font-size: 12px;
		margin-left: 2px;
		cursor: pointer;
		padding: 0 2px;
		line-height: 1;
	}

	.tag-del:hover {
		color: var(--color-red, #e53935);
	}
</style>
