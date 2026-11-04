<script lang="ts">
	import type { GameSceneInput } from "$/lib/type/game";

	type Props = {
		scenes: GameSceneInput[];
		gameId: string;
	};
	let { scenes = $bindable([]), gameId }: Props = $props();

	let selectedIndex = $state<number | null>(scenes.length > 0 ? 0 : null);

	let activeScene = $derived(selectedIndex !== null && scenes[selectedIndex] ? scenes[selectedIndex] : null);

	function handleAddScene() {
		const newScene: GameSceneInput = {
			id: crypto.randomUUID(), // 고유 ID 생성 (UUID v4)
			gameId: gameId,
			sceneName: "새로운 씬",
			description: "",
			sceneKey: `scene_${Date.now()}`,
			settingsJson: null,
			isEnding: false,
			sortOrder: scenes.length,
			createdAt: Date.now(),
			updatedAt: Date.now(),
		};
		scenes.push(newScene);
		selectedIndex = scenes.length - 1;
	}

	function handleDeleteScene(index: number) {
		if (!confirm(`"${scenes[index].sceneName}" 씬을 삭제하시겠습니까?`)) return;
		scenes.splice(index, 1);
		selectedIndex = scenes.length > 0 ? Math.max(0, index - 1) : null;
	}
</script>

<div class="sub-editor-layout">
	<!-- 좌측: 씬 목록 -->
	<div class="item-list-pane">
		<div class="pane-header">
			<h4>씬 목록 ({scenes.length})</h4>
			<button class="add-btn" onclick={handleAddScene}>+ 씬 추가</button>
		</div>
		<div class="scroll-list">
			{#each scenes as scene, index (scene.id)}
				<!-- svelte-ignore a11y_click_events_have_key_events -->
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div class="list-item" class:active={selectedIndex === index} onclick={() => (selectedIndex = index)}>
					<div class="item-info">
						<span class="title">{scene.sceneName}</span>
						<span class="key"><code>{scene.sceneKey}</code></span>
					</div>
					{#if scene.isEnding}
						<span class="badge ending">엔딩</span>
					{/if}
				</div>
			{:else}
				<div class="empty-text">등록된 씬이 없습니다.</div>
			{/each}
		</div>
	</div>

	<!-- 우측: 선택된 씬 상세 편집 폼 -->
	<div class="item-detail-pane">
		{#if activeScene && selectedIndex !== null}
			<div class="detail-header">
				<h3>씬 상세 편집</h3>
				<button class="delete-btn" onclick={() => handleDeleteScene(selectedIndex!)}>삭제</button>
			</div>
			<div class="form-grid">
				<div class="field">
					<label for="scene_name">씬 이름 (Scene Name)</label>
					<input id="scene_name" type="text" bind:value={activeScene.sceneName} />
				</div>
				<div class="field">
					<label for="scene_key">씬 고유 키 (Scene Key)</label>
					<input id="scene_key" type="text" bind:value={activeScene.sceneKey} />
				</div>
				<div class="field">
					<label for="scene_des">설명 (Description)</label>
					<textarea id="scene_des" bind:value={activeScene.description} rows="4"></textarea>
				</div>
				<div class="field-row">
					<label class="checkbox-label">
						<input type="checkbox" bind:checked={activeScene.isEnding} />
						<span>엔딩 씬 여부 (Is Ending)</span>
					</label>
				</div>
				<div class="field">
					<label for="scene_sort">정렬 순서 (Sort Order)</label>
					<input id="scene_sort" type="number" bind:value={activeScene.sortOrder} />
				</div>
			</div>
		{:else}
			<div class="no-selection">편집할 씬을 선택하거나 새로 추가해주세요.</div>
		{/if}
	</div>
</div>

<style>
	.sub-editor-layout {
		display: grid;
		grid-template-columns: 320px 1fr;
		gap: 1.5rem;
		height: 100%;
		min-height: 500px;
	}
	.item-list-pane,
	.item-detail-pane {
		background: #1e293b;
		border: 1px solid #334155;
		border-radius: 8px;
		display: flex;
		flex-direction: column;
		overflow: hidden;
	}

	.pane-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		border-bottom: 1px solid #334155;
		background: #0f172a;
	}
	.pane-header h4 {
		margin: 0;
		font-size: 0.95rem;
		color: #f8fafc;
	}
	.add-btn {
		background: #3b82f6;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.add-btn:hover {
		background: #2563eb;
	}

	.scroll-list {
		flex: 1;
		overflow-y: auto;
		padding: 0.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.list-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem 1rem;
		background: #0f172a;
		border-radius: 6px;
		cursor: pointer;
		border: 1px solid transparent;
		transition: all 0.15s;
	}
	.list-item:hover {
		border-color: #475569;
	}
	.list-item.active {
		border-color: #38bdf8;
		background: #172554;
	}

	.item-info {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		overflow: hidden;
	}
	.item-info .title {
		font-weight: 600;
		font-size: 0.9rem;
		color: #f8fafc;
	}
	.item-info .key {
		font-size: 0.75rem;
		color: #94a3b8;
	}
	.badge.ending {
		background: #84cc16;
		color: #0f172a;
		font-size: 0.65rem;
		font-weight: 700;
		padding: 0.1rem 0.4rem;
		border-radius: 4px;
	}
	.empty-text {
		text-align: center;
		color: #64748b;
		padding: 2rem;
		font-size: 0.85rem;
	}

	.item-detail-pane {
		padding: 1.5rem;
		overflow-y: auto;
	}
	.detail-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1.2rem;
		border-bottom: 1px solid #334155;
		padding-bottom: 0.8rem;
	}
	.detail-header h3 {
		margin: 0;
		font-size: 1.1rem;
		color: #f8fafc;
	}
	.delete-btn {
		background: #ef4444;
		color: white;
		border: none;
		padding: 0.4rem 0.8rem;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
	}
	.delete-btn:hover {
		background: #dc2626;
	}

	.form-grid {
		display: flex;
		flex-direction: column;
		gap: 1rem;
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: 0.4rem;
	}
	.field label {
		font-size: 0.85rem;
		font-weight: 600;
		color: #94a3b8;
	}
	.field input,
	.field textarea {
		background: #0f172a;
		border: 1px solid #334155;
		border-radius: 6px;
		padding: 0.6rem;
		color: #e2e8f0;
		font-size: 0.95rem;
	}
	.field input:focus,
	.field textarea:focus {
		outline: none;
		border-color: #38bdf8;
	}
	.field-row {
		display: flex;
		align-items: center;
		margin-top: 0.2rem;
	}
	.checkbox-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		cursor: pointer;
		font-size: 0.9rem;
		color: #e2e8f0;
	}
	.no-selection {
		display: flex;
		align-items: center;
		justify-content: center;
		height: 100%;
		color: #64748b;
		font-size: 0.95rem;
		text-align: center;
	}
</style>
