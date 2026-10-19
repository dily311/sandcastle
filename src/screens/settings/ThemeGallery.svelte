<script lang="ts">
	import type { CssTheme } from "$/lib/type/settingTypes";
	import { themeSetting } from "$/lib/stores/themeStore.svelte";

	// 모달 상태 관리
	let isModalOpen = false;
	let modalMode: "add" | "edit" = "add";

	// 폼 데이터 상태
	let editingId = "";
	let themeForm: Omit<CssTheme, "filename"> = {
		name: "",
		author: "",
		version: "1.0.0",
		previewUrl: "",
		cssContent: "",
	};

	// 새 테마 추가 모달 열기
	function openAddModal() {
		modalMode = "add";
		editingId = "";
		themeForm = { name: "", author: "", version: "1.0.0", previewUrl: "", cssContent: "" };
		isModalOpen = true;
	}

	// 기존 테마 수정 모달 열기
	async function openEditModal(theme: CssTheme, event: Event) {
		event.stopPropagation(); // 카드 클릭(테마 적용) 이벤트 전파 방지
		modalMode = "edit";
		editingId = theme.filename;
		const cssContent = await themeSetting.loadTheme(editingId);
		themeForm = {
			name: theme.name,
			author: theme.author || "",
			version: theme.version,
			previewUrl: theme.previewUrl || "",
			cssContent: cssContent,
		};
		isModalOpen = true;
	}

	// 저장 (추가 및 수정 공용)
	async function handleSaveTheme(e: MouseEvent) {
		if (!themeForm.name.trim()) return;

		await themeSetting.saveTheme(modalMode, modalMode == "add" ? `${themeForm.name}.json` : editingId, themeForm);
		isModalOpen = false;
	}

	// 삭제
	async function handleDeleteTheme(id: string, event: Event) {
		event.stopPropagation();
		if (!confirm("이 테마를 삭제하시겠습니까?")) return;

		await themeSetting.deleteTheme(id);
	}

	function closeModal(e: MouseEvent) {
		e.stopPropagation();
		if (e.target === e.currentTarget) {
			isModalOpen = false;
		}
	}

	// 이미지 파일 선택 시 DataURL 변환 처리
	function handleImageUpload(e: Event) {
		const target = e.target as HTMLInputElement;
		if (target.files && target.files[0]) {
			const reader = new FileReader();
			reader.onload = (event) => {
				themeForm.previewUrl = event.target?.result as string;
			};
			reader.readAsDataURL(target.files[0]);
		}
	}
</script>

