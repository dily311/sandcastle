<script lang="ts">
	import { createEventDispatcher } from "svelte";
	import { pop, router } from "svelte-spa-router";

	let params: string | null;

	$effect(() => {
		if (router.params && 'characterId' in router.params) {
			params = router.params?.characterId ?? null;
		}
		return;
	});
	
	
	const isEdit = false;
	const initialData = {
		name: "",
		description: "",
		personality: "",
		firstMessage: "",
		avatarUrl: "",
	};

	// 바인딩용 상태 변수들
	let name = $state(initialData.name);
	let description = $state(initialData.description);
	let personality = $state(initialData.personality);
	let firstMessage = $state(initialData.firstMessage);
	let avatarUrl = $state(initialData.avatarUrl);

	const dispatch = createEventDispatcher<{
		save: typeof initialData;
		cancel: void;
	}>();

	// 이미지 업로드 모사 함수
	function handleImageUpload(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files && target.files[0]) {
			const file = target.files[0];
			// 실제 구현에서는 URL.createObjectURL 또는 서버 업로드 로직 사용
			avatarUrl = URL.createObjectURL(file);
		}
	}

	function handleSubmit(e: SubmitEvent) {
		e.preventDefault();
		dispatch("save", { name, description, personality, firstMessage, avatarUrl });
	}

	function handleCancel() {
		dispatch("cancel");
	}
</script>

<div class="character-form-container">
	<header class="form-header">
		<h2>{isEdit ? "캐릭터 편집" : "새 캐릭터 만들기"}</h2>
		<p>{isEdit ? "캐릭터의 설정과 정보를 수정합니다." : "나만의 AI 캐릭터를 새롭게 정의하고 만들어보세요."}</p>
	</header>

	<form class="character-form" onsubmit={handleSubmit}>
		<div class="form-section top-section">
			<div class="avatar-upload-wrapper">
				<div class="avatar-preview">
					{#if avatarUrl}
						<img src={avatarUrl} alt="캐릭터 아바타 미리보기" />
					{:else}
						<div class="avatar-placeholder">
							<svg
								width="24"
								height="24"
								viewBox="0 0 24 24"
								fill="none"
								stroke="currentColor"
								stroke-width="2"
							>
								<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"></path>
								<circle cx="12" cy="7" r="4"></circle>
							</svg>
						</div>
					{/if}
				</div>
				<label class="upload-btn">
					아바타 선택
					<input type="file" accept="image/*" onchange={handleImageUpload} hidden />
				</label>
			</div>

			<div class="input-group flex-1">
				<label for="name">캐릭터 이름</label>
				<input id="name" type="text" bind:value={name} placeholder="예: Detective Irons" required />
			</div>
		</div>

		<div class="input-group">
			<label for="description">간단한 설명 (Short Description)</label>
			<input
				id="description"
				type="text"
				bind:value={description}
				placeholder="카드에 표시될 짧은 소개를 입력하세요."
			/>
		</div>

		<div class="input-group">
			<label for="personality">성격 및 세부 설정 (Personality / Prompt)</label>
			<textarea
				id="personality"
				bind:value={personality}
				rows="5"
				placeholder="캐릭터의 성격, 말투, 배경 이야기 등을 상세히 적어주세요."
			></textarea>
		</div>

		<div class="input-group">
			<label for="firstMessage">첫 메시지 (First Message)</label>
			<textarea
				id="firstMessage"
				bind:value={firstMessage}
				rows="3"
				placeholder="채팅이 시작될 때 캐릭터가 가장 먼저 건넬 대화를 입력하세요."
			></textarea>
		</div>

		<div class="form-actions">
			<button type="button" class="btn-cancel" onclick={() => handleCancel()}>취소</button>
			<button type="submit" class="btn-submit">
				{isEdit ? "저장하기" : "캐릭터 생성"}
			</button>
		</div>
	</form>
</div>

<style>
	.character-form-container {
		background-color: #18181b;
		color: #f4f4f5;
		padding: 32px;
		border-radius: 12px;
		max-width: 720px;
		margin: 0 auto;
		box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
		border: 1px solid #27272a;
		font-family: inherit;
	}

	.form-header {
		margin-bottom: 24px;
	}

	.form-header h2 {
		font-size: 1.5rem;
		font-weight: 600;
		margin: 0 0 8px 0;
	}

	.form-header p {
		color: #a1a1aa;
		font-size: 0.875rem;
		margin: 0;
	}

	.character-form {
		display: flex;
		flex-direction: column;
		gap: 20px;
	}

	.top-section {
		display: flex;
		gap: 20px;
		align-items: center;
	}

	.flex-1 {
		flex: 1;
	}

	.avatar-upload-wrapper {
		display: flex;
		flex-direction: column;
		align-items: center;
		gap: 10px;
	}

	.avatar-preview {
		width: 80px;
		height: 80px;
		border-radius: 50%;
		background-color: #27272a;
		border: 2px dashed #3f3f46;
		overflow: hidden;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.avatar-preview img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.avatar-placeholder {
		color: #71717a;
	}

	.upload-btn {
		background-color: #27272a;
		color: #d4d4d8;
		padding: 6px 12px;
		border-radius: 6px;
		font-size: 0.75rem;
		cursor: pointer;
		border: 1px solid #3f3f46;
		transition: background 0.2s;
	}

	.upload-btn:hover {
		background-color: #3f3f46;
	}

	.input-group {
		display: flex;
		flex-direction: column;
		gap: 8px;
	}

	.input-group label {
		font-size: 0.875rem;
		font-weight: 500;
		color: #d4d4d8;
	}

	.input-group input,
	.input-group textarea {
		background-color: #09090b;
		border: 1px solid #27272a;
		border-radius: 8px;
		padding: 10px 14px;
		color: #f4f4f5;
		font-size: 0.875rem;
		outline: none;
		transition: border-color 0.2s;
	}

	.input-group input:focus,
	.input-group textarea:focus {
		border-color: #8b5cf6; /* 스크린샷의 보라색 포인트 컬러 연상 */
	}

	.input-group textarea {
		resize: vertical;
	}

	.form-actions {
		display: flex;
		justify-content: flex-end;
		gap: 12px;
		margin-top: 12px;
	}

	.btn-cancel,
	.btn-submit {
		padding: 10px 20px;
		border-radius: 8px;
		font-size: 0.875rem;
		font-weight: 500;
		cursor: pointer;
		border: none;
	}

	.btn-cancel {
		background-color: transparent;
		color: #a1a1aa;
		border: 1px solid #27272a;
	}

	.btn-cancel:hover {
		background-color: #27272a;
		color: #f4f4f5;
	}

	.btn-submit {
		background-color: #7c3aed; /* 메인 포인트 보라색 */
		color: white;
	}

	.btn-submit:hover {
		background-color: #6d28d9;
	}
</style>
