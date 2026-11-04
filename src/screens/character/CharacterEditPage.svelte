<script lang="ts">
	import { _ } from "svelte-i18n";
	import Button from "$/components/ui/button/Button.svelte";
	import Icon from "$/components/ui/Icon.svelte";
	import { characters } from "$/lib/chat/characterStore.svelte";
	import type { Character } from "$/lib/type/conv";
	import { pop, push, router } from "svelte-spa-router";
	import { handleGoBack } from "$/app/routes";

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
			<Button variant="ghost" size="icon-lg" aria-label={$_("global.back")} onclick={handleGoBack}>
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
				{form.avatar}
			</button>
		</div>
		<div class="editor-hero-fields">
			<input class="title-input" placeholder="캐릭터 이름" value={form.name} />
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
				<span class="avatar">{form.avatar}</span>
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

	.editor-hero {
		display: flex;
		gap: 18px;
		align-items: center;
		margin-top: 8px;
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

	.field {
		margin-top: 22px;
	}

	/* ============================================================
   Responsive
   ============================================================ */

	@media (max-width: 900px) {
		.page {
			padding: 28px 22px 80px;
		}
	}

	@media (max-width: 700px) {
		.page-title {
			font-size: 24px;
		}
	}
	/* ============================================================
   Editor Specific Additions
   ============================================================ */

	.divider {
		border: none;
		border-top: 1px solid var(--border-color);
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
		border: 1px solid var(--border-strong-color);
		border-radius: var(--radius-md);
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
		border-color: var(--accent-color);
		box-shadow: 0 0 0 3px var(--accent-soft);
	}

	.tag-input-box {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 6px;
		padding: 6px 10px;
		border: 1px solid var(--border-strong-color);
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
