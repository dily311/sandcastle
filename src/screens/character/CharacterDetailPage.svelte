<script lang="ts">
	import Icon from "$/components/Icon.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import { pop, push } from "svelte-spa-router";

	// props로 편집할 characterId를 받습니다. (null일 경우 새로 만들기)
	let { characterId = null }: { characterId?: string | null } = $props();

	// 기본 이모지 선택지
	const EMOJI_OPTIONS = ["🤖", "🐱", "🦊", "🐶", "🐰", "🐼", "🐻", "🐲", "🧙‍♂️", "👩‍🚀", "🕵️‍♂️", "🎨"];

	// 초기 캐릭터 데이터 탐색
	const existingChar = $derived(characterId ? characters.value.find((c) => c.id === characterId) : null);

	// 폼 상태 관리 ($state)
	let avatar = $state(existingChar?.avatar ?? "🤖");
	let name = $state(existingChar?.name ?? "");
	let description = $state(existingChar?.description ?? "");
	let firstMessage = $state(existingChar?.firstMessage ?? "");
	let tags = $state<string[]>(existingChar?.tags ? [...existingChar.tags] : []);

	// UI 보조 상태
	let showEmojiPicker = $state(false);
	let tagInput = $state("");
	let isSaved = $state(false);

	// 태그 추가
	function addTag() {
		const trimmed = tagInput.trim();
		if (trimmed && !tags.includes(trimmed)) {
			tags = [...tags, trimmed];
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
		tags = tags.filter((t) => t !== tagToRemove);
	}

	// 저장 처리
	function handleSave() {
		if (!name.trim()) return;

		const payload: Partial<Character> = {
			name,
			avatar,
			description,
			firstMessage,
			tags,
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
		// const conv = app.startConversation(characterId);
		// app.go({ name: "chat", conversationId: conv.id });
	}
</script>

<div class="page narrow">
	<!-- 상단 헤더 -->
	<header class="page-head sticky">
		<div class="head-left">
			<button class="icon-btn" onclick={() => pop()} title="戻る">
				<Icon name="ArrowLeft" size={18} />
			</button>
			<div>
				<div class="breadcrumb">ワークスペース / 캐릭터 편집</div>
				<h1 class="page-title sm">
					{characterId ? name || "キャラクター編集" : "新規キャラクター作成"}
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
					<Icon name="chat" size={14} /> 会話する
				</button>
			{/if}
			<button class="btn primary sm" onclick={handleSave}> 저장 </button>
		</div>
	</header>

	<!-- 히어로 히더 (이모지, 이름, 한줄 소개) -->
	<section class="editor-hero">
		<div class="emoji-picker-wrap">
			<button class="avatar xl" onclick={() => (showEmojiPicker = !showEmojiPicker)} title="絵文字を変更">
				{avatar}
			</button>

			{#if showEmojiPicker}
				<div class="emoji-pop">
					{#each EMOJI_OPTIONS as item}
						<button
							class={avatar === item ? "on" : ""}
							onclick={() => {
								avatar = item;
								showEmojiPicker = false;
							}}
						>
							{item}
						</button>
					{/each}
				</div>
			{/if}
		</div>
	</section>

	<hr class="divider" />

	<!-- 주요 필드 설정 -->
	<div class="field">
		<label class="field-label" for="char-desc">詳細ペルソナ / プロンプト</label>
		<p class="field-desc">AIの性格、口調、背景設定、制約事項などを詳しく記述します。</p>
		<textarea
			id="char-desc"
			class="textarea tall"
			placeholder="例: あなたは明るく元気なAIアシスタントです。語尾に「〜だよ」をつけて話します。"
			bind:value={description}></textarea>
	</div>

	<div class="field">
		<label class="field-label" for="char-greeting">最初の挨拶メッセージ</label>
		<p class="field-desc">会話を開始したときにキャラクターが最初に送信するメッセージです。</p>
		<textarea
			id="char-greeting"
			class="textarea"
			placeholder="例: こんにちは！今日はどんなことを話す？"
			bind:value={firstMessage}
			rows="3"></textarea>
	</div>

	<!-- 실시간 프리뷰 영역 -->
	{#if firstMessage || name}
		<div class="preview-block">
			<div class="preview-label">プレビュー</div>
			<div class="preview-bubble">
				<span class="avatar">{avatar}</span>
				<div>
					<div class="preview-name">{name || "キャラクター"}</div>
					<div class="preview-text">{firstMessage || "..."}</div>
				</div>
			</div>
		</div>
	{/if}

	<hr class="divider" />

	<!-- 태그 입력 -->
	<div class="field">
		<label class="field-label" for="tag-input-field">タグ</label>
		<div class="tag-input-box">
			<div class="tag-row small">
				{#each tags as t (t)}
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
				placeholder="タグを入力してEnter..."
				bind:value={tagInput}
				onkeydown={handleTagKeyDown}
				onblur={addTag}
			/>
		</div>
	</div>
</div>

<style>
	.icon-btn {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 28px;
		height: 28px;
		border-radius: var(--radius);
		color: var(--text-soft);
		flex: none;
		transition:
			background 120ms ease,
			color 120ms ease;
		background: transparent;
		border: none;
		cursor: pointer;
	}
	.icon-btn:hover {
		background: var(--bg-hover);
		color: var(--text);
	}
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
		background: var(--bg);
		padding: 12px 0;
		margin: -12px 0 20px;
		border-bottom: 1px solid var(--border);
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

	.saved-chip {
		display: inline-flex;
		align-items: center;
		gap: 4px;
		font-size: 12px;
		color: #3f9e6d;
		margin-right: 4px;
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

	.avatar {
		width: 30px;
		height: 30px;
		font-size: 15px;
		display: inline-flex;
		align-items: center;
		justify-content: center;
	}

	.avatar.xl {
		width: 56px;
		height: 56px;
		font-size: 28px;
		cursor: pointer;
	}
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
		background: var(--bg);
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
		background: var(--bg-subtle);
		border: 1px solid var(--border);
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

	.field {
		margin-top: 22px;
	}

	.divider {
		border: none;
		border-top: 1px solid var(--border);
		margin: 28px 0;
	}

	.field-label {
		display: block;
		font-size: 13.5px;
		font-weight: 600;
		color: var(--text);
		margin-bottom: 4px;
	}

	.field-desc {
		font-size: 12px;
		color: var(--text-muted);
		margin-bottom: 8px;
		line-height: 1.4;
	}

	.textarea {
		width: 100%;
		padding: 10px 12px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
		background: var(--bg-input);
		color: var(--text);
		font-size: 13.5px;
		line-height: 1.6;
		outline: none;
		resize: vertical;
		transition:
			border-color 120ms ease,
			box-shadow 120ms ease;
	}

	.textarea:focus {
		border-color: var(--accent);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	.tag-input-box {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border-strong);
		border-radius: var(--radius);
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
		color: var(--danger, #e53935);
	}
</style>