<div class="theme-gallery-container">
	<header class="gallery-header">
		<div>
			<h3 class="title">테마 갤러리</h3>
			<p class="subtitle">적용할 커스텀 테마를 선택하거나 새로 만드세요.</p>
		</div>
		<button class="btn btn-primary" onclick={() => openAddModal()}>
			<span class="icon">+</span> 새 테마 추가
		</button>
	</header>

	<!-- Theme Cards Grid Gallery -->
	<div class="gallery-grid">
		<!-- 1. 기본 테마 카드 -->
		<!-- svelte-ignore a11y_click_events_have_key_events -->
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div
			class="theme-card"
			class:selected={themeSetting.selectTheme === "default"}
			onclick={() => themeSetting.update({ selectTheme: "default" })}
		>
			<div class="preview-box default-preview">
				<span class="preview-placeholder">System Default</span>
			</div>
			<div class="card-info">
				<div class="card-title-wrap">
					<span class="theme-name">기본 테마</span>
				</div>
				<p class="card-desc">시스템 기본 스타일입니다.</p>
			</div>
		</div>

		<!-- 2. 커스텀 테마 카드 목록 -->
		{#each themeSetting.cssThemeValue as theme}
			<!-- svelte-ignore a11y_click_events_have_key_events -->
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="theme-card"
				class:selected={themeSetting.selectTheme === theme.filename}
				onclick={() => themeSetting.update({ selectTheme: theme.filename })}
			>
				<div class="preview-box">
					{#if theme.previewUrl}
						<img src={theme.previewUrl} alt={theme.name} class="preview-img" />
					{:else}
						<div class="no-preview">No Preview</div>
					{/if}

					<!-- Hover Action Overlay -->
					<div class="card-overlay">
						<button class="action-btn" onclick={(e) => openEditModal(theme, e)}>수정</button>
						<button class="action-btn danger" onclick={(e) => handleDeleteTheme(theme.filename, e)}
							>삭제</button
						>
					</div>
				</div>

				<div class="card-info">
					<div class="card-title-wrap">
						<span class="theme-name">{theme.name}</span>
						<span class="version-tag">v{theme.version}</span>
					</div>
				</div>
			</div>
		{/each}
	</div>
</div>

<!-- 테마 추가 / 수정 모달 -->
{#if isModalOpen}
	<div class="modal-backdrop" onclick={(e) => closeModal(e)} role="presentation">
		<div class="modal-card">
			<header class="modal-header">
				<h3>{modalMode === "add" ? "새 테마 추가" : "테마 수정"}</h3>
				<button class="close-btn" onclick={(e) => closeModal(e)}>&times;</button>
			</header>

			<div class="modal-body">
				<div class="form-row">
					<div class="form-group flex-2">
						<label for="themeName">테마 이름</label>
						<input
							id="themeName"
							type="text"
							class="custom-input"
							bind:value={themeForm.name}
							placeholder="예: Neon Night"
						/>
					</div>
					<div class="form-group flex-1">
						<label for="themeVersion">버전</label>
						<input
							id="themeVersion"
							type="text"
							class="custom-input"
							bind:value={themeForm.version}
							placeholder="1.0.0"
						/>
					</div>
				</div>

				<div class="form-group">
					<label for="previewImg">미리보기 이미지</label>
					<div class="image-input-group">
						<input id="previewImg" type="file" accept="image/*" onchange={handleImageUpload} />
						{#if themeForm.previewUrl}
							<div class="img-preview-mini">
								<img src={themeForm.previewUrl} alt="미리보기" />
							</div>
						{/if}
					</div>
				</div>

				<div class="form-group">
					<label for="cssContent">CSS 코드</label>
					<textarea
						id="cssContent"
						class="custom-textarea"
						placeholder=":root &#123; --accent-color: #10b981; &#125;"
						bind:value={themeForm.cssContent}
					></textarea>
				</div>
			</div>

			<footer class="modal-footer">
				<button class="btn btn-secondary" onclick={(e) => closeModal(e)}>취소</button>
				<button class="btn btn-primary" onclick={(e) => handleSaveTheme(e)} disabled={!themeForm.name.trim()}>
					{modalMode === "add" ? "추가" : "저장"}
				</button>
			</footer>
		</div>
	</div>
{/if}

<style>
	.theme-gallery-container {
		display: flex;
		flex-direction: column;
		gap: 1.25rem;
	}

	.gallery-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.gallery-header .title {
		font-size: 1.1rem;
		font-weight: 600;
		margin: 0;
	}

	.gallery-header .subtitle {
		font-size: 0.8rem;
		color: var(--text-muted, #6b7280);
		margin: 0.2rem 0 0 0;
	}

	/* Gallery Grid */
	.gallery-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
		gap: 1rem;
	}

	/* Theme Card UI */
	.theme-card {
		border: 2px solid var(--border-color, #e5e7eb);
		border-radius: 10px;
		overflow: hidden;
		background: var(--card-bg, #fff);
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		flex-direction: column;
	}

	.theme-card:hover {
		border-color: var(--border-hover, #a5f3fc);
		transform: translateY(-2px);
		box-shadow: 0 4px 12px rgba(0, 0, 0, 0.08);
	}

	.theme-card.selected {
		border-color: var(--accent-color, #2563eb);
		box-shadow: 0 0 0 2px var(--accent-color, #2563eb);
	}

	/* Preview Frame & Overlay */
	.preview-box {
		position: relative;
		height: 120px;
		background-color: #f3f4f6;
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}

	.preview-img {
		width: 100%;
		height: 100%;
		object-fit: cover;
	}

	.default-preview {
		background: linear-gradient(135deg, #e0e7ff 0%, #fae8ff 100%);
	}

	.preview-placeholder,
	.no-preview {
		font-size: 0.8rem;
		color: #9ca3af;
		font-weight: 500;
	}

	.card-overlay {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		background: rgba(0, 0, 0, 0.5);
		display: flex;
		align-items: center;
		justify-content: center;
		gap: 0.5rem;
		opacity: 0;
		transition: opacity 0.2s ease;
	}

	.theme-card:hover .card-overlay {
		opacity: 1;
	}

	.action-btn {
		padding: 0.35rem 0.75rem;
		border-radius: 4px;
		border: none;
		background: #ffffff;
		font-size: 0.75rem;
		font-weight: 600;
		cursor: pointer;
	}

	.action-btn.danger {
		background: var(--color-red);
		color: #fff;
	}

	/* Card Details */
	.card-info {
		padding: 0.75rem;
	}

	.card-title-wrap {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.theme-name {
		font-size: 0.9rem;
		font-weight: 600;
	}

	.version-tag {
		font-size: 0.75rem;
		background-color: var(--badge-bg, #f3f4f6);
		color: var(--text-muted, #6b7280);
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
	}

	.card-desc {
		font-size: 0.75rem;
		color: #9ca3af;
		margin: 0.25rem 0 0 0;
	}

	/* Modal UI */
	.modal-backdrop {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: rgba(0, 0, 0, 0.4);
		display: flex;
		align-items: center;
		justify-content: center;
		z-index: 999;
	}

	.modal-card {
		background: #fff;
		width: 90%;
		max-width: 480px;
		border-radius: 12px;
		padding: 1.25rem;
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}

	.modal-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.modal-header h3 {
		margin: 0;
		font-size: 1.1rem;
	}

	.close-btn {
		background: none;
		border: none;
		font-size: 1.4rem;
		cursor: pointer;
	}

	.modal-body {
		display: flex;
		flex-direction: column;
		gap: 0.85rem;
	}

	.form-row {
		display: flex;
		gap: 0.5rem;
	}
	.flex-1 {
		flex: 1;
	}
	.flex-2 {
		flex: 2;
	}

	.form-group {
		display: flex;
		flex-direction: column;
		gap: 0.35rem;
	}

	.form-group label {
		font-size: 0.8rem;
		font-weight: 500;
		color: #374151;
	}

	.custom-input,
	.custom-textarea {
		padding: 0.5rem 0.75rem;
		border-radius: 6px;
		border: 1px solid #d1d5db;
		font-size: 0.85rem;
	}

	.custom-textarea {
		resize: vertical;
		overflow-y: auto;
		min-height: 90px;
		font-family: monospace;
	}

	.image-input-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.img-preview-mini img {
		width: 36px;
		height: 36px;
		object-fit: cover;
		border-radius: 4px;
		border: 1px solid #d1d5db;
	}

	.modal-footer {
		display: flex;
		justify-content: flex-end;
		gap: 0.5rem;
	}

	.btn {
		padding: 0.5rem 0.9rem;
		border-radius: 6px;
		font-size: 0.85rem;
		font-weight: 500;
		cursor: pointer;
		border: none;
	}

	.btn-primary {
		background: #2563eb;
		color: #fff;
	}
	.btn-secondary {
		background: #f3f4f6;
		color: #374151;
	}
</style>
